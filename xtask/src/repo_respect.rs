use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter, Write as _};
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::git_helpers::git_command;

type DynError = Box<dyn Error>;

const RECEIPT_DIR: &str = ".repo-respect/receipts";
const RECEIPT_TRAILER: &str = "Repo-Respect-Receipt:";
const REQUIRED_RECEIPT_FIELDS: &[&str] = &[
    "Task:",
    "Files read:",
    "Files edited:",
    "Topic docs:",
    "Generated artifacts:",
    "Checks run:",
    "Known risks:",
    "Human reviewer:",
];

#[derive(Debug)]
struct RepoRespectError {
    message: String,
}

impl RepoRespectError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for RepoRespectError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RepoRespectError {}

#[derive(Clone, Copy)]
enum CheckMode {
    Branch,
    Staged,
}

pub(super) fn handle(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        print_help();
        return Err(RepoRespectError::new("repo-respect requires a command").into());
    };

    match command.as_str() {
        "receipt" => {
            let Some(topic) = args.next() else {
                return Err(
                    RepoRespectError::new("usage: repo-respect receipt <short-topic>").into()
                );
            };
            if args.next().is_some() {
                return Err(RepoRespectError::new(
                    "repo-respect receipt accepts exactly one topic",
                )
                .into());
            }
            create_receipt(&topic)
        }
        "check" => {
            let mode = parse_check_mode(args)?;
            check(mode)
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            Err(RepoRespectError::new(format!("unknown repo-respect command: {other}")).into())
        }
    }
}

pub(super) fn check_branch() -> Result<(), DynError> {
    check(CheckMode::Branch)
}

pub(super) fn check_staged() -> Result<(), DynError> {
    check(CheckMode::Staged)
}

pub(super) fn commit_message_failures(non_comment: &[&str]) -> Result<Vec<String>, DynError> {
    let root = git_root()?;
    Ok(commit_message_failures_for_root(&root, non_comment))
}

fn commit_message_failures_for_root(root: &Path, non_comment: &[&str]) -> Vec<String> {
    let trailers = receipt_trailers(non_comment);
    if trailers.is_empty() {
        return vec![format!("non-merge commits require '{RECEIPT_TRAILER} <path>' trailer")];
    }

    let mut failures = Vec::new();
    for trailer in trailers {
        if !is_receipt_path(Path::new(&trailer)) {
            failures.push(format!(
                "{RECEIPT_TRAILER} path must be under {RECEIPT_DIR}/ and end in .md"
            ));
            continue;
        }

        match read_receipt_from_git_index(root, &trailer) {
            Ok(text) => failures.extend(validate_receipt_text(&trailer, &text)),
            Err(error) => failures.push(format!("{trailer}: {error}")),
        }
    }

    failures
}

fn print_help() {
    println!("Usage:");
    println!("  cargo run -p xtask -- repo-respect receipt <short-topic>");
    println!("  cargo run -p xtask -- repo-respect check (--branch|--staged)");
}

fn parse_check_mode(args: impl IntoIterator<Item = String>) -> Result<CheckMode, DynError> {
    let mut mode = None;
    for arg in args {
        match arg.as_str() {
            "--branch" => set_mode(&mut mode, CheckMode::Branch)?,
            "--staged" => set_mode(&mut mode, CheckMode::Staged)?,
            "--help" | "-h" => {
                print_help();
                return Err(RepoRespectError::new("help requested").into());
            }
            other => {
                return Err(RepoRespectError::new(format!(
                    "unknown repo-respect check argument: {other}"
                ))
                .into())
            }
        }
    }
    mode.ok_or_else(|| {
        RepoRespectError::new("repo-respect check requires --branch or --staged").into()
    })
}

fn set_mode(mode: &mut Option<CheckMode>, next: CheckMode) -> Result<(), DynError> {
    if mode.is_some() {
        return Err(RepoRespectError::new("choose only one receipt check mode").into());
    }
    *mode = Some(next);
    Ok(())
}

