use std::fs;

fn main() {
    // Re-run if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");

    let cargo_toml = fs::read_to_string("Cargo.toml").unwrap();
    let version = cargo_toml
        .lines()
        .find(|line| line.trim().starts_with("wesley-core"))
        .and_then(|line| line.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"').trim_matches('\'').trim())
        .map(|s| s.to_string())
        .expect("Failed to find wesley-core dependency in Cargo.toml");

    println!("cargo:rustc-env=WESLEY_CORE_VERSION={}", version);
}
