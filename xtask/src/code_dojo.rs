use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Command;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    BinOp, Block, Expr, ExprCall, ExprForLoop, ExprIf, ExprIndex, ExprLoop, ExprMacro, ExprMatch,
    ExprMethodCall, ExprPath, ExprWhile, File, ImplItemFn, ItemFn, Path as SynPath, Signature,
    Stmt, TraitItemFn, TypePath,
};

type DynError = Box<dyn Error>;

const CORE_CRATES: &[&str] =
    &["bunny-num", "bunny-linalg", "bunny-geom", "bunny-query", "bunny-broadphase", "bunny-mesh"];
const GENERATOR_CRATES: &[&str] = &["bunny-wesley"];
const TOOLING_CRATES: &[&str] = &["xtask"];
#[derive(Clone, Copy)]
enum Mode {
    All,
    Staged,
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

pub(super) fn handle(args: impl IntoIterator<Item = String>) -> Result<(), DynError> {
    let mode = parse_mode(args)?;
    let root = git_root()?;
    let mut violations = Vec::new();

    for path in rust_files(&root, mode)? {
        let source = std::fs::read_to_string(&path)?;
        let lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let relative = relative_path(&root, &path);
        let category = crate_category(&relative);
        let context = FileContext {
            path: relative,
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
    for violation in violations {
        eprintln!("  {violation}");
    }
    Err(DojoError::new("Code Dojo Rust AST policy violations found").into())
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

fn git_root() -> Result<PathBuf, DynError> {
    let output = Command::new("git").args(["rev-parse", "--show-toplevel"]).output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        return Ok(PathBuf::from(stdout.trim()));
    }

    Err(command_error("git rev-parse --show-toplevel", &output.stderr).into())
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

        let full_path = root.join(&relative);
        if full_path.is_file() {
            files.push(full_path);
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

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
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
        } else if has_suffix(&segments, &["File", "open"]) || has_suffix(&segments, &["std", "fs"])
        {
            Some(("ambient-state", "filesystem access is banned in deterministic core crates"))
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
        let line = line_of(node.span());
        if !self.limits.panics_allowed && !self.waived(line, "panic-path") {
            let segments = path_segments(&node.mac.path);
            if let Some(name) = segments.last() {
                if matches!(name.as_str(), "panic" | "todo" | "unimplemented") {
                    self.violation(
                        line,
                        "panic-path",
                        format!("{name}! is banned in library/core code"),
                    );
                }
            }
        }
        visit::visit_expr_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        let line = line_of(node.span());
        if let Expr::Path(path) = node.func.as_ref() {
            self.check_path_call(line, &path.path);
        }
        visit::visit_expr_call(self, node);
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

fn is_single_segment(path: &[String], segment: &str) -> bool {
    path.len() == 1 && path.first().is_some_and(|value| value == segment)
}

fn line_of(span: Span) -> usize {
    span.start().line
}
