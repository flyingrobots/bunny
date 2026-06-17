use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Command;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, BinOp, Block, ExprForLoop, ExprIf, ExprIndex, ExprLit, ExprLoop, ExprMacro,
    ExprMatch, ExprMethodCall, ExprPath, ExprWhile, File, ImplItemFn, Item, ItemFn, Lit, Macro,
    Path as SynPath, Signature, Stmt, StmtMacro, TraitItemFn, TypePath,
};

type DynError = Box<dyn Error>;

const CORE_CRATES: &[&str] =
    &["bunny-num", "bunny-linalg", "bunny-geom", "bunny-query", "bunny-broadphase", "bunny-mesh"];
const GENERATOR_CRATES: &[&str] = &["bunny-wesley"];
const TOOLING_CRATES: &[&str] = &["xtask"];
const STRICT_CLIPPY_PACKAGES: &[&str] = &[
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
    "bunny-codec",
    "bunny-contract",
];
const WASM_PACKAGES: &[&str] = &[
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-contract",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
    "bunny-codec",
];
const GOLDEN_TEST_NAMES: &[&str] =
    &["golden_vectors.rs", "determinism.rs", "fixed_q32x32_vectors.rs", "geometry_degenerates.rs"];
const MERGE_PREFIXES: &[&str] = &["Merge ", "Revert "];
const AI_MARKERS: &[&str] = &[
    "Co-Authored-By: ChatGPT",
    "Co-authored-by: ChatGPT",
    "AI-Assisted: true",
    "AI-Authored: true",
    "Generated-By: ChatGPT",
    "Generated-By: OpenAI",
];

#[derive(Clone, Copy)]
enum Mode {
    All,
    Staged,
}

#[derive(Default)]
struct FullGateArgs {
    all: bool,
    ci: bool,
}

#[derive(Clone, Copy)]
struct Limits {
    file_lines: Option<usize>,
    line_length: Option<usize>,
    function_lines: Option<usize>,
    statements: Option<usize>,
    nesting_depth: Option<usize>,
    parameters: Option<usize>,
    cyclomatic: Option<usize>,
    panics_allowed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Category {
    Core,
    Generator,
    Tests,
    Tooling,
    Unknown,
}

impl Category {
    const fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Generator => "generator",
            Self::Tests => "tests",
            Self::Tooling => "tooling",
            Self::Unknown => "unknown",
        }
    }

    const fn limits(self) -> Limits {
        match self {
            Self::Core => Limits {
                file_lines: Some(300),
                line_length: Some(100),
                function_lines: Some(25),
                statements: Some(15),
                nesting_depth: Some(3),
                parameters: Some(4),
                cyclomatic: Some(6),
                panics_allowed: false,
            },
            Self::Generator => Limits {
                file_lines: Some(500),
                line_length: Some(100),
                function_lines: Some(50),
                statements: Some(30),
                nesting_depth: Some(4),
                parameters: Some(6),
                cyclomatic: Some(10),
                panics_allowed: true,
            },
            Self::Tests | Self::Tooling => Limits {
                file_lines: None,
                line_length: Some(120),
                function_lines: None,
                statements: None,
                nesting_depth: None,
                parameters: None,
                cyclomatic: None,
                panics_allowed: true,
            },
            Self::Unknown => Limits {
                file_lines: Some(400),
                line_length: Some(120),
                function_lines: Some(50),
                statements: Some(30),
                nesting_depth: Some(4),
                parameters: Some(6),
                cyclomatic: Some(10),
                panics_allowed: false,
            },
        }
    }
}

#[derive(Debug)]
struct DojoError {
    message: String,
}

impl DojoError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for DojoError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DojoError {}

struct Violation {
    path: PathBuf,
    line: usize,
    rule: &'static str,
    message: String,
}

struct FileContext<'a> {
    path: PathBuf,
    category: Category,
    limits: Limits,
    source: &'a str,
    lines: &'a [String],
}

struct RustSource {
    path: PathBuf,
    source: String,
}

impl Violation {
    fn new(path: PathBuf, line: usize, rule: &'static str, message: impl Into<String>) -> Self {
        Self { path, line, rule, message: message.into() }
    }
}

impl Display for Violation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}: [{}] {}", self.path.display(), self.rule, self.message)
        } else {
            write!(
                formatter,
                "{}:{}: [{}] {}",
                self.path.display(),
                self.line,
                self.rule,
                self.message
            )
        }
    }
}

pub(super) fn handle_full(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let args = parse_full_gate_args(args)?;
    if !args.all {
        return Err(DojoError::new("code-dojo requires --all").into());
    }
    if args.ci {
        println!("Code Dojo: CI mode");
    }

    check_rust(Mode::All)?;
    check_determinism_receipts(true)?;
    ensure_cargo_manifest("full gate")?;
    run_quality_commands(true)?;
    println!("Code Dojo: full gate clean");
    Ok(())
}

