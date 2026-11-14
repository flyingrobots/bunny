//! Sample hash and pixel sampler cases: the values a renderer draws for every ray.
//!
//! Each call draws for one (pixel, sample, dimension) key of a 32 by 32 pixel tile with 16
//! samples per pixel and 4 dimensions or pairs, so pixel coordinates, sample indices and
//! dimensions all vary across the batch.

use std::hint::black_box;

use bunny_optics::{PixelSampler, PixelStream};

use crate::cases::{case, Case};

const SEED: u64 = 0x0123_4567_89AB_CDEF;

/// The batch keys: `(x, y, sample, dimension)`; 1 024 of them.
fn keys() -> Vec<(u32, u32, u32, u32)> {
    (0..1024_u32).map(|i| (i % 32, (i / 32) % 32, (i * 7) % 16, (i * 3) % 4)).collect()
}

pub(crate) fn sample_cases() -> Vec<Case> {
    let keys = keys();
    let streams: Vec<(PixelStream, u32, u32)> =
        keys.iter().map(|&(x, y, s, d)| (PixelStream::new(SEED, x, y), s, d)).collect();
    let samplers: Vec<(PixelSampler, u32, u32)> =
        keys.iter().map(|&(x, y, s, d)| (PixelSampler::new(SEED, x, y), s, d)).collect();
    let fresh = keys.clone();
    vec![
        // 64 bits from a pixel's stream that the caller already holds.
        case("sample/stream_bits", move || {
            streams
                .iter()
                .map(|&(stream, s, d)| black_box(black_box(stream).bits(s, d)).cast_signed())
                .fold(0, i64::wrapping_add)
        }),
        // One unit-square point from a pixel's sampler that the caller already holds.
        case("sample/sampler_point", move || {
            samplers
                .iter()
                .map(|&(sampler, s, pair)| black_box(black_box(sampler).point(s, pair))[1].raw())
                .fold(0, i64::wrapping_add)
        }),
        // A sampler built for the pixel and one point drawn from it.
        case("sample/sampler_new_point", move || {
            fresh
                .iter()
                .map(|&(x, y, s, pair)| {
                    black_box(PixelSampler::new(black_box(SEED), x, y).point(s, pair))[0].raw()
                })
                .fold(0, i64::wrapping_add)
        }),
    ]
}
