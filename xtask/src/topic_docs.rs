use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Command;

use syn::visit::{self, Visit};
use syn::{Attribute, File, ItemFn, ItemMod, Meta};

type DynError = Box<dyn Error>;

const REQUIREMENT_HEADER: &str = "[[requirement]]";
const CASE_HEADER: &str = "[[case]]";
const EXECUTABLE_EVIDENCE: &[&str] = &["test", "property"];
const EVIDENCE_TYPES: &[&str] = &[
    "test",
    "doctest",
    "compile-fail",
    "property",
    "fuzz-corpus",
    "static-check",
    "benchmark",
    "manual-audit",
];
const REQUIREMENT_STATUSES: &[&str] = &["active", "reserved", "retired"];
const CASE_STATUSES: &[&str] = &["planned", "implemented", "blocked"];
const CASE_TIERS: &[&str] = &["fast", "slow", "fuzz", "bench", "manual"];

#[derive(Debug)]
struct TopicDocsError {
    message: String,
}

impl TopicDocsError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for TopicDocsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for TopicDocsError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordKind {
    Requirement,
    Case,
}

#[derive(Debug)]
struct Record {
    kind: RecordKind,
    line: usize,
    fields: BTreeMap<String, Field>,
}

#[derive(Debug)]
struct Field {
    value: FieldValue,
}

#[derive(Debug, PartialEq, Eq)]
enum FieldValue {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug)]
struct Requirement {
    id: String,
    status: String,
    line: usize,
}

#[derive(Debug)]
struct Case {
    id: String,
    requirements: Vec<String>,
    evidence: String,
    test: Option<String>,
    artifact: Option<String>,
    status: String,
    line: usize,
}

#[derive(Debug)]
struct Violation {
    path: PathBuf,
    line: usize,
    message: String,
}

#[derive(Default)]
struct Report {
    active_requirements: usize,
    requirements_with_case_coverage: usize,
    uncovered_requirements: usize,
    planned_cases: usize,
    implemented_cases: usize,
    blocked_cases: usize,
    missing_executable_cases: usize,
    duplicate_ids: usize,
    dead_requirement_refs: usize,
    metadata_parse_errors: usize,
}

#[derive(Default)]
struct TestCollector {
    evidence: BTreeMap<String, TestEvidence>,
    cfg_gated_stack: Vec<bool>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TestEvidence {
    runnable: bool,
    ignored: bool,
    cfg_gated: bool,
}

struct PlanContext<'a> {
    path: &'a Path,
    test_evidence: &'a BTreeMap<String, TestEvidence>,
    report: &'a mut Report,
    violations: &'a mut Vec<Violation>,
}

#[derive(Clone, Copy)]
struct KnownValue<'a> {
    label: &'static str,
    value: &'a str,
    allowed: &'static [&'static str],
}

impl Violation {
    fn new(path: &Path, line: usize, message: impl Into<String>) -> Self {
        Self { path: path.to_path_buf(), line, message: message.into() }
    }
}

impl Display for Violation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}: {}", self.path.display(), self.message)
        } else {
            write!(formatter, "{}:{}: {}", self.path.display(), self.line, self.message)
        }
    }
}

impl<'ast> Visit<'ast> for TestCollector {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let cfg_gated = attrs_contain_non_test_cfg(&node.attrs);
        self.cfg_gated_stack.push(cfg_gated);
        visit::visit_item_mod(self, node);
        self.cfg_gated_stack.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if attrs_contain_test(&node.attrs) {
            let cfg_gated_ancestor = self.cfg_gated_stack.iter().any(|cfg_gated| *cfg_gated);
            let evidence = TestEvidence::from_attrs(&node.attrs, cfg_gated_ancestor);
            self.evidence
                .entry(node.sig.ident.to_string())
                .and_modify(|existing| existing.merge(evidence))
                .or_insert(evidence);
        }
        visit::visit_item_fn(self, node);
    }
}

impl TestEvidence {
    fn from_attrs(attrs: &[Attribute], cfg_gated_ancestor: bool) -> Self {
        let ignored = attrs_contain_ignore(attrs);
        let cfg_gated = cfg_gated_ancestor || attrs_contain_non_test_cfg(attrs);
        Self { runnable: !ignored && !cfg_gated, ignored, cfg_gated }
    }

