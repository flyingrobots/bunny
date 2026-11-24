//! An independent reader for `bunny-binary-stl-triangle-mesh/1`, compared bit for bit with
//! `parse_binary_stl` on seeded generated files and on random and mutated byte strings.
//!
//! The oracle shares no code with `bunny-codec` or `bunny-num`. It assembles integers from
//! bytes by shifts, decides the length in `u128`, and converts a binary32 bit pattern to
//! Q32.32 by splitting it into sign, exponent field and fraction and rounding the exact
//! value `significand * 2^(power + 32)` by comparing twice the remainder with the divisor.
//! Every generated case prints its seed and index on failure.

mod stl_support;

use bunny_codec::{parse_binary_stl, StlError};
use stl_support::{facet, stl, stl_with_count, tetrahedron, Seeds, SPECIAL_BITS};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0x057_1127;

/// What a reader makes of an input: the raw coordinates of every vertex, or the refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Mesh(Vec<[i64; 3]>),
    TooShort(usize),
    Length { count: u32, expected: u64, actual: u64 },
    NonFinite { triangle: u32, vertex: u8, axis: u8 },
    OutOfRange { triangle: u32, vertex: u8, axis: u8 },
}

#[derive(Debug, PartialEq, Eq)]
enum Bad {
    NonFinite,
    OutOfRange,
}

/// The Q32.32 raw value of the binary32 value with these bits, rounded to nearest with ties
/// to even, or the reason there is none.
fn oracle_raw(bits: u32) -> Result<i64, Bad> {
    let negative = bits >> 31 == 1;
    let field = (bits >> 23) & 0xFF;
    let fraction = u128::from(bits & 0x007F_FFFF);
    if field == 0xFF {
        return Err(Bad::NonFinite);
    }
    // value = significand * 2^power, exactly.
    let (significand, power) = if field == 0 {
        (fraction, -149_i32)
    } else {
        (fraction | (1 << 23), i32::try_from(field).expect("eight bits") - 150)
    };
    let shift = power + 32;
    let magnitude: u128 = if significand == 0 {
        0
    } else if shift >= 0 {
        if shift > 64 {
            return Err(Bad::OutOfRange);
        }
        significand << shift
    } else if -shift > 30 {
        // significand < 2^24, so the value is below 2^24 / 2^31: under half a raw unit.
        0
    } else {
        let divisor = 1_u128 << -shift;
        let (quotient, remainder) = (significand / divisor, significand % divisor);
        match (2 * remainder).cmp(&divisor) {
            core::cmp::Ordering::Less => quotient,
            core::cmp::Ordering::Greater => quotient + 1,
            core::cmp::Ordering::Equal => quotient + (quotient % 2),
        }
    };
    let limit = if negative { 1_u128 << 63 } else { (1_u128 << 63) - 1 };
    if magnitude > limit {
        return Err(Bad::OutOfRange);
    }
    let signed = i128::try_from(magnitude).expect("at most 2^63");
    Ok(i64::try_from(if negative { -signed } else { signed }).expect("checked against the limit"))
}

fn word_at(bytes: &[u8], at: usize) -> u32 {
    (0..4).fold(0_u32, |word, i| word | (u32::from(bytes[at + i]) << (8 * i)))
}

