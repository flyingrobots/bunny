//! `cargo xtask perf`: before/after timing of Bunny's kernels across two revisions.
//!
//! The current `crates/bunny-perf` harness is built twice: once against this working tree
//! and once against the base revision, checked out in a temporary worktree under
//! `target/perf/`. Both binaries run interleaved on the same machine, and the report
//! compares medians, so machine-wide noise affects both sides alike. Harness feature
//! groups the base revision cannot compile (APIs it does not have yet) are left out of the
//! base build and reported as new. See `docs/topics/performance/README.md`.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git_helpers::git_command;
use crate::perf_evidence::{evidence_text, host_triple, is_dirty, BuildIdentity, Measured, Side};
use crate::perf_report::{compare, regressions, render, Heading, Row};

const DEFAULT_BASE: &str = "origin/main";
const DEFAULT_ROUNDS: usize = 5;
/// The crates the harness may depend on. A crate missing from the base revision is left out
/// of that side's manifest, and the groups that need it are reported as new.
const BASE_CRATES: &[&str] =
    &["bunny-broadphase", "bunny-geom", "bunny-linalg", "bunny-num", "bunny-optics", "bunny-query"];

struct PerfOptions {
    base: String,
    rounds: usize,
    max_regression: Option<f64>,
    summary: Option<PathBuf>,
    harness_args: Vec<String>,
    align_code: bool,
    /// A Rust target triple to build the harness for; the host when absent.
    target: Option<String>,
    /// A command that runs the harness binary (for example `wasmtime`); run directly when absent.
    runner: Option<String>,
    /// Where to write the per-round evidence file (`bunny-perf-evidence/1`); none when absent.
    evidence: Option<PathBuf>,
}

impl PerfOptions {
    /// The target being measured, for report headings.
    fn platform(&self) -> String {
        self.target.clone().unwrap_or_else(|| {
            format!("{}-{} (host)", std::env::consts::ARCH, std::env::consts::OS)
        })
    }
}

impl PerfOptions {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            base: DEFAULT_BASE.to_owned(),
            rounds: DEFAULT_ROUNDS,
            max_regression: None,
            summary: None,
            harness_args: Vec::new(),
            align_code: false,
            target: None,
            runner: None,
            evidence: None,
        };
        while let Some(flag) = args.next() {
            if flag == "--align-code" {
                options.align_code = true;
                continue;
            }
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--base" => options.base = value,
                "--rounds" => options.rounds = value.parse::<usize>()?.max(1),
                "--max-regression" => options.max_regression = Some(value.parse()?),
                "--summary" => options.summary = Some(PathBuf::from(value)),
                "--target" => options.target = Some(value),
                "--runner" => options.runner = Some(value),
                "--evidence" => options.evidence = Some(PathBuf::from(value)),
                "--filter" | "--samples" => options.harness_args.extend([flag, value]),
                other => return Err(format!("unknown perf option {other}").into()),
            }
        }
        Ok(options)
    }
}

