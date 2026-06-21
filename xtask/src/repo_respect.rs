use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter, Write as _};
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
    let trailers = receipt_trailers(non_comment);
    if trailers.is_empty() {
        return Ok(vec![format!("non-merge commits require '{RECEIPT_TRAILER} <path>' trailer")]);
    }

    let root = git_root()?;
    let mut failures = Vec::new();
    for trailer in trailers {
        if !is_receipt_path(Path::new(&trailer)) {
            failures.push(format!(
                "{RECEIPT_TRAILER} path must be under {RECEIPT_DIR}/ and end in .md"
            ));
            continue;
        }

        match read_receipt_from_git_index(&root, &trailer) {
            Ok(text) => failures.extend(validate_receipt_text(&trailer, &text)),
            Err(error) => failures.push(format!("{trailer}: {error}")),
        }
    }

    Ok(failures)
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
    let changed = match mode {
        CheckMode::Branch => branch_paths(&root)?,
        CheckMode::Staged => branch_and_staged_paths(&root)?,
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
        let text = std::fs::read_to_string(root.join(&path))?;
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
    let Some(start) = text.find(field) else {
        return false;
    };
    let after = &text[start + field.len()..];
    let end = REQUIRED_RECEIPT_FIELDS
        .iter()
        .filter(|candidate| **candidate != field)
        .filter_map(|candidate| after.find(candidate))
        .min()
        .unwrap_or(after.len());
    after[..end].lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with("<!--")
    })
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
    let output = Command::new("git").args(["show", &object]).current_dir(root).output()?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)?);
    }

    let worktree_path = root.join(path);
    if worktree_path.is_file() {
        return Ok(std::fs::read_to_string(worktree_path)?);
    }

    Err(command_error("git show receipt", &output.stderr).into())
}

fn branch_and_worktree_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let mut paths = branch_paths(root)?;
    paths.extend(git_paths(root, &["diff", "--cached", "--name-only", "--diff-filter=ACMR"])?);
    paths.extend(git_paths(root, &["diff", "--name-only", "--diff-filter=ACMR"])?);
    paths.extend(git_paths(root, &["ls-files", "--others", "--exclude-standard"])?);
    Ok(paths)
}

fn branch_and_staged_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let mut paths = branch_paths(root)?;
    paths.extend(git_paths(root, &["diff", "--cached", "--name-only", "--diff-filter=ACMR"])?);
    Ok(paths)
}

fn branch_paths(root: &Path) -> Result<BTreeSet<PathBuf>, DynError> {
    let base = base_ref();
    ensure_ref_exists(root, &base)?;
    git_paths(root, &["diff", "--name-only", "--diff-filter=ACMR", &format!("{base}...HEAD")])
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
    let output = Command::new("git")
        .args(["rev-parse", "--verify", reference])
        .current_dir(root)
        .output()?;
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
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(command_error(&format!("git {}", args.join(" ")), &output.stderr).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.lines().map(str::trim).filter(|line| !line.is_empty()).map(PathBuf::from).collect())
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
        "- TBD\n".to_string()
    } else {
        let mut edited = String::new();
        for path in changed_files.iter().filter(|path| !is_receipt_path(path)) {
            let _ = writeln!(&mut edited, "- `{}`", display_path(path));
        }
        edited
    };

    format!(
        "\
# Receipt: {title}

Task:
TBD.

Files read:
- TBD

Files edited:
{files_edited}
Topic docs:
- TBD, or `None - <reason>`.

Generated artifacts:
- TBD, or `None`.

Checks run:
- TBD

Known risks:
- TBD

Human reviewer:
TBD
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
    use super::*;

    #[test]
    fn slug_normalization_is_stable() {
        assert_eq!(normalize_slug("Matrix Types!").expect("slug should normalize"), "matrix-types");
    }

    #[test]
    fn receipt_template_contains_required_fields() {
        let template = receipt_template("matrix types", &BTreeSet::new());

        for field in REQUIRED_RECEIPT_FIELDS {
            assert!(receipt_field_has_content(&template, field), "{field} should have content");
        }
    }

    #[test]
    fn receipt_validation_requires_all_fields() {
        let failures = validate_receipt_text("receipt.md", "Task:\nDo work.\n");

        assert!(failures.iter().any(|failure| failure.contains("Files read:")));
    }

    #[test]
    fn non_merge_commit_messages_require_receipt_trailers() {
        let failures =
            commit_message_failures(&["docs: update contributor guide"]).expect("check should run");

        assert!(failures.iter().any(|failure| failure.contains("Repo-Respect-Receipt:")));
    }

    #[test]
    fn civil_date_matches_unix_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }
}