fn create_receipt(topic: &str) -> Result<(), DynError> {
    let root = git_root()?;
    let slug = normalize_slug(topic)?;
    let date = current_utc_date()?;
    let path = unique_receipt_path(&root, &date, &slug);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let changed_files = branch_and_worktree_paths(&root)?;
    let template = receipt_template(topic, &changed_files);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
    file.write_all(template.as_bytes())?;

    let relative = path.strip_prefix(&root).unwrap_or(&path);
    println!("Created {}", relative.display());
    println!();
    println!("Add this commit trailer:");
    println!("{RECEIPT_TRAILER} {}", display_path(relative));
    Ok(())
}

fn check(mode: CheckMode) -> Result<(), DynError> {
    let root = git_root()?;
    check_root(&root, mode)?;
    if matches!(mode, CheckMode::Branch) {
        check_branch_commit_messages(&root)?;
    }
    Ok(())
}

fn check_root(root: &Path, mode: CheckMode) -> Result<(), DynError> {
    let changed = match mode {
        CheckMode::Branch => branch_paths(root)?,
        CheckMode::Staged => branch_and_staged_paths(root)?,
    };

    let non_receipt_changes =
        changed.iter().filter(|path| !is_receipt_path(path)).cloned().collect::<BTreeSet<_>>();
    let receipt_paths =
        changed.iter().filter(|path| is_receipt_path(path)).cloned().collect::<BTreeSet<_>>();

    if non_receipt_changes.is_empty() {
        println!("Repo Respect: no receipt required for receipt-only or empty changes");
        return Ok(());
    }

    if receipt_paths.is_empty() {
        print_missing_receipt(mode, &non_receipt_changes);
        return Err(RepoRespectError::new("repo-respect receipt is required").into());
    }

    let mut failures = Vec::new();
    for path in receipt_paths {
        let text = read_receipt_for_mode(root, mode, &path)?;
        failures.extend(validate_receipt_text(&display_path(&path), &text));
    }

    if failures.is_empty() {
        println!("Repo Respect: receipt coverage clean");
        Ok(())
    } else {
        eprintln!("Repo Respect: receipt violations found");
        for failure in failures {
            eprintln!("  - {failure}");
        }
        Err(RepoRespectError::new("repo-respect receipt violations found").into())
    }
}

fn check_branch_commit_messages(root: &Path) -> Result<(), DynError> {
    let failures = branch_commit_message_failures(root)?;
    if failures.is_empty() {
        println!("Repo Respect: branch commit trailers clean");
        return Ok(());
    }

    eprintln!("Repo Respect: commit trailer violations found");
    for failure in failures {
        eprintln!("  - {failure}");
    }
    Err(RepoRespectError::new("repo-respect commit trailer violations found").into())
}

fn print_missing_receipt(mode: CheckMode, changed: &BTreeSet<PathBuf>) {
    let label = match mode {
        CheckMode::Branch => "branch",
        CheckMode::Staged => "staged/branch",
    };
    eprintln!("Repo Respect: {label} changes require a receipt under {RECEIPT_DIR}/");
    eprintln!("Changed non-receipt paths:");
    for path in changed.iter().take(12) {
        eprintln!("  - {}", path.display());
    }
    if changed.len() > 12 {
        eprintln!("  - ... and {} more", changed.len() - 12);
    }
    eprintln!();
    eprintln!("Create a template with:");
    eprintln!("  cargo run --locked -p xtask -- repo-respect receipt <short-topic>");
    eprintln!();
    eprintln!("Then stage the receipt and use this commit trailer:");
    eprintln!("  {RECEIPT_TRAILER} {RECEIPT_DIR}/<id>.md");
}

fn validate_receipt_text(path: &str, text: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for field in REQUIRED_RECEIPT_FIELDS {
        if !receipt_field_has_content(text, field) {
            failures.push(format!("{path}: missing non-empty '{field}' section"));
        }
    }
    failures
}

fn receipt_field_has_content(text: &str, field: &str) -> bool {
    let mut in_target_section = false;
    let mut in_html_comment = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(candidate) = receipt_heading(trimmed) {
            if in_target_section && candidate != field {
                return false;
            }
            in_target_section = candidate == field;
            in_html_comment = false;
            continue;
        }

        if in_target_section && receipt_line_has_content(trimmed, &mut in_html_comment) {
            return true;
        }
    }

    false
}

fn receipt_heading(trimmed: &str) -> Option<&'static str> {
    REQUIRED_RECEIPT_FIELDS.iter().copied().find(|field| trimmed == *field)
}

