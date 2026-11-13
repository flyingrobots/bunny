//! Batch timing and percentile summaries.

use std::hint::black_box;
use std::time::Instant;

use crate::cases::Case;

/// Timed batches per case. Odd, so the median is one sample.
pub(crate) const DEFAULT_SAMPLES: usize = 101;
/// Untimed batches run first, to settle caches, branch predictors and clocks.
const WARMUP_BATCHES: usize = 10;

/// Nanoseconds per call at three percentiles of the timed batches.
pub(crate) struct Summary {
    pub(crate) p50: f64,
    pub(crate) p95: f64,
    pub(crate) p99: f64,
}

/// Runs `case` in batches over its inputs and summarizes the per-call time of each batch.
pub(crate) fn measure(case: &Case, samples: usize) -> Summary {
    for _ in 0..WARMUP_BATCHES {
        black_box((case.run)());
    }
    let mut per_call: Vec<f64> = (0..samples)
        .map(|_| {
            let start = Instant::now();
            black_box((case.run)());
            nanos(start) / per_batch(case)
        })
        .collect();
    per_call.sort_by(f64::total_cmp);
    Summary {
        p50: percentile(&per_call, 50),
        p95: percentile(&per_call, 95),
        p99: percentile(&per_call, 99),
    }
}

#[allow(clippy::cast_precision_loss)]
fn nanos(start: Instant) -> f64 {
    start.elapsed().as_nanos() as f64
}

#[allow(clippy::cast_precision_loss)]
fn per_batch(case: &Case) -> f64 {
    case.calls as f64
}

/// Nearest-rank percentile of sorted values.
fn percentile(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent).div_ceil(100).max(1);
    sorted.get(rank - 1).copied().unwrap_or(f64::NAN)
}