    fn merge(&mut self, other: Self) {
        self.runnable |= other.runnable;
        self.ignored |= other.ignored;
        self.cfg_gated |= other.cfg_gated;
    }

    fn blocking_reasons(self) -> String {
        let mut reasons = Vec::new();
        if self.ignored {
            reasons.push("ignored");
        }
        if self.cfg_gated {
            reasons.push("cfg-gated");
        }
        if reasons.is_empty() {
            reasons.push("not runnable");
        }
        reasons.join(", ")
    }
}

impl PlanContext<'_> {
    fn violation(&mut self, line: usize, message: impl Into<String>) {
        self.violations.push(Violation::new(self.path, line, message));
    }

    fn parse_error(&mut self, line: usize, message: impl Into<String>) {
        self.violation(line, message);
        self.report.metadata_parse_errors += 1;
    }

    fn required_string(&mut self, record: &Record, key: &str) -> Option<String> {
        match record.fields.get(key).map(|field| &field.value) {
            Some(FieldValue::String(value)) => Some(value.clone()),
            Some(FieldValue::Array(_)) => {
                self.parse_error(record.line, format!("metadata field `{key}` must be a string"));
                None
            }
            None => {
                self.parse_error(
                    record.line,
                    format!("metadata record is missing required field `{key}`"),
                );
                None
            }
        }
    }

    fn required_array(&mut self, record: &Record, key: &str) -> Option<Vec<String>> {
        match record.fields.get(key).map(|field| &field.value) {
            Some(FieldValue::Array(value)) => Some(value.clone()),
            Some(FieldValue::String(_)) => {
                self.parse_error(record.line, format!("metadata field `{key}` must be an array"));
                None
            }
            None => {
                self.parse_error(
                    record.line,
                    format!("metadata record is missing required field `{key}`"),
                );
                None
            }
        }
    }

    fn validate_known_value(&mut self, line: usize, known: KnownValue<'_>) {
        if known.allowed.contains(&known.value) {
            return;
        }

        self.parse_error(
            line,
            format!(
                "{} `{}` is not one of: {}",
                known.label,
                known.value,
                known.allowed.join(", ")
            ),
        );
    }
}

pub(super) fn handle() -> Result<(), DynError> {
    check()
}

pub(super) fn check() -> Result<(), DynError> {
    let root = git_root()?;
    let test_evidence = discover_test_evidence(&root)?;
    let plans = topic_test_plans(&root)?;
    let mut report = Report::default();
    let mut violations = Vec::new();

    for plan in &plans {
        let text = std::fs::read_to_string(root.join(plan))?;
        let mut context = PlanContext {
            path: plan,
            test_evidence: &test_evidence,
            report: &mut report,
            violations: &mut violations,
        };
        validate_plan(&text, &mut context);
    }

    print_report(&report);

    if violations.is_empty() {
        println!("Topic docs: contract graph clean");
        return Ok(());
    }

    eprintln!("Topic docs: contract graph violations found");
    for violation in &violations {
        eprintln!("  {violation}");
    }
    Err(TopicDocsError::new("topic documentation contract violations found").into())
}

