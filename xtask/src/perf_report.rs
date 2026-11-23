//! Parsing, aggregation and comparison of `bunny-perf` runs.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Per-call nanoseconds for one case in one run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Timing {
    pub(crate) p50: f64,
    pub(crate) p95: f64,
    pub(crate) p99: f64,
}

/// Every case's timing in each run, in run order.
pub(crate) type Runs = BTreeMap<String, Vec<Timing>>;

/// Parses `case<TAB>p50<TAB>p95<TAB>p99` lines from one run into `runs`.
pub(crate) fn add_run(runs: &mut Runs, output: &str) -> Result<(), String> {
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        let [case, p50, p95, p99] = fields.as_slice() else {
            return Err(format!("unexpected bunny-perf line: {line:?}"));
        };
        let number = |text: &str| {
            text.parse::<f64>().map_err(|err| format!("bad number {text:?} in {line:?}: {err}"))
        };
        let timing = Timing { p50: number(p50)?, p95: number(p95)?, p99: number(p99)? };
        runs.entry((*case).to_owned()).or_default().push(timing);
    }
    Ok(())
}

/// The median across runs of each percentile, which damps one noisy run.
pub(crate) fn median_timing(timings: &[Timing]) -> Option<Timing> {
    Some(Timing {
        p50: median(timings.iter().map(|t| t.p50))?,
        p95: median(timings.iter().map(|t| t.p95))?,
        p99: median(timings.iter().map(|t| t.p99))?,
    })
}

fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values: Vec<f64> = values.collect();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    match values.len() {
        0 => None,
        n if n % 2 == 1 => values.get(middle).copied(),
        _ => Some((values.get(middle - 1)? + values.get(middle)?) / 2.0),
    }
}

/// One case compared across the two revisions.
#[derive(Debug, PartialEq)]
pub(crate) struct Row {
    pub(crate) case: String,
    /// Median of each percentile across rounds.
    pub(crate) base: Option<Timing>,
    pub(crate) head: Option<Timing>,
    /// Lowest p50 across rounds. Noise only adds time, so with enough rounds this finds a
    /// case's fast mode even when process placement makes it bimodal.
    pub(crate) base_best: Option<f64>,
    pub(crate) head_best: Option<f64>,
    /// The p50 the gate compares: the lowest, or the second-lowest once there are enough
    /// rounds that one lucky round is not the only evidence of a fast mode.
    pub(crate) base_floor: Option<f64>,
    pub(crate) head_floor: Option<f64>,
}

impl Row {
    /// Relative change of the median p50, in percent (positive is slower).
    pub(crate) fn change_percent(&self) -> Option<f64> {
        relative_change(self.base?.p50, self.head?.p50)
    }

    /// Relative change of the best p50, in percent (positive is slower). It is shown in the
    /// table, but one lucky round on either side moves it, so the gate does not use it alone.
    pub(crate) fn best_change_percent(&self) -> Option<f64> {
        relative_change(self.base_best?, self.head_best?)
    }

    /// Relative change of the gate's floor p50 (see [`floor_p50`]), in percent (positive is
    /// slower).
    pub(crate) fn floor_change_percent(&self) -> Option<f64> {
        relative_change(self.base_floor?, self.head_floor?)
    }
}

/// Rounds from which the gate's floor skips the single lowest p50.
pub(crate) const ROBUST_FLOOR_ROUNDS: usize = 5;

fn relative_change(before: f64, after: f64) -> Option<f64> {
    (before > 0.0).then(|| (after - before) / before * 100.0)
}

fn best_p50(timings: &[Timing]) -> Option<f64> {
    timings.iter().map(|t| t.p50).min_by(f64::total_cmp)
}

/// The p50 the gate treats as a side's fast mode: the lowest across rounds, except that with
/// at least [`ROBUST_FLOOR_ROUNDS`] rounds the second-lowest, so one lucky round on either
/// side neither fails an unrelated change nor hides a real regression. A fast mode that
/// shows up in two or more rounds is still found.
fn floor_p50(timings: &[Timing]) -> Option<f64> {
    let mut p50s: Vec<f64> = timings.iter().map(|t| t.p50).collect();
    p50s.sort_by(f64::total_cmp);
    let at = usize::from(p50s.len() >= ROBUST_FLOOR_ROUNDS);
    p50s.get(at).copied()
}

/// Pairs every case seen in either revision, in case order.
pub(crate) fn compare(base: &Runs, head: &Runs) -> Vec<Row> {
    let mut cases: Vec<&String> = base.keys().chain(head.keys()).collect();
    cases.sort();
    cases.dedup();
    cases
        .into_iter()
        .map(|case| Row {
            case: case.clone(),
            base: base.get(case).and_then(|t| median_timing(t)),
            head: head.get(case).and_then(|t| median_timing(t)),
            base_best: base.get(case).and_then(|t| best_p50(t)),
            head_best: head.get(case).and_then(|t| best_p50(t)),
            base_floor: base.get(case).and_then(|t| floor_p50(t)),
            head_floor: head.get(case).and_then(|t| floor_p50(t)),
        })
        .collect()
}