/// Runs the comparison and fails when a case regressed past `--max-regression`.
pub(crate) fn handle(args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let options = PerfOptions::parse(args)?;
    let root = workspace_root();
    let sha = resolve(&root, &options.base)?;
    let short = sha.get(..12).unwrap_or(&sha).to_owned();
    let perf_dir = root.join("target").join("perf");
    // Taken before anything is built, so it describes the tree that was measured.
    let evidence = match &options.evidence {
        Some(path) => Some((path, build_identity(&root, &options, &sha, path)?)),
        None => None,
    };

    // Both sides build the same way (a standalone copy of the harness), so codegen differences
    // between a workspace build and a standalone build cannot bias the comparison.
    eprintln!("perf: building the harness for the working tree");
    let head_bin = build_harness(&root, &root, &perf_dir.join("harness-head"), &options)?;
    eprintln!("perf: building the harness for {} ({short})", options.base);
    let base_tree = perf_dir.join(format!("base-{short}"));
    add_worktree(&root, &base_tree, &sha)?;
    let base_bin =
        build_harness(&root, &base_tree, &perf_dir.join(format!("harness-{short}")), &options);
    remove_worktree(&root, &base_tree);
    let base_bin = base_bin?;

    let measured = run_interleaved(&base_bin, &head_bin, &options)?;
    let platform = options.platform();
    let scope =
        ReportScope { base: &options.base, sha: &sha, platform: &platform, rounds: options.rounds };
    let (rows, table) = report(&measured, &scope);
    println!("{table}");
    if let Some(path) = &options.summary {
        fs::OpenOptions::new().create(true).append(true).open(path)?.write_all(table.as_bytes())?;
    }
    if let Some((path, identity)) = &evidence {
        let text = evidence_text(identity, &measured.rounds)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(text.as_bytes())?;
        eprintln!("perf: wrote the evidence file {}", path.display());
    }
    check_threshold(&rows, options.max_regression)
}

/// What a report compares: the base revision as given and as resolved, and the target.
pub(crate) struct ReportScope<'a> {
    pub(crate) base: &'a str,
    /// The full SHA the base resolved to; the heading shows its first 12 digits.
    pub(crate) sha: &'a str,
    pub(crate) platform: &'a str,
    pub(crate) rounds: usize,
}

/// The rows and the Markdown table for the measured rounds.
pub(crate) fn report(measured: &Measured, scope: &ReportScope<'_>) -> (Vec<Row>, String) {
    let rows = compare(&measured.base, &measured.candidate);
    let short = scope.sha.get(..12).unwrap_or(scope.sha);
    let base_label = format!("{} ({short})", scope.base);
    let heading = Heading {
        base: &base_label,
        head: "working tree",
        platform: scope.platform,
        rounds: scope.rounds,
    };
    let table = render(&rows, &heading);
    (rows, table)
}

/// The build identity an evidence file records. Refuses an evidence path that already exists,
/// before anything is built, so a ledger's evidence is never overwritten.
fn build_identity(
    root: &Path,
    options: &PerfOptions,
    sha: &str,
    path: &Path,
) -> Result<BuildIdentity, Box<dyn Error>> {
    refuse_existing(path)?;
    let (target, target_is_host) = match &options.target {
        Some(triple) => (triple.clone(), false),
        None => (host_target(root)?, true),
    };
    let status = git_stdout(root, &["status", "--porcelain", "--untracked-files=normal"])?;
    Ok(BuildIdentity {
        base_ref: options.base.clone(),
        base_revision: sha.to_owned(),
        candidate_revision: git_stdout(root, &["rev-parse", "--verify", "HEAD^{commit}"])?
            .trim()
            .to_owned(),
        candidate_dirty: is_dirty(&status),
        target,
        target_is_host,
        platform: options.platform(),
        rounds: options.rounds,
        align_code: options.align_code,
        rustflags: rustflags(options.align_code),
        harness_args: options.harness_args.clone(),
        runner: options.runner.clone(),
        max_regression: options.max_regression,
    })
}

fn refuse_existing(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(format!("the evidence file {} already exists", path.display()).into());
    }
    Ok(())
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = git_command(root).args(args).output()?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// The host triple of the toolchain that builds the harness, from `rustc -vV`.
fn host_target(root: &Path) -> Result<String, Box<dyn Error>> {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc).current_dir(root).arg("-vV").output()?;
    if !output.status.success() {
        return Err("rustc -vV failed".into());
    }
    host_triple(&String::from_utf8(output.stdout)?)
        .ok_or_else(|| "rustc -vV printed no host line".into())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

/// The harness binary's file name for `target` (the host when absent).
fn binary_name(target: Option<&str>) -> String {
    let suffix = match target {
        None => std::env::consts::EXE_SUFFIX,
        Some(triple) if triple.starts_with("wasm") => ".wasm",
        Some(triple) if triple.contains("windows") => ".exe",
        Some(_) => "",
    };
    format!("bunny-perf{suffix}")
}

