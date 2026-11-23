//! Tests for the `bunny-perf-evidence/1` file.
//!
//! The reader at the bottom of this file is written for these tests only. It parses the JSON
//! Lines itself and recomputes the report table with no helper from `perf_report` (no shared
//! median, minimum, floor, change, speedup or formatting code), so a table it reproduces byte
//! for byte is a table any reader of the file could rebuild.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use super::*;
use crate::perf::{report, ReportScope};
use crate::perf_report::Row;

const BASE_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const CANDIDATE_SHA: &str = "fedcba9876543210fedcba9876543210fedcba98";
/// `0.1 + 0.2`, whose shortest round-trip decimal has 17 significant digits.
const AWKWARD: f64 = 0.1 + 0.2;

fn identity(rounds: usize, dirty: bool) -> BuildIdentity {
    BuildIdentity {
        base_ref: "origin/main".to_owned(),
        base_revision: BASE_SHA.to_owned(),
        candidate_revision: CANDIDATE_SHA.to_owned(),
        candidate_dirty: dirty,
        target: "aarch64-apple-darwin".to_owned(),
        target_is_host: true,
        platform: "aarch64-macos (host)".to_owned(),
        rounds,
        align_code: true,
        rustflags:
            "-C llvm-args=-align-all-functions=6 -C llvm-args=-align-all-nofallthru-blocks=6"
                .to_owned(),
        harness_args: vec!["--filter".to_owned(), "num/".to_owned()],
        runner: None,
        max_regression: Some(20.0),
    }
}

/// One constructed case: its p50 on each side, per round, cycled when a run has more rounds
/// than values. A side that is `None` never prints the case.
struct Constructed {
    name: &'static str,
    base: Option<&'static [f64]>,
    candidate: Option<&'static [f64]>,
}

const IDENTICAL: &[f64] = &[52.0, 40.0, 52.0, 40.0, 41.0];

/// Ties at the median and at the minimum, identical sides, a case on one side only, zero
/// timings on either side, and decimals that need every digit to round-trip.
const CASES: &[Constructed] = &[
    Constructed {
        name: "float/awkward",
        base: Some(&[0.1, AWKWARD, 1e-7, 97.62, 80.69, 0.2, 3183.93]),
        candidate: Some(&[97.5, 0.1, 0.2, 1e-7, 3181.0, AWKWARD, 97.0]),
    },
    Constructed { name: "gone/base_only", base: Some(&[4.0, 4.5]), candidate: None },
    Constructed { name: "new/candidate_only", base: None, candidate: Some(&[10.0, 9.5, 10.5]) },
    Constructed { name: "same/identical_sides", base: Some(IDENTICAL), candidate: Some(IDENTICAL) },
    Constructed { name: "tie/flat", base: Some(&[5.0]), candidate: Some(&[5.0]) },
    Constructed {
        name: "tie/middle",
        base: Some(&[7.25, 7.25, 9.0, 7.25, 6.5, 7.25, 8.0]),
        candidate: Some(&[6.5, 7.25, 7.25, 7.25, 9.0, 8.0, 7.25]),
    },
    Constructed { name: "zero/base", base: Some(&[0.0]), candidate: Some(&[1.5, 1.25]) },
    Constructed { name: "zero/candidate", base: Some(&[2.0, 2.5]), candidate: Some(&[0.0]) },
];

/// What `bunny-perf` would print for `side` in `round` (1-based) of the constructed cases.
fn harness_output(side: Side, round: u32) -> String {
    let mut out = String::new();
    let step = f64::from(round);
    let index = usize::try_from(round - 1).expect("small round");
    for case in CASES {
        let values = match side {
            Side::Base => case.base,
            Side::Candidate => case.candidate,
        };
        let Some(values) = values else { continue };
        let p50 = values[index % values.len()];
        let (p95, p99) = (p50 + 0.125 * step, p50 + 0.25 * step);
        if case.name == "tie/flat" {
            // The harness prints two decimals; the file keeps the parsed value.
            let _ = writeln!(out, "{}\t{p50:.2}\t{p95:.2}\t{p99:.2}", case.name);
        } else {
            let _ = writeln!(out, "{}\t{p50}\t{p95}\t{p99}", case.name);
        }
    }
    out
}

