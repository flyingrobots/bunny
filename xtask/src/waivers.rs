//! Code Dojo waivers: which comments are waivers and which lines each one covers.
//!
//! A waiver is a `//` or `/* */` comment whose text is exactly one clause
//! `dojo: allow <rule> -- <reason>`, with a reason of at least eight bytes. Comments are
//! found from the token stream: whatever text of a line no token covers is a comment, so a
//! clause inside a string, raw-string, byte-string or character literal, a doc comment
//! (which is a `#[doc]` attribute), or a macro's tokens is not a waiver.
//!
//! A waiver on a line that holds code covers that line. A waiver on a line of its own
//! covers that line, the comment lines below it, and the outermost item, statement, field, variant,
//! match arm or struct-literal field that starts on the next line holding code, every
//! line of it; when no such node starts there, only that line.

use std::collections::BTreeMap;
use std::str::FromStr;

use proc_macro2::{LineColumn, Span, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Arm, Field, FieldValue, File, ForeignItem, ImplItem, Item, Stmt, TraitItem, Variant};

const CLAUSE: &str = "dojo: allow ";
const MIN_REASON_BYTES: usize = 8;

/// The waivers of one source file, each with the inclusive 1-based lines it covers.
pub(super) struct Waivers {
    entries: Vec<Waiver>,
}

struct Waiver {
    rule: String,
    first: usize,
    last: usize,
}

impl Waivers {
    /// Finds the waivers in `source`. `file` is its parse, used to find the node below
    /// each waiver; without it a waiver covers only the next line holding code.
    pub(super) fn parse(source: &str, file: Option<&File>) -> Self {
        let Ok(tokens) = TokenStream::from_str(source) else {
            return Self { entries: Vec::new() };
        };
        let lines = source.lines().map(|line| line.chars().collect::<Vec<_>>()).collect::<Vec<_>>();
        let mut code = lines.iter().map(|line| vec![false; line.len()]).collect::<Vec<_>>();
        mark_tokens(&tokens, &mut code);
        let nodes = file.map(node_ends).unwrap_or_default();
        let has_code = code.iter().map(|line| line.contains(&true)).collect::<Vec<_>>();
        let mut entries = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            for rule in comment_segments(line, &code[index]).filter_map(|text| clause_rule(&text)) {
                let (first, last) = coverage(index, &has_code, &nodes);
                entries.push(Waiver { rule, first, last });
            }
        }
        Self { entries }
    }

    /// Whether some waiver for `rule` covers the 1-based `line`.
    pub(super) fn covers(&self, line: usize, rule: &str) -> bool {
        self.entries
            .iter()
            .any(|waiver| waiver.rule == rule && waiver.first <= line && line <= waiver.last)
    }
}

/// Whether a manifest waives `rule`: a whole-line TOML comment holding one clause.
pub(super) fn manifest_waives(manifest: &str, rule: &str) -> bool {
    manifest
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('#'))
        .any(|text| clause_body_rule(text).is_some_and(|named| named == rule))
}

/// The lines a waiver on the 0-based line `index` covers, 1-based and inclusive.
fn coverage(index: usize, has_code: &[bool], nodes: &BTreeMap<usize, usize>) -> (usize, usize) {
    let line = index + 1;
    if has_code.get(index).copied().unwrap_or(false) {
        return (line, line);
    }
    let Some(target) =
        has_code.iter().enumerate().skip(index + 1).find(|(_, code)| **code).map(|(at, _)| at + 1)
    else {
        return (line, line);
    };
    let last = nodes.get(&target).copied().unwrap_or(target).max(target);
    (line, last)
}

/// The rule named by one comment's text, when it is a well-formed waiver.
fn clause_rule(comment: &str) -> Option<String> {
    let text = comment.trim();
    let body = if let Some(line) = text.strip_prefix("//") {
        line
    } else {
        let block = text.strip_prefix("/*")?;
        block.strip_suffix("*/").unwrap_or(block)
    };
    clause_body_rule(body).map(str::to_owned)
}

