//! Texture coordinates at a hit and bilinear lookups (`bunny-texture-bilinear/1`).
//!
//! `texture/interpolate_uv` times `interpolate_uv` on 1 024 seeded hits: barycentric numerators
//! with `det` in `[2^99, 2^100)`, the size of a trace-domain triangle hit's, and vertex coordinates in
//! `[-4, 4)`. Its cost does not depend on the values: the exact long division takes 64 steps
//! per component. `texture/bilinear` times `bilinear` on the coordinates those calls return,
//! in a 1 024 by 1 024 texture, repeating along `u` and clamping along `v`.

use std::hint::black_box;
use std::rc::Rc;

use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;
use bunny_optics::{bilinear, interpolate_uv, TextureWrap};
use bunny_query::BarycentricNumerators;

use crate::cases::{case, Case, BATCH};

const TEXTURE_SEED: u64 = 0xB002_0367_9E2F_0001;
/// The texture's side in texels.
const SIDE: u32 = 1024;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A raw coordinate in `[-4, 4)` units.
    fn coordinate(&mut self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(i64::from_ne_bytes(self.next().to_ne_bytes()) >> 29)
    }
}

/// One hit: three vertex coordinates and the hit's barycentric numerators.
struct Hit {
    uvs: [FixedVec2; 3],
    weights: BarycentricNumerators,
}

fn hits() -> Vec<Hit> {
    let mut seeds = Seeds(TEXTURE_SEED);
    (0..BATCH)
        .filter_map(|_| {
            let det = i128::from((seeds.next() >> 1) | (1 << 62)) << 37;
            let u = i128::from(seeds.next() >> 1) << 35;
            let v = i128::from(seeds.next() >> 1) << 35;
            let weights = BarycentricNumerators::new(u, v, det)?;
            let uvs = [0; 3].map(|_| FixedVec2::new(seeds.coordinate(), seeds.coordinate()));
            Some(Hit { uvs, weights })
        })
        .collect()
}

pub(crate) fn texture_cases() -> Vec<Case> {
    let hits = Rc::new(hits());
    let points: Rc<Vec<FixedVec2>> =
        Rc::new(hits.iter().map(|h| interpolate_uv(h.uvs, h.weights)).collect());
    let wrap = [TextureWrap::Repeat, TextureWrap::Clamp];
    vec![
        case("texture/interpolate_uv", move || {
            hits.iter()
                .map(|h| black_box(interpolate_uv(black_box(h.uvs), black_box(h.weights))).x.raw())
                .fold(0, i64::wrapping_add)
        }),
        case("texture/bilinear", move || {
            points
                .iter()
                .map(|&p| {
                    black_box(bilinear(black_box(p), [SIDE, SIDE], wrap))
                        .map_or(0, |taps| taps.weights()[0].raw())
                })
                .fold(0, i64::wrapping_add)
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every seeded numerator triple is on the triangle, so the batch is full.
    #[test]
    fn the_batch_is_full() {
        assert_eq!(hits().len(), BATCH);
    }
}
