use std::fmt::{self, Write as _};

use wesley_core::{TypeDefinition, TypeKind, WesleyIR};

const SCALAR_PROFILE_DIRECTIVE: &str = "bunnyScalarProfile";
const SCALAR_PROFILE_NAME_ARGUMENT: &str = "name";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScalarProfile {
    name: &'static str,
    rust_type: &'static str,
    typescript_type: &'static str,
    wire_type: &'static str,
    byte_width: Option<usize>,
}

const SCALAR_PROFILES: &[ScalarProfile] = &[
    ScalarProfile {
        name: "bytes.bounded.u32",
        rust_type: "Vec<u8>",
        typescript_type: "Uint8Array",
        wire_type: "u32-len-bytes",
        byte_width: None,
    },
    ScalarProfile {
        name: "bytes.fixed.20",
        rust_type: "[u8; 20]",
        typescript_type: "Uint8Array",
        wire_type: "bytes-fixed-20",
        byte_width: Some(20),
    },
    ScalarProfile {
        name: "f32",
        rust_type: "f32",
        typescript_type: "number",
        wire_type: "f32-le",
        byte_width: Some(4),
    },
    ScalarProfile {
        name: "i32",
        rust_type: "i32",
        typescript_type: "number",
        wire_type: "i32-le",
        byte_width: Some(4),
    },
    ScalarProfile {
        name: "i64",
        rust_type: "i64",
        typescript_type: "bigint",
        wire_type: "i64-le",
        byte_width: Some(8),
    },
    ScalarProfile {
        name: "q32.32",
        rust_type: "i64",
        typescript_type: "bigint",
        wire_type: "i64-le-q32.32",
        byte_width: Some(8),
    },
    ScalarProfile {
        name: "u8",
        rust_type: "u8",
        typescript_type: "number",
        wire_type: "u8",
        byte_width: Some(1),
    },
    ScalarProfile {
        name: "u16",
        rust_type: "u16",
        typescript_type: "number",
        wire_type: "u16-le",
        byte_width: Some(2),
    },
    ScalarProfile {
        name: "u32",
        rust_type: "u32",
        typescript_type: "number",
        wire_type: "u32-le",
        byte_width: Some(4),
    },
    ScalarProfile {
        name: "u64",
        rust_type: "u64",
        typescript_type: "bigint",
        wire_type: "u64-le",
        byte_width: Some(8),
    },
    ScalarProfile {
        name: "utf8.bounded.u32",
        rust_type: "String",
        typescript_type: "string",
        wire_type: "u32-len-utf8",
        byte_width: None,
    },
];

struct ResolvedScalarProfile<'schema> {
    scalar: &'schema str,
    profile: &'static ScalarProfile,
}

fn push_fmt(output: &mut String, args: fmt::Arguments<'_>) {
    output.write_fmt(args).expect("writing to String cannot fail");
}

pub(super) fn push_rust_scalar_profiles(
    output: &mut String,
    schema: &WesleyIR,
) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = schema_scalar_profiles(schema)?;
    if profiles.is_empty() {
        return Ok(());
    }

    output.push_str(RUST_PROFILE_STRUCT);
    output.push_str("/// Scalar profiles used by the generated graphics contract.\n");
    output.push_str("pub const BUNNY_GRAPHICS_SCALAR_PROFILES: &[BunnyScalarProfile] = &[\n");
    for resolved in profiles {
        push_rust_profile_entry(output, &resolved);
    }
    output.push_str("];\n\n");
    Ok(())
}

const RUST_PROFILE_STRUCT: &str = concat!(
    "/// Deterministic wire profile metadata for generated Bunny scalars.\n",
    "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n",
    "pub struct BunnyScalarProfile {\n",
    "    /// GraphQL scalar name that selected this profile.\n",
    "    pub scalar: &'static str,\n",
    "    /// Bunny scalar profile selected by `@bunnyScalarProfile`.\n",
    "    pub profile: &'static str,\n",
    "    /// Rust boundary representation emitted for this scalar.\n",
    "    pub rust_type: &'static str,\n",
    "    /// TypeScript boundary representation emitted for this scalar.\n",
    "    pub typescript_type: &'static str,\n",
    "    /// Deterministic wire representation profile for future codecs.\n",
    "    pub wire_type: &'static str,\n",
    "    /// Fixed byte width for fixed-size profiles, if one exists.\n",
    "    pub byte_width: Option<usize>,\n",
    "}\n\n",
);

