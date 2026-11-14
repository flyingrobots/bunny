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
use syn::{
    Arm, Attribute, Field, FieldValue, File, ForeignItem, ImplItem, Item, Stmt, TraitItem, Variant,
};

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
        let has_code = code.iter().map(|line| line.contains(&true)).collect::<Vec<_>>();
        let nodes = file.map(|file| node_ends(file, &code)).unwrap_or_default();
        let entries = comments(&lines, &code)
            .into_iter()
            .filter_map(|(index, text)| {
                let rule = clause_rule(&text)?;
                let (first, last) = coverage(index, &has_code, &nodes);
                Some(Waiver { rule, first, last })
            })
            .collect();
        Self { entries }
    }

    /// Whether some waiver for `rule` covers the 1-based `line`.
    pub(super) fn covers(&self, line: usize, rule: &str) -> bool {
        self.entries
            .iter()
            .any(|waiver| waiver.rule == rule && waiver.first <= line && line <= waiver.last)
    }
}

/// Whether a manifest waives `rule`: a whole-line TOML comment holding one clause. Lines
/// inside a multi-line basic (`"""`) or literal (`'''`) string are string content.
pub(super) fn manifest_waives(manifest: &str, rule: &str) -> bool {
    let mut open = None::<&str>;
    for line in manifest.lines() {
        if let Some(delimiter) = open {
            if line.matches(delimiter).count() % 2 == 1 {
                open = None;
            }
            continue;
        }
        if let Some(text) = line.trim_start().strip_prefix('#') {
            if clause_body_rule(text).is_some_and(|named| named == rule) {
                return true;
            }
            continue;
        }
        open = ["\"\"\"", "'''"]
            .into_iter()
            .find(|delimiter| line.matches(delimiter).count() % 2 == 1);
    }
    false
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

/// Every comment of the source with the 0-based line it starts on, each whole: a block
/// comment over several lines is one comment. Text no token covers is whitespace and
/// comments, so the runs of it between tokens are split into comments.
fn comments(lines: &[Vec<char>], code: &[Vec<bool>]) -> Vec<(usize, String)> {
    let mut runs = Vec::<(usize, String)>::new();
    let mut current = None::<(usize, String)>;
    for (index, (line, mask)) in lines.iter().zip(code).enumerate() {
        for (character, is_code) in line.iter().zip(mask) {
            if *is_code {
                runs.extend(current.take());
            } else {
                current.get_or_insert_with(|| (index, String::new())).1.push(*character);
            }
        }
        current.get_or_insert_with(|| (index, String::new())).1.push('\n');
    }
    runs.extend(current);
    runs.iter().flat_map(|(line, text)| split_comments(*line, text)).collect()
}

/// Splits a run of whitespace and comments into its line and block comments, nested block
/// comments included.
fn split_comments(first_line: usize, run: &str) -> Vec<(usize, String)> {
    let chars = run.chars().collect::<Vec<_>>();
    let mut found = Vec::new();
    let (mut line, mut at) = (first_line, 0);
    while let Some(character) = chars.get(at) {
        let end = match (character, chars.get(at + 1)) {
            ('/', Some('/')) => {
                chars.iter().skip(at).position(|c| *c == '\n').map_or(chars.len(), |n| at + n)
            }
            ('/', Some('*')) => block_comment_end(&chars, at),
            ('\n', _) => {
                line += 1;
                at += 1;
                continue;
            }
            _ => {
                at += 1;
                continue;
            }
        };
        let comment = chars.get(at..end).unwrap_or_default();
        found.push((line, comment.iter().collect::<String>()));
        line += comment.iter().filter(|c| **c == '\n').count();
        at = end;
    }
    found
}

/// The index just past the `*/` that closes the block comment opening at `start`.
fn block_comment_end(chars: &[char], start: usize) -> usize {
    let (mut depth, mut at) = (0_usize, start);
    while let Some(first) = chars.get(at) {
        match (first, chars.get(at + 1)) {
            ('/', Some('*')) => {
                depth += 1;
                at += 2;
            }
            ('*', Some('/')) => {
                depth = depth.saturating_sub(1);
                at += 2;
                if depth == 0 {
                    return at;
                }
            }
            _ => at += 1,
        }
    }
    chars.len()
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
/// such node. A node starts both on its first line and on its first line that holds code
/// other than attributes, so a waiver between a function's doc comment and its `fn`
/// covers the function.
fn node_ends(file: &File, code: &[Vec<bool>]) -> BTreeMap<usize, usize> {
    let mut collector = NodeCollector::default();
    collector.visit_file(file);
    let mut attribute = code.iter().map(|line| vec![false; line.len()]).collect::<Vec<_>>();
    for span in &collector.attributes {
        mark_span(*span, &mut attribute);
    }
    let attribute_only = code
        .iter()
        .zip(&attribute)
        .map(|(code, attr)| code.iter().zip(attr).all(|(code, attr)| !code || *attr))
        .collect::<Vec<_>>();
    let mut ends = BTreeMap::new();
    for (start, end) in collector.nodes {
        let body = (start..=end).find(|line| {
            let index = line - 1;
            code.get(index).is_some_and(|row| row.contains(&true))
                && !attribute_only.get(index).copied().unwrap_or(true)
        });
        for line in [Some(start), body].into_iter().flatten() {
            let last = ends.entry(line).or_insert(end);
            *last = (*last).max(end);
        }
    }
    ends
}

#[derive(Default)]
struct NodeCollector {
    nodes: Vec<(usize, usize)>,
    attributes: Vec<Span>,
}

impl NodeCollector {
    fn record(&mut self, span: Span) {
        let (start, end): (LineColumn, LineColumn) = (span.start(), span.end());
        self.nodes.push((start.line, end.line));
    }
}

impl<'ast> Visit<'ast> for NodeCollector {
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        self.attributes.push(node.span());
        visit::visit_attribute(self, node);
    }

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
