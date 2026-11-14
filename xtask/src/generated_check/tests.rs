//! The regeneration check's comparison and its argument list. The end-to-end behaviour (the
//! generator run in a scratch directory, seeded drift failing the gate) is CG-TP-016, recorded
//! in the contract-generation test plan.

use std::fs;
use std::path::{Path, PathBuf};

use super::{compare, first_difference, generator_args, read_snapshot, with_scratch, Drift};
use super::{Snapshot, OUTPUTS, SCHEMA};
use crate::git_helpers::git_command;

/// A directory removed when the test ends; `name` keeps parallel tests apart.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bunny-xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = git_command(root).args(args).output().expect("git runs");
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

fn write(root: &Path, path: &str, text: &str) {
    let full = root.join(path);
    fs::create_dir_all(full.parent().expect("a parent")).expect("directories");
    fs::write(full, text).expect("written");
}

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

#[test]
fn cg_tp_017_the_pre_commit_check_judges_the_staged_artifacts() {
    // A repository whose staged artifacts are the regenerated ones while the working tree holds
    // a stale TypeScript file, and one where the staged TypeScript file is stale while the
    // working tree holds the regenerated one: the index must decide, not the working tree.
    let repo = TempDir::new("generated-staged");
    let scratch = TempDir::new("generated-staged-scratch");
    git(&repo.0, &["init", "-q"]);
    write(&repo.0, SCHEMA, "staged schema\n");
    for (_, path) in OUTPUTS {
        write(&repo.0, path, &format!("fresh {path}\n"));
        write(&scratch.0, path, &format!("fresh {path}\n"));
    }
    git(&repo.0, &["add", "-A"]);
    write(&repo.0, SCHEMA, "edited, not staged\n");
    let typescript = OUTPUTS[1].1;
    write(&repo.0, typescript, "stale\n");
    let stale = vec![Drift { path: typescript, line: Some(1) }];

    let staged_schema = read_snapshot(&repo.0, Snapshot::Index, SCHEMA).expect("reads");
    assert_eq!(staged_schema.as_deref(), Some(&b"staged schema\n"[..]), "the generator's input");
    assert_eq!(compare(&repo.0, Snapshot::Index, &scratch.0).expect("compares"), vec![]);
    assert_eq!(compare(&repo.0, Snapshot::WorkTree, &scratch.0).expect("compares"), stale);

    // The other way round: stale staged, fresh on disk, and an artifact not staged at all.
    git(&repo.0, &["add", typescript]);
    write(&repo.0, typescript, &format!("fresh {typescript}\n"));
    git(&repo.0, &["rm", "-q", "--cached", OUTPUTS[2].1]);
    let expected =
        vec![Drift { path: typescript, line: Some(1) }, Drift { path: OUTPUTS[2].1, line: None }];
    assert_eq!(compare(&repo.0, Snapshot::Index, &scratch.0).expect("compares"), expected);
    assert_eq!(compare(&repo.0, Snapshot::WorkTree, &scratch.0).expect("compares"), vec![]);
}

#[test]
fn cg_tp_018_overlapping_runs_keep_their_own_scratch_files() {
    // A second run that starts while the first is between regenerating and comparing (as when
    // the pre-commit hook runs during a pre-push gate in the same checkout) must not delete the
    // first run's files, and each run removes only its own directory when it ends.
    let repo = TempDir::new("generated-overlap");
    let outer = with_scratch(&repo.0, 1, |first| {
        fs::write(first.join("output"), "first run").expect("written");
        let inner = with_scratch(&repo.0, 2, |second| {
            assert_ne!(first, second, "each run has its own directory");
            Ok(second.to_path_buf())
        })
        .expect("the second run");
        assert!(!inner.exists(), "the second run removed its directory");
        let kept = fs::read_to_string(first.join("output")).expect("the first run's file");
        Ok((first.to_path_buf(), kept))
    })
    .expect("the first run");
    assert_eq!(outer.1, "first run");
    assert!(!outer.0.exists(), "the first run removed its directory");
}
