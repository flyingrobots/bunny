//! The waiver contract, driven through the gate's per-file entry point
//! (`code_dojo::file_violations`) and dependency entry point
//! (`code_dojo::dependency_violations`).
//!
//! Oracle: each case is a hand-written source whose expected findings follow from the
//! contract in `docs/CODE_DOJO.md#waivers`: a waiver is a `//` or `/* */` comment holding
//! exactly one `dojo: allow <rule> -- <reason>` clause, and it covers the comment lines
//! below it and the item, statement, field, variant or match arm that starts on the next
//! line holding code. The expected lines are part of each case.

use std::path::PathBuf;

use crate::code_dojo::{dependency_violations, file_violations};

const CORE_PATH: &str = "crates/bunny-num/src/lib.rs";
const REASON: &str = "reviewed boundary adapter, issue 321";

/// The lines reported under `rule`, in order.
fn lines_of(rule: &str, source: &str) -> Vec<usize> {
    file_violations(PathBuf::from(CORE_PATH), source)
        .iter()
        .filter(|violation| violation.rule == rule)
        .map(|violation| violation.line)
        .collect()
}

#[test]
fn waiver_text_outside_a_comment_waives_nothing() {
    let read = "pub fn f() { let _ = std::fs::read(\"x\"); }";
    let cases = [
        (
            "string literal on the finding's line",
            format!("pub fn f() {{ let _ = std::fs::read(\"dojo: allow std-fs -- {REASON}\"); }}"),
            1,
        ),
        (
            "raw string on the line above",
            format!("pub const W: &str = r\"dojo: allow std-fs -- {REASON}\";\n{read}"),
            2,
        ),
        (
            "byte string on the line above",
            format!("pub const W: &[u8] = b\"dojo: allow std-fs -- {REASON}\";\n{read}"),
            2,
        ),
        (
            "line of a multi-line raw string",
            format!("pub const W: &str = r\"\n// dojo: allow std-fs -- {REASON}\n\";\n{read}"),
            4,
        ),
        ("outer doc comment", format!("/// dojo: allow std-fs -- {REASON}\n{read}"), 2),
        ("inner doc comment", format!("//! dojo: allow std-fs -- {REASON}\n{read}"), 2),
        ("block doc comment", format!("/** dojo: allow std-fs -- {REASON} */\n{read}"), 2),
        ("doc attribute", format!("#[doc = \"dojo: allow std-fs -- {REASON}\"]\n{read}"), 2),
        (
            "macro tokens",
            format!("macro_rules! m {{ ($($t:tt)*) => {{}} }}\nm!(dojo: allow std-fs -- {REASON});\n{read}"),
            3,
        ),
    ];
    let mut waived = Vec::new();
    for (label, source, line) in &cases {
        if !lines_of("std-fs", source).contains(line) {
            waived.push(*label);
        }
    }
    assert!(waived.is_empty(), "text outside a comment waived a finding: {waived:?}");
}

#[test]
fn line_and_block_comments_waive() {
    let read = "pub fn f() { let _ = std::fs::read(\"x\"); }";
    let cases = [
        ("line comment above", format!("// dojo: allow std-fs -- {REASON}\n{read}")),
        ("block comment above", format!("/* dojo: allow std-fs -- {REASON} */\n{read}")),
        ("comment on the finding's line", format!("{read} // dojo: allow std-fs -- {REASON}")),
    ];
    let mut reported = Vec::new();
    for (label, source) in &cases {
        if !lines_of("std-fs", source).is_empty() {
            reported.push(*label);
        }
    }
    assert!(reported.is_empty(), "comment waivers did not suppress: {reported:?}");
}

#[test]
fn a_comment_with_two_clauses_waives_nothing() {
    let source = format!(
        "// dojo: allow std-fs -- {REASON}; dojo: allow std-env -- {REASON}\n\
         pub fn f() {{ let _ = (std::fs::read(\"x\"), std::env::var(\"X\")); }}"
    );

    assert_eq!(
        (lines_of("std-fs", &source), lines_of("std-env", &source)),
        (vec![2], vec![2]),
        "a comment naming two rules must waive neither"
    );
}

#[test]
fn a_valid_waiver_after_a_refused_one_applies() {
    let source = format!(
        "// dojo: allow std-fs -- ok\n// dojo: allow std-fs -- {REASON}\n\
         pub fn f() {{ let _ = std::fs::read(\"x\"); }}"
    );

    assert_eq!(lines_of("std-fs", &source), Vec::<usize>::new());
}

#[test]
fn a_waiver_covers_only_the_next_statement() {
    let source = format!(
        "pub fn f() {{\n    // dojo: allow std-fs -- {REASON}\n    let _a = std::fs::read(\"a\");\n    \
         let _b = std::fs::read(\"b\");\n}}"
    );

    assert_eq!(lines_of("std-fs", &source), vec![4], "the second statement is not waived");
}

