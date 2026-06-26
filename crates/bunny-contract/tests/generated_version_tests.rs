//! Regression tests for generated contract witness metadata.

use bunny_contract::generated::graphics::{BUNNY_GRAPHICS_SCALAR_PROFILES, BUNNY_WESLEY_GENERATOR};

const EXPECTED_GENERATOR: &str = concat!("bunny-wesley/", env!("CARGO_PKG_VERSION"));
const GENERATED_TYPESCRIPT: &str = include_str!("../../../generated/typescript/bunny-graphics.ts");
const GENERATED_MANIFEST: &str = include_str!("../../../generated/bunny-graphics.manifest.json");

#[test]
fn generated_witnesses_match_the_released_generator_version() {
    assert_eq!(BUNNY_WESLEY_GENERATOR, EXPECTED_GENERATOR);

    let expected_typescript =
        format!("export const BUNNY_WESLEY_GENERATOR = \"{EXPECTED_GENERATOR}\" as const;");
    assert!(GENERATED_TYPESCRIPT.contains(&expected_typescript));

    let expected_manifest = format!("\"generator\": \"{EXPECTED_GENERATOR}\"");
    assert!(GENERATED_MANIFEST.contains(&expected_manifest));
}

#[test]
fn generated_scalar_profile_witnesses_cover_checked_in_artifacts() {
    let fixed = BUNNY_GRAPHICS_SCALAR_PROFILES
        .iter()
        .find(|profile| profile.scalar == "BunnyFixedQ32_32Raw")
        .expect("fixed-point profile should be generated");
    assert_eq!(fixed.profile, "q32.32");
    assert_eq!(fixed.rust_type, "i64");
    assert_eq!(fixed.typescript_type, "bigint");
    assert_eq!(fixed.wire_type, "i64-le-q32.32");
    assert_eq!(fixed.byte_width, Some(8));

    let scalar = BUNNY_GRAPHICS_SCALAR_PROFILES
        .iter()
        .find(|profile| profile.scalar == "BunnyScalar")
        .expect("scalar profile should be generated");
    assert_eq!(scalar.profile, "f32");
    assert_eq!(scalar.wire_type, "f32-le");
    assert_eq!(scalar.byte_width, Some(4));

    assert!(GENERATED_TYPESCRIPT.contains("BUNNY_GRAPHICS_SCALAR_PROFILES"));
    assert!(GENERATED_TYPESCRIPT.contains("wireType: \"i64-le-q32.32\""));
    assert!(GENERATED_MANIFEST.contains("\"scalarProfiles\""));
    assert!(GENERATED_MANIFEST.contains("\"wireType\": \"f32-le\""));
}