/// Cases whose floor p50 and median p50 both slowed down by more than `max_percent`. The floor
/// keeps the bimodal-case reasoning (noise only adds time); the median must agree, so a fast
/// mode that appears on one side only does not fail a case whose typical round did not move.
pub(crate) fn regressions(rows: &[Row], max_percent: f64) -> Vec<&Row> {
    rows.iter()
        .filter(|row| {
            row.floor_change_percent().is_some_and(|change| change > max_percent)
                && row.change_percent().is_some_and(|change| change > max_percent)
        })
        .collect()
}

/// A Markdown before/after table.
/// What a report compares, for its heading.
pub(crate) struct Heading<'a> {
    pub(crate) base: &'a str,
    pub(crate) head: &'a str,
    pub(crate) platform: &'a str,
    pub(crate) rounds: usize,
}

pub(crate) fn render(rows: &[Row], heading: &Heading<'_>) -> String {
    let Heading { base: base_label, head: head_label, platform, rounds } = *heading;
    let mut out = String::new();
    let _ = writeln!(out, "### Bunny perf on `{platform}`: `{base_label}` → `{head_label}`\n");
    let _ = writeln!(
        out,
        "{rounds} interleaved rounds per side; nanoseconds per call. Median and best are taken \
         across rounds; speedup and the regression gate use the best p50. Negative change is faster.\n"
    );
    out.push_str("| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup |");
    out.push_str(" Before p95 | After p95 |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for row in rows {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.case,
            ns(row.base.map(|t| t.p50)),
            ns(row.head.map(|t| t.p50)),
            percent(row.change_percent()),
            ns(row.base_best),
            ns(row.head_best),
            percent(row.best_change_percent()),
            speedup(row.base_best, row.head_best),
            ns(row.base.map(|t| t.p95)),
            ns(row.head.map(|t| t.p95)),
        );
    }
    out
}

fn ns(value: Option<f64>) -> String {
    value.map_or("—".to_owned(), |v| format!("{v:.2}"))
}

fn percent(value: Option<f64>) -> String {
    value.map_or("new".to_owned(), |v| format!("{v:+.1}%"))
}