fn resolve(root: &Path, rev: &str) -> Result<String, Box<dyn Error>> {
    let output =
        git_command(root).args(["rev-parse", "--verify", &format!("{rev}^{{commit}}")]).output()?;
    if !output.status.success() {
        return Err(format!("cannot resolve base revision {rev:?}; fetch it first").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn add_worktree(root: &Path, path: &Path, sha: &str) -> Result<(), Box<dyn Error>> {
    remove_worktree(root, path);
    let status = git_command(root)
        .args(["worktree", "add", "--detach", "--quiet"])
        .arg(path)
        .arg(sha)
        .status()?;
    if !status.success() {
        return Err(format!("git worktree add {} failed", path.display()).into());
    }
    Ok(())
}

/// Removes a worktree this command created. The harness builds outside it, so it stays clean.
fn remove_worktree(root: &Path, path: &Path) {
    if path.exists() {
        let _ = git_command(root).args(["worktree", "remove"]).arg(path).status();
    }
    let _ = git_command(root).args(["worktree", "prune"]).status();
}

/// Builds the working tree's harness source against the crates in `tree`.
fn build_harness(
    root: &Path,
    tree: &Path,
    harness: &Path,
    options: &PerfOptions,
) -> Result<PathBuf, Box<dyn Error>> {
    let source = root.join("crates").join("bunny-perf");
    let groups = feature_groups(&fs::read_to_string(source.join("Cargo.toml"))?);
    fs::create_dir_all(harness.join("src"))?;
    for entry in fs::read_dir(source.join("src"))? {
        let entry = entry?;
        fs::copy(entry.path(), harness.join("src").join(entry.file_name()))?;
    }
    fs::copy(root.join("rust-toolchain.toml"), harness.join("rust-toolchain.toml"))?;
    fs::copy(tree.join("Cargo.lock"), harness.join("Cargo.lock"))?;
    fs::write(harness.join("Cargo.toml"), harness_manifest(tree, &groups, &present_crates(tree)))?;

    let built = |features: &[&str]| -> Result<bool, Box<dyn Error>> {
        let status = cargo()
            .current_dir(harness)
            .env("RUSTFLAGS", rustflags(options.align_code))
            .args([
                "build",
                "--release",
                "--quiet",
                "--no-default-features",
                "--features",
                &features.join(","),
            ])
            .args(options.target.iter().flat_map(|target| ["--target", target.as_str()]))
            // An inherited CARGO_TARGET_DIR would send both sides' builds to one directory.
            .arg("--target-dir")
            .arg(harness.join("target"))
            .status()?;
        Ok(status.success())
    };
    let all: Vec<&str> = groups.iter().map(String::as_str).collect();
    if !built(&all)? {
        // Keep only the groups whose APIs exist at the base revision.
        let mut compatible = Vec::new();
        for group in &all {
            if built(&[group])? {
                compatible.push(*group);
            }
        }
        eprintln!(
            "perf: {} builds groups {compatible:?} of {all:?}; the rest are reported as new",
            tree.display()
        );
        if compatible.is_empty() || !built(&compatible)? {
            return Err(format!("the harness does not build against {}", tree.display()).into());
        }
    }
    let output = harness.join("target");
    let output = options.target.as_ref().map_or(output.clone(), |target| output.join(target));
    Ok(output.join("release").join(binary_name(options.target.as_deref())))
}

/// Compiler flags for both harness builds. With `align_code`, every function and branch
/// target starts on a 64-byte boundary, so a change elsewhere in the binary cannot move a
/// hot loop across a cache-line or fetch boundary and fake a speedup or a regression.
fn rustflags(align_code: bool) -> String {
    let inherited = std::env::var("RUSTFLAGS").unwrap_or_default();
    if !align_code {
        return inherited;
    }
    let aligned = "-C llvm-args=-align-all-functions=6 -C llvm-args=-align-all-nofallthru-blocks=6";
    if inherited.trim().is_empty() {
        aligned.to_owned()
    } else {
        format!("{inherited} {aligned}")
    }
}

/// The feature groups declared in the harness manifest (everything but `default`).
pub(crate) fn feature_groups(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[features]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim().to_owned()))
        .filter(|name| !name.is_empty() && !name.starts_with('#') && name != "default")
        .collect()
}

/// The harness crates that exist in `tree`; an older revision may lack a newer crate.
fn present_crates(tree: &Path) -> Vec<&'static str> {
    BASE_CRATES
        .iter()
        .copied()
        .filter(|name| tree.join("crates").join(name).join("Cargo.toml").is_file())
        .collect()
}

