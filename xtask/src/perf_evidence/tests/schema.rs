//! The evidence lines against the JSON Schemas that define them, in
//! `schemas/bunny/perf-evidence/v1/` (`build.schema.json`, `round.schema.json`).
//!
//! The validator below implements exactly this subset of JSON Schema draft 2020-12, which is
//! all the two schemas use:
//!
//! - `type`: a name or a list of names among `object`, `array`, `string`, `number`, `integer`
//!   (a number with no fractional part), `boolean` and `null`;
//! - `const` and `enum`, with string, boolean and `null` values;
//! - `required`, `properties`, `additionalProperties: false` (`true` or a schema is refused);
//! - `minimum`, on numbers;
//! - `items`, a single schema applied to every element;
//! - the annotations `$schema`, `$id`, `title` and `description`, which assert nothing.
//!
//! Any other keyword, or one of these used in a form not listed, is refused as unsupported
//! rather than ignored, so a schema can never assert more than this test checks.

use std::path::PathBuf;

use super::super::json::{self, Json};
use super::{identity, measured};
use crate::perf_evidence::{evidence_text, BuildIdentity, PERF_EVIDENCE_SCHEMA};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
const ANNOTATIONS: &[&str] = &["$schema", "$id", "title", "description"];
const ASSERTIONS: &[&str] = &[
    "type",
    "const",
    "enum",
    "required",
    "properties",
    "additionalProperties",
    "minimum",
    "items",
];
const TYPES: &[&str] = &["object", "array", "string", "number", "integer", "boolean", "null"];

#[derive(Debug)]
enum Invalid {
    /// The schema uses a keyword or form this validator does not implement.
    Unsupported(String),
    /// The value breaks the schema.
    Violation(String),
}

fn unsupported<T>(at: &str, what: &str) -> Result<T, Invalid> {
    Err(Invalid::Unsupported(format!("{at}: {what}")))
}

fn violation<T>(at: &str, what: &str) -> Result<T, Invalid> {
    Err(Invalid::Violation(format!("{at}: {what}")))
}

