use std::fs;
use toml::Value;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let parsed: Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");

    let version = parsed
        .get("dependencies")
        .and_then(|d| d.get("wesley-core"))
        .and_then(|dep| match dep {
            Value::String(v) => Some(v.clone()),
            Value::Table(t) => t.get("version").and_then(Value::as_str).map(String::from),
            _ => None,
        })
        .expect("Failed to find wesley-core version in [dependencies]");

    println!("cargo:rustc-env=WESLEY_CORE_VERSION={}", version);
}