fn speedup(before: Option<f64>, after: Option<f64>) -> String {
    match (before, after) {
        (Some(before), Some(after)) if after > 0.0 => format!("{:.2}×", before / after),
        _ => "—".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(p50: f64) -> Timing {
        Timing { p50, p95: p50 + 1.0, p99: p50 + 2.0 }
    }

    #[test]
    fn runs_parse_tab_separated_lines() {
        let mut runs = Runs::new();
        add_run(&mut runs, "num/mul\t1.50\t2.00\t3.00\n\nnum/sqrt\t60\t61\t62\n")
            .expect("valid output");
        assert_eq!(runs["num/mul"], vec![Timing { p50: 1.5, p95: 2.0, p99: 3.0 }]);
        assert_eq!(runs["num/sqrt"].len(), 1);
        assert!(add_run(&mut runs, "num/mul\t1.5\n").is_err());
        assert!(add_run(&mut runs, "num/mul\tx\t2\t3\n").is_err());
    }

    #[test]
    fn median_damps_one_noisy_round() {
        let median =
            median_timing(&[timing(10.0), timing(90.0), timing(11.0)]).expect("three runs");
        assert!((median.p50 - 11.0).abs() < f64::EPSILON);
        let even = median_timing(&[timing(10.0), timing(12.0)]).expect("two runs");
        assert!((even.p50 - 11.0).abs() < f64::EPSILON);
        assert_eq!(median_timing(&[]), None);
    }

    #[test]
    fn comparison_reports_change_and_new_cases() {
        let base = Runs::from([("num/sqrt".to_owned(), vec![timing(60.0)])]);
        let head = Runs::from([
            ("num/sqrt".to_owned(), vec![timing(15.0)]),
            ("angle/sin_cos".to_owned(), vec![timing(10.0)]),
        ]);
        let rows = compare(&base, &head);
        assert_eq!(
            rows.iter().map(|r| r.case.as_str()).collect::<Vec<_>>(),
            ["angle/sin_cos", "num/sqrt"]
        );
        assert_eq!(rows[0].change_percent(), None);
        assert!((rows[1].change_percent().expect("both sides") + 75.0).abs() < 1e-9);
        let table = render(
            &rows,
            &Heading { base: "main", head: "HEAD", platform: "test-platform", rounds: 5 },
        );
        assert!(
            table.contains(
                "| `num/sqrt` | 60.00 | 15.00 | -75.0% | 60.00 | 15.00 | -75.0% | 4.00× |"
            ),
            "{table}"
        );
        assert!(
            table.contains("| `angle/sin_cos` | — | 10.00 | new | — | 10.00 | new | — |"),
            "{table}"
        );
    }

    #[test]
    fn regressions_respect_the_threshold() {
        let base = Runs::from([
            ("a".to_owned(), vec![timing(10.0)]),
            ("b".to_owned(), vec![timing(10.0)]),
        ]);
        let head = Runs::from([
            ("a".to_owned(), vec![timing(11.0)]),
            ("b".to_owned(), vec![timing(12.0)]),
        ]);
        let rows = compare(&base, &head);
        let slow: Vec<&str> = regressions(&rows, 15.0).iter().map(|r| r.case.as_str()).collect();
        assert_eq!(slow, ["b"]);
        assert!(regressions(&rows, 25.0).is_empty());
    }

    #[test]
    fn the_gate_ignores_a_bimodal_case_whose_median_lands_in_different_modes() {
        // Identical code that runs at 40 or 52 ns depending on process placement.
        let base =
            Runs::from([("aabb".to_owned(), vec![timing(40.0), timing(52.0), timing(52.0)])]);
        let head =
            Runs::from([("aabb".to_owned(), vec![timing(52.0), timing(40.0), timing(40.0)])]);
        let rows = compare(&base, &head);
        assert!(
            rows[0].change_percent().expect("both sides") < -20.0,
            "the medians differ by mode"
        );
        assert_eq!(rows[0].best_change_percent(), Some(0.0));
        assert!(regressions(&rows, 15.0).is_empty());
    }

    /// The shape of the false failure on pull request 252: seven rounds each, one base round
    /// that happened to run fast, every other round on both sides within 0.5 %.
    fn pr_252() -> (Runs, Runs) {
        let base = [97.62, 80.69, 97.70, 97.55, 97.62, 97.80, 97.60].map(timing).to_vec();
        let head = [97.50, 97.00, 97.60, 97.40, 97.50, 97.70, 97.45].map(timing).to_vec();
        (
            Runs::from([("query/ray_triangle".to_owned(), base)]),
            Runs::from([("query/ray_triangle".to_owned(), head)]),
        )
    }

    #[test]
    fn one_lucky_base_round_does_not_fail_the_gate() {
        let (base, head) = pr_252();
        let rows = compare(&base, &head);
        assert!(
            rows[0].best_change_percent().expect("both sides") > 20.0,
            "the minimum alone would trip the gate"
        );
        assert!(regressions(&rows, 20.0).is_empty(), "one lucky round is not a regression");
    }

    #[test]
    fn a_regression_in_every_round_still_fails_the_gate() {
        let base = [100.0, 101.0, 99.5, 100.5, 100.2, 99.8, 100.1].map(timing).to_vec();
        let head = [130.0, 131.0, 129.5, 130.5, 130.2, 129.8, 130.1].map(timing).to_vec();
        let rows =
            compare(&Runs::from([("c".to_owned(), base)]), &Runs::from([("c".to_owned(), head)]));
        let slow: Vec<&str> = regressions(&rows, 20.0).iter().map(|r| r.case.as_str()).collect();
        assert_eq!(slow, ["c"]);
    }

    #[test]
    fn one_lucky_head_round_does_not_hide_a_regression() {
        let base = [100.0, 101.0, 99.5, 100.5, 100.2, 99.8, 100.1].map(timing).to_vec();
        let head = [130.0, 95.0, 129.5, 130.5, 130.2, 129.8, 130.1].map(timing).to_vec();
        let rows =
            compare(&Runs::from([("c".to_owned(), base)]), &Runs::from([("c".to_owned(), head)]));
        assert!(rows[0].best_change_percent().expect("both sides") < 0.0, "the minimum hides it");
        assert_eq!(regressions(&rows, 20.0).len(), 1);
    }

    #[test]
    fn a_floor_that_regresses_while_the_median_does_not_passes() {
        // Fast mode on the base side in 3 of 7 rounds, never on the head side; medians agree.
        let base = [40.0, 40.0, 40.0, 52.0, 52.0, 52.0, 52.0].map(timing).to_vec();
        let head = [52.0, 52.0, 52.0, 52.0, 52.0, 52.0, 52.0].map(timing).to_vec();
        let rows =
            compare(&Runs::from([("c".to_owned(), base)]), &Runs::from([("c".to_owned(), head)]));
        assert!(rows[0].change_percent().expect("both sides").abs() < f64::EPSILON);
        assert!(regressions(&rows, 20.0).is_empty());
    }

    #[test]
    fn the_floor_skips_the_lowest_round_only_from_five_rounds() {
        let four = [10.0, 20.0, 21.0, 22.0].map(timing);
        let five = [10.0, 20.0, 21.0, 22.0, 23.0].map(timing);
        assert_eq!(floor_p50(&four), Some(10.0));
        assert_eq!(floor_p50(&five), Some(20.0));
        assert_eq!(floor_p50(&[]), None);
        // Two rounds that reach the fast mode keep it: the floor is not a median.
        let two_fast = [40.0, 40.0, 52.0, 52.0, 52.0].map(timing);
        assert_eq!(floor_p50(&two_fast), Some(40.0));
    }
}
