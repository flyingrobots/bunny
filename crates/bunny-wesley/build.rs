//! Build script for exposing the exactly pinned `wesley-core` version to generated DTOs.

use std::fs;

use toml::Value;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let parsed: Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");

    let version = parsed
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("wesley-core"))
        .and_then(|dependency| match dependency {
            Value::String(version) => Some(version.clone()),
            Value::Table(table) => table.get("version").and_then(Value::as_str).map(String::from),
            _ => None,
        })
        .expect("Failed to find wesley-core version in [dependencies]");

    // The manifest pins `wesley-core` exactly (`=x.y.z`), so the requirement names the one
    // version Cargo can resolve. Emit it without the comparison operator.
    let version = version
        .strip_prefix('=')
        .expect("wesley-core must be pinned exactly (`=x.y.z`) so the version witness is exact")
        .trim();

    println!("cargo:rustc-env=WESLEY_CORE_VERSION={version}");
}