fn validate(schema: &Json, value: &Json, at: &str) -> Result<(), Invalid> {
    let Json::Object(keywords) = schema else {
        return unsupported(at, "a schema that is not an object");
    };
    for (keyword, _) in keywords {
        if !ANNOTATIONS.contains(&keyword.as_str()) && !ASSERTIONS.contains(&keyword.as_str()) {
            return unsupported(at, &format!("the keyword {keyword}"));
        }
    }
    for (keyword, rule) in keywords {
        match keyword.as_str() {
            "type" => check_type(rule, value, at)?,
            "const" => check_enum(std::slice::from_ref(rule), value, at)?,
            "enum" => match rule {
                Json::Array(allowed) => check_enum(allowed, value, at)?,
                _ => return unsupported(at, "an enum that is not an array"),
            },
            "required" => check_required(rule, value, at)?,
            "properties" => check_properties(rule, value, at)?,
            "additionalProperties" => check_additional(rule, schema.get("properties"), value, at)?,
            "minimum" => check_minimum(rule, value, at)?,
            "items" => {
                if let Json::Array(items) = value {
                    for (index, item) in items.iter().enumerate() {
                        validate(rule, item, &format!("{at}[{index}]"))?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_type(rule: &Json, value: &Json, at: &str) -> Result<(), Invalid> {
    let names: Vec<&str> = match rule {
        Json::String(name) => vec![name.as_str()],
        Json::Array(names) => names
            .iter()
            .map(|name| match name {
                Json::String(name) => Ok(name.as_str()),
                _ => unsupported(at, "a type list with a non-string entry"),
            })
            .collect::<Result<_, _>>()?,
        _ => return unsupported(at, "a type that is not a name or a list"),
    };
    if let Some(unknown) = names.iter().find(|name| !TYPES.contains(name)) {
        return unsupported(at, &format!("the type {unknown}"));
    }
    let fits = |name: &str| match (name, value) {
        ("object", Json::Object(_))
        | ("array", Json::Array(_))
        | ("string", Json::String(_))
        | ("number", Json::Number(_))
        | ("boolean", Json::Bool(_))
        | ("null", Json::Null) => true,
        ("integer", Json::Number(text)) => text.parse::<f64>().is_ok_and(|v| v.fract() == 0.0),
        _ => false,
    };
    if names.iter().any(|name| fits(name)) {
        Ok(())
    } else {
        violation(at, &format!("{value:?} is not of type {names:?}"))
    }
}

fn check_enum(allowed: &[Json], value: &Json, at: &str) -> Result<(), Invalid> {
    if allowed.iter().any(|item| matches!(item, Json::Number(_) | Json::Array(_) | Json::Object(_)))
    {
        return unsupported(at, "a const or enum value that is not a string, boolean or null");
    }
    if allowed.contains(value) {
        Ok(())
    } else {
        violation(at, &format!("{value:?} is not one of {allowed:?}"))
    }
}

fn check_required(rule: &Json, value: &Json, at: &str) -> Result<(), Invalid> {
    let Json::Array(names) = rule else { return unsupported(at, "required that is not an array") };
    for name in names {
        let Json::String(name) = name else { return unsupported(at, "a non-string required") };
        if matches!(value, Json::Object(_)) && value.get(name).is_none() {
            return violation(at, &format!("the required field {name} is missing"));
        }
    }
    Ok(())
}

fn check_properties(rule: &Json, value: &Json, at: &str) -> Result<(), Invalid> {
    let Json::Object(properties) = rule else {
        return unsupported(at, "properties that is not an object");
    };
    for (name, property) in properties {
        if let Some(member) = value.get(name) {
            validate(property, member, &format!("{at}.{name}"))?;
        }
    }
    Ok(())
}

fn check_additional(
    rule: &Json,
    properties: Option<&Json>,
    value: &Json,
    at: &str,
) -> Result<(), Invalid> {
    if *rule != Json::Bool(false) {
        return unsupported(at, "additionalProperties other than false");
    }
    let Json::Object(members) = value else { return Ok(()) };
    for (name, _) in members {
        if properties.and_then(|properties| properties.get(name)).is_none() {
            return violation(at, &format!("the field {name} is not allowed"));
        }
    }
    Ok(())
}

fn check_minimum(rule: &Json, value: &Json, at: &str) -> Result<(), Invalid> {
    let Json::Number(minimum) = rule else { return unsupported(at, "a non-number minimum") };
    let minimum: f64 = minimum.parse().map_err(|_| Invalid::Unsupported(at.to_owned()))?;
    match value {
        Json::Number(text) if text.parse::<f64>().is_ok_and(|v| v < minimum) => {
            violation(at, &format!("{text} is below {minimum}"))
        }
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------

fn schema_path(kind: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = root.parent().expect("workspace root").to_path_buf();
    root.join("schemas")
        .join("bunny")
        .join("perf-evidence")
        .join("v1")
        .join(format!("{kind}.schema.json"))
}

fn load_schema(kind: &str) -> Json {
    let path = schema_path(kind);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path:?}: {err}"));
    json::parse(&text).unwrap_or_else(|err| panic!("{path:?}: {err}"))
}

/// Identities covering every value shape the build line can hold.
fn identities(rounds: usize) -> Vec<BuildIdentity> {
    let cross = BuildIdentity {
        target: "wasm32-wasip1".to_owned(),
        target_is_host: false,
        platform: "wasm32-wasip1".to_owned(),
        align_code: false,
        rustflags: String::new(),
        harness_args: Vec::new(),
        runner: Some("wasmtime run".to_owned()),
        max_regression: None,
        ..identity(rounds, true)
    };
    vec![identity(rounds, false), identity(rounds, true), cross]
}

/// Every line the writer produces for the constructed cases, parsed.
fn real_lines() -> Vec<Json> {
    let mut lines = Vec::new();
    for rounds in [1_u32, 2, 4, 5, 7] {
        let rounds_usize = usize::try_from(rounds).expect("small round");
        let measured = measured(rounds);
        for identity in identities(rounds_usize) {
            let text = evidence_text(&identity, &measured.rounds).expect("finite timings");
            lines.extend(text.lines().map(|line| json::parse(line).expect("a JSON line")));
        }
    }
    lines
}

fn kind_of(line: &Json) -> String {
    match line.get("kind") {
        Some(Json::String(kind)) => kind.clone(),
        other => panic!("a line without a string kind: {other:?}"),
    }
}

fn keys(value: &Json) -> Vec<String> {
    match value {
        Json::Object(members) => members.iter().map(|(name, _)| name.clone()).collect(),
        Json::Array(items) => items
            .iter()
            .map(|item| match item {
                Json::String(name) => name.clone(),
                other => panic!("a non-string entry {other:?}"),
            })
            .collect(),
        other => panic!("neither an object nor an array: {other:?}"),
    }
}

#[test]
fn every_line_the_writer_produces_validates_against_its_kind_schema() {
    let (build, round) = (load_schema("build"), load_schema("round"));
    let lines = real_lines();
    let mut counts = (0_usize, 0_usize);
    for line in &lines {
        let schema = match kind_of(line).as_str() {
            "build" => {
                counts.0 += 1;
                &build
            }
            "round" => {
                counts.1 += 1;
                &round
            }
            other => panic!("no schema for the kind {other}"),
        };
        if let Err(err) = validate(schema, line, "line") {
            panic!("{err:?} for {line:?}");
        }
    }
    // Five round counts times three identities; every case and side of every round.
    assert_eq!(counts.0, 15);
    assert!(counts.1 > 300, "{} round lines", counts.1);
}

#[test]
fn deliberately_wrong_lines_fail_their_schema() {
    let (build_schema, round_schema) = (load_schema("build"), load_schema("round"));
    let text = evidence_text(&identity(1, false), &measured(1).rounds).expect("finite");
    let mut lines = text.lines().map(|line| json::parse(line).expect("a JSON line"));
    let build = lines.next().expect("a build line");
    let round = lines.next().expect("a round line");
    validate(&build_schema, &build, "build").expect("the real build line is valid");
    validate(&round_schema, &round, "round").expect("the real round line is valid");

    let num = |text: &str| Json::Number(text.to_owned());
    let string = |text: &str| Json::String(text.to_owned());
    // (what, is a build line, field, replacement; `None` removes the field)
    let wrong: Vec<(&str, bool, &str, Option<Json>)> = vec![
        ("missing p99", false, "p99", None),
        ("missing candidate_dirty", true, "candidate_dirty", None),
        ("extra field on a round", false, "extra", Some(Json::Null)),
        ("extra field on a build", true, "toolchain", Some(string("1.96.0"))),
        ("side head", false, "side", Some(string("head"))),
        ("p50 as a string", false, "p50", Some(string("1.5"))),
        ("round schema id 2", false, "schema", Some(string("bunny-perf-evidence/2"))),
        ("build schema id 2", true, "schema", Some(string("bunny-perf-evidence/2"))),
        ("build kind round", true, "kind", Some(string("round"))),
        ("round 0", false, "round", Some(num("0"))),
        ("round 1.5", false, "round", Some(num("1.5"))),
        ("rounds as a string", true, "rounds", Some(string("7"))),
        ("a number in harness_args", true, "harness_args", Some(Json::Array(vec![num("1")]))),
        ("runner true", true, "runner", Some(Json::Bool(true))),
        ("max_regression as a string", true, "max_regression", Some(string("20"))),
        ("candidate_dirty as a string", true, "candidate_dirty", Some(string("false"))),
    ];
    for (what, is_build, field, replacement) in wrong {
        let (schema, line) =
            if is_build { (&build_schema, &build) } else { (&round_schema, &round) };
        let Json::Object(mut members) = line.clone() else { panic!("an object") };
        let existing = members.iter().position(|(name, _)| name == field);
        match (existing, replacement) {
            (Some(at), None) => {
                members.remove(at);
            }
            (Some(at), Some(value)) => members[at].1 = value,
            (None, Some(value)) => members.push((field.to_owned(), value)),
            (None, None) => panic!("{what}: nothing to remove"),
        }
        match validate(schema, &Json::Object(members), "line") {
            // The message names the field it found wrong.
            Err(Invalid::Violation(message)) => {
                assert!(message.contains(field), "{what}: {message}");
            }
            other => panic!("{what}: expected a violation, got {other:?}"),
        }
    }
}

#[test]
fn the_schema_files_are_well_formed_and_match_the_writer_fields() {
    let lines = real_lines();
    for kind in ["build", "round"] {
        let schema = load_schema(kind);
        assert_eq!(schema.get("$schema"), Some(&Json::String(DRAFT.to_owned())), "{kind}");
        assert!(matches!(schema.get("$id"), Some(Json::String(_))), "{kind}");
        assert_eq!(schema.get("type"), Some(&Json::String("object".to_owned())), "{kind}");
        assert_eq!(schema.get("additionalProperties"), Some(&Json::Bool(false)), "{kind}");
        let properties = schema.get("properties").expect("properties");
        let required = keys(schema.get("required").expect("required"));
        assert_eq!(keys(properties), required, "{kind}: every field required, in property order");
        let Json::Object(members) = properties else { panic!("properties object") };
        for (name, property) in members {
            assert!(
                matches!(property.get("description"), Some(Json::String(text)) if !text.is_empty()),
                "{kind}.{name} has no description"
            );
        }
        let constant = |field: &str| properties.get(field).and_then(|p| p.get("const")).cloned();
        assert_eq!(constant("schema"), Some(Json::String(PERF_EVIDENCE_SCHEMA.to_owned())));
        assert_eq!(constant("kind"), Some(Json::String(kind.to_owned())));
        // The writer emits exactly the required fields, in the same order, on every line.
        let of_kind: Vec<&Json> = lines.iter().filter(|line| kind_of(line) == kind).collect();
        assert_ne!(of_kind.len(), 0, "{kind}: no line of this kind");
        for line in of_kind {
            assert_eq!(keys(line), required, "{kind}");
        }
    }
}

#[test]
fn the_validator_refuses_keywords_it_does_not_implement() {
    let value = Json::String("a".to_owned());
    for (schema, named) in [
        (r#"{"type":"string","pattern":"^a"}"#, "pattern"),
        (r#"{"format":"uri"}"#, "format"),
        (r#"{"type":"text"}"#, "text"),
        (r#"{"additionalProperties":true}"#, "additionalProperties"),
        (r#"{"const":1}"#, "const"),
        (r##"{"$ref":"#/x"}"##, "$ref"),
    ] {
        let schema = json::parse(schema).expect("JSON");
        match validate(&schema, &value, "value") {
            Err(Invalid::Unsupported(message)) => assert!(message.contains(named), "{message}"),
            other => panic!("{schema:?}: expected a refusal, got {other:?}"),
        }
    }
}