fn receipt_line_has_content(trimmed: &str, in_html_comment: &mut bool) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    if *in_html_comment {
        if trimmed.contains("-->") {
            *in_html_comment = false;
        }
        return false;
    }
    if trimmed.starts_with("<!--") {
        if !trimmed.contains("-->") {
            *in_html_comment = true;
        }
        return false;
    }
    if trimmed.contains("-->") {
        return false;
    }

    let value = trimmed.strip_prefix('-').map_or(trimmed, str::trim);
    let normalized = value.trim_end_matches('.').trim().to_ascii_lowercase();
    !matches!(normalized.as_str(), "tbd" | "todo")
        && !normalized.starts_with("tbd,")
        && !normalized.starts_with("todo,")
}

fn receipt_trailers(non_comment: &[&str]) -> Vec<String> {
    non_comment
        .iter()
        .filter_map(|line| line.strip_prefix(RECEIPT_TRAILER))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn read_receipt_from_git_index(root: &Path, path: &str) -> Result<String, DynError> {
    let object = format!(":{path}");
    let output = git_command(root).args(["show", &object]).output()?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)?);
    }

    let worktree_path = root.join(path);
    if worktree_path.is_file() {
        return Ok(std::fs::read_to_string(worktree_path)?);
    }

    Err(command_error("git show receipt", &output.stderr).into())
}

fn read_receipt_for_mode(root: &Path, mode: CheckMode, path: &Path) -> Result<String, DynError> {
    match mode {
        CheckMode::Branch => Ok(std::fs::read_to_string(root.join(path))?),
        CheckMode::Staged => read_receipt_from_staged_index(root, &display_path(path)),
    }
}

fn read_receipt_from_staged_index(root: &Path, path: &str) -> Result<String, DynError> {
    let object = format!(":{path}");
    let output = git_command(root).args(["show", &object]).output()?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)?);
    }

    Err(command_error("git show staged receipt", &output.stderr).into())
}

fn branch_commit_message_failures(root: &Path) -> Result<Vec<String>, DynError> {
    let base = base_ref();
    ensure_ref_exists(root, &base)?;
    let merge_base = git_merge_base(root, &base)?;
    let range = format!("{merge_base}..HEAD");
    let commits = git_lines(root, &["rev-list", "--no-merges", "--reverse", &range])?;
    let mut failures = Vec::new();

    for commit in commits {
        let message = git_commit_message(root, &commit)?;
        let non_comment = message
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>();
        let short = commit.chars().take(7).collect::<String>();
        failures.extend(
            commit_message_failures_for_root(root, &non_comment)
                .into_iter()
                .map(|failure| format!("{short}: {failure}")),
        );
    }

    Ok(failures)
}

fn git_commit_message(root: &Path, commit: &str) -> Result<String, DynError> {
    let output = git_command(root).args(["log", "-1", "--format=%B", commit]).output()?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)?);
    }

    Err(command_error("git log commit message", &output.stderr).into())
}

fn git_merge_base(root: &Path, reference: &str) -> Result<String, DynError> {
    let mut lines = git_lines(root, &["merge-base", reference, "HEAD"])?;
    lines.pop().ok_or_else(|| {
        RepoRespectError::new(format!("cannot find merge-base with {reference}")).into()
    })
}

fn branch_and_worktree_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let mut paths = branch_paths(root)?;
    paths.extend(git_paths(root, &["diff", "--cached", "--name-only"])?);
    paths.extend(git_paths(root, &["diff", "--name-only"])?);
    paths.extend(git_paths(root, &["ls-files", "--others", "--exclude-standard"])?);
    Ok(paths)
}

fn branch_and_staged_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let mut paths = branch_paths(root)?;
    paths.extend(git_paths(root, &["diff", "--cached", "--name-only"])?);
    Ok(paths)
}

fn branch_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let base = base_ref();
    ensure_ref_exists(root, &base)?;
    git_paths(root, &["diff", "--name-only", &format!("{base}...HEAD")])
}

fn base_ref() -> String {
    if let Ok(value) = std::env::var("CODE_DOJO_BASE_REF") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if let Ok(value) = std::env::var("GITHUB_BASE_REF") {
        if !value.trim().is_empty() {
            return format!("origin/{value}");
        }
    }
    "origin/main".to_string()
}

