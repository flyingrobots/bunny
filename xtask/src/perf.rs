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
use crate::perf_report::{add_run, compare, regressions, render, Runs};

const DEFAULT_BASE: &str = "origin/main";
const DEFAULT_ROUNDS: usize = 5;
const BASE_CRATES: &[&str] = &["bunny-geom", "bunny-linalg", "bunny-num", "bunny-query"];

struct PerfOptions {
    base: String,
    rounds: usize,
    max_regression: Option<f64>,
    summary: Option<PathBuf>,
    harness_args: Vec<String>,
    align_code: bool,
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

    // Both sides build the same way (a standalone copy of the harness), so codegen differences
    // between a workspace build and a standalone build cannot bias the comparison.
    eprintln!("perf: building the harness for the working tree");
    let head_bin = build_harness(&root, &root, &perf_dir.join("harness-head"), options.align_code)?;
    eprintln!("perf: building the harness for {} ({short})", options.base);
    let base_tree = perf_dir.join(format!("base-{short}"));
    add_worktree(&root, &base_tree, &sha)?;
    let base_bin = build_harness(
        &root,
        &base_tree,
        &perf_dir.join(format!("harness-{short}")),
        options.align_code,
    );
    remove_worktree(&root, &base_tree);
    let base_bin = base_bin?;

    let (base_runs, head_runs) = run_interleaved(&base_bin, &head_bin, &options)?;
    let rows = compare(&base_runs, &head_runs);
    let table =
        render(&rows, &format!("{} ({short})", options.base), "working tree", options.rounds);
    println!("{table}");
    if let Some(path) = &options.summary {
        fs::OpenOptions::new().create(true).append(true).open(path)?.write_all(table.as_bytes())?;
    }
    check_threshold(&rows, options.max_regression)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn binary_name() -> String {
    format!("bunny-perf{}", std::env::consts::EXE_SUFFIX)
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
    align_code: bool,
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
    fs::write(harness.join("Cargo.toml"), harness_manifest(tree, &groups))?;

    let built = |features: &[&str]| -> Result<bool, Box<dyn Error>> {
        let status = cargo()
            .current_dir(harness)
            .env("RUSTFLAGS", rustflags(align_code))
            .args([
                "build",
                "--release",
                "--quiet",
                "--no-default-features",
                "--features",
                &features.join(","),
            ])
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
    Ok(harness.join("target").join("release").join(binary_name()))
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
fn feature_groups(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != "[features]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim().to_owned()))
        .filter(|name| !name.is_empty() && !name.starts_with('#') && name != "default")
        .collect()
}

fn harness_manifest(tree: &Path, groups: &[String]) -> String {
    let mut manifest = String::from(
        "[package]\nname = \"bunny-perf\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
         [workspace]\n\n[[bin]]\nname = \"bunny-perf\"\npath = \"src/main.rs\"\n\n[features]\n",
    );
    for group in groups {
        let _ = writeln!(manifest, "{group} = []");
    }
    manifest.push_str("\n[dependencies]\n");
    for name in BASE_CRATES {
        let path = tree.join("crates").join(name);
        let _ = writeln!(manifest, "{name} = {{ path = {:?} }}", path.display().to_string());
    }
    manifest
}

fn run_interleaved(
    base: &Path,
    head: &Path,
    options: &PerfOptions,
) -> Result<(Runs, Runs), Box<dyn Error>> {
    let (mut base_runs, mut head_runs) = (Runs::new(), Runs::new());
    for round in 1..=options.rounds {
        eprintln!("perf: round {round} of {}", options.rounds);
        // Alternate which side runs first, so warm-up or thermal drift favours neither.
        let base_output;
        let head_output;
        if round % 2 == 1 {
            base_output = run_harness(base, &options.harness_args)?;
            head_output = run_harness(head, &options.harness_args)?;
        } else {
            head_output = run_harness(head, &options.harness_args)?;
            base_output = run_harness(base, &options.harness_args)?;
        }
        add_run(&mut base_runs, &base_output)?;
        add_run(&mut head_runs, &head_output)?;
    }
    Ok((base_runs, head_runs))
}

fn run_harness(binary: &Path, args: &[String]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(binary).args(args).output()?;
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

fn check_threshold(
    rows: &[crate::perf_report::Row],
    max_regression: Option<f64>,
) -> Result<(), Box<dyn Error>> {
    let Some(limit) = max_regression else {
        return Ok(());
    };
    let slow = regressions(rows, limit);
    if slow.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = slow
        .iter()
        .map(|row| format!("{} ({:+.1}%)", row.case, row.change_percent().unwrap_or_default()))
        .collect();
    Err(format!("p50 regressed past {limit}%: {}", names.join(", ")).into())
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
        let manifest = harness_manifest(Path::new("/tmp/base"), &["num".to_owned()]);
        assert!(manifest.contains("num = []\n"));
        assert!(manifest.contains("bunny-num = { path = \"/tmp/base/crates/bunny-num\" }"));
        assert!(manifest.contains("[workspace]"));
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
        assert!(PerfOptions::parse(["--bogus", "1"].iter().map(|s| (*s).to_owned())).is_err());
    }
}