pub(super) fn handle_pre_commit() -> Result<(), DynError> {
    println!("Code Dojo: checking staged Rust changes");
    check_rust(Mode::Staged)?;
    ensure_cargo_manifest("pre-commit gate")?;
    run_quality_commands(false)?;
    Ok(())
}

pub(super) fn handle_rust(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let mode = parse_mode(args)?;
    check_rust(mode)
}

pub(super) fn handle_determinism(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let enforce = parse_determinism_args(args)?;
    check_determinism_receipts(enforce)
}

pub(super) fn handle_commit_msg(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let path = parse_commit_msg_args(args)?;
    check_commit_message(&path)
}

fn check_rust(mode: Mode) -> Result<(), DynError> {
    let root = git_root()?;
    let mut violations = Vec::new();

    for file in rust_sources(&root, mode)? {
        let source = file.source;
        let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let category = crate_category(&file.path);
        let context = FileContext {
            path: file.path,
            category,
            limits: category.limits(),
            source: &source,
            lines: &lines,
        };

        check_textual_limits(&mut violations, &context);
        check_ast(&mut violations, &context);
    }

    if violations.is_empty() {
        println!("Code Dojo Rust AST: clean");
        return Ok(());
    }

    eprintln!("Code Dojo Rust AST: violations found");
    for violation in &violations {
        eprintln!("  {violation}");
    }
    Err(DojoError::new("Code Dojo Rust AST policy violations found").into())
}

fn parse_full_gate_args(args: impl IntoIterator<Item = String>) -> Result<FullGateArgs, DynError> {
    let mut parsed = FullGateArgs::default();
    for arg in args {
        match arg.as_str() {
            "--all" => parsed.all = true,
            "--ci" => parsed.ci = true,
            "--help" | "-h" => {
                println!("Usage: cargo run -p xtask -- code-dojo --all [--ci]");
                return Err(DojoError::new("help requested").into());
            }
            other => {
                return Err(DojoError::new(format!("unknown code-dojo argument: {other}")).into())
            }
        }
    }
    Ok(parsed)
}

fn parse_mode(args: impl IntoIterator<Item = String>) -> Result<Mode, DynError> {
    let mut mode = None;
    for arg in args {
        match arg.as_str() {
            "--all" => set_mode(&mut mode, Mode::All)?,
            "--staged" => set_mode(&mut mode, Mode::Staged)?,
            "--help" | "-h" => {
                println!("Usage: cargo run -p xtask -- code-dojo-rust (--all|--staged)");
                return Err(DojoError::new("help requested").into());
            }
            other => {
                return Err(
                    DojoError::new(format!("unknown code-dojo-rust argument: {other}")).into()
                )
            }
        }
    }

    mode.ok_or_else(|| DojoError::new("code-dojo-rust requires --all or --staged").into())
}

fn set_mode(mode: &mut Option<Mode>, next: Mode) -> Result<(), DynError> {
    if mode.is_some() {
        return Err(DojoError::new("choose only one of --all or --staged").into());
    }
    *mode = Some(next);
    Ok(())
}

fn parse_determinism_args(args: impl IntoIterator<Item = String>) -> Result<bool, DynError> {
    let mut enforce = false;
    for arg in args {
        match arg.as_str() {
            "--enforce" => enforce = true,
            "--help" | "-h" => {
                println!("Usage: cargo run -p xtask -- code-dojo-determinism [--enforce]");
                return Err(DojoError::new("help requested").into());
            }
            other => {
                return Err(DojoError::new(format!(
                    "unknown code-dojo-determinism argument: {other}"
                ))
                .into())
            }
        }
    }
    Ok(enforce)
}

fn parse_commit_msg_args(args: impl IntoIterator<Item = String>) -> Result<PathBuf, DynError> {
    let mut args = args.into_iter();
    let Some(path) = args.next() else {
        return Err(DojoError::new("usage: code-dojo-commit-msg <commit-msg-file>").into());
    };
    if args.next().is_some() {
        return Err(DojoError::new("code-dojo-commit-msg accepts exactly one path").into());
    }
    Ok(PathBuf::from(path))
}

fn ensure_cargo_manifest(context: &str) -> Result<(), DynError> {
    let root = git_root()?;
    if root.join("Cargo.toml").exists() {
        return Ok(());
    }
    Err(DojoError::new(format!("Code Dojo: Cargo.toml is required for the {context}")).into())
}