fn push_rust_profile_entry(output: &mut String, resolved: &ResolvedScalarProfile<'_>) {
    output.push_str("    BunnyScalarProfile {\n");
    push_fmt(output, format_args!("        scalar: \"{}\",\n", resolved.scalar));
    push_fmt(output, format_args!("        profile: \"{}\",\n", resolved.profile.name));
    push_fmt(output, format_args!("        rust_type: \"{}\",\n", resolved.profile.rust_type));
    push_fmt(
        output,
        format_args!("        typescript_type: \"{}\",\n", resolved.profile.typescript_type),
    );
    push_fmt(output, format_args!("        wire_type: \"{}\",\n", resolved.profile.wire_type));
    push_fmt(output, format_args!("        byte_width: {},\n", rust_byte_width(resolved.profile)));
    output.push_str("    },\n");
}

pub(super) fn push_typescript_scalar_profiles(
    output: &mut String,
    schema: &WesleyIR,
) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = schema_scalar_profiles(schema)?;
    if profiles.is_empty() {
        return Ok(());
    }

    output.push_str(TYPESCRIPT_PROFILE_INTERFACE);
    output.push_str("export const BUNNY_GRAPHICS_SCALAR_PROFILES = [\n");
    for resolved in profiles {
        push_typescript_profile_entry(output, &resolved);
    }
    output.push_str("] as const satisfies readonly BunnyScalarProfile[];\n\n");
    Ok(())
}

const TYPESCRIPT_PROFILE_INTERFACE: &str = concat!(
    "export interface BunnyScalarProfile {\n",
    "  readonly scalar: string;\n",
    "  readonly profile: string;\n",
    "  readonly rustType: string;\n",
    "  readonly typescriptType: string;\n",
    "  readonly wireType: string;\n",
    "  readonly byteWidth: number | null;\n",
    "}\n\n",
);

fn push_typescript_profile_entry(output: &mut String, resolved: &ResolvedScalarProfile<'_>) {
    output.push_str("  {\n");
    push_fmt(output, format_args!("    scalar: \"{}\",\n", resolved.scalar));
    push_fmt(output, format_args!("    profile: \"{}\",\n", resolved.profile.name));
    push_fmt(output, format_args!("    rustType: \"{}\",\n", resolved.profile.rust_type));
    push_fmt(
        output,
        format_args!("    typescriptType: \"{}\",\n", resolved.profile.typescript_type),
    );
    push_fmt(output, format_args!("    wireType: \"{}\",\n", resolved.profile.wire_type));
    push_fmt(output, format_args!("    byteWidth: {},\n", ts_byte_width(resolved.profile)));
    output.push_str("  },\n");
}

fn scalar_profile_by_name(name: &str) -> Option<&'static ScalarProfile> {
    SCALAR_PROFILES.iter().find(|profile| profile.name == name)
}

fn schema_scalar_profiles(schema: &WesleyIR) -> Result<Vec<ResolvedScalarProfile<'_>>, String> {
    reject_field_level_scalar_profiles(schema)?;
    let mut profiles = Vec::new();
    for scalar in schema
        .types
        .iter()
        .filter(|type_definition| matches!(type_definition.kind, TypeKind::Scalar))
    {
        profiles.push(ResolvedScalarProfile {
            scalar: &scalar.name,
            profile: resolve_profile(scalar)?,
        });
    }
    profiles.sort_by(|left, right| left.scalar.cmp(right.scalar));
    Ok(profiles)
}