fn ensure_ref_exists(root: &Path, reference: &str) -> Result<(), DynError> {
    let output = git_command(root).args(["rev-parse", "--verify", reference]).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(RepoRespectError::new(format!(
            "cannot find {reference}; run 'git fetch origin' before repo-respect checks"
        ))
        .into())
    }
}

fn git_paths(root: &Path, args: &[&str]) -> Result<BTreeSet<PathBuf>, DynError> {
    Ok(git_lines(root, args)?.into_iter().map(PathBuf::from).collect())
}

fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>, DynError> {
    let output = git_command(root).args(args).output()?;
    if !output.status.success() {
        return Err(command_error(&format!("git {}", args.join(" ")), &output.stderr).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_owned).collect())
}

fn git_root() -> Result<PathBuf, DynError> {
    let output = Command::new("git").args(["rev-parse", "--show-toplevel"]).output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        return Ok(PathBuf::from(stdout.trim()));
    }

    Err(command_error("git rev-parse --show-toplevel", &output.stderr).into())
}

fn command_error(command: &str, stderr: &[u8]) -> RepoRespectError {
    let detail = String::from_utf8_lossy(stderr);
    RepoRespectError::new(format!("{command} failed: {}", detail.trim()))
}

fn normalize_slug(value: &str) -> Result<String, DynError> {
    let mut slug = String::new();
    let mut previous_dash = false;
    for byte in value.bytes() {
        let next = if byte.is_ascii_alphanumeric() {
            previous_dash = false;
            Some(byte.to_ascii_lowercase() as char)
        } else if previous_dash {
            None
        } else {
            previous_dash = true;
            Some('-')
        };
        if let Some(character) = next {
            slug.push(character);
        }
    }

    let normalized = slug.trim_matches('-').to_string();
    if normalized.is_empty() {
        return Err(
            RepoRespectError::new("receipt topic must contain an ASCII letter or digit").into()
        );
    }
    Ok(normalized)
}

fn unique_receipt_path(root: &Path, date: &str, slug: &str) -> PathBuf {
    for suffix in 0.. {
        let name = if suffix == 0 {
            format!("{date}-{slug}.md")
        } else {
            format!("{date}-{slug}-{suffix}.md")
        };
        let path = root.join(RECEIPT_DIR).join(name);
        if !path.exists() {
            return path;
        }
    }
    unreachable!("unbounded suffix search should return before integer overflow")
}

fn receipt_template(topic: &str, changed_files: &BTreeSet<PathBuf>) -> String {
    let title = title_case(topic);
    let files_edited = if changed_files.is_empty() {
        "<!-- TODO: list edited non-receipt paths. -->\n".to_string()
    } else {
        let mut edited = String::new();
        for path in changed_files.iter().filter(|path| !is_receipt_path(path)) {
            let _ = writeln!(&mut edited, "- `{}`", display_path(path));
        }
        if edited.is_empty() {
            "<!-- TODO: list edited non-receipt paths. -->\n".to_string()
        } else {
            edited
        }
    };

    format!(
        "\
# Receipt: {title}

Task:
<!-- TODO: summarize the task. -->

Files read:
<!-- TODO: list files read while making the change. -->

Files edited:
{files_edited}
Topic docs:
<!-- TODO: list topic docs updated, or `None - <reason>`. -->

Generated artifacts:
<!-- TODO: list generated artifacts, or `None`. -->

Checks run:
<!-- TODO: list exact commands run. -->

Known risks:
<!-- TODO: list known risks, or `None`. -->

Human reviewer:
<!-- TODO: name the human reviewer or approval source. -->
"
    )
}