/// The straightforward reader: length rule first, then each coordinate in file order.
fn oracle(bytes: &[u8]) -> Outcome {
    if bytes.len() < 84 {
        return Outcome::TooShort(bytes.len());
    }
    let count = word_at(bytes, 80);
    let expected = 84 + 50 * u128::from(count);
    let actual = u128::try_from(bytes.len()).expect("lengths fit u128");
    if actual != expected {
        let narrow = |value: u128| u64::try_from(value).expect("below 2^38");
        return Outcome::Length { count, expected: narrow(expected), actual: narrow(actual) };
    }
    let mut vertices = Vec::new();
    for triangle in 0..count {
        let base = 84 + 50 * usize::try_from(triangle).expect("small test files");
        for vertex in 0..3_u8 {
            let mut position = [0_i64; 3];
            for (axis, slot) in (0..3_u8).zip(position.iter_mut()) {
                let at = base + 12 + 12 * usize::from(vertex) + 4 * usize::from(axis);
                match oracle_raw(word_at(bytes, at)) {
                    Ok(raw) => *slot = raw,
                    Err(Bad::NonFinite) => return Outcome::NonFinite { triangle, vertex, axis },
                    Err(Bad::OutOfRange) => return Outcome::OutOfRange { triangle, vertex, axis },
                }
            }
            vertices.push(position);
        }
    }
    Outcome::Mesh(vertices)
}

/// The reader under test, in the oracle's terms, with the mesh's own consistency checked.
fn observed(bytes: &[u8]) -> Outcome {
    match parse_binary_stl(bytes) {
        Ok(mesh) => {
            let count = mesh.triangle_count();
            assert_eq!(bytes.len(), 84 + 50 * count, "the length rule");
            assert_eq!(mesh.vertex_count(), 3 * count);
            let triangles: Vec<_> = mesh.triangles().map(|t| t.expect("Ok")).collect();
            assert_eq!(triangles.len(), count);
            for (i, t) in (0_u32..).zip(&triangles) {
                assert_eq!((t.v0, t.v1, t.v2), (3 * i, 3 * i + 1, 3 * i + 2));
            }
            let vertices =
                mesh.vertices().map(|v| v.expect("Ok")).map(|v| [v.x.raw(), v.y.raw(), v.z.raw()]);
            Outcome::Mesh(vertices.collect())
        }
        Err(StlError::TooShort { length }) => Outcome::TooShort(length),
        Err(StlError::LengthMismatch { triangles, expected, actual }) => {
            Outcome::Length { count: triangles, expected, actual }
        }
        Err(StlError::NonFiniteVertex { triangle, vertex, axis }) => {
            Outcome::NonFinite { triangle, vertex, axis }
        }
        Err(StlError::VertexOutOfRange { triangle, vertex, axis }) => {
            Outcome::OutOfRange { triangle, vertex, axis }
        }
        Err(other) => panic!("unexpected refusal for a small input: {other:?}"),
    }
}

#[derive(Default, Debug)]
struct Tally {
    mesh: u32,
    too_short: u32,
    length: u32,
    non_finite: u32,
    out_of_range: u32,
}