/// Records `rounds` rounds of both sides, in the order `run_interleaved` runs them.
fn measured(rounds: u32) -> Measured {
    let mut measured = Measured::default();
    for round in 1..=rounds {
        let round_index = usize::try_from(round).expect("small round");
        let order = if round % 2 == 1 {
            [Side::Base, Side::Candidate]
        } else {
            [Side::Candidate, Side::Base]
        };
        for side in order {
            measured.record(side, round_index, harness_output(side, round)).expect("valid output");
        }
    }
    measured
}

fn production_report(measured: &Measured, identity: &BuildIdentity) -> (Vec<Row>, String) {
    report(
        measured,
        &ReportScope {
            base: &identity.base_ref,
            sha: &identity.base_revision,
            platform: &identity.platform,
            rounds: identity.rounds,
        },
    )
}

#[test]
fn an_independent_reading_of_the_evidence_recomputes_the_report_table() {
    // One round, an even and an odd count below five, and two counts where the floor skips
    // the lowest round.
    for rounds in [1_u32, 2, 4, 5, 7] {
        let rounds_usize = usize::try_from(rounds).expect("small round");
        let measured = measured(rounds);
        let identity = identity(rounds_usize, false);
        let evidence = evidence_text(&identity, &measured.rounds).expect("finite timings");
        let (rows, table) = production_report(&measured, &identity);

        let reading = Reading::parse(&evidence);
        reading.assert_every_round_is_present(rounds_usize);
        assert_eq!(reading.table(), table, "{rounds} rounds:\n{evidence}");
        for row in &rows {
            let (base_floor, candidate_floor) = reading.floors(&row.case);
            assert_eq!(row.base_floor.map(f64::to_bits), base_floor.map(f64::to_bits));
            assert_eq!(row.head_floor.map(f64::to_bits), candidate_floor.map(f64::to_bits));
        }
    }
}

#[test]
fn the_evidence_format_is_pinned_line_by_line() {
    let mut measured = Measured::default();
    measured.record(Side::Candidate, 1, "num/mul\t1.375\t1.5\t1.75\n".to_owned()).expect("valid");
    measured.record(Side::Base, 1, "num/mul\t1.50\t2\t3.25\n".to_owned()).expect("valid");
    let identity = BuildIdentity {
        base_ref: "a\"b\\c\u{1}".to_owned(),
        candidate_dirty: true,
        target: "wasm32-wasip1".to_owned(),
        target_is_host: false,
        platform: "wasm32-wasip1".to_owned(),
        rounds: 1,
        align_code: false,
        rustflags: String::new(),
        harness_args: Vec::new(),
        runner: Some("wasmtime run".to_owned()),
        max_regression: None,
        ..identity(1, true)
    };
    let expected = concat!(
        r#"{"schema":"bunny-perf-evidence/1","kind":"build","base_ref":"a\"b\\c\u0001","#,
        r#""base_revision":"0123456789abcdef0123456789abcdef01234567","#,
        r#""candidate_revision":"fedcba9876543210fedcba9876543210fedcba98","#,
        r#""candidate_dirty":true,"target":"wasm32-wasip1","target_is_host":false,"#,
        r#""platform":"wasm32-wasip1","rounds":1,"align_code":false,"rustflags":"","#,
        r#""harness_args":[],"runner":"wasmtime run","max_regression":null}"#,
        "\n",
        r#"{"schema":"bunny-perf-evidence/1","kind":"round","case":"num/mul","side":"base","#,
        r#""round":1,"p50":1.5,"p95":2,"p99":3.25}"#,
        "\n",
        r#"{"schema":"bunny-perf-evidence/1","kind":"round","case":"num/mul","#,
        r#""side":"candidate","round":1,"p50":1.375,"p95":1.5,"p99":1.75}"#,
        "\n",
    );
    assert_eq!(evidence_text(&identity, &measured.rounds).expect("finite"), expected);
    let build = &Reading::parse(expected).build;
    assert_eq!(build["base_ref"], Value::Str("a\"b\\c\u{1}".to_owned()));
    assert_eq!(build["harness_args"], Value::List(Vec::new()));
}