fn harness_manifest(tree: &Path, groups: &[String], crates: &[&str]) -> String {
    let mut manifest = String::from(
        "[package]\nname = \"bunny-perf\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
         [workspace]\n\n[[bin]]\nname = \"bunny-perf\"\npath = \"src/main.rs\"\n\n[features]\n",
    );
    for group in groups {
        let _ = writeln!(manifest, "{group} = []");
    }
    manifest.push_str("\n[dependencies]\n");
    for name in crates {
        let path = toml_path(&tree.join("crates").join(name));
        let _ = writeln!(manifest, "{name} = {{ path = \"{path}\" }}");
    }
    manifest
}

/// A path as the body of a TOML basic string. Forward slashes work on every platform Cargo
/// supports, so Windows backslashes need no escaping; quotes are escaped.
fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/").replace('"', "\\\"")
}

fn run_interleaved(
    base: &Path,
    head: &Path,
    options: &PerfOptions,
) -> Result<Measured, Box<dyn Error>> {
    let mut measured = Measured::default();
    for round in 1..=options.rounds {
        eprintln!("perf: round {round} of {}", options.rounds);
        // Alternate which side runs first, so warm-up or thermal drift favours neither.
        let base_output;
        let head_output;
        if round % 2 == 1 {
            base_output = run_harness(base, options)?;
            head_output = run_harness(head, options)?;
        } else {
            head_output = run_harness(head, options)?;
            base_output = run_harness(base, options)?;
        }
        measured.record(Side::Base, round, base_output)?;
        measured.record(Side::Candidate, round, head_output)?;
    }
    Ok(measured)
}