impl Tally {
    fn add(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Mesh(_) => self.mesh += 1,
            Outcome::TooShort(_) => self.too_short += 1,
            Outcome::Length { .. } => self.length += 1,
            Outcome::NonFinite { .. } => self.non_finite += 1,
            Outcome::OutOfRange { .. } => self.out_of_range += 1,
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_022_the_oracle_agrees_with_hand_derived_values() {
    let cases: [(u32, Result<i64, Bad>); 14] = [
        (0x3F80_0000, Ok(1 << 32)),
        (0x3F00_0000, Ok(1 << 31)),
        (0xC010_0000, Ok(-(9 << 30))),
        (0x2F00_0000, Ok(0)),
        (0x2FC0_0000, Ok(2)),
        (0x3020_0000, Ok(2)),
        (0x3060_0000, Ok(4)),
        (0x0000_0001, Ok(0)),
        (0x4EFF_FFFF, Ok(((1 << 31) - 128) << 32)),
        (0xCF00_0000, Ok(i64::MIN)),
        (0x4F00_0000, Err(Bad::OutOfRange)),
        (0xCF00_0001, Err(Bad::OutOfRange)),
        (0x7F80_0000, Err(Bad::NonFinite)),
        (0xFFC0_0000, Err(Bad::NonFinite)),
    ];
    for (bits, expected) in cases {
        assert_eq!(oracle_raw(bits), expected, "{bits:#x}");
    }
    let one = 1_i64 << 32;
    let mesh = oracle(&stl(&tetrahedron()));
    let o = [0, 0, 0];
    let (x, y, z) = ([one, 0, 0], [0, one, 0], [0, 0, one]);
    assert_eq!(mesh, Outcome::Mesh(vec![o, y, x, o, x, z, o, z, y, x, y, z]));
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_020_seeded_files_match_the_independent_reader_bit_for_bit() {
    let mut seeds = Seeds(SEED);
    let mut tally = Tally::default();
    for case in 0..3_000_u32 {
        let facets: Vec<_> = (0..seeds.below(12)).map(|_| facet(&mut seeds)).collect();
        let bytes = stl(&facets);
        let expected = oracle(&bytes);
        assert_eq!(observed(&bytes), expected, "seed {SEED:#x} case {case}");
        tally.add(&expected);
    }
    // Every special pattern converts as the oracle says, in every position.
    for (case, bits) in SPECIAL_BITS.iter().enumerate() {
        let mut facets = tetrahedron();
        facets[2].vertices[1][2] = *bits;
        let bytes = stl(&facets);
        assert_eq!(observed(&bytes), oracle(&bytes), "special case {case}: {bits:#x}");
    }
    assert!(
        tally.mesh >= 1_000 && tally.non_finite >= 500 && tally.out_of_range >= 700,
        "seed {SEED:#x}: {tally:?}"
    );
}

/// Flips, overwrites, inserts or deletes a few bytes, or rewrites the triangle count.
fn mutated(seeds: &mut Seeds, mut bytes: Vec<u8>) -> Vec<u8> {
    for _ in 0..=seeds.below(3) {
        let len = u64::try_from(bytes.len()).expect("small");
        let at = usize::try_from(seeds.below(len.max(1))).expect("small");
        match seeds.below(6) {
            0 if at < bytes.len() => bytes[at] ^= 1 << seeds.below(8),
            1 if at < bytes.len() => bytes[at] = seeds.byte(),
            2 => bytes.insert(at.min(bytes.len()), seeds.byte()),
            3 if at < bytes.len() => {
                bytes.remove(at);
            }
            4 if bytes.len() >= 84 => {
                let count =
                    [0, 1, 85_899_345, 1_431_655_765, 1_431_655_766, u32::MAX, seeds.word()];
                let pick = count[usize::try_from(seeds.below(7)).expect("small")];
                bytes[80..84].copy_from_slice(&pick.to_le_bytes());
            }
            _ => bytes.truncate(at),
        }
    }
    bytes
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_021_random_and_mutated_bytes_never_panic_and_match_the_oracle() {
    let mut seeds = Seeds(SEED ^ 0xF0);
    let mut tally = Tally::default();
    for case in 0..20_000_u32 {
        let valid = |seeds: &mut Seeds| {
            let facets: Vec<_> = (0..seeds.below(6)).map(|_| facet(seeds)).collect();
            stl(&facets)
        };
        let bytes = match case % 4 {
            0 => (0..seeds.below(400)).map(|_| seeds.byte()).collect(),
            1 => {
                let bytes = valid(&mut seeds);
                mutated(&mut seeds, bytes)
            }
            2 => {
                // A random count over a short body: the count never sizes anything.
                let facets: Vec<_> = (0..seeds.below(3)).map(|_| facet(&mut seeds)).collect();
                stl_with_count(&[0x20; 80], seeds.word(), &facets)
            }
            _ => valid(&mut seeds),
        };
        let expected = oracle(&bytes);
        assert_eq!(observed(&bytes), expected, "seed {:#x} case {case}: {bytes:02x?}", SEED ^ 0xF0);
        tally.add(&expected);
    }
    assert!(
        tally.mesh >= 3_000
            && tally.too_short >= 1_500
            && tally.length >= 10_000
            && tally.non_finite >= 600
            && tally.out_of_range >= 1_000,
        "seed {:#x}: {tally:?}",
        SEED ^ 0xF0
    );
}