fn title_case(value: &str) -> String {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn current_utc_date() -> Result<String, DynError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let days = i64::try_from(duration.as_secs() / 86_400)?;
    let (year, month, day) = civil_from_days(days);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn is_receipt_path(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("md"))
        && path.starts_with(RECEIPT_DIR)
        && path.file_name() != Some(OsStr::new(".gitkeep"))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("bunny-repo-respect-{name}-{}-{count}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temporary directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = git_command(root).args(args).output().expect("git should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_git_stdout(root: &Path, args: &[&str]) -> String {
        let output = git_command(root).args(args).output().expect("git should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("git output should be utf8")
    }

    fn write_file(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path should have a parent"))
            .expect("fixture directory should be created");
        fs::write(path, text).expect("fixture file should be written");
    }

    fn init_repo_with_origin_main(root: &Path) {
        run_git(root, &["init"]);
        run_git(root, &["config", "user.email", "bunny@example.invalid"]);
        run_git(root, &["config", "user.name", "Bunny Test"]);
        run_git(root, &["config", "commit.gpgsign", "false"]);
        write_file(root, "src/lib.rs", "pub fn baseline() {}\n");
        run_git(root, &["add", "src/lib.rs"]);
        run_git(root, &["commit", "-m", "seed: create baseline"]);
        run_git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    }

    fn valid_receipt_text() -> &'static str {
        "\
# Receipt: Fixture

Task:
Validate fixture behavior.

Files read:
- `src/lib.rs`

Files edited:
- `src/lib.rs`

Topic docs:
None - test fixture.

Generated artifacts:
None.

Checks run:
- `cargo test -p xtask`

Known risks:
None.

Human reviewer:
Fixture reviewer.
"
    }

    #[test]
    fn slug_normalization_is_stable() {
        assert_eq!(normalize_slug("Matrix Types!").expect("slug should normalize"), "matrix-types");
    }

    #[test]
    fn receipt_template_contains_required_field_headings() {
        let template = receipt_template("matrix types", &BTreeSet::new());

        for field in REQUIRED_RECEIPT_FIELDS {
            assert!(template.contains(field), "{field} should be present");
        }
    }

    #[test]
    fn receipt_validation_requires_all_fields() {
        let failures = validate_receipt_text("receipt.md", "Task:\nDo work.\n");

        assert!(failures.iter().any(|failure| failure.contains("Files read:")));
    }

    #[test]
    fn receipt_validation_rejects_placeholder_sections() {
        let failures =
            validate_receipt_text("receipt.md", &receipt_template("placeholder", &BTreeSet::new()));

        assert!(failures.iter().any(|failure| failure.contains("Task:")));
        assert!(failures.iter().any(|failure| failure.contains("Files read:")));
        assert!(failures.iter().any(|failure| failure.contains("Human reviewer:")));
    }

    #[test]
    fn receipt_validation_requires_exact_field_headings() {
        let text = "\
# Receipt: Fixture

Task:
Files read: mentioned in task prose only.

Files edited:
- `src/lib.rs`

Topic docs:
None - test fixture.

Generated artifacts:
None.

Checks run:
- `cargo test -p xtask`

Known risks:
None.

Human reviewer:
Fixture reviewer.
";

        let failures = validate_receipt_text("receipt.md", text);

        assert!(failures.iter().any(|failure| failure.contains("Files read:")));
    }

    #[test]
    fn receipt_validation_ignores_multiline_html_comments() {
        let text = "\
# Receipt: Fixture

Task:
<!-- This is only
comment content -->

Files read:
- `src/lib.rs`

Files edited:
- `src/lib.rs`

Topic docs:
None - test fixture.

Generated artifacts:
None.

Checks run:
- `cargo test -p xtask`

Known risks:
None.

Human reviewer:
Fixture reviewer.
";

        let failures = validate_receipt_text("receipt.md", text);

        assert!(failures.iter().any(|failure| failure.contains("Task:")));
    }

    #[test]
    fn non_merge_commit_messages_require_receipt_trailers() {
        let failures =
            commit_message_failures(&["docs: update contributor guide"]).expect("check should run");

        assert!(failures.iter().any(|failure| failure.contains("Repo-Respect-Receipt:")));
    }

    #[test]
    fn changed_path_lists_include_deleted_files() {
        let temp = TempDir::new("deleted-paths");
        init_repo_with_origin_main(temp.path());

        fs::remove_file(temp.path().join("src/lib.rs")).expect("fixture file should be deleted");
        run_git(temp.path(), &["add", "src/lib.rs"]);
        run_git(
            temp.path(),
            &[
                "commit",
                "-m",
                "test: delete fixture file",
                "-m",
                "Repo-Respect-Receipt: .repo-respect/receipts/deletion.md",
            ],
        );

        let branch = branch_paths(temp.path()).expect("branch paths should load");
        let staged = branch_and_staged_paths(temp.path()).expect("staged paths should load");

        assert!(branch.contains(Path::new("src/lib.rs")));
        assert!(staged.contains(Path::new("src/lib.rs")));
    }

    #[test]
    fn changed_path_lists_include_typechanged_files() {
        let temp = TempDir::new("typechanged-paths");
        init_repo_with_origin_main(temp.path());
        write_file(temp.path(), "target", "src/lib.rs\n");
        let blob = run_git_stdout(temp.path(), &["hash-object", "-w", "target"]);
        let blob = blob.trim();
        run_git(temp.path(), &["update-index", "--cacheinfo", "120000", blob, "src/lib.rs"]);
        run_git(
            temp.path(),
            &[
                "commit",
                "-m",
                "test: typechange fixture path",
                "-m",
                "Repo-Respect-Receipt: .repo-respect/receipts/typechange.md",
            ],
        );

        let branch = branch_paths(temp.path()).expect("branch paths should load");
        let staged = branch_and_staged_paths(temp.path()).expect("staged paths should load");

        assert!(branch.contains(Path::new("src/lib.rs")));
        assert!(staged.contains(Path::new("src/lib.rs")));
    }

    #[test]
    fn staged_check_validates_receipt_content_from_index() {
        let temp = TempDir::new("staged-receipt");
        init_repo_with_origin_main(temp.path());
        write_file(temp.path(), "src/feature.rs", "pub fn feature() {}\n");
        write_file(temp.path(), ".repo-respect/receipts/staged.md", "Task:\nStaged only.\n");
        run_git(temp.path(), &["add", "src/feature.rs", ".repo-respect/receipts/staged.md"]);
        write_file(temp.path(), ".repo-respect/receipts/staged.md", valid_receipt_text());

        let result = check_root(temp.path(), CheckMode::Staged);

        assert!(result.is_err(), "staged receipt contents should be validated, not worktree edits");
    }

    #[test]
    fn branch_commit_message_failures_report_missing_trailers() {
        let temp = TempDir::new("branch-commit-trailers");
        init_repo_with_origin_main(temp.path());
        write_file(temp.path(), "src/feature.rs", "pub fn feature() {}\n");
        write_file(temp.path(), ".repo-respect/receipts/feature.md", valid_receipt_text());
        run_git(temp.path(), &["add", "src/feature.rs", ".repo-respect/receipts/feature.md"]);
        run_git(temp.path(), &["commit", "-m", "test: add feature without trailer"]);

        let failures =
            branch_commit_message_failures(temp.path()).expect("branch messages should be checked");

        assert!(failures.iter().any(|failure| failure.contains("Repo-Respect-Receipt:")));
    }

    #[test]
    fn branch_commit_message_failures_ignore_new_base_commits() {
        let temp = TempDir::new("branch-commit-range");
        init_repo_with_origin_main(temp.path());
        write_file(temp.path(), ".repo-respect/receipts/feature.md", valid_receipt_text());
        run_git(temp.path(), &["add", ".repo-respect/receipts/feature.md"]);
        run_git(temp.path(), &["commit", "-m", "seed: add shared receipt"]);
        run_git(temp.path(), &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        run_git(temp.path(), &["checkout", "-b", "feature"]);
        write_file(temp.path(), "src/feature.rs", "pub fn feature() {}\n");
        run_git(temp.path(), &["add", "src/feature.rs"]);
        run_git(
            temp.path(),
            &[
                "commit",
                "-m",
                "test: add feature",
                "-m",
                "Repo-Respect-Receipt: .repo-respect/receipts/feature.md",
            ],
        );
        run_git(temp.path(), &["checkout", "-b", "upstream", "origin/main"]);
        write_file(temp.path(), "src/upstream.rs", "pub fn upstream() {}\n");
        run_git(temp.path(), &["add", "src/upstream.rs"]);
        run_git(temp.path(), &["commit", "-m", "test: advance upstream without trailer"]);
        run_git(temp.path(), &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        run_git(temp.path(), &["checkout", "feature"]);

        let failures =
            branch_commit_message_failures(temp.path()).expect("branch messages should be checked");

        assert!(failures.is_empty(), "base-only commits should not be checked: {failures:?}");
    }

    #[test]
    fn civil_date_matches_unix_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }
}