fn run_harness(binary: &Path, options: &PerfOptions) -> Result<String, Box<dyn Error>> {
    let mut command = match options.runner.as_deref() {
        // A runner such as `wasmtime` takes the binary as its first argument.
        Some(runner) => {
            let mut words = runner.split_whitespace();
            let mut command = Command::new(words.next().ok_or("--runner is empty")?);
            command.args(words).arg(binary);
            command
        }
        None => Command::new(binary),
    };
    let output = command.args(&options.harness_args).output()?;
    if !output.status.success() {
        return Err(format!(
            "{} failed: {}",
            binary.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn check_threshold(rows: &[Row], max_regression: Option<f64>) -> Result<(), Box<dyn Error>> {
    if rows.iter().all(|row| row.base.is_none() || row.head.is_none()) {
        return Err("no case was measured on both sides; check --filter".into());
    }
    let Some(limit) = max_regression else {
        return Ok(());
    };
    let slow = regressions(rows, limit);
    if slow.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = slow
        .iter()
        .map(|row| {
            format!(
                "{} ({:+.1}% floor, {:+.1}% median)",
                row.case,
                row.floor_change_percent().unwrap_or_default(),
                row.change_percent().unwrap_or_default()
            )
        })
        .collect();
    Err(format!("floor and median p50 both regressed past {limit}%: {}", names.join(", ")).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_groups_skip_default_and_comments() {
        let manifest = concat!(
            "[package]\nname = \"x\"\n\n[features]\ndefault = [\"num\"]\n# note\nnum = []\nangle = []\n",
            "\n[dependencies]\nfoo = \"1\"\n",
        );
        assert_eq!(feature_groups(manifest), ["num", "angle"]);
    }

    #[test]
    fn harness_manifest_points_at_the_given_crates() {
        let manifest = harness_manifest(Path::new("/tmp/base"), &["num".to_owned()], BASE_CRATES);
        // Built with Path::join, so this also holds on Windows, where join adds backslashes.
        assert!(manifest.contains("num = []\n"));
        assert!(manifest.contains("bunny-num = { path = \"/tmp/base/crates/bunny-num\" }"));
        assert!(manifest.contains("[workspace]"));
    }

    #[test]
    fn harness_manifest_includes_every_crate_the_cases_use() {
        let manifest = harness_manifest(Path::new("/tmp/base"), &["scene".to_owned()], BASE_CRATES);
        for name in [
            "bunny-broadphase",
            "bunny-geom",
            "bunny-linalg",
            "bunny-num",
            "bunny-optics",
            "bunny-query",
        ] {
            assert!(manifest.contains(&format!("{name} = {{ path = ")), "{name} is missing");
        }
    }

    #[test]
    fn harness_manifest_leaves_out_crates_the_base_lacks() {
        // This repository has every harness crate; a directory without them has none.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("workspace root");
        assert_eq!(present_crates(root), BASE_CRATES);
        assert!(present_crates(&root.join("xtask")).is_empty());
        let older = ["bunny-num", "bunny-query"];
        let manifest = harness_manifest(Path::new("/tmp/base"), &["num".to_owned()], &older);
        assert!(!manifest.contains("bunny-optics"));
        assert!(manifest.contains("bunny-query = { path = "));
    }

    #[test]
    fn options_forward_harness_flags() {
        let args = [
            "--base",
            "v0.6.0",
            "--rounds",
            "3",
            "--align-code",
            "--filter",
            "num/",
            "--max-regression",
            "15",
        ];
        let options =
            PerfOptions::parse(args.iter().map(|s| (*s).to_owned())).expect("valid options");
        assert_eq!(options.base, "v0.6.0");
        assert_eq!(options.rounds, 3);
        assert_eq!(options.harness_args, ["--filter", "num/"]);
        assert_eq!(options.max_regression, Some(15.0));
        assert!(options.align_code);
        let cross = ["--target", "wasm32-wasip1", "--runner", "wasmtime run"];
        let cross =
            PerfOptions::parse(cross.iter().map(|s| (*s).to_owned())).expect("valid options");
        assert_eq!(cross.target.as_deref(), Some("wasm32-wasip1"));
        assert_eq!(cross.runner.as_deref(), Some("wasmtime run"));
        assert_eq!(cross.platform(), "wasm32-wasip1");
        assert!(PerfOptions::parse(["--bogus", "1"].iter().map(|s| (*s).to_owned())).is_err());
    }

    #[test]
    fn binary_names_follow_the_target() {
        assert_eq!(binary_name(Some("wasm32-wasip1")), "bunny-perf.wasm");
        assert_eq!(binary_name(Some("x86_64-pc-windows-msvc")), "bunny-perf.exe");
        assert_eq!(binary_name(Some("aarch64-unknown-linux-gnu")), "bunny-perf");
        assert_eq!(binary_name(None), format!("bunny-perf{}", std::env::consts::EXE_SUFFIX));
    }

    #[test]
    fn toml_paths_use_forward_slashes_and_escape_quotes() {
        assert_eq!(
            toml_path(Path::new(r"D:\a\bunny\crates\bunny-num")),
            "D:/a/bunny/crates/bunny-num"
        );
        assert_eq!(toml_path(Path::new("/tmp/odd\"name/é")), "/tmp/odd\\\"name/é");
    }

    fn runs(case: &str, p50s: &[f64]) -> crate::perf_report::Runs {
        let timings = p50s
            .iter()
            .map(|&p50| crate::perf_report::Timing { p50, p95: p50, p99: p50 })
            .collect();
        crate::perf_report::Runs::from([(case.to_owned(), timings)])
    }

    #[test]
    fn the_gate_refuses_a_run_that_measured_nothing() {
        let rows = crate::perf_report::compare(
            &crate::perf_report::Runs::new(),
            &crate::perf_report::Runs::new(),
        );
        let err = check_threshold(&rows, Some(20.0)).expect_err("nothing measured");
        assert!(err.to_string().contains("no case was measured"), "{err}");
        let only_new =
            crate::perf_report::compare(&crate::perf_report::Runs::new(), &runs("a", &[1.0]));
        assert!(check_threshold(&only_new, None).is_err());
    }

    #[test]
    fn the_evidence_option_is_opt_in() {
        let none = PerfOptions::parse(std::iter::empty()).expect("no options");
        assert_eq!(none.evidence, None);
        let args = ["--evidence", "docs/topics/performance/evidence/run.jsonl"];
        let some = PerfOptions::parse(args.iter().map(|s| (*s).to_owned())).expect("valid");
        assert_eq!(
            some.evidence.as_deref(),
            Some(Path::new("docs/topics/performance/evidence/run.jsonl"))
        );
        assert!(PerfOptions::parse(["--evidence".to_owned()].into_iter()).is_err());
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(refuse_existing(&manifest).is_err(), "an existing file is never overwritten");
        assert!(refuse_existing(&manifest.with_file_name("no-such-evidence.jsonl")).is_ok());
    }

    #[test]
    fn the_report_is_rendered_as_before() {
        // The table `handle` printed before evidence existed: `render` of `compare` over the
        // runs `add_run` accumulated, side by side, with the same heading labels.
        let outputs = [
            (Side::Base, "a\t10\t11\t12\nb\t5\t6\t7\n"),
            (Side::Candidate, "a\t9\t10\t11\n"),
            (Side::Candidate, "a\t8\t9\t10\nc\t1\t2\t3\n"),
            (Side::Base, "a\t11\t12\t13\nb\t6\t7\t8\n"),
        ];
        let (mut base, mut head) =
            (crate::perf_report::Runs::new(), crate::perf_report::Runs::new());
        let mut measured = Measured::default();
        for (index, (side, output)) in outputs.iter().enumerate() {
            let runs = if *side == Side::Base { &mut base } else { &mut head };
            crate::perf_report::add_run(runs, output).expect("valid");
            measured.record(*side, index / 2 + 1, (*output).to_owned()).expect("valid");
        }
        assert_eq!(measured.base, base);
        assert_eq!(measured.candidate, head);
        let sha = "0123456789abcdef0123456789abcdef01234567";
        let scope = ReportScope { base: "origin/main", sha, platform: "p", rounds: 2 };
        let (rows, table) = report(&measured, &scope);
        let expected_rows = compare(&base, &head);
        let heading = Heading {
            base: "origin/main (0123456789ab)",
            head: "working tree",
            platform: "p",
            rounds: 2,
        };
        assert_eq!(table, render(&expected_rows, &heading));
        assert_eq!(rows, expected_rows);
        assert!(measured.record(Side::Base, 3, "a\t1\n".to_owned()).is_err());
    }

    #[test]
    fn the_gate_message_reports_the_floor_and_median_change_it_tested() {
        // Three rounds: the floor is the lowest p50. Floor +30 %, median +30 %.
        let rows = crate::perf_report::compare(
            &runs("a", &[100.0, 100.0, 100.0]),
            &runs("a", &[130.0, 130.0, 130.0]),
        );
        let err = check_threshold(&rows, Some(20.0)).expect_err("regressed");
        assert!(err.to_string().contains("a (+30.0% floor, +30.0% median)"), "{err}");
    }
}