fn run_quality_commands(include_wasm_check: bool) -> Result<(), DynError> {
    run_command(&["cargo", "fmt", "--all", "--", "--check"])?;
    run_command(&[
        "cargo",
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;
    let strict_clippy = strict_clippy_command();
    run_command_strings(&strict_clippy)?;
    run_command(&["cargo", "deny", "check"])?;
    run_command(&["cargo", "test", "--locked", "--workspace", "--all-targets", "--all-features"])?;

    if include_wasm_check {
        run_command(&["rustup", "target", "add", "wasm32-unknown-unknown"])?;
        let wasm_check = wasm_check_command();
        run_command_strings(&wasm_check)?;
    }

    Ok(())
}

fn strict_clippy_command() -> Vec<String> {
    let mut args =
        vec!["cargo", "clippy", "--locked"].into_iter().map(str::to_owned).collect::<Vec<_>>();
    args.extend(package_args(STRICT_CLIPPY_PACKAGES));
    args.extend(
        [
            "--all-features",
            "--",
            "-D",
            "clippy::unwrap_used",
            "-D",
            "clippy::expect_used",
            "-D",
            "clippy::panic",
            "-D",
            "clippy::todo",
            "-D",
            "clippy::unimplemented",
            "-D",
            "clippy::indexing_slicing",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    args
}

fn wasm_check_command() -> Vec<String> {
    let mut args =
        vec!["cargo", "check", "--locked"].into_iter().map(str::to_owned).collect::<Vec<_>>();
    args.extend(package_args(WASM_PACKAGES));
    args.extend(
        ["--target", "wasm32-unknown-unknown", "--all-features"].into_iter().map(str::to_owned),
    );
    args
}

fn package_args(packages: &[&str]) -> Vec<String> {
    let mut args = Vec::with_capacity(packages.len() * 2);
    for package in packages {
        args.push("-p".to_string());
        args.push((*package).to_string());
    }
    args
}

fn run_command(args: &[&str]) -> Result<(), DynError> {
    let args = args.iter().map(|arg| (*arg).to_string()).collect::<Vec<_>>();
    run_command_strings(&args)
}

fn run_command_strings(args: &[String]) -> Result<(), DynError> {
    let Some((program, rest)) = args.split_first() else {
        return Err(DojoError::new("empty command").into());
    };
    println!("Code Dojo: {}", args.join(" "));
    let root = git_root()?;
    let status = Command::new(program).args(rest).current_dir(root).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(DojoError::new(format!("command failed: {}", args.join(" "))).into())
    }
}

fn check_determinism_receipts(enforce: bool) -> Result<(), DynError> {
    let root = git_root()?;
    let mut violations = Vec::new();

    for crate_name in sorted_names(CORE_CRATES) {
        let crate_dir = root.join("crates").join(crate_name);
        if !crate_dir.exists() {
            continue;
        }
        if !crate_has_determinism_receipt(&crate_dir)? {
            violations.push(Violation::new(
                PathBuf::from("crates").join(crate_name),
                0,
                "determinism-tests",
                "core crate should include deterministic golden-vector or degeneracy tests",
            ));
        }
    }

    if violations.is_empty() {
        println!("Code Dojo: deterministic test receipts present");
        return Ok(());
    }

    if enforce {
        print_violations("Code Dojo: deterministic receipt violations found", &violations);
        return Err(DojoError::new("deterministic receipt policy violations found").into());
    }

    println!("Code Dojo: deterministic test warnings");
    for violation in violations {
        println!("  {violation}");
    }
    Ok(())
}

fn sorted_names<'a>(names: &'a [&'a str]) -> Vec<&'a str> {
    let mut names = names.to_vec();
    names.sort_unstable();
    names
}

fn crate_has_determinism_receipt(crate_dir: &Path) -> Result<bool, DynError> {
    let tests_dir = crate_dir.join("tests");
    if !tests_dir.exists() {
        return Ok(false);
    }
    for path in rust_files_under(&tests_dir)? {
        let has_receipt_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| GOLDEN_TEST_NAMES.contains(&name));
        let text = std::fs::read_to_string(&path)?;
        let lower = text.to_ascii_lowercase();
        let has_receipt_terms = lower.contains("golden") && lower.contains("determin");
        if (has_receipt_name || has_receipt_terms) && rust_source_contains_test(&text)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn rust_source_contains_test(source: &str) -> Result<bool, DynError> {
    Ok(items_contain_test(&syn::parse_file(source)?.items))
}

fn items_contain_test(items: &[Item]) -> bool {
    items.iter().any(|item| match item {
        Item::Fn(function) => attrs_contain_test(&function.attrs),
        Item::Mod(module) => {
            module.content.as_ref().is_some_and(|(_, items)| items_contain_test(items))
        }
        _ => false,
    })
}

fn attrs_contain_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

fn is_test_attr(attr: &Attribute) -> bool {
    attr.path().segments.last().is_some_and(|segment| {
        matches!(segment.ident.to_string().as_str(), "test" | "wasm_bindgen_test")
    })
}

fn rust_files_under(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)?;
    Ok(files)
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), DynError> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
    Ok(())
}

fn check_commit_message(path: &Path) -> Result<(), DynError> {
    let text = std::fs::read_to_string(path)?;
    let lines = text.lines().map(str::trim_end).collect::<Vec<_>>();
    let non_comment = lines
        .iter()
        .copied()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();

    let Some(subject) = non_comment.first().copied() else {
        return commit_message_rejected(&["empty commit message".to_string()]);
    };

    if MERGE_PREFIXES.iter().any(|prefix| subject.starts_with(prefix)) {
        return Ok(());
    }

    let mut failures = Vec::new();
    if subject.len() > 72 {
        failures.push(format!("subject is {} characters; keep it <= 72", subject.len()));
    }
    if !subject_has_required_shape(subject) {
        failures.push("subject must look like '<scope>: <imperative summary>'".to_string());
    }
    if subject.ends_with('.') {
        failures.push("subject must not end with a period".to_string());
    }

    let summary =
        subject.split_once(':').map_or(subject, |(_, summary)| summary).trim().to_ascii_lowercase();
    if matches!(
        summary.as_str(),
        "fix" | "update" | "changes" | "misc" | "wip" | "cleanup" | "stuff"
    ) {
        failures.push("subject is too vague; name the causal change".to_string());
    }

    let ai_assisted = AI_MARKERS.iter().any(|marker| text.contains(marker));
    let has_receipt = non_comment.iter().any(|line| line.starts_with("Repo-Respect-Receipt:"));
    if ai_assisted && !has_receipt {
        failures.push(
            "AI-assisted commits require 'Repo-Respect-Receipt: <id-or-path>' trailer".to_string(),
        );
    }

    if failures.is_empty() {
        println!("Code Dojo: commit message clean");
        Ok(())
    } else {
        commit_message_rejected(&failures)
    }
}

fn subject_has_required_shape(subject: &str) -> bool {
    let Some((scope, summary)) = subject.split_once(": ") else {
        return false;
    };
    valid_scope(scope) && valid_summary(summary)
}

fn valid_scope(scope: &str) -> bool {
    !scope.is_empty() && scope.split('/').all(valid_scope_segment)
}

fn valid_scope_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
}

fn valid_summary(summary: &str) -> bool {
    let mut chars = summary.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.next().is_some()
        && !summary.contains('.')
        && !summary.ends_with('.')
}

fn commit_message_rejected(failures: &[String]) -> Result<(), DynError> {
    eprintln!("Code Dojo: commit message rejected");
    for failure in failures {
        eprintln!("  - {failure}");
    }
    eprintln!("\nExample: bunny-num: define checked Q32x32 multiplication");
    Err(DojoError::new("commit message rejected").into())
}

fn print_violations(header: &str, violations: &[Violation]) {
    eprintln!("{header}");
    for violation in violations {
        eprintln!("  {violation}");
    }
}

fn git_root() -> Result<PathBuf, DynError> {
    let output = Command::new("git").args(["rev-parse", "--show-toplevel"]).output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        return Ok(PathBuf::from(stdout.trim()));
    }

    Err(command_error("git rev-parse --show-toplevel", &output.stderr).into())
}

fn rust_sources(root: &Path, mode: Mode) -> Result<Vec<RustSource>, DynError> {
    let mut sources = Vec::new();
    for path in rust_files(root, mode)? {
        sources.push(RustSource { source: read_rust_source(root, mode, &path)?, path });
    }
    Ok(sources)
}

fn read_rust_source(root: &Path, mode: Mode, path: &Path) -> Result<String, DynError> {
    match mode {
        Mode::All => Ok(std::fs::read_to_string(root.join(path))?),
        Mode::Staged => read_staged_source(root, path),
    }
}

fn read_staged_source(root: &Path, path: &Path) -> Result<String, DynError> {
    let object = format!(":{}", path.to_string_lossy());
    let output = Command::new("git").args(["show", &object]).current_dir(root).output()?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)?);
    }

    Err(command_error("git show staged source", &output.stderr).into())
}