fn validate_plan(text: &str, context: &mut PlanContext<'_>) {
    let records = parse_records(text, context);
    if records.is_empty() {
        context
            .parse_error(1, "test-plan.md must include fenced toml requirement and case metadata");
        return;
    }

    let mut requirements = BTreeMap::new();
    let mut cases = Vec::new();
    let mut seen_ids = BTreeMap::new();

    for record in records {
        if let Some(id) = optional_string(&record, "id") {
            if let Some(first_line) = seen_ids.insert(id.clone(), record.line) {
                context.violation(
                    record.line,
                    format!("duplicate metadata id `{id}`; first declared on line {first_line}"),
                );
                context.report.duplicate_ids += 1;
            }
        }

        match record.kind {
            RecordKind::Requirement => {
                if let Some(requirement) = requirement_from_record(context, &record) {
                    requirements.insert(requirement.id.clone(), requirement);
                }
            }
            RecordKind::Case => {
                if let Some(case) = case_from_record(context, &record) {
                    cases.push(case);
                }
            }
        }
    }

    let mut covered_active_requirements = BTreeSet::new();
    for case in &cases {
        count_case(context.report, case);
        validate_case(context, case, &requirements);
        for requirement_id in &case.requirements {
            if requirements
                .get(requirement_id)
                .is_some_and(|requirement| requirement.status == "active")
            {
                covered_active_requirements.insert(requirement_id.clone());
            }
        }
    }

    for requirement in requirements.values().filter(|requirement| requirement.status == "active") {
        context.report.active_requirements += 1;
        if covered_active_requirements.contains(&requirement.id) {
            context.report.requirements_with_case_coverage += 1;
        } else {
            context.violation(
                requirement.line,
                format!(
                    "active requirement `{}` has no planned or implemented case",
                    requirement.id
                ),
            );
            context.report.uncovered_requirements += 1;
        }
    }
}

fn requirement_from_record(context: &mut PlanContext<'_>, record: &Record) -> Option<Requirement> {
    let id = context.required_string(record, "id")?;
    context.required_string(record, "summary")?;
    let status = context.required_string(record, "status")?;
    context.validate_known_value(
        record.line,
        KnownValue { label: "requirement status", value: &status, allowed: REQUIREMENT_STATUSES },
    );
    Some(Requirement { id, status, line: record.line })
}

fn case_from_record(context: &mut PlanContext<'_>, record: &Record) -> Option<Case> {
    let id = context.required_string(record, "id")?;
    let requirements = context.required_array(record, "requirements")?;
    let evidence = context.required_string(record, "evidence")?;
    let oracle = context.required_string(record, "oracle")?;
    let tier = context.required_string(record, "tier")?;
    let status = context.required_string(record, "status")?;
    context.validate_known_value(
        record.line,
        KnownValue { label: "evidence", value: &evidence, allowed: EVIDENCE_TYPES },
    );
    context.validate_known_value(
        record.line,
        KnownValue { label: "case tier", value: &tier, allowed: CASE_TIERS },
    );
    context.validate_known_value(
        record.line,
        KnownValue { label: "case status", value: &status, allowed: CASE_STATUSES },
    );

    if oracle.trim().is_empty() {
        context.parse_error(record.line, "case oracle must not be empty");
    }

    Some(Case {
        id,
        requirements,
        evidence,
        test: optional_string(record, "test"),
        artifact: optional_string(record, "artifact"),
        status,
        line: record.line,
    })
}

fn count_case(report: &mut Report, case: &Case) {
    match case.status.as_str() {
        "planned" => report.planned_cases += 1,
        "implemented" => report.implemented_cases += 1,
        "blocked" => report.blocked_cases += 1,
        _ => {}
    }
}

fn validate_case(
    context: &mut PlanContext<'_>,
    case: &Case,
    requirements: &BTreeMap<String, Requirement>,
) {
    for requirement_id in &case.requirements {
        if !requirements.contains_key(requirement_id) {
            context.violation(
                case.line,
                format!("case `{}` references unknown requirement `{requirement_id}`", case.id),
            );
            context.report.dead_requirement_refs += 1;
        }
    }

    if case.status != "implemented" {
        return;
    }

    if EXECUTABLE_EVIDENCE.contains(&case.evidence.as_str()) {
        validate_executable_case(context, case);
    } else if case.artifact.as_deref().is_none_or(str::is_empty) {
        context.violation(
            case.line,
            format!(
                "implemented non-test case `{}` must name an `artifact` evidence anchor",
                case.id
            ),
        );
        context.report.missing_executable_cases += 1;
    }
}

fn validate_executable_case(context: &mut PlanContext<'_>, case: &Case) {
    let Some(test_name) = case.test.as_deref().filter(|name| !name.is_empty()) else {
        context.violation(
            case.line,
            format!("implemented executable case `{}` must name a Rust test function", case.id),
        );
        context.report.missing_executable_cases += 1;
        return;
    };

    match context.test_evidence.get(test_name) {
        Some(evidence) if evidence.runnable => {}
        Some(evidence) => {
            context.violation(
                case.line,
                format!(
                    "implemented case `{}` names Rust test `{test_name}` but the discovered \
                     function is not runnable evidence ({})",
                    case.id,
                    evidence.blocking_reasons()
                ),
            );
            context.report.missing_executable_cases += 1;
        }
        None => {
            context.violation(
                case.line,
                format!("implemented case `{}` names missing Rust test `{test_name}`", case.id),
            );
            context.report.missing_executable_cases += 1;
        }
    }
}