/// Parses `dojo: allow <rule> -- <reason>` with nothing before it and no second clause.
fn clause_body_rule(body: &str) -> Option<&str> {
    let rest = body.trim().strip_prefix(CLAUSE)?;
    let (rule, reason) = rest.split_once(" -- ")?;
    let rule_ok = !rule.is_empty()
        && rule
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    let reason = reason.trim();
    (rule_ok && reason.len() >= MIN_REASON_BYTES && !reason.contains(CLAUSE.trim_end()))
        .then_some(rule)
}

/// The maximal runs of a line that no token covers, which are whitespace and comments.
fn comment_segments<'a>(line: &'a [char], code: &'a [bool]) -> impl Iterator<Item = String> + 'a {
    let mut segments = Vec::new();
    let mut current = String::new();
    for (character, is_code) in line.iter().zip(code) {
        if *is_code {
            segments.push(std::mem::take(&mut current));
        } else {
            current.push(*character);
        }
    }
    segments.push(current);
    segments.into_iter().filter(|segment| !segment.trim().is_empty())
}

/// Marks the characters each token covers. A group contributes its delimiters, not its
/// whole span, so comments inside braces stay comments; a doc comment becomes tokens that
/// all carry the comment's span, so it is marked whole.
fn mark_tokens(tokens: &TokenStream, code: &mut [Vec<bool>]) {
    for token in tokens.clone() {
        match token {
            TokenTree::Group(group) => {
                mark_span(group.span_open(), code);
                mark_span(group.span_close(), code);
                mark_tokens(&group.stream(), code);
            }
            TokenTree::Ident(ident) => mark_span(ident.span(), code),
            TokenTree::Punct(punct) => mark_span(punct.span(), code),
            TokenTree::Literal(literal) => mark_span(literal.span(), code),
        }
    }
}

fn mark_span(span: Span, code: &mut [Vec<bool>]) {
    let (start, end) = (span.start(), span.end());
    for line in start.line..=end.line {
        let Some(row) = line.checked_sub(1).and_then(|index| code.get_mut(index)) else {
            continue;
        };
        let from = if line == start.line { start.column } else { 0 };
        let to = if line == end.line { end.column } else { row.len() };
        for cell in row.iter_mut().take(to).skip(from) {
            *cell = true;
        }
    }
}

/// For each line on which a node a waiver can cover starts, the last line of the longest
/// such node.
fn node_ends(file: &File) -> BTreeMap<usize, usize> {
    let mut collector = NodeCollector::default();
    collector.visit_file(file);
    collector.ends
}

#[derive(Default)]
struct NodeCollector {
    ends: BTreeMap<usize, usize>,
}

impl NodeCollector {
    fn record(&mut self, span: Span) {
        let (start, end): (LineColumn, LineColumn) = (span.start(), span.end());
        let last = self.ends.entry(start.line).or_insert(end.line);
        *last = (*last).max(end.line);
    }
}

impl<'ast> Visit<'ast> for NodeCollector {
    fn visit_item(&mut self, node: &'ast Item) {
        self.record(node.span());
        visit::visit_item(self, node);
    }

    fn visit_impl_item(&mut self, node: &'ast ImplItem) {
        self.record(node.span());
        visit::visit_impl_item(self, node);
    }

    fn visit_trait_item(&mut self, node: &'ast TraitItem) {
        self.record(node.span());
        visit::visit_trait_item(self, node);
    }

    fn visit_foreign_item(&mut self, node: &'ast ForeignItem) {
        self.record(node.span());
        visit::visit_foreign_item(self, node);
    }

    fn visit_stmt(&mut self, node: &'ast Stmt) {
        self.record(node.span());
        visit::visit_stmt(self, node);
    }

    fn visit_field(&mut self, node: &'ast Field) {
        self.record(node.span());
        visit::visit_field(self, node);
    }

    fn visit_variant(&mut self, node: &'ast Variant) {
        self.record(node.span());
        visit::visit_variant(self, node);
    }

    fn visit_arm(&mut self, node: &'ast Arm) {
        self.record(node.span());
        visit::visit_arm(self, node);
    }

    fn visit_field_value(&mut self, node: &'ast FieldValue) {
        self.record(node.span());
        visit::visit_field_value(self, node);
    }
}

#[cfg(test)]
mod tests;
