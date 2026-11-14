//! Smooth shading per hit (`bunny-smooth-normal/1`, `bunny-smooth-shading/1`).
//!
//! The inputs are 1 024 closest hits on the sphere of the benchmark scene (`scene.rs`): camera
//! rays on a 96 by 96 grid through the window of the scene's camera, every `n`-th sphere hit kept
//! so the batch spans the image (ground hits are left out: their normals are shared). The
//! scene's positions are welded by value into an indexed mesh and its vertex normals come from
//! `VertexNormals::new`, so the determinants, weights and normals are a renderer's.
//!
//! `shade/smooth_normal` is the shipped interpolation (the exact numerators, then
//! `smooth_normal`). `shade/smooth_normal_divided` is the alternative that was not selected,
//! replicated here from public APIs: the two floored weights of `TriangleHit::barycentrics`
//! (two long divisions), `w0 = 2^32 - b1 - b2`, then the same shared-normal step, exact
//! combination and normalization. `shade/smooth_hit_shading` and `shade/flat_hit_shading_mesh`
//! run the whole per-hit step with and without the interpolated normal on the same hits.

use std::collections::BTreeMap;
use std::hint::black_box;
use std::rc::Rc;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_optics::{
    smooth_normal, PixelSampler, SmoothBounce, SmoothHit, SurfaceHit, TriangleSurface,
    VertexNormals,
};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit};

use crate::cases::{case, Case, BATCH};
use crate::scene::{self, Scene, Triangle};
use crate::shade::{lighting, Lighting};

const SAMPLER_SEED: u64 = 0x0123_4567_89AB_CDEF;
/// Camera rays per side of the grid the hits are drawn from.
const SIDE: i64 = 96;
/// The sphere's triangles come first in the scene; the two ground triangles after them share
/// their vertex normals, so a hit there takes the shared-normal step in both paths.
const SPHERE_TRIANGLES: u32 = 27_648;
/// Raw Q32.32 one, widened.
const ONE: i128 = 1 << 32;
/// The largest component of a vector is scaled into `[2^61, 2^62)` before it is normalized.
const PRESCALE_BIT: u32 = 61;

/// One hit with what both shading paths need.
#[derive(Clone, Copy)]
pub(crate) struct Shot {
    surface: TriangleSurface,
    ray: TraceRay,
    pub(crate) hit: TriangleHit,
    pub(crate) normals: [FixedUnitVec3; 3],
    vertex_normals: [Option<FixedUnitVec3>; 3],
    sample: [FixedQ32_32; 2],
}

/// The scene with its triangles prepared, its faces indexed and its vertex normals.
struct Mesh {
    scene: Scene,
    triangles: Vec<TraceTriangle>,
    faces: Vec<[u32; 3]>,
    normals: Vec<Option<FixedUnitVec3>>,
}

/// Positions with equal coordinates share one index, in order of first appearance.
fn indexed(triangles: &[Triangle]) -> (Vec<FixedVec3>, Vec<[u32; 3]>) {
    let mut index = BTreeMap::new();
    let mut positions = Vec::new();
    let mut faces = Vec::with_capacity(triangles.len());
    for triangle in triangles {
        faces.push(triangle.map(|p| {
            let next = u32::try_from(positions.len()).unwrap_or(u32::MAX);
            *index.entry([p.x.raw(), p.y.raw(), p.z.raw()]).or_insert_with(|| {
                positions.push(p);
                next
            })
        }));
    }
    (positions, faces)
}

impl Mesh {
    fn new() -> Option<Self> {
        let scene = scene::scene();
        let triangles = scene::trace_triangles(&scene);
        let (positions, faces) = indexed(&scene.triangles);
        let normals = VertexNormals::new(&positions, &faces).ok()?.normals().to_vec();
        Some(Self { scene, triangles, faces, normals })
    }

    /// The closest hit of `ray`, or `None` when it misses the sphere or a vertex has no normal.
    fn shot(&self, ray: TraceRay, index: u32) -> Option<Shot> {
        let primitive = scene::trace_closest(&self.scene, &self.triangles, &ray)?.primitive;
        if primitive >= SPHERE_TRIANGLES {
            return None;
        }
        let triangle = self.triangles.get(primitive as usize)?;
        let hit = ray.hit_triangle(triangle, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX))?;
        let face = self.faces.get(primitive as usize)?;
        let vertex_normals = face.map(|v| self.normals.get(v as usize).copied().flatten());
        let [a, b, c] = vertex_normals;
        let normals = [a?, b?, c?];
        let surface = TriangleSurface::new(triangle)?;
        let sample = PixelSampler::new(SAMPLER_SEED, index % 96, index / 96).point(index % 16, 1);
        Some(Shot { surface, ray, hit, normals, vertex_normals, sample })
    }
}