fn rust_files(root: &Path, mode: Mode) -> Result<Vec<PathBuf>, DynError> {
    let args = match mode {
        Mode::All => vec!["ls-files", "--cached", "--others", "--exclude-standard"],
        Mode::Staged => vec!["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
    };
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(command_error("git file listing", &output.stderr).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut files = Vec::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let relative = PathBuf::from(line.trim());
        if !is_rust_source(&relative) {
            continue;
        }

        if matches!(mode, Mode::Staged) || root.join(&relative).is_file() {
            files.push(relative);
        }
    }
    Ok(files)
}

fn command_error(command: &str, stderr: &[u8]) -> DojoError {
    let detail = String::from_utf8_lossy(stderr);
    DojoError::new(format!("{command} failed: {}", detail.trim()))
}

fn is_rust_source(path: &Path) -> bool {
    if path.extension() != Some(OsStr::new("rs")) {
        return false;
    }

    !path.components().any(|component| {
        let part = component.as_os_str();
        part == OsStr::new("target")
            || part == OsStr::new(".git")
            || part == OsStr::new("vendor")
            || part == OsStr::new("third_party")
    })
}

fn crate_category(path: &Path) -> Category {
    if path.iter().any(|part| part == OsStr::new("tests")) {
        return Category::Tests;
    }

    match crate_name(path) {
        Some(name) if CORE_CRATES.contains(&name) => Category::Core,
        Some(name) if GENERATOR_CRATES.contains(&name) => Category::Generator,
        Some(name) if TOOLING_CRATES.contains(&name) => Category::Tooling,
        _ => Category::Unknown,
    }
}

fn crate_name(path: &Path) -> Option<&str> {
    let mut parts = path.iter().filter_map(OsStr::to_str);
    match parts.next() {
        Some("crates") => parts.next(),
        Some("xtask") => Some("xtask"),
        _ => None,
    }
}

fn check_textual_limits(violations: &mut Vec<Violation>, context: &FileContext<'_>) {
    let limits = context.limits;
    if let Some(limit) = limits.file_lines {
        let count = source_line_count(context.lines.iter().map(String::as_str));
        if count > limit {
            violations.push(Violation::new(
                context.path.clone(),
                1,
                "file-size",
                format!(
                    "{} file has {count} source lines; limit is {limit}",
                    context.category.name()
                ),
            ));
        }
    }

    if let Some(limit) = limits.line_length {
        for (index, line) in context.lines.iter().enumerate() {
            if line.len() <= limit || line.contains("http://") || line.contains("https://") {
                continue;
            }
            if has_waiver(context.lines, index, "line-length") {
                continue;
            }
            violations.push(Violation::new(
                context.path.clone(),
                index + 1,
                "line-length",
                format!("line is {} chars; limit is {limit}", line.len()),
            ));
        }
    }
}

fn source_line_count<'a>(lines: impl Iterator<Item = &'a str>) -> usize {
    lines
        .filter(|line| {
            let stripped = line.trim();
            !stripped.is_empty() && !stripped.starts_with("//")
        })
        .count()
}

