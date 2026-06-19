//! Regression tests for generated contract witness metadata.

use bunny_contract::generated::graphics::BUNNY_WESLEY_GENERATOR;

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