/// The 1 024 hits of the batch, in raster order.
pub(crate) fn shots() -> Vec<Shot> {
    let Some(mesh) = Mesh::new() else { return Vec::new() };
    let (origin, directions) = scene::camera_grid(SIDE);
    let found: Vec<Shot> = directions
        .iter()
        .zip(0_u32..)
        .filter_map(|(&direction, index)| mesh.shot(TraceRay::new(origin, direction)?, index))
        .collect();
    let step = (found.len() / BATCH).max(1);
    found.into_iter().step_by(step).take(BATCH).collect()
}

/// The weights `(2^32 - b1 - b2, b1, b2)` from the two floored weights of `barycentrics`.
pub(crate) fn divided_weights(hit: &TriangleHit) -> [i128; 3] {
    let (b1, b2) = hit.barycentrics();
    let (b1, b2) = (i128::from(b1.raw()), i128::from(b2.raw()));
    [ONE - b1 - b2, b1, b2]
}

/// `sum w_i n_i`, exact: each weight is at most `2^32`, so each product is below `2^65`.
pub(crate) fn combine(normals: [FixedUnitVec3; 3], weights: [i128; 3]) -> [i128; 3] {
    normals.iter().zip(weights).fold([0; 3], |[x, y, z], (normal, w)| {
        let n = normal.into_inner();
        [
            x + w * i128::from(n.x.raw()),
            y + w * i128::from(n.y.raw()),
            z + w * i128::from(n.z.raw()),
        ]
    })
}

/// The power-of-two prescale to `[2^61, 2^62)` and `FixedUnitVec3::new`, as `smooth_normal`
/// normalizes.
fn unit(v: [i128; 3]) -> Option<FixedUnitVec3> {
    let bits = v.iter().map(|c| c.unsigned_abs()).max()?.checked_ilog2()?;
    let scale = |c: i128| {
        let scaled = if bits <= PRESCALE_BIT {
            c << (PRESCALE_BIT - bits)
        } else {
            c >> (bits - PRESCALE_BIT)
        };
        FixedQ32_32::from_raw(i64::try_from(scaled).unwrap_or(i64::MAX))
    };
    let [x, y, z] = v.map(scale);
    FixedUnitVec3::new(FixedVec3::new(x, y, z))
}

/// The alternative interpolation: floored weights, then the steps of `smooth_normal`.
pub(crate) fn divided_normal(
    normals: [FixedUnitVec3; 3],
    hit: &TriangleHit,
) -> Option<FixedUnitVec3> {
    let weights = divided_weights(hit);
    let mut present = normals.iter().zip(weights).filter(|(_, w)| *w > 0).map(|(n, _)| *n);
    let first = present.next()?;
    if present.all(|n| n == first) {
        return Some(first);
    }
    unit(combine(normals, weights))
}

fn flat_shading(shot: &Shot, (light, sky, albedo): &Lighting) -> i64 {
    let Some(hit) = SurfaceHit::new(&shot.surface, &shot.ray, &shot.hit) else { return 0 };
    let direct = black_box(hit.light_sample(light)).radiance(true);
    let bounce = black_box(hit.bounce_ray(shot.sample)).map_or(0, |r| r.direction().x.raw());
    let radiance = black_box(albedo.reflect(direct, sky.radiance())).map_or(0, |r| r[0].raw());
    radiance.wrapping_add(bounce)
}

fn smooth_shading(shot: &Shot, (light, sky, albedo): &Lighting) -> i64 {
    let Some(hit) = SurfaceHit::new(&shot.surface, &shot.ray, &shot.hit) else { return 0 };
    let smooth = SmoothHit::new(hit, shot.hit.barycentric_numerators(), shot.vertex_normals);
    let direct = black_box(smooth.light_sample(light)).radiance(true);
    let bounce = match black_box(smooth.bounce(shot.sample)) {
        Some(SmoothBounce::Ray(ray)) => ray.direction().x.raw(),
        _ => 0,
    };
    let radiance = black_box(albedo.reflect(direct, sky.radiance())).map_or(0, |r| r[0].raw());
    radiance.wrapping_add(bounce)
}

fn sum(shots: &[Shot], each: impl Fn(&Shot) -> i64) -> i64 {
    shots.iter().map(|shot| each(black_box(shot))).fold(0, i64::wrapping_add)
}

/// The first component's raw value, for a checksum.
fn first_raw(normal: Option<FixedUnitVec3>) -> i64 {
    normal.map_or(0, |n| n.into_inner().x.raw())
}