fn check_ast(violations: &mut Vec<Violation>, context: &FileContext<'_>) {
    match syn::parse_file(context.source) {
        Ok(file) => visit_file_ast(violations, context, &file),
        Err(error) => violations.push(Violation::new(
            context.path.clone(),
            line_of(error.span()),
            "rust-parse",
            format!("Rust parser rejected file: {error}"),
        )),
    }
}

fn visit_file_ast(violations: &mut Vec<Violation>, context: &FileContext<'_>, file: &File) {
    let mut visitor = PolicyVisitor {
        violations,
        path: context.path.clone(),
        category: context.category,
        limits: context.limits,
        lines: context.lines,
    };
    visitor.visit_file(file);
}

struct PolicyVisitor<'a> {
    violations: &'a mut Vec<Violation>,
    path: PathBuf,
    category: Category,
    limits: Limits,
    lines: &'a [String],
}

impl PolicyVisitor<'_> {
    fn violation(&mut self, line: usize, rule: &'static str, message: impl Into<String>) {
        self.violations.push(Violation::new(self.path.clone(), line, rule, message));
    }

    fn waived(&self, line: usize, rule: &str) -> bool {
        has_waiver(self.lines, line.saturating_sub(1), rule)
    }

    fn check_function(&mut self, span: Span, signature: &Signature, body: Option<&Block>) {
        let start_line = line_of(span);
        if let Some(limit) = self.limits.parameters {
            let params = parameter_count(signature);
            if params > limit {
                self.violation(
                    start_line,
                    "parameter-count",
                    format!("function has {params} parameters; limit is {limit}"),
                );
            }
        }

        let Some(block) = body else {
            return;
        };

        if let Some(limit) = self.limits.function_lines {
            let lines = source_line_count_in_span(self.lines, span);
            if lines > limit {
                self.violation(
                    start_line,
                    "function-size",
                    format!("function has {lines} source lines; limit is {limit}"),
                );
            }
        }

        let metrics = FunctionMetrics::measure(block);
        if let Some(limit) = self.limits.statements {
            if metrics.statements > limit {
                self.violation(
                    start_line,
                    "statement-count",
                    format!("function has {} AST statements; limit is {limit}", metrics.statements),
                );
            }
        }
        if let Some(limit) = self.limits.nesting_depth {
            if metrics.nesting_depth > limit {
                self.violation(
                    start_line,
                    "nesting-depth",
                    format!(
                        "function nesting depth is {}; limit is {limit}",
                        metrics.nesting_depth
                    ),
                );
            }
        }
        if let Some(limit) = self.limits.cyclomatic {
            if metrics.cyclomatic > limit {
                self.violation(
                    start_line,
                    "complexity",
                    format!(
                        "approximate cyclomatic complexity is {}; limit is {limit}",
                        metrics.cyclomatic
                    ),
                );
            }
        }
    }

    fn check_path_call(&mut self, line: usize, path: &SynPath) {
        if self.category != Category::Core {
            return;
        }

        let segments = path_segments(path);
        let banned = if has_suffix(&segments, &["SystemTime", "now"])
            || has_suffix(&segments, &["Instant", "now"])
        {
            Some(("ambient-state", "ambient clock access is banned; inject a deterministic clock"))
        } else if is_single_segment(&segments, "thread_rng")
            || has_suffix(&segments, &["rand", "thread_rng"])
        {
            Some(("ambient-state", "thread_rng is banned; inject a seeded generator"))
        } else if has_suffix(&segments, &["rand", "random"]) {
            Some(("ambient-state", "rand::random is banned; inject a seeded generator"))
        } else if has_suffix(&segments, &["std", "env", "args"])
            || has_suffix(&segments, &["std", "env", "var"])
        {
            Some((
                "ambient-state",
                "ambient environment/process access is banned in deterministic core crates",
            ))
        } else if is_filesystem_path(&segments) {
            Some(("ambient-state", "filesystem access is banned in deterministic core crates"))
        } else if is_nondeterministic_map_constructor(&segments) {
            Some((
                "nondeterministic-map",
                "HashMap/HashSet constructors are banned in canonical core output paths",
            ))
        } else if has_suffix(&segments, &["TcpStream", "connect"])
            || has_suffix(&segments, &["UdpSocket", "bind"])
        {
            Some(("ambient-state", "network access is banned in deterministic core crates"))
        } else {
            None
        };

        if let Some((rule, message)) = banned {
            if !self.waived(line, rule) {
                self.violation(line, rule, message);
            }
        }
    }

    fn check_type_path(&mut self, line: usize, path: &SynPath) {
        if self.category != Category::Core {
            return;
        }

        let segments = path_segments(path);
        if segments.iter().any(|segment| segment == "HashMap" || segment == "HashSet")
            && !self.waived(line, "nondeterministic-map")
        {
            self.violation(
                line,
                "nondeterministic-map",
                "HashMap/HashSet are banned in canonical core output paths; use BTree* or sorted Vec",
            );
        }

        if segments.iter().any(|segment| segment == "f32" || segment == "f64")
            && !self.waived(line, "float-boundary")
        {
            self.violation(
                line,
                "float-boundary",
                "native floats in core require a boundary/proof waiver and conversion to canonical fixed-point",
            );
        }
    }

    fn check_float_literal(&mut self, span: Span) {
        if self.category != Category::Core {
            return;
        }

        let line = line_of(span);
        if !self.waived(line, "float-boundary") {
            self.violation(
                line,
                "float-boundary",
                "inferred float literals in core require explicit boundary conversion to canonical fixed-point",
            );
        }
    }

    fn check_macro(&mut self, span: Span, mac: &Macro) {
        let line = line_of(span);
        if self.limits.panics_allowed || self.waived(line, "panic-path") {
            return;
        }

        let segments = path_segments(&mac.path);
        if let Some(name) = segments.last() {
            if is_panicking_macro(name) {
                self.violation(
                    line,
                    "panic-path",
                    format!("{name}! is banned in library/core code"),
                );
            }
        }
    }
}

