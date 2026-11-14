//! The regeneration check's comparison and its argument list. The end-to-end behaviour (the
//! generator run in a scratch directory, seeded drift failing the gate) is CG-TP-016, recorded
//! in the contract-generation test plan.

use super::{first_difference, generator_args};

#[test]
fn cg_tp_014_drift_is_reported_at_the_first_differing_line() {
    let committed = b"one\ntwo\nthree\n";
    let cases: [(&[u8], Option<usize>); 8] = [
        (b"one\ntwo\nthree\n", None),
        (b"One\ntwo\nthree\n", Some(1)),
        (b"one\ntwx\nthree\n", Some(2)),
        (b"one\ntwo\nthreE\n", Some(3)),
        (b"one\r\ntwo\nthree\n", Some(1)),
        (b"one\ntwo\nthree", Some(3)),
        (b"one\ntwo\nthree\nfour\n", Some(4)),
        (b"", Some(1)),
    ];
    for (fresh, expected) in cases {
        assert_eq!(
            first_difference(committed, fresh),
            expected,
            "{:?}",
            String::from_utf8_lossy(fresh)
        );
        assert_eq!(first_difference(fresh, committed), expected, "reversed");
    }
}

#[test]
fn cg_tp_015_the_check_regenerates_what_generate_writes() {
    assert_eq!(
        generator_args(),
        [
            "schemas/bunny/v0/graphics.graphql",
            "--rust",
            "crates/bunny-contract/src/generated/graphics.rs",
            "--typescript",
            "generated/typescript/bunny-graphics.ts",
            "--manifest",
            "generated/bunny-graphics.manifest.json",
        ]
    );
}
