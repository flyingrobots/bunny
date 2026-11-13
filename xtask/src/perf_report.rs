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
    pub(crate) base: Option<Timing>,
    pub(crate) head: Option<Timing>,
}

impl Row {
    /// Relative change of the median p50, in percent (positive is slower).
    pub(crate) fn change_percent(&self) -> Option<f64> {
        let (base, head) = (self.base?, self.head?);
        (base.p50 > 0.0).then(|| (head.p50 - base.p50) / base.p50 * 100.0)
    }
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
        })
        .collect()
}

/// Cases whose median p50 slowed down by more than `max_percent`.
pub(crate) fn regressions(rows: &[Row], max_percent: f64) -> Vec<&Row> {
    rows.iter()
        .filter(|row| row.change_percent().is_some_and(|change| change > max_percent))
        .collect()
}

/// A Markdown before/after table.
pub(crate) fn render(rows: &[Row], base_label: &str, head_label: &str, rounds: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "### Bunny perf: `{base_label}` → `{head_label}`\n");
    let _ = writeln!(
        out,
        "Median of {rounds} interleaved rounds; nanoseconds per call. Negative change is faster.\n"
    );
    out.push_str("| Case | Before p50 | After p50 | Change | Speedup | Before p95 | After p95 |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for row in rows {
        let cell = |timing: Option<Timing>, pick: fn(Timing) -> f64| {
            timing.map_or("—".to_owned(), |t| format!("{:.2}", pick(t)))
        };
        let change = row.change_percent().map_or("new".to_owned(), |c| format!("{c:+.1}%"));
        let speedup = match (row.base, row.head) {
            (Some(base), Some(head)) if head.p50 > 0.0 => format!("{:.2}×", base.p50 / head.p50),
            _ => "—".to_owned(),
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {change} | {speedup} | {} | {} |",
            row.case,
            cell(row.base, |t| t.p50),
            cell(row.head, |t| t.p50),
            cell(row.base, |t| t.p95),
            cell(row.head, |t| t.p95),
        );
    }
    out
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
        let table = render(&rows, "main", "HEAD", 5);
        assert!(table.contains("| `num/sqrt` | 60.00 | 15.00 | -75.0% | 4.00× |"), "{table}");
        assert!(table.contains("| `angle/sin_cos` | — | 10.00 | new | — |"), "{table}");
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
}