impl<'ast> Visit<'ast> for PolicyVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.check_function(node.span(), &node.sig, Some(&node.block));
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.check_function(node.span(), &node.sig, Some(&node.block));
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        self.check_function(node.span(), &node.sig, node.default.as_ref());
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_expr_index(&mut self, node: &'ast ExprIndex) {
        let line = line_of(node.span());
        if !self.limits.panics_allowed && !self.waived(line, "indexing") {
            self.violation(
                line,
                "indexing",
                "unchecked indexing requires a local proof waiver or .get()/.first()",
            );
        }
        visit::visit_expr_index(self, node);
    }

    fn visit_expr_lit(&mut self, node: &'ast ExprLit) {
        if matches!(node.lit, Lit::Float(_)) {
            self.check_float_literal(node.span());
        }
        visit::visit_expr_lit(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let line = line_of(node.span());
        let method = node.method.to_string();
        if !self.limits.panics_allowed
            && (method == "unwrap" || method == "expect")
            && !self.waived(line, "panic-path")
        {
            self.violation(
                line,
                "panic-path",
                format!("{method}() is banned in library/core code"),
            );
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        self.check_macro(node.span(), &node.mac);
        visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast StmtMacro) {
        self.check_macro(node.span(), &node.mac);
        visit::visit_stmt_macro(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast ExprPath) {
        self.check_path_call(line_of(node.span()), &node.path);
        visit::visit_expr_path(self, node);
    }

    fn visit_type_path(&mut self, node: &'ast TypePath) {
        self.check_type_path(line_of(node.span()), &node.path);
        visit::visit_type_path(self, node);
    }
}

struct FunctionMetrics {
    statements: usize,
    nesting_depth: usize,
    cyclomatic: usize,
}

impl FunctionMetrics {
    fn measure(block: &Block) -> Self {
        let mut visitor =
            MetricVisitor { statements: 0, nesting_depth: 0, cyclomatic: 1, current_depth: 0 };
        visitor.visit_block(block);
        Self {
            statements: visitor.statements,
            nesting_depth: visitor.nesting_depth,
            cyclomatic: visitor.cyclomatic,
        }
    }
}

struct MetricVisitor {
    statements: usize,
    nesting_depth: usize,
    cyclomatic: usize,
    current_depth: usize,
}

impl MetricVisitor {
    fn enter_branch(&mut self) {
        self.cyclomatic += 1;
        self.current_depth += 1;
        self.nesting_depth = self.nesting_depth.max(self.current_depth);
    }

    fn leave_branch(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }
}

impl<'ast> Visit<'ast> for MetricVisitor {
    fn visit_stmt(&mut self, node: &'ast Stmt) {
        if !matches!(node, Stmt::Item(_)) {
            self.statements += 1;
        }
        visit::visit_stmt(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        self.enter_branch();
        visit::visit_expr_if(self, node);
        self.leave_branch();
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        self.enter_branch();
        visit::visit_expr_match(self, node);
        self.leave_branch();
    }

    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        self.enter_branch();
        visit::visit_expr_for_loop(self, node);
        self.leave_branch();
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        self.enter_branch();
        visit::visit_expr_while(self, node);
        self.leave_branch();
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.enter_branch();
        visit::visit_expr_loop(self, node);
        self.leave_branch();
    }

    fn visit_bin_op(&mut self, node: &'ast BinOp) {
        if matches!(node, BinOp::And(_) | BinOp::Or(_)) {
            self.cyclomatic += 1;
        }
        visit::visit_bin_op(self, node);
    }
}

fn parameter_count(signature: &Signature) -> usize {
    signature.inputs.iter().filter(|input| !matches!(input, syn::FnArg::Receiver(_))).count()
}

fn source_line_count_in_span(lines: &[String], span: Span) -> usize {
    let start = line_of(span);
    let end = span.end().line;
    lines
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let line = index + 1;
            line >= start && line <= end
        })
        .map(|(_, line)| line.as_str())
        .pipe(source_line_count)
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

fn has_waiver(lines: &[String], index: usize, rule: &str) -> bool {
    let needle = format!("dojo: allow {rule}");
    let start = index.saturating_sub(2);
    for line_index in start..=index {
        let Some(text) = lines.get(line_index) else {
            continue;
        };
        if !text.contains(&needle) {
            continue;
        }
        if let Some((_, reason)) = text.split_once("--") {
            return reason.trim().len() >= 8;
        }
    }
    false
}

fn path_segments(path: &SynPath) -> Vec<String> {
    path.segments.iter().map(|segment| segment.ident.to_string()).collect()
}

fn has_suffix(path: &[String], suffix: &[&str]) -> bool {
    path.len() >= suffix.len()
        && path.iter().rev().zip(suffix.iter().rev()).all(|(left, right)| left.as_str() == *right)
}

fn has_prefix(path: &[String], prefix: &[&str]) -> bool {
    path.len() >= prefix.len()
        && path.iter().zip(prefix.iter()).all(|(left, right)| left.as_str() == *right)
}

fn is_filesystem_path(path: &[String]) -> bool {
    has_suffix(path, &["File", "open"])
        || has_prefix(path, &["std", "fs"])
        || has_prefix(path, &["fs"])
}

fn is_nondeterministic_map_constructor(path: &[String]) -> bool {
    let [.., collection, constructor] = path else {
        return false;
    };
    constructor == "new" && matches!(collection.as_str(), "HashMap" | "HashSet")
}

fn is_panicking_macro(name: &str) -> bool {
    matches!(
        name,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "debug_assert"
            | "debug_assert_eq"
            | "debug_assert_ne"
            | "panic"
            | "todo"
            | "unimplemented"
            | "unreachable"
    )
}

fn is_single_segment(path: &[String], segment: &str) -> bool {
    path.len() == 1 && path.first().is_some_and(|value| value == segment)
}

fn line_of(span: Span) -> usize {
    span.start().line
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
                .join(format!("bunny-xtask-{name}-{}-{count}", std::process::id()));
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
        let output =
            Command::new("git").args(args).current_dir(root).output().expect("git should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn core_policy_violations(source: &str) -> Vec<Violation> {
        let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let path = PathBuf::from("crates/bunny-num/src/lib.rs");
        let category = Category::Core;
        let context =
            FileContext { path, category, limits: category.limits(), source, lines: &lines };
        let mut violations = Vec::new();
        check_textual_limits(&mut violations, &context);
        check_ast(&mut violations, &context);
        violations
    }

    fn has_rule(violations: &[Violation], rule: &str) -> bool {
        violations.iter().any(|violation| violation.rule == rule)
    }

    fn rule_count(violations: &[Violation], rule: &str) -> usize {
        violations.iter().filter(|violation| violation.rule == rule).count()
    }

    #[test]
    fn staged_rust_sources_are_read_from_index() {
        let temp = TempDir::new("staged-index");
        run_git(temp.path(), &["init"]);
        let relative = PathBuf::from("crates/bunny-num/src/lib.rs");
        let source_path = temp.path().join(&relative);
        fs::create_dir_all(source_path.parent().expect("fixture source should have a parent"))
            .expect("fixture source directory should be created");
        fs::write(&source_path, "pub fn staged() { panic!(\"staged\"); }\n")
            .expect("staged fixture should be written");
        run_git(temp.path(), &["add", "crates/bunny-num/src/lib.rs"]);
        fs::write(&source_path, "pub fn staged() {}\n")
            .expect("worktree fixture should be written");

        let sources = rust_sources(temp.path(), Mode::Staged).expect("staged sources should load");
        let staged = sources
            .iter()
            .find(|source| source.path == relative)
            .expect("staged source should be present");

        assert!(staged.source.contains("panic!(\"staged\")"));
    }

    #[test]
    fn core_policy_flags_std_fs_helper_calls() {
        let source = r"
pub fn read_path(path: &std::path::Path) {
    let _bytes = std::fs::read(path);
}
";

        let violations = core_policy_violations(source);

        assert!(has_rule(&violations, "ambient-state"));
    }

    #[test]
    fn core_policy_flags_inferred_hash_map_and_set_constructors() {
        let source = r"
use std::collections::{HashMap, HashSet};

pub fn build() {
    let mut map = HashMap::new();
    let mut set = HashSet::new();
    let _old = map.insert(1, 2);
    let _inserted = set.insert(1);
}
";

        let violations = core_policy_violations(source);

        assert_eq!(rule_count(&violations, "nondeterministic-map"), 2);
    }

    #[test]
    fn core_policy_flags_assertion_and_unreachable_macros() {
        let source = r"
pub fn require(value: i32) {
    assert!(value > 0);
    debug_assert!(value < 100);
    unreachable!();
}
";

        let violations = core_policy_violations(source);

        assert_eq!(rule_count(&violations, "panic-path"), 3);
    }

    #[test]
    fn core_policy_flags_inferred_float_literals() {
        let source = r"
pub fn scale(input: i32) -> i32 {
    let gain = 0.5;
    if gain > 0.0 { input } else { 0 }
}
";

        let violations = core_policy_violations(source);

        assert_eq!(rule_count(&violations, "float-boundary"), 2);
    }

    #[test]
    fn deterministic_receipts_require_real_tests() {
        let temp = TempDir::new("determinism-receipt");
        let tests_dir = temp.path().join("tests");
        fs::create_dir_all(&tests_dir).expect("tests directory should be created");
        let receipt = tests_dir.join("determinism.rs");

        fs::write(&receipt, "// placeholder\n").expect("placeholder receipt should be written");
        assert!(!crate_has_determinism_receipt(temp.path()).expect("receipt check should run"));

        fs::write(&receipt, "#[test]\nfn golden_vector_is_stable() {}\n")
            .expect("test receipt should be written");
        assert!(crate_has_determinism_receipt(temp.path()).expect("receipt check should run"));
    }
}