fn optional_string(record: &Record, key: &str) -> Option<String> {
    record.fields.get(key).and_then(|field| match &field.value {
        FieldValue::String(value) => Some(value.clone()),
        FieldValue::Array(_) => None,
    })
}

fn parse_records(text: &str, context: &mut PlanContext<'_>) -> Vec<Record> {
    let mut records = Vec::new();
    let mut in_toml = false;
    let mut block = Vec::new();
    let mut block_start = 0;

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if in_toml {
            if trimmed == "```" {
                parse_block(&block, context, &mut records);
                block.clear();
                in_toml = false;
            } else {
                block.push((line_number, line));
            }
        } else if trimmed == "```toml" {
            in_toml = true;
            block_start = line_number;
        }
    }

    if in_toml {
        context.parse_error(block_start, "unclosed toml metadata fence");
    }

    records
}

fn parse_block(lines: &[(usize, &str)], context: &mut PlanContext<'_>, records: &mut Vec<Record>) {
    let mut current = None;
    for (line_number, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(kind) = record_kind(trimmed) {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = Some(Record { kind, line: *line_number, fields: BTreeMap::new() });
            continue;
        }

        let Some(record) = current.as_mut() else {
            context.parse_error(
                *line_number,
                "metadata field appeared before [[requirement]] or [[case]]",
            );
            continue;
        };

        let Some((key, raw_value)) = trimmed.split_once('=') else {
            context.parse_error(*line_number, "metadata line must be `key = value`");
            continue;
        };
        let key = key.trim().to_string();

        match parse_value(raw_value.trim()) {
            Ok(value) => {
                if record.fields.insert(key.clone(), Field { value }).is_some() {
                    context.parse_error(
                        *line_number,
                        format!("metadata field `{key}` is declared more than once"),
                    );
                }
            }
            Err(message) => {
                context.parse_error(*line_number, message);
            }
        }
    }

    if let Some(record) = current {
        records.push(record);
    }
}

fn record_kind(line: &str) -> Option<RecordKind> {
    match line {
        REQUIREMENT_HEADER => Some(RecordKind::Requirement),
        CASE_HEADER => Some(RecordKind::Case),
        _ => None,
    }
}

fn parse_value(raw: &str) -> Result<FieldValue, String> {
    if let Some(value) = parse_quoted(raw) {
        return Ok(FieldValue::String(value));
    }

    if let Some(inner) = raw.strip_prefix('[').and_then(|value| value.strip_suffix(']')) {
        let mut values = Vec::new();
        for part in inner.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Some(value) = parse_quoted(trimmed) else {
                return Err(format!("array item `{trimmed}` must be a quoted string"));
            };
            values.push(value);
        }
        return Ok(FieldValue::Array(values));
    }

    Err(format!("metadata value `{raw}` must be a quoted string or string array"))
}

fn parse_quoted(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('"')?.strip_suffix('"')?;
    if inner.contains('"') {
        return None;
    }
    Some(inner.to_string())
}

fn print_report(report: &Report) {
    println!("Topic docs: contract graph report");
    println!("  Active requirements:             {}", report.active_requirements);
    println!("  Requirements with case coverage: {}", report.requirements_with_case_coverage);
    println!("  Uncovered requirements:          {}", report.uncovered_requirements);
    println!("  Planned cases:                   {}", report.planned_cases);
    println!("  Implemented cases:               {}", report.implemented_cases);
    println!("  Blocked cases:                   {}", report.blocked_cases);
    println!("  Missing executable cases:        {}", report.missing_executable_cases);
    println!("  Duplicate IDs:                   {}", report.duplicate_ids);
    println!("  Dead requirement references:     {}", report.dead_requirement_refs);
    println!("  Metadata parse errors:           {}", report.metadata_parse_errors);
}

