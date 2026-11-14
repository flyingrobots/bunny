//! Times Bunny's deterministic math and query kernels on fixed, seeded inputs.
//!
//! Prints one line per case: `case<TAB>p50<TAB>p95<TAB>p99`, in nanoseconds per call.
//! `cargo xtask perf` runs this binary against two revisions and compares them; see
//! `docs/topics/performance/README.md`.
//!
//! Options: `--filter <text>` keeps cases whose name contains the text; `--samples <n>`
//! sets the number of timed batches per case (default 101).

mod cases;
#[cfg(feature = "sample")]
mod sample;
#[cfg(any(feature = "scene", feature = "trace"))]
mod scene;
mod timing;

use std::error::Error;

fn main() {
    if let Err(err) = run() {
        eprintln!("bunny-perf: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    #[cfg(all(feature = "scene", feature = "trace"))]
    {
        // The two closest-hit cases are comparable only if they find the same primitives.
        let bad = scene::disagreeing_rays();
        if !bad.is_empty() {
            return Err(format!("closest-hit paths disagree on rays {bad:?}").into());
        }
    }
    for case in cases::all() {
        if options.filter.as_deref().is_some_and(|filter| !case.name.contains(filter)) {
            continue;
        }
        let summary = timing::measure(&case, options.samples);
        println!("{}\t{:.2}\t{:.2}\t{:.2}", case.name, summary.p50, summary.p95, summary.p99);
    }
    Ok(())
}

struct Options {
    filter: Option<String>,
    samples: usize,
}

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self { filter: None, samples: timing::DEFAULT_SAMPLES };
        while let Some(arg) = args.next() {
            let value = args.next().ok_or_else(|| format!("{arg} needs a value"))?;
            match arg.as_str() {
                "--filter" => options.filter = Some(value),
                "--samples" => options.samples = value.parse::<usize>()?.max(1),
                other => return Err(format!("unknown option {other}").into()),
            }
        }
        Ok(options)
    }
}
