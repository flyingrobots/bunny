//! The dependency check run end to end through `cargo tree` on a fixture workspace, so the
//! feature sets and edge kinds the gate asks Cargo for are part of what is tested.
//!
//! Oracle: the fixture is built by hand. Its nine deterministic crates are named like
//! Bunny's; two path crates stand in for banned crates.io crates by taking their names
//! (`rand`, `serde_json`). `bunny-num` reaches `rand` only through an optional dependency
//! behind a feature that nothing enables, and `bunny-mesh` reaches `serde_json` only as a
//! build-dependency. Both must be reported, and nothing else. Since flyingrobots/bunny#349
//! the fixture also holds the issue's two forms: `bunny-geom` depends on
//! `crates/zz-probe-crate`, a workspace member outside the deterministic set, and
//! `bunny-num` has `build = "../../scripts/zz_build.rs"`. With `bunny-mesh`'s `build.rs`
//! and the stand-ins, which are workspace members no list names, each is reported once.
//!
//! Each run uses its own temporary directory as the workspace and as `CARGO_HOME`, and
//! Cargo runs `--offline`: the fixture has path dependencies only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::code_dojo::dependency_rule_violations;

const DETERMINISTIC: &[&str] = &[
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
    "bunny-optics",
    "bunny-codec",
    "bunny-contract",
];

static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A fixture workspace in its own temporary directory, removed on drop.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let count = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("bunny-xtask-dependency-graph-{}-{count}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture root should be created");
        Self { root }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("fixture directory should be created");
        fs::write(path, text).expect("fixture file should be written");
    }

    fn cargo(&self) -> Command {
        let mut command = Command::new("cargo");
        command.env("CARGO_HOME", self.root.join("cargo-home")).env("CARGO_NET_OFFLINE", "true");
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn package(name: &str, extra: &str) -> String {
    let (package_keys, tables) = match extra.strip_prefix("build = ") {
        Some(rest) => {
            let (build, tables) = rest.split_once('\n').unwrap_or((rest, ""));
            (format!("build = {build}\n"), tables.to_string())
        }
        None => (String::new(), extra.to_string()),
    };
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{package_keys}\n{tables}"
    )
}

/// Writes the workspace and its lockfile.
fn banned_behind_features_and_build_edges() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\", \"stand-ins/*\"]\nresolver = \"2\"\n",
    );
    for name in DETERMINISTIC {
        let extra = match *name {
            "bunny-num" => {
                "build = \"../../scripts/zz_build.rs\"\n\n\
                 [dependencies]\nrand = { path = \"../../stand-ins/rand\", optional = true }\n\n\
                 [features]\nnoise = [\"dep:rand\"]\n"
            }
            "bunny-geom" => "[dependencies]\nzz-probe-crate = { path = \"../zz-probe-crate\" }\n",
            "bunny-mesh" => {
                "[build-dependencies]\nserde_json = { path = \"../../stand-ins/serde_json\" }\n"
            }
            _ => "",
        };
        fixture.write(&format!("crates/{name}/Cargo.toml"), &package(name, extra));
        fixture.write(&format!("crates/{name}/src/lib.rs"), "");
    }
    fixture.write("crates/bunny-mesh/build.rs", "fn main() {}\n");
    fixture.write("scripts/zz_build.rs", "fn main() {}\n");
    fixture.write("crates/zz-probe-crate/Cargo.toml", &package("zz-probe-crate", ""));
    fixture.write("crates/zz-probe-crate/src/lib.rs", "");
    for name in ["rand", "serde_json"] {
        fixture.write(&format!("stand-ins/{name}/Cargo.toml"), &package(name, ""));
        fixture.write(&format!("stand-ins/{name}/src/lib.rs"), "");
    }
    let lock = fixture
        .cargo()
        .args(["generate-lockfile", "--offline"])
        .current_dir(&fixture.root)
        .output()
        .expect("cargo should run");
    assert!(lock.status.success(), "lockfile: {}", String::from_utf8_lossy(&lock.stderr));
    fixture
}

fn manifest(name: &str) -> PathBuf {
    Path::new("crates").join(name).join("Cargo.toml")
}

#[test]
fn optional_and_build_dependencies_on_banned_crates_fail_the_gate() {
    let fixture = banned_behind_features_and_build_edges();

    let found = dependency_rule_violations(&fixture.root, &|| fixture.cargo())
        .expect("cargo tree should run on the fixture")
        .iter()
        .map(|violation| (violation.path.clone(), violation.rule))
        .collect::<Vec<_>>();

    let root = PathBuf::from("Cargo.toml");
    assert_eq!(
        found,
        vec![
            (root.clone(), "unclassified-crate"),
            (root.clone(), "unclassified-crate"),
            (root, "unclassified-crate"),
            (manifest("bunny-geom"), "unscanned-dependency"),
            (manifest("bunny-mesh"), "build-script"),
            (manifest("bunny-mesh"), "json-serialization"),
            (manifest("bunny-num"), "build-script"),
            (manifest("bunny-num"), "random"),
        ],
        "the build-dependency and the optional dependency must each be reported once, beside \
         the unclassified members, the unscanned path crate and both build scripts"
    );
}