fn topic_test_plans(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let topics = root.join("docs").join("topics");
    if !topics.exists() {
        return Ok(Vec::new());
    }

    let mut plans = Vec::new();
    collect_test_plans(root, &topics, &mut plans)?;
    plans.sort();
    Ok(plans)
}

fn collect_test_plans(root: &Path, dir: &Path, plans: &mut Vec<PathBuf>) -> Result<(), DynError> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_test_plans(root, &path, plans)?;
        } else if path.file_name() == Some(OsStr::new("test-plan.md")) {
            plans.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn discover_test_evidence(root: &Path) -> Result<BTreeMap<String, TestEvidence>, DynError> {
    let mut collector = TestCollector::default();
    for path in git_files(root)? {
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let absolute = root.join(&path);
        if !absolute.is_file() {
            continue;
        }
        let source = std::fs::read_to_string(&absolute)?;
        let file: File = syn::parse_file(&source)?;
        collector.visit_file(&file);
    }
    Ok(collector.evidence)
}

fn git_files(root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let output = Command::new("git").args(["ls-files", "--cached"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(command_error("git file listing", &output.stderr).into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.lines().filter(|line| !line.is_empty()).map(PathBuf::from).collect())
}

fn attrs_contain_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

fn attrs_contain_ignore(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr_path_is(attr, "ignore") || cfg_attr_can_ignore(attr))
}

fn attrs_contain_non_test_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr_path_is(attr, "cfg") && !cfg_is_plain_test(attr))
}

fn is_test_attr(attr: &Attribute) -> bool {
    attr_path_is(attr, "test") || attr_path_is(attr, "wasm_bindgen_test")
}

fn attr_path_is(attr: &Attribute, expected: &str) -> bool {
    attr.path().segments.last().is_some_and(|segment| segment.ident == expected)
}

fn cfg_attr_can_ignore(attr: &Attribute) -> bool {
    attr_path_is(attr, "cfg_attr") && meta_tokens(&attr.meta).contains("ignore")
}

fn cfg_is_plain_test(attr: &Attribute) -> bool {
    attr_path_is(attr, "cfg") && meta_tokens(&attr.meta) == "test"
}

fn meta_tokens(meta: &Meta) -> String {
    match meta {
        Meta::List(list) => list.tokens.to_string().split_whitespace().collect(),
        Meta::Path(path) => path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Meta::NameValue(_) => String::new(),
    }
}

fn git_root() -> Result<PathBuf, DynError> {
    let output = Command::new("git").args(["rev-parse", "--show-toplevel"]).output()?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        return Ok(PathBuf::from(stdout.trim()));
    }

    Err(command_error("git rev-parse --show-toplevel", &output.stderr).into())
}