fn reject_field_level_scalar_profiles(schema: &WesleyIR) -> Result<(), String> {
    for type_definition in &schema.types {
        for field in &type_definition.fields {
            if field.directives.contains_key(SCALAR_PROFILE_DIRECTIVE) {
                return Err(format!(
                    concat!(
                        "Field-level `@bunnyScalarProfile` directives are reserved until ",
                        "field profile semantics exist: {}.{}"
                    ),
                    type_definition.name, field.name
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn render_manifest_scalar_profiles(
    schema: &WesleyIR,
) -> Result<String, Box<dyn std::error::Error>> {
    let profiles = schema_scalar_profiles(schema)?;
    let mut output = String::new();
    output.push('[');
    if !profiles.is_empty() {
        output.push('\n');
    }
    for (index, resolved) in profiles.iter().enumerate() {
        push_manifest_profile_entry(&mut output, resolved, index + 1 == profiles.len());
    }
    output.push_str("  ]");
    Ok(output)
}

fn push_manifest_profile_entry(
    output: &mut String,
    resolved: &ResolvedScalarProfile<'_>,
    final_entry: bool,
) {
    output.push_str("    {\n");
    push_fmt(output, format_args!("      \"scalar\": \"{}\",\n", resolved.scalar));
    push_fmt(output, format_args!("      \"profile\": \"{}\",\n", resolved.profile.name));
    push_fmt(output, format_args!("      \"rustType\": \"{}\",\n", resolved.profile.rust_type));
    push_fmt(
        output,
        format_args!("      \"typescriptType\": \"{}\",\n", resolved.profile.typescript_type),
    );
    push_fmt(output, format_args!("      \"wireType\": \"{}\",\n", resolved.profile.wire_type));
    push_fmt(output, format_args!("      \"byteWidth\": {}\n", json_byte_width(resolved.profile)));
    if final_entry {
        output.push_str("    }\n");
    } else {
        output.push_str("    },\n");
    }
}

fn resolve_profile(scalar_def: &TypeDefinition) -> Result<&'static ScalarProfile, String> {
    let profile_value = scalar_def.directives.get(SCALAR_PROFILE_DIRECTIVE).ok_or_else(|| {
        format!("Missing `@bunnyScalarProfile` directive on scalar '{}'", scalar_def.name)
    })?;
    let profile_args = profile_value.as_object().ok_or_else(|| {
        format!(
            "`@bunnyScalarProfile` directive on scalar '{}' must be an object with arguments",
            scalar_def.name
        )
    })?;
    let profile_name = profile_args
        .get(SCALAR_PROFILE_NAME_ARGUMENT)
        .ok_or_else(|| {
            format!(
                "Missing 'name' argument in `@bunnyScalarProfile` on scalar '{}'",
                scalar_def.name
            )
        })?
        .as_str()
        .ok_or_else(|| {
            format!(
                "'name' argument in `@bunnyScalarProfile` on scalar '{}' must be a string",
                scalar_def.name
            )
        })?;

    scalar_profile_by_name(profile_name).ok_or_else(|| {
        format!("Unsupported scalar profile '{}' on scalar '{}'", profile_name, scalar_def.name)
    })
}

pub(super) fn rust_scalar_type(scalar_def: &TypeDefinition) -> Result<&'static str, String> {
    Ok(resolve_profile(scalar_def)?.rust_type)
}

pub(super) fn ts_scalar_type(scalar_def: &TypeDefinition) -> Result<&'static str, String> {
    Ok(resolve_profile(scalar_def)?.typescript_type)
}

fn rust_byte_width(profile: &ScalarProfile) -> String {
    match profile.byte_width {
        Some(width) => format!("Some({width})"),
        None => "None".to_string(),
    }
}

fn ts_byte_width(profile: &ScalarProfile) -> String {
    match profile.byte_width {
        Some(width) => width.to_string(),
        None => "null".to_string(),
    }
}

fn json_byte_width(profile: &ScalarProfile) -> String {
    ts_byte_width(profile)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::render::{render_rust, render_typescript};

    #[test]
    fn scalar_profiles_are_registry_driven() {
        assert_eq!(scalar_profile_by_name("f32").unwrap().rust_type, "f32");
        assert_eq!(scalar_profile_by_name("q32.32").unwrap().typescript_type, "bigint");
        assert_eq!(scalar_profile_by_name("u64").unwrap().wire_type, "u64-le");
        assert_eq!(scalar_profile_by_name("bytes.fixed.20").unwrap().byte_width, Some(20));
        assert_eq!(scalar_profile_by_name("utf8.bounded.u32").unwrap().wire_type, "u32-len-utf8");
        assert!(scalar_profile_by_name("f64").is_none());
    }

    #[test]
    fn deterministic_contract_profiles_render_for_wire_envelopes() {
        let schema = r#"
            directive @bunnyScalarProfile(name: String!) on SCALAR

            scalar BunnyEnvelopeVersion @bunnyScalarProfile(name: "u16")
            scalar BunnyEnvelopeKind @bunnyScalarProfile(name: "u16")
            scalar BunnyObjectId @bunnyScalarProfile(name: "bytes.fixed.20")
            scalar BunnyPayload @bunnyScalarProfile(name: "bytes.bounded.u32")
            scalar BunnyCounter @bunnyScalarProfile(name: "u64")
            scalar BunnyLabel @bunnyScalarProfile(name: "utf8.bounded.u32")

            type BunnyWireEnvelope {
              version: BunnyEnvelopeVersion!
              kind: BunnyEnvelopeKind!
              objectId: BunnyObjectId!
              payload: BunnyPayload!
              counter: BunnyCounter!
              label: BunnyLabel!
            }
        "#;
        let ir = wesley_core::lower_schema_sdl(schema).unwrap();

        let rust = render_rust(&ir, "hash", Path::new("schema.graphql")).unwrap();
        assert!(rust.contains("pub type BunnyEnvelopeVersion = u16;"));
        assert!(rust.contains("pub type BunnyObjectId = [u8; 20];"));
        assert!(rust.contains("pub type BunnyPayload = Vec<u8>;"));
        assert!(rust.contains("pub type BunnyCounter = u64;"));
        assert!(rust.contains("pub type BunnyLabel = String;"));
        assert!(rust.contains("pub const BUNNY_GRAPHICS_SCALAR_PROFILES"));
        assert!(rust.contains("profile: \"bytes.fixed.20\""));
        assert!(rust.contains("wire_type: \"u32-len-bytes\""));
        assert!(rust.contains("byte_width: Some(20)"));
        assert!(rust.contains("pub objectId: BunnyObjectId,"));

        let typescript = render_typescript(&ir, "hash", Path::new("schema.graphql")).unwrap();
        assert!(typescript.contains("export type BunnyEnvelopeVersion = number;"));
        assert!(typescript.contains("export type BunnyObjectId = Uint8Array;"));
        assert!(typescript.contains("export type BunnyPayload = Uint8Array;"));
        assert!(typescript.contains("export type BunnyCounter = bigint;"));
        assert!(typescript.contains("export type BunnyLabel = string;"));
        assert!(typescript.contains("export const BUNNY_GRAPHICS_SCALAR_PROFILES"));
        assert!(typescript.contains("profile: \"utf8.bounded.u32\""));
        assert!(typescript.contains("wireType: \"u64-le\""));
        assert!(typescript.contains("byteWidth: null"));
        assert!(typescript.contains("readonly objectId: BunnyObjectId;"));

        let manifest = render_manifest_scalar_profiles(&ir).unwrap();
        assert!(manifest.contains("\"scalar\": \"BunnyObjectId\""));
        assert!(manifest.contains("\"profile\": \"bytes.fixed.20\""));
        assert!(manifest.contains("\"byteWidth\": 20"));
        assert!(manifest.contains("\"wireType\": \"u32-len-utf8\""));
    }

    #[test]
    fn field_level_scalar_profiles_fail_closed_until_supported() {
        let schema = r#"
            directive @bunnyScalarProfile(name: String!) on SCALAR | FIELD_DEFINITION

            type BunnyThing {
              label: String! @bunnyScalarProfile(name: "utf8.bounded.u32")
            }
        "#;
        let ir = wesley_core::lower_schema_sdl(schema).unwrap();

        let rust_error = render_rust(&ir, "hash", Path::new("schema.graphql")).unwrap_err();
        assert!(rust_error
            .to_string()
            .contains("Field-level `@bunnyScalarProfile` directives are reserved"));

        let typescript_error =
            render_typescript(&ir, "hash", Path::new("schema.graphql")).unwrap_err();
        assert!(typescript_error
            .to_string()
            .contains("Field-level `@bunnyScalarProfile` directives are reserved"));

        let manifest_error = render_manifest_scalar_profiles(&ir).unwrap_err();
        assert!(manifest_error
            .to_string()
            .contains("Field-level `@bunnyScalarProfile` directives are reserved"));
    }
}