#[test]
fn the_evidence_is_the_same_whatever_order_the_rounds_were_recorded_in() {
    let forward = measured(5);
    let mut backward = Measured::default();
    for output in forward.rounds.iter().rev() {
        backward.record(output.side, output.round, output.output.clone()).expect("valid output");
    }
    let identity = identity(5, false);
    let text = evidence_text(&identity, &forward.rounds).expect("finite");
    assert_eq!(evidence_text(&identity, &backward.rounds).expect("finite"), text);
    assert_eq!(production_report(&backward, &identity).1, production_report(&forward, &identity).1);
    // Lines are ordered by case, then side (base first), then round.
    let keys: Vec<(String, String, usize)> = Reading::parse(&text)
        .rounds
        .iter()
        .map(|line| (line.case.clone(), line.side.clone(), line.round))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn the_dirty_flag_follows_git_status() {
    assert!(!is_dirty(""));
    assert!(!is_dirty("\n"));
    assert!(is_dirty(" M xtask/src/perf.rs\n"));
    assert!(is_dirty("M  xtask/src/perf.rs\n"));
    assert!(is_dirty("?? crates/bunny-perf/src/new_case.rs\n"));
    for dirty in [false, true] {
        let text = evidence_text(&identity(1, dirty), &measured(1).rounds).expect("finite");
        assert_eq!(Reading::parse(&text).build["candidate_dirty"], Value::Bool(dirty));
    }
}

#[test]
fn non_finite_numbers_are_refused() {
    for output in ["a\tinf\t1\t1\n", "a\t1\t1\tNaN\n", "a\t1\t-inf\t1\n"] {
        let mut measured = Measured::default();
        measured.record(Side::Base, 1, output.to_owned()).expect("the report accepts it");
        let err = evidence_text(&identity(1, false), &measured.rounds).expect_err("not finite");
        assert!(err.contains("non-finite"), "{err}");
    }
    let unbounded = BuildIdentity { max_regression: Some(f64::INFINITY), ..identity(1, false) };
    let err = evidence_text(&unbounded, &measured(1).rounds).expect_err("not finite");
    assert!(err.contains("non-finite"), "{err}");
}

#[test]
fn the_host_triple_comes_from_rustc_verbose_version() {
    let verbose = "rustc 1.96.0 (abc 2026-01-01)\nbinary: rustc\nhost: aarch64-apple-darwin\n";
    assert_eq!(host_triple(verbose).as_deref(), Some("aarch64-apple-darwin"));
    assert_eq!(
        host_triple("host: x86_64-pc-windows-msvc\r\n").as_deref(),
        Some("x86_64-pc-windows-msvc")
    );
    assert_eq!(host_triple("rustc 1.96.0\n"), None);
}

// ---------------------------------------------------------------------------------------------
// The independent reader.

#[derive(Debug, PartialEq)]
enum Value {
    Str(String),
    /// A number as written, parsed by whoever needs it.
    Num(String),
    Bool(bool),
    Null,
    List(Vec<String>),
}

struct RoundLine {
    case: String,
    side: String,
    round: usize,
    p50: f64,
    p95: f64,
}

struct Reading {
    build: BTreeMap<String, Value>,
    rounds: Vec<RoundLine>,
}

impl Reading {
    fn parse(text: &str) -> Self {
        let mut objects = text.lines().map(|line| parse_object(line.trim_end_matches('\r')));
        let build = objects.next().expect("a build line");
        assert_eq!(build["schema"], Value::Str("bunny-perf-evidence/1".to_owned()));
        assert_eq!(build["kind"], Value::Str("build".to_owned()));
        let rounds = objects
            .map(|line| {
                assert_eq!(line["schema"], Value::Str("bunny-perf-evidence/1".to_owned()));
                assert_eq!(line["kind"], Value::Str("round".to_owned()));
                RoundLine {
                    case: text_of(&line["case"]),
                    side: text_of(&line["side"]),
                    round: number_of(&line["round"]).parse().expect("a round number"),
                    p50: number_of(&line["p50"]).parse().expect("a p50"),
                    p95: number_of(&line["p95"]).parse().expect("a p95"),
                }
            })
            .collect();
        Self { build, rounds }
    }

    fn side(&self, case: &str, side: &str) -> Option<Vec<&RoundLine>> {
        let lines: Vec<&RoundLine> =
            self.rounds.iter().filter(|line| line.case == case && line.side == side).collect();
        (!lines.is_empty()).then_some(lines)
    }

    fn assert_every_round_is_present(&self, rounds: usize) {
        for case in CASES {
            for (side, values) in [("base", case.base), ("candidate", case.candidate)] {
                let mut seen: Vec<usize> = self
                    .side(case.name, side)
                    .map(|lines| lines.iter().map(|line| line.round).collect())
                    .unwrap_or_default();
                seen.sort_unstable();
                let expected: Vec<usize> =
                    if values.is_some() { (1..=rounds).collect() } else { Vec::new() };
                assert_eq!(seen, expected, "{} {side}", case.name);
            }
        }
    }

    fn floors(&self, case: &str) -> (Option<f64>, Option<f64>) {
        let floor = |side| {
            let mut p50s: Vec<f64> = self.side(case, side)?.iter().map(|l| l.p50).collect();
            p50s.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            Some(if p50s.len() >= 5 { p50s[1] } else { p50s[0] })
        };
        (floor("base"), floor("candidate"))
    }

    /// The report table, rebuilt from the file alone.
    fn table(&self) -> String {
        let base_sha = text_of(&self.build["base_revision"]);
        let mut out = format!(
            "### Bunny perf on `{}`: `{} ({})` → `working tree`\n\n",
            text_of(&self.build["platform"]),
            text_of(&self.build["base_ref"]),
            &base_sha[..12],
        );
        let _ = write!(
            out,
            "{} interleaved rounds per side; nanoseconds per call. Median and best are taken \
             across rounds; speedup and the regression gate use the best p50. Negative change \
             is faster.\n\n",
            number_of(&self.build["rounds"]),
        );
        out.push_str(
            "| Case | Before p50 | After p50 | Change | Before best | After best | Best change \
             | Speedup | Before p95 | After p95 |\n",
        );
        out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        let cases: BTreeSet<&str> = self.rounds.iter().map(|line| line.case.as_str()).collect();
        for case in cases {
            out.push_str(&self.row(case));
        }
        out
    }

    fn row(&self, case: &str) -> String {
        let summary = |side| -> Option<(f64, f64, f64)> {
            let lines = self.side(case, side)?;
            let middle = |values: Vec<f64>| {
                let mut values = values;
                values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                let n = values.len();
                if n % 2 == 1 {
                    values[n / 2]
                } else {
                    (values[n / 2 - 1] + values[n / 2]) / 2.0
                }
            };
            let lowest = lines.iter().map(|l| l.p50).fold(f64::INFINITY, f64::min);
            Some((
                middle(lines.iter().map(|l| l.p50).collect()),
                lowest,
                middle(lines.iter().map(|l| l.p95).collect()),
            ))
        };
        let (before, after) = (summary("base"), summary("candidate"));
        let change = |b: Option<f64>, a: Option<f64>| match (b, a) {
            (Some(b), Some(a)) if b > 0.0 => format!("{:+.1}%", (a - b) / b * 100.0),
            _ => "new".to_owned(),
        };
        let ns = |v: Option<f64>| v.map_or_else(|| "—".to_owned(), |v| format!("{v:.2}"));
        let speedup = match (before, after) {
            (Some(b), Some(a)) if a.1 > 0.0 => format!("{:.2}×", b.1 / a.1),
            _ => "—".to_owned(),
        };
        format!(
            "| `{case}` | {} | {} | {} | {} | {} | {} | {speedup} | {} | {} |\n",
            ns(before.map(|s| s.0)),
            ns(after.map(|s| s.0)),
            change(before.map(|s| s.0), after.map(|s| s.0)),
            ns(before.map(|s| s.1)),
            ns(after.map(|s| s.1)),
            change(before.map(|s| s.1), after.map(|s| s.1)),
            ns(before.map(|s| s.2)),
            ns(after.map(|s| s.2)),
        )
    }
}

fn text_of(value: &Value) -> String {
    match value {
        Value::Str(text) => text.clone(),
        other => panic!("expected a string, found {other:?}"),
    }
}

fn number_of(value: &Value) -> &str {
    match value {
        Value::Num(text) => text,
        other => panic!("expected a number, found {other:?}"),
    }
}

/// Parses one flat JSON object whose values are strings, numbers, booleans, `null` or arrays of
/// strings: everything `bunny-perf-evidence/1` writes.
fn parse_object(line: &str) -> BTreeMap<String, Value> {
    let mut reader = Reader { chars: line.chars().peekable() };
    reader.expect('{');
    let mut object = BTreeMap::new();
    loop {
        let key = reader.string();
        reader.expect(':');
        let value = reader.value();
        assert!(object.insert(key, value).is_none(), "duplicate key in {line}");
        match reader.next() {
            ',' => {}
            '}' => break,
            other => panic!("unexpected {other:?} in {line}"),
        }
    }
    assert!(reader.chars.next().is_none(), "trailing text in {line}");
    object
}

struct Reader<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Reader<'_> {
    fn next(&mut self) -> char {
        self.chars.next().expect("unexpected end of line")
    }

    fn expect(&mut self, wanted: char) {
        assert_eq!(self.next(), wanted);
    }

    fn string(&mut self) -> String {
        self.expect('"');
        let mut text = String::new();
        loop {
            match self.next() {
                '"' => return text,
                '\\' => text.push(self.escape()),
                c => {
                    assert!(c >= ' ', "raw control character");
                    text.push(c);
                }
            }
        }
    }

    fn escape(&mut self) -> char {
        match self.next() {
            '"' => '"',
            '\\' => '\\',
            '/' => '/',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'u' => {
                let hex: String = (0..4).map(|_| self.next()).collect();
                char::from_u32(u32::from_str_radix(&hex, 16).expect("hex")).expect("a char")
            }
            other => panic!("bad escape \\{other}"),
        }
    }

    fn value(&mut self) -> Value {
        match self.chars.peek().copied() {
            Some('"') => Value::Str(self.string()),
            Some('[') => {
                self.expect('[');
                let mut items = Vec::new();
                if self.chars.peek() == Some(&']') {
                    self.expect(']');
                    return Value::List(items);
                }
                loop {
                    items.push(self.string());
                    match self.next() {
                        ',' => {}
                        ']' => return Value::List(items),
                        other => panic!("unexpected {other:?} in a list"),
                    }
                }
            }
            Some(_) => {
                let mut word = String::new();
                while let Some(&c) = self.chars.peek() {
                    if c == ',' || c == '}' {
                        break;
                    }
                    word.push(c);
                    self.chars.next();
                }
                match word.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    "null" => Value::Null,
                    number => {
                        assert!(
                            number.chars().all(|c| c.is_ascii_digit() || "+-.eE".contains(c)),
                            "not a number: {number}"
                        );
                        Value::Num(number.to_owned())
                    }
                }
            }
            None => panic!("missing value"),
        }
    }
}