fn command_error(command: &str, stderr: &[u8]) -> TopicDocsError {
    let detail = String::from_utf8_lossy(stderr);
    TopicDocsError::new(format!("{command} failed: {}", detail.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_evidence(names: &[&str]) -> BTreeMap<String, TestEvidence> {
        names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    TestEvidence { runnable: true, ignored: false, cfg_gated: false },
                )
            })
            .collect()
    }

    fn test_evidence_from_source(source: &str) -> BTreeMap<String, TestEvidence> {
        let mut collector = TestCollector::default();
        let file = syn::parse_file(source).expect("test source should parse");
        collector.visit_file(&file);
        collector.evidence
    }

    fn plan_context<'a>(
        evidence: &'a BTreeMap<String, TestEvidence>,
        report: &'a mut Report,
        violations: &'a mut Vec<Violation>,
    ) -> PlanContext<'a> {
        PlanContext {
            path: Path::new("docs/topics/example/test-plan.md"),
            test_evidence: evidence,
            report,
            violations,
        }
    }

    fn string_value<'a>(record: &'a Record, key: &str) -> Option<&'a str> {
        record.fields.get(key).and_then(|field| match &field.value {
            FieldValue::String(value) => Some(value.as_str()),
            FieldValue::Array(_) => None,
        })
    }

    #[test]
    fn parses_requirement_and_case_metadata() {
        let text = r#"
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Right-handed frame."
status = "active"

[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001"]
evidence = "test"
test = "cl_tp_001"
oracle = "Exact equality."
tier = "fast"
status = "implemented"
```
        "#;
        let mut violations = Vec::new();
        let mut report = Report::default();
        let evidence = test_evidence(&[]);
        let records = {
            let mut context = plan_context(&evidence, &mut report, &mut violations);
            parse_records(text, &mut context)
        };

        assert!(violations.is_empty());
        assert_eq!(records.len(), 2);
        assert_eq!(
            records.first().and_then(|record| string_value(record, "id")),
            Some("CL-REQ-001")
        );
        assert_eq!(records.get(1).and_then(|record| string_value(record, "id")), Some("CL-TP-001"));
    }

    #[test]
    fn implemented_executable_case_must_have_discovered_test() {
        let text = r#"
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Right-handed frame."
status = "active"

[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001"]
evidence = "test"
test = "missing_test"
oracle = "Exact equality."
tier = "fast"
status = "implemented"
```
        "#;
        let mut violations = Vec::new();
        let mut report = Report::default();
        let evidence = test_evidence(&[]);
        {
            let mut context = plan_context(&evidence, &mut report, &mut violations);
            validate_plan(text, &mut context);
        }

        assert!(violations.iter().any(|violation| violation.message.contains("missing Rust test")));
        assert_eq!(report.missing_executable_cases, 1);
    }

    #[test]
    fn planned_case_covers_active_requirement_without_executable_test() {
        let text = r#"
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Right-handed frame."
status = "active"

[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001"]
evidence = "test"
oracle = "Exact equality."
tier = "fast"
status = "planned"
```
        "#;
        let mut violations = Vec::new();
        let mut report = Report::default();
        let evidence = test_evidence(&[]);
        {
            let mut context = plan_context(&evidence, &mut report, &mut violations);
            validate_plan(text, &mut context);
        }

        assert!(violations.is_empty());
        assert_eq!(report.active_requirements, 1);
        assert_eq!(report.requirements_with_case_coverage, 1);
        assert_eq!(report.planned_cases, 1);
    }

    #[test]
    fn ignored_executable_case_is_not_implemented_evidence() {
        let text = r#"
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Right-handed frame."
status = "active"

[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001"]
evidence = "test"
test = "ignored_case"
oracle = "Exact equality."
tier = "fast"
status = "implemented"
```
"#;
        let evidence = test_evidence_from_source(
            r"
#[test]
#[ignore]
fn ignored_case() {}
",
        );
        let mut violations = Vec::new();
        let mut report = Report::default();
        {
            let mut context = plan_context(&evidence, &mut report, &mut violations);
            validate_plan(text, &mut context);
        }

        assert!(violations.iter().any(|violation| violation.message.contains("ignored")));
        assert_eq!(report.missing_executable_cases, 1);
    }

    #[test]
    fn cfg_gated_executable_case_is_not_implemented_evidence() {
        let text = r#"
```toml
[[requirement]]
id = "CL-REQ-001"
summary = "Right-handed frame."
status = "active"

[[case]]
id = "CL-TP-001"
requirements = ["CL-REQ-001"]
evidence = "test"
test = "cfg_disabled_case"
oracle = "Exact equality."
tier = "fast"
status = "implemented"
```
"#;
        let evidence = test_evidence_from_source(
            r"
#[cfg(any())]
#[test]
fn cfg_disabled_case() {}
",
        );
        let mut violations = Vec::new();
        let mut report = Report::default();
        {
            let mut context = plan_context(&evidence, &mut report, &mut violations);
            validate_plan(text, &mut context);
        }

        assert!(violations.iter().any(|violation| violation.message.contains("cfg-gated")));
        assert_eq!(report.missing_executable_cases, 1);
    }

    #[test]
    fn cfg_test_module_counts_as_runnable_evidence() {
        let evidence = test_evidence_from_source(
            r"
#[cfg(test)]
mod tests {
    #[test]
    fn nested_case() {}
}
",
        );

        assert_eq!(
            evidence.get("nested_case"),
            Some(&TestEvidence { runnable: true, ignored: false, cfg_gated: false })
        );
    }
}
