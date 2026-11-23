//! The per-round evidence file of `cargo xtask perf --evidence <file>` (`bunny-perf-evidence/1`).
//!
//! Planned: every round of both sides with the build identity of the run, as JSON Lines, so a
//! reader can recompute the report table. Stubbed until the implementation lands.
#![allow(dead_code, clippy::unnecessary_wraps)]

use crate::perf_report::{add_run, Runs};

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

/// The evidence file for `rounds` (stub).
pub(crate) fn evidence_text(
    _identity: &BuildIdentity,
    _rounds: &[RoundOutput],
) -> Result<String, String> {
    Err("not implemented".to_owned())
}

/// Whether `git status --porcelain` output names any change (stub).
pub(crate) fn is_dirty(_porcelain: &str) -> bool {
    false
}

/// The `host:` line of `rustc -vV` (stub).
pub(crate) fn host_triple(_verbose_version: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests;
