//! Macro metavariables in the positions where they build code the gate cannot read.
//!
//! The source rules match paths, macro names, method names and `#[path]` values as they
//! are written. In a `macro_rules!` transcriber a metavariable can supply any of them at
//! the invocation (`std::time::$t::now()` with `$t` = `Instant`, `$m!($p)` with `$m` =
//! `include_str`, `#[path = $p]`), and the invocation's own tokens then hold only an
//! identifier or a literal, so neither the definition nor the call shows the banned form.
//! A lexical gate cannot know what the metavariable will be, so in a deterministic crate
//! these positions are banned outright (`macro-metavariable`):
//!
//! - next to a path separator: `$t::now`, `std::$m`, `std::$($s)::*`, or a repetition
//!   whose separator is `::`;
//! - the name of a macro call: `$m!(..)`, `$m![..]`, `$m! {..}`;
//! - the name of a method call: `$x.$m(..)`, `$x.$m::<T>(..)`;
//! - anywhere inside an attribute except as the value of `doc = ..`: `#[path = $p]`,
//!   `#[$meta]`, `#![$meta]`, `#[cfg_attr(.., path = $p)]`.
//!
//! `$crate` names the defining crate, which the gate scans, and is accepted everywhere.
//! The matchers of a `macro_rules!` definition are patterns, not code, and are not read.

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use syn::Macro;

use super::{ident_name, line_of};

pub(super) const RULE: &str = "macro-metavariable";
pub(super) const MESSAGE: &str = "a macro metavariable in a path, macro-name, method-name or \
     attribute position builds code the gate cannot read; write the full path in the macro";

/// The lines of every metavariable in a banned position in a macro's tokens: the
/// transcribers of a `macro_rules!` definition, or the whole of any other invocation.
pub(super) fn lines(mac: &Macro) -> Vec<usize> {
    let mut found = Vec::new();
    if mac.path.is_ident("macro_rules") {
        transcribers(&mac.tokens, &mut found);
    } else {
        scan(&mac.tokens, &mut found);
    }
    found
}

fn scan(tokens: &TokenStream, found: &mut Vec<usize>) {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        if let Some(body) = macro_rules_body(&tokens, index) {
            transcribers(&body, found);
            index += 4;
            continue;
        }
        match token {
            TokenTree::Group(group) => scan(&group.stream(), found),
            TokenTree::Punct(punct) if punct.as_char() == '$' => {
                if banned_position(&tokens, index) {
                    found.push(line_of(punct.span()));
                }
            }
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(line) = attribute_metavariable(&tokens, index) {
                    found.push(line);
                }
            }
            TokenTree::Punct(_) | TokenTree::Ident(_) | TokenTree::Literal(_) => {}
        }
        index += 1;
    }
}

/// The body of a `macro_rules! name { .. }` that starts at `tokens[index]`.
fn macro_rules_body(tokens: &[TokenTree], index: usize) -> Option<TokenStream> {
    match (tokens.get(index), tokens.get(index + 1), tokens.get(index + 2), tokens.get(index + 3)) {
        (
            Some(TokenTree::Ident(keyword)),
            Some(TokenTree::Punct(bang)),
            Some(TokenTree::Ident(_)),
            Some(TokenTree::Group(body)),
        ) if keyword == "macro_rules" && bang.as_char() == '!' => Some(body.stream()),
        _ => None,
    }
}

/// Scans each rule's transcriber, the group after `=>`, and skips its matcher.
fn transcribers(body: &TokenStream, found: &mut Vec<usize>) {
    let tokens = body.clone().into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        if let (TokenTree::Group(transcriber), Some(index)) = (token, index.checked_sub(2)) {
            if is_arrow(&tokens, index) {
                scan(&transcriber.stream(), found);
            }
        }
    }
}

fn is_arrow(tokens: &[TokenTree], index: usize) -> bool {
    matches!(
        (tokens.get(index), tokens.get(index + 1)),
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second)))
            if first.as_char() == '=' && first.spacing() == Spacing::Joint && second.as_char() == '>'
    )
}

/// Whether the `$` at `tokens[index]` starts a metavariable or a repetition in a banned
/// position: next to `::`, as a macro name or as a method name.
fn banned_position(tokens: &[TokenTree], index: usize) -> bool {
    let after = match tokens.get(index + 1) {
        Some(TokenTree::Ident(name)) if ident_name(name) == "crate" => return false,
        Some(TokenTree::Ident(_)) => index + 2,
        Some(TokenTree::Group(_)) => repetition_end(tokens, index + 2),
        _ => return false,
    };
    let after_separator = index >= 2 && is_path_separator(tokens, index - 2);
    let before_separator = is_path_separator(tokens, after);
    let macro_name =
        is_bang(tokens.get(after)) && matches!(tokens.get(after + 1), Some(TokenTree::Group(_)));
    let method_name = index >= 1
        && is_dot(tokens.get(index - 1))
        && matches!(tokens.get(after), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis);
    after_separator || before_separator || macro_name || method_name
}

/// The index after a repetition's optional separator and its `*`, `+` or `?`; a separator
/// of `::` is returned as the position, so it reads as a path separator.
fn repetition_end(tokens: &[TokenTree], at: usize) -> usize {
    if is_path_separator(tokens, at) {
        return at;
    }
    match tokens.get(at) {
        Some(TokenTree::Punct(op)) if matches!(op.as_char(), '*' | '+' | '?') => at + 1,
        Some(_) => at + 2,
        None => at,
    }
}

/// The line of an attribute at `tokens[index]` (`#[..]` or `#![..]`) that holds a
/// metavariable anywhere but as the value of `doc = ..`.
fn attribute_metavariable(tokens: &[TokenTree], index: usize) -> Option<usize> {
    let body = match (tokens.get(index + 1), tokens.get(index + 2)) {
        (Some(TokenTree::Group(body)), _) => body,
        (Some(TokenTree::Punct(bang)), Some(TokenTree::Group(body))) if bang.as_char() == '!' => {
            body
        }
        _ => return None,
    };
    if body.delimiter() != Delimiter::Bracket || is_doc_value(&body.stream()) {
        return None;
    }
    if has_metavariable(&body.stream()) {
        tokens.get(index).map(|pound| line_of(pound.span()))
    } else {
        None
    }
}

fn is_doc_value(tokens: &TokenStream) -> bool {
    let mut tokens = tokens.clone().into_iter();
    matches!(
        (tokens.next(), tokens.next()),
        (Some(TokenTree::Ident(name)), Some(TokenTree::Punct(eq)))
            if name == "doc" && eq.as_char() == '='
    )
}

fn has_metavariable(tokens: &TokenStream) -> bool {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => has_metavariable(&group.stream()),
        TokenTree::Punct(dollar) if dollar.as_char() == '$' => !matches!(
            tokens.get(index + 1),
            Some(TokenTree::Ident(name)) if ident_name(name) == "crate"
        ),
        TokenTree::Punct(_) | TokenTree::Ident(_) | TokenTree::Literal(_) => false,
    })
}

fn is_path_separator(tokens: &[TokenTree], index: usize) -> bool {
    matches!(
        (tokens.get(index), tokens.get(index + 1)),
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second)))
            if first.as_char() == ':' && first.spacing() == Spacing::Joint && second.as_char() == ':'
    )
}

fn is_bang(token: Option<&TokenTree>) -> bool {
    matches!(token, Some(TokenTree::Punct(bang)) if bang.as_char() == '!')
}

fn is_dot(token: Option<&TokenTree>) -> bool {
    matches!(token, Some(TokenTree::Punct(dot)) if dot.as_char() == '.')
}