pub(crate) fn smooth_cases() -> Vec<Case> {
    let all = Rc::new(shots());
    let [shipped, divided, smooth, flat] = [0; 4].map(|_| Rc::clone(&all));
    let (smooth_lights, flat_lights) = (lighting(), lighting());
    vec![
        case("shade/smooth_normal", move || {
            sum(&shipped, |s| {
                first_raw(black_box(smooth_normal(s.normals, s.hit.barycentric_numerators())))
            })
        }),
        case("shade/smooth_normal_divided", move || {
            sum(&divided, |s| first_raw(black_box(divided_normal(s.normals, &s.hit))))
        }),
        case("shade/smooth_hit_shading", move || {
            smooth_lights.map_or(0, |lights| sum(&smooth, |s| smooth_shading(s, &lights)))
        }),
        case("shade/flat_hit_shading_mesh", move || {
            flat_lights.map_or(0, |lights| sum(&flat, |s| flat_shading(s, &lights)))
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `4 (2^32 + 1)`: the floors of the two divided weights move `sum w_i n_i` by less.
    const FLOOR_ERROR: i128 = 4 * (ONE + 1);
    /// `(8 (1 + 2^-32) + 2^-26) 2^64`, the `1 / rho` terms of both bounds over `2^-64`.
    const BOUND_SCALED: i128 = (8 << 64) + (8 << 32) + (1 << 38);

    fn largest_difference(a: FixedUnitVec3, b: FixedUnitVec3) -> i64 {
        let (a, b) = (a.into_inner(), b.into_inner());
        let d = |p: FixedQ32_32, q: FixedQ32_32| (p.raw() - q.raw()).abs();
        d(a.x, b.x).max(d(a.y, b.y)).max(d(a.z, b.z))
    }

    /// The `k` that `smooth_normal` shifts the weights by.
    fn shift(hit: &TriangleHit) -> u32 {
        hit.barycentric_numerators().det().ilog2().saturating_sub(61)
    }

    /// Option (a) is within `1 + 2^-26 / rho` raw units of the exact interpolation and option (b)
    /// within `1 + 8 (1 + 2^-32) / rho`, so they differ by at most the sum. `rho` is bounded
    /// below by `(max |m_b| - FLOOR_ERROR) / 2^64` from (b)'s exact sum `m_b`.
    #[test]
    fn the_divided_replica_agrees_with_smooth_normal_within_both_bounds() {
        let mut differences = BTreeMap::new();
        for (index, shot) in shots().iter().enumerate() {
            let shipped = smooth_normal(shot.normals, shot.hit.barycentric_numerators());
            let divided = divided_normal(shot.normals, &shot.hit);
            let (a, b) = shipped.zip(divided).expect("both paths give a normal on the scene");
            let m = combine(shot.normals, divided_weights(&shot.hit)).map(i128::unsigned_abs);
            let m = m.into_iter().max().and_then(|m| i128::try_from(m).ok()).unwrap_or(0);
            let difference = largest_difference(a, b);
            assert!(m > FLOOR_ERROR, "shot {index}: rho has no positive lower bound");
            let excess = i128::from(difference - 2).max(0);
            assert!(
                excess * (m - FLOOR_ERROR) <= BOUND_SCALED,
                "shot {index}: (a) {a:?} and (b) {b:?} differ by {difference} raw units"
            );
            *differences.entry(difference).or_insert(0_usize) += 1;
        }
        let differences: Vec<(i64, usize)> = differences.into_iter().collect();
        assert_eq!(
            differences,
            [(0, 573), (1, 447), (2, 4)],
            "how often (a) and (b) differ, by raw units"
        );
    }

    /// The batch is what the ledger describes: 1 024 distinct camera rays, and (a regression
    /// lock) the weight shifts of `smooth_normal` and no hit with three equal vertex normals.
    #[test]
    fn the_batch_is_1024_distinct_sphere_hits_with_the_recorded_shifts() {
        let shots = shots();
        let rays: std::collections::BTreeSet<[i64; 3]> = shots
            .iter()
            .map(|s| s.ray.direction())
            .map(|d| [d.x.raw(), d.y.raw(), d.z.raw()])
            .collect();
        assert_eq!((shots.len(), rays.len()), (BATCH, BATCH), "hits, then distinct rays");
        let mut shifts = BTreeMap::new();
        for shot in &shots {
            *shifts.entry(shift(&shot.hit)).or_insert(0_usize) += 1;
        }
        let shifts: Vec<(u32, usize)> = shifts.into_iter().collect();
        let shared = shots.iter().filter(|s| s.normals.iter().all(|n| *n == s.normals[0])).count();
        let expected: Vec<(u32, usize)> =
            vec![(17, 1), (20, 1), (21, 3), (22, 15), (23, 86), (24, 191), (25, 600), (26, 127)];
        assert_eq!(shifts, expected, "shifts and their counts");
        assert_eq!(shared, 0, "hits whose three vertex normals are equal");
    }
}
