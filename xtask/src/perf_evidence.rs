//! The per-round evidence file of `cargo xtask perf --evidence <file>` (`bunny-perf-evidence/1`).
//!
//! JSON Lines: one `build` line with the identity of the run, then one `round` line per case,
//! side and round, sorted by case, then side (base first), then round. Every line carries the
//! schema identifier, keys are written in a fixed order, numbers are the shortest decimal that
//! reads back as the same `f64`, and nothing machine-local (no path, no clock) is added. A
//! reader recomputes the report table from the file alone. The fields are documented in
//! `docs/topics/performance/README.md#evidence-files`.

use std::fmt::Write as _;

use crate::perf_report::{add_run, Runs, Timing};

/// The schema identifier and version carried by every line of an evidence file.
pub(crate) const PERF_EVIDENCE_SCHEMA: &str = "bunny-perf-evidence/1";

/// Which build a round timed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Side {
    /// The base revision.
    Base,
    /// The working tree.
    Candidate,
}

/// One side's harness output for one round, kept as measured.
#[derive(Clone, Debug)]
pub(crate) struct RoundOutput {
    pub(crate) side: Side,
    /// 1-based.
    pub(crate) round: usize,
    pub(crate) output: String,
}

/// Every round of a comparison: each side's timings as the report reads them, and the raw
/// outputs they were parsed from.
#[derive(Debug, Default)]
pub(crate) struct Measured {
    pub(crate) base: Runs,
    pub(crate) candidate: Runs,
    pub(crate) rounds: Vec<RoundOutput>,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Candidate => "candidate",
        }
    }
}

impl Measured {
    /// Parses one round of one side into that side's runs, and keeps the output.
    pub(crate) fn record(
        &mut self,
        side: Side,
        round: usize,
        output: String,
    ) -> Result<(), String> {
        let runs = match side {
            Side::Base => &mut self.base,
            Side::Candidate => &mut self.candidate,
        };
        add_run(runs, &output)?;
        self.rounds.push(RoundOutput { side, round, output });
        Ok(())
    }
}

/// What was built and how it was run.
#[derive(Clone, Debug)]
pub(crate) struct BuildIdentity {
    pub(crate) base_ref: String,
    pub(crate) base_revision: String,
    pub(crate) candidate_revision: String,
    pub(crate) candidate_dirty: bool,
    pub(crate) target: String,
    pub(crate) target_is_host: bool,
    pub(crate) platform: String,
    pub(crate) rounds: usize,
    pub(crate) align_code: bool,
    pub(crate) rustflags: String,
    pub(crate) harness_args: Vec<String>,
    pub(crate) runner: Option<String>,
    pub(crate) max_regression: Option<f64>,
}

/// The evidence file for `rounds`.
///
/// # Errors
///
/// A harness output that does not parse, or a timing or threshold that is not finite (JSON has
/// no spelling for infinity or NaN).
pub(crate) fn evidence_text(
    identity: &BuildIdentity,
    rounds: &[RoundOutput],
) -> Result<String, String> {
    let mut lines = Vec::new();
    for output in rounds {
        let mut parsed = Runs::new();
        add_run(&mut parsed, &output.output)?;
        for (case, timings) in parsed {
            lines.extend(
                timings.into_iter().map(|timing| (case.clone(), output.side, output.round, timing)),
            );
        }
    }
    // A stable sort: a case printed twice in one round keeps its printed order.
    lines.sort_by(|a, b| (&a.0, a.1, a.2).cmp(&(&b.0, b.1, b.2)));
    let mut text = build_line(identity)?;
    for (case, side, round, timing) in &lines {
        text.push_str(&round_line(case, *side, *round, *timing)?);
    }
    Ok(text)
}

fn build_line(identity: &BuildIdentity) -> Result<String, String> {
    let mut line = Line::new("build");
    line.string("base_ref", &identity.base_ref);
    line.string("base_revision", &identity.base_revision);
    line.string("candidate_revision", &identity.candidate_revision);
    line.raw("candidate_dirty", &identity.candidate_dirty.to_string());
    line.string("target", &identity.target);
    line.raw("target_is_host", &identity.target_is_host.to_string());
    line.string("platform", &identity.platform);
    line.raw("rounds", &identity.rounds.to_string());
    line.raw("align_code", &identity.align_code.to_string());
    line.string("rustflags", &identity.rustflags);
    line.strings("harness_args", &identity.harness_args);
    match &identity.runner {
        Some(runner) => line.string("runner", runner),
        None => line.raw("runner", "null"),
    }
    match identity.max_regression {
        Some(limit) => line.number("max_regression", limit, "--max-regression")?,
        None => line.raw("max_regression", "null"),
    }
    Ok(line.finish())
}

fn round_line(case: &str, side: Side, round: usize, timing: Timing) -> Result<String, String> {
    let mut line = Line::new("round");
    line.string("case", case);
    line.string("side", side.name());
    line.raw("round", &round.to_string());
    let what = |field: &str| format!("{case} {} round {round} {field}", side.name());
    line.number("p50", timing.p50, &what("p50"))?;
    line.number("p95", timing.p95, &what("p95"))?;
    line.number("p99", timing.p99, &what("p99"))?;
    Ok(line.finish())
}

/// One JSON object on one line, keys in the order they are added.
struct Line(String);

impl Line {
    fn new(kind: &str) -> Self {
        let mut line = Self(String::from("{"));
        line.string("schema", PERF_EVIDENCE_SCHEMA);
        line.string("kind", kind);
        line
    }

    fn key(&mut self, key: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        push_json_string(&mut self.0, key);
        self.0.push(':');
    }

    fn raw(&mut self, key: &str, value: &str) {
        self.key(key);
        self.0.push_str(value);
    }

    fn string(&mut self, key: &str, value: &str) {
        self.key(key);
        push_json_string(&mut self.0, value);
    }

    fn strings(&mut self, key: &str, values: &[String]) {
        self.key(key);
        self.0.push('[');
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.0.push(',');
            }
            push_json_string(&mut self.0, value);
        }
        self.0.push(']');
    }

    /// Rust's `Display` for `f64` is the shortest decimal that parses back to the same value,
    /// with no exponent, which is a valid JSON number when the value is finite.
    fn number(&mut self, key: &str, value: f64, what: &str) -> Result<(), String> {
        if !value.is_finite() {
            return Err(format!(
                "{what} is non-finite ({value}); {PERF_EVIDENCE_SCHEMA} records finite numbers only"
            ));
        }
        self.raw(key, &value.to_string());
        Ok(())
    }

    fn finish(mut self) -> String {
        self.0.push_str("}\n");
        self.0
    }
}

/// `value` as a JSON string: quotes and backslashes escaped, control characters as `\u00XX`.
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Whether `git status --porcelain` output names any change: a modified, staged, deleted or
/// renamed tracked file, or an untracked file that is not ignored. Untracked files count because
/// the harness copies every file of `crates/bunny-perf/src`, tracked or not.
pub(crate) fn is_dirty(porcelain: &str) -> bool {
    porcelain.lines().any(|line| !line.trim().is_empty())
}

/// The `host:` line of `rustc -vV`.
pub(crate) fn host_triple(verbose_version: &str) -> Option<String> {
    verbose_version
        .lines()
        .find_map(|line| line.strip_prefix("host:"))
        .map(|triple| triple.trim().to_owned())
}

#[cfg(test)]
mod json;
#[cfg(test)]
mod tests;