#[test]
fn a_waiver_covers_every_line_of_a_multi_line_statement() {
    let source = format!(
        "pub fn f() {{\n    // dojo: allow std-fs -- {REASON}\n    let _a = (\n        0,\n        \
         0,\n        std::fs::read(\"a\"),\n    );\n    let _b = std::fs::read(\"b\");\n}}"
    );

    assert_eq!(lines_of("std-fs", &source), vec![8], "line 6 is inside the waived statement");
}

#[test]
fn a_float_waiver_covers_its_item_and_nothing_after_it() {
    let source = "\
// dojo: allow float-boundary -- egress conversion from raw bits, reviewed
pub fn f(bits: u32) -> u32 {
    let a = 1;
    let b = 2;
    let _ = a + b;
    f32::from_bits(bits).to_bits()
}
pub fn g(bits: u32) -> u32 { f32::from_bits(bits).to_bits() }
";

    assert_eq!(lines_of("float-boundary", source), vec![8]);
}

#[test]
fn a_waiver_covers_the_comment_lines_below_it() {
    let source = "\
// dojo: allow hand-hashed-table -- reviewed: probe order never reaches output
// Open addressing keyed by vertex.
pub struct Table;
";

    assert_eq!(lines_of("hand-hashed-table", source), Vec::<usize>::new());
}

#[test]
fn stacked_waivers_each_cover_the_item() {
    let source = format!(
        "// dojo: allow std-fs -- {REASON}\n// dojo: allow std-env -- {REASON}\n\
         pub fn f() {{ let _ = (std::fs::read(\"x\"), std::env::var(\"X\")); }}"
    );

    assert_eq!((lines_of("std-fs", &source), lines_of("std-env", &source)), (vec![], vec![]));
}

#[test]
fn a_waiver_names_its_rule_exactly() {
    let source = format!(
        "// dojo: allow std-fs-extra -- {REASON}\npub fn f() {{ let _ = std::fs::read(\"x\"); }}"
    );

    assert_eq!(lines_of("std-fs", &source), vec![2]);
}

/// A `cargo tree` listing in which `bunny-num` reaches `rand`.
const TREE: &str = "0bunny-num v0.7.1 (/work/crates/bunny-num)\n1rand v0.8.5\n";

fn dependency_rules(manifest: &str) -> Vec<&'static str> {
    let manifest = manifest.to_string();
    dependency_violations(TREE, &|_crate_name: &str| manifest.clone())
        .iter()
        .map(|violation| violation.rule)
        .collect()
}

#[test]
fn manifest_waivers_are_whole_line_comments() {
    let comment = format!("# dojo: allow random -- {REASON}\n");
    let in_string = format!("description = \"# dojo: allow random -- {REASON}\"\n");
    let two_clauses =
        format!("# dojo: allow random -- {REASON}; dojo: allow network -- {REASON}\n");

    assert_eq!(
        (dependency_rules(&comment), dependency_rules(&in_string), dependency_rules(&two_clauses)),
        (vec![], vec!["random"], vec!["random"]),
    );
}

#[test]
fn a_waiver_after_code_covers_only_its_line() {
    let source = format!(
        "pub fn f() {{\n    let _a = std::fs::read(\"a\"); // dojo: allow std-fs -- {REASON}\n    \
         let _b = std::fs::read(\"b\");\n}}"
    );

    assert_eq!(lines_of("std-fs", &source), vec![3]);
}

#[test]
fn a_block_comment_over_two_lines_with_two_clauses_waives_nothing() {
    let source = format!(
        "/* dojo: allow std-fs -- {REASON}\n   dojo: allow std-env -- {REASON} */\npub fn f() {{\n    \
         let _ = std::fs::read(\"x\");\n    let _ = std::env::var(\"X\");\n}}"
    );

    assert_eq!((lines_of("std-fs", &source), lines_of("std-env", &source)), (vec![4], vec![5]));
}

#[test]
fn a_block_comment_over_two_lines_with_one_clause_waives_the_item_below() {
    let source = "/* dojo: allow std-fs -- reviewed fixture loader,\n   kept for the golden vectors */\n\
                  pub fn f() { let _ = std::fs::read(\"x\"); }";

    assert_eq!(lines_of("std-fs", source), Vec::<usize>::new());
}

#[test]
fn manifest_waiver_text_inside_a_multi_line_string_is_not_a_comment() {
    let basic = format!("[package.metadata]\nnote = \"\"\"\n# dojo: allow random -- {REASON}\n\"\"\"\n");
    let literal = format!("note = '''\n# dojo: allow random -- {REASON}\n'''\n");
    let closed = format!("note = \"\"\"one line\"\"\"\n# dojo: allow random -- {REASON}\n");

    assert_eq!(
        (dependency_rules(&basic), dependency_rules(&literal), dependency_rules(&closed)),
        (vec!["random"], vec!["random"], vec![]),
        "text in a multi-line basic or literal string waives nothing; a comment after a closed one does"
    );
}
