//! Doc tests in deterministic crates, scanned with the same rules as the file around them:
//! `cargo test` compiles and runs them, so a clock or a file read in one is a
//! nondeterministic test like any other.
//!
//! A doc block is a run of `#[doc]` attributes on consecutive lines (`///`, `//!`, or one
//! `/** */`). Each fenced block in it that rustdoc treats as Rust (no info string, or only
//! rustdoc's own attributes such as `rust`, `no_run`, `ignore`, `should_panic`,
//! `compile_fail` or an edition) is read with its hidden `# ` lines restored, parsed as a
//! file or, failing that, as the body of `fn main`, and scanned; a finding is reported at
//! the source line of the doc comment that holds it. A block that does not parse either
//! way is skipped, since rustdoc would reject it too. Doc text brought in with
//! `#[doc = include_str!(..)]` is not read.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprLit, File, Lit, Meta};

use super::Finding;

/// Info-string words rustdoc reads as attributes of a Rust block.
const RUST_BLOCK_WORDS: &[&str] = &[
    "rust",
    "ignore",
    "no_run",
    "should_panic",
    "compile_fail",
    "test_harness",
    "standalone_crate",
    "edition2015",
    "edition2018",
    "edition2021",
    "edition2024",
];

/// One line of doc text and the source line it came from.
struct DocLine {
    source_line: usize,
    text: String,
}

/// The findings of every Rust doc test in `file`, at the source lines of their doc comments.
pub(super) fn findings(path: &Path, file: &File) -> Vec<Finding> {
    let mut collector = DocCollector::default();
    collector.visit_file(file);
    let mut found = Vec::new();
    for block in doc_blocks(collector.docs) {
        for code in rust_blocks(&block) {
            found.extend(scan_code(path, &code));
        }
    }
    found
}

/// Doc attributes with their first and last source lines and their text.
#[derive(Default)]
struct DocCollector {
    docs: Vec<(usize, usize, String)>,
}

impl<'ast> Visit<'ast> for DocCollector {
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        if let (true, Meta::NameValue(pair)) = (node.path().is_ident("doc"), &node.meta) {
            if let Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) = &pair.value {
                let span = node.span();
                self.docs.push((span.start().line, span.end().line, text.value()));
            }
        }
        visit::visit_attribute(self, node);
    }
}

/// Groups doc attributes on consecutive lines into blocks of lines.
fn doc_blocks(mut docs: Vec<(usize, usize, String)>) -> Vec<Vec<DocLine>> {
    docs.sort_by_key(|(start, _, _)| *start);
    let mut blocks: Vec<Vec<DocLine>> = Vec::new();
    let mut previous_end = None::<usize>;
    for (start, end, text) in docs {
        if previous_end.is_none_or(|last| start > last + 1) {
            blocks.push(Vec::new());
        }
        previous_end = Some(end);
        let Some(block) = blocks.last_mut() else {
            continue;
        };
        for (offset, line) in text.split('\n').enumerate() {
            block.push(DocLine { source_line: (start + offset).min(end), text: line.to_string() });
        }
    }
    blocks
}

/// The Rust code blocks of one doc block, each as its lines with hidden lines restored.
fn rust_blocks(block: &[DocLine]) -> Vec<Vec<DocLine>> {
    let mut blocks = Vec::new();
    let mut open = None::<(bool, Vec<DocLine>)>;
    for line in block {
        let trimmed = line.text.trim();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        match (&mut open, fence) {
            (None, true) => {
                open = Some((is_rust_info(trimmed.get(3..).unwrap_or_default()), Vec::new()));
            }
            (None, false) => {}
            (Some(_), true) => {
                if let Some((true, code)) = open.take() {
                    blocks.push(code);
                }
            }
            (Some((_, code)), false) => {
                code.push(DocLine { source_line: line.source_line, text: unhide(&line.text) });
            }
        }
    }
    blocks
}

fn is_rust_info(info: &str) -> bool {
    info.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|word| !word.is_empty())
        .all(|word| RUST_BLOCK_WORDS.contains(&word))
}

/// Rustdoc's hidden lines: `# code` is compiled as `code`, a lone `#` as an empty line.
fn unhide(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed == "#" {
        return String::new();
    }
    trimmed.strip_prefix("# ").map_or_else(|| line.to_string(), str::to_string)
}

fn scan_code(path: &Path, code: &[DocLine]) -> Vec<Finding> {
    let text = code.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n");
    let (parsed, offset) = match syn::parse_file(&text) {
        Ok(parsed) => (parsed, 0),
        Err(_) => match syn::parse_file(&format!("fn main() {{\n{text}\n}}")) {
            Ok(parsed) => (parsed, 1),
            Err(_) => return Vec::new(),
        },
    };
    super::source_findings(path, &parsed)
        .into_iter()
        .filter_map(|finding| {
            let line = finding.line.checked_sub(offset + 1).and_then(|index| code.get(index))?;
            Some(Finding { line: line.source_line, ..finding })
        })
        .collect()
}
