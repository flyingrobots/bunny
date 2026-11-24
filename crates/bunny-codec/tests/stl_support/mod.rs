//! Byte builders and a seeded generator for the binary STL tests.
//!
//! Every fixture of `bunny-binary-stl-triangle-mesh/1` is built here from integers: a
//! coordinate is the bit pattern of an IEEE-754 binary32 value, written little-endian, so no
//! test reads or writes a float. Nothing here decodes STL; the reader under test and the
//! oracle in `stl_oracle_tests.rs` share no code with this module beyond the bytes it writes.
//! The provenance of these fixtures is recorded in `tests/fixtures/README.md`.

#![allow(dead_code)]

/// `1.0` as binary32 bits.
pub(crate) const ONE: u32 = 0x3F80_0000;
/// `0.5`.
pub(crate) const HALF: u32 = 0x3F00_0000;
/// `-2.25`.
pub(crate) const MINUS_TWO_AND_A_QUARTER: u32 = 0xC010_0000;
/// `0.125`.
pub(crate) const EIGHTH: u32 = 0x3E00_0000;
/// A quiet NaN, a signalling NaN, a negative quiet NaN, `+inf` and `-inf`.
pub(crate) const NON_FINITE: [u32; 5] =
    [0x7FC0_0000, 0x7F80_0001, 0xFFC0_0000, 0x7F80_0000, 0xFF80_0000];

/// The raw Q32.32 value of `1.0`.
pub(crate) const RAW_ONE: i64 = 1 << 32;

/// One STL facet: the stored normal, three vertices and the attribute byte count, each
/// coordinate as binary32 bits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facet {
    pub(crate) normal: [u32; 3],
    pub(crate) vertices: [[u32; 3]; 3],
    pub(crate) attribute: u16,
}

impl Facet {
    /// A facet with a zero normal and a zero attribute byte count.
    pub(crate) const fn new(vertices: [[u32; 3]; 3]) -> Self {
        Self { normal: [0; 3], vertices, attribute: 0 }
    }
}

/// An 80-byte header holding `text` followed by zero bytes.
pub(crate) fn header(text: &[u8]) -> [u8; 80] {
    let mut bytes = [0_u8; 80];
    for (slot, byte) in bytes.iter_mut().zip(text) {
        *slot = *byte;
    }
    bytes
}

/// The bytes of a binary STL file with `count` written as the triangle count, whatever the
/// number of `facets` that follow.
pub(crate) fn stl_with_count(header: &[u8; 80], count: u32, facets: &[Facet]) -> Vec<u8> {
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&count.to_le_bytes());
    for facet in facets {
        for word in facet.normal.iter().chain(facet.vertices.iter().flatten()) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(&facet.attribute.to_le_bytes());
    }
    bytes
}

/// The bytes of a well-formed binary STL file holding `facets`.
pub(crate) fn stl(facets: &[Facet]) -> Vec<u8> {
    let count = u32::try_from(facets.len()).expect("test meshes are small");
    stl_with_count(&header(b"bunny binary STL test fixture"), count, facets)
}

/// The unit tetrahedron with vertices `(0,0,0)`, `(1,0,0)`, `(0,1,0)`, `(0,0,1)`, each face
/// counter-clockwise seen from outside, with its outward normal stored unnormalised.
pub(crate) fn tetrahedron() -> [Facet; 4] {
    let (o, x, y, z) = ([0, 0, 0], [ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]);
    let minus = |bits: u32| bits | 0x8000_0000;
    [
        Facet { normal: [0, 0, minus(ONE)], vertices: [o, y, x], attribute: 0 },
        Facet { normal: [0, minus(ONE), 0], vertices: [o, x, z], attribute: 0 },
        Facet { normal: [minus(ONE), 0, 0], vertices: [o, z, y], attribute: 0 },
        Facet { normal: [ONE, ONE, ONE], vertices: [x, y, z], attribute: 0 },
    ]
}

/// A 64-bit linear congruential generator (Knuth's MMIX constants), the high bits used.
pub(crate) struct Seeds(pub(crate) u64);

impl Seeds {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    pub(crate) fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    pub(crate) fn word(&mut self) -> u32 {
        u32::try_from(self.next() & 0xFFFF_FFFF).expect("masked to 32 bits")
    }

    pub(crate) fn byte(&mut self) -> u8 {
        u8::try_from(self.next() & 0xFF).expect("masked to 8 bits")
    }
}

/// Binary32 bit patterns on and around every boundary of the conversion: zeros, the
/// smallest subnormal, exact ties at `2^-33` and `3 * 2^-33`, the last values in range and
/// the first out of it at both ends, the largest finite values and the non-finite ones.
pub(crate) const SPECIAL_BITS: [u32; 18] = [
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // smallest subnormal
    0x2F00_0000, // 2^-33: half a raw unit, a tie to 0
    0x2FC0_0000, // 3 * 2^-33: 1.5 raw units, a tie to 2
    0x3020_0000, // 5 * 2^-33: 2.5 raw units, a tie to 2
    0xAFC0_0000, // -3 * 2^-33: a tie to -2
    0x4EFF_FFFF, // 2^31 - 128, the largest value in range
    0x4F00_0000, // 2^31, the first positive value out of range
    0xCF00_0000, // -2^31, i64::MIN raw, in range
    0xCF00_0001, // -2^31 - 256, the first negative value out of range
    0x7F7F_FFFF, // f32::MAX
    0xFF7F_FFFF, // -f32::MAX
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // quiet NaN
    0x7F80_0001, // signalling NaN
    0xFFFF_FFFF, // negative NaN with every payload bit
];

/// A coordinate: usually a modest value with a random significand, sometimes a boundary
/// pattern, sometimes any 32 bits at all.
pub(crate) fn coordinate(seeds: &mut Seeds) -> u32 {
    match seeds.below(64) {
        0 => seeds.word(),
        1 | 2 => SPECIAL_BITS[usize::try_from(seeds.below(18)).expect("small")],
        _ => {
            let sign = u32::try_from(seeds.below(2)).expect("bit") << 31;
            // Exponents 100 to 157: magnitudes from about 2^-27 to just below 2^31.
            let exponent = (100 + u32::try_from(seeds.below(58)).expect("small")) << 23;
            sign | exponent | (seeds.word() & 0x007F_FFFF)
        }
    }
}

/// A random facet built from [`coordinate`], with random normal bits and attribute.
pub(crate) fn facet(seeds: &mut Seeds) -> Facet {
    let mut vertices = [[0_u32; 3]; 3];
    for slot in vertices.iter_mut().flatten() {
        *slot = coordinate(seeds);
    }
    let normal = [seeds.word(), seeds.word(), seeds.word()];
    let attribute = u16::try_from(seeds.next() & 0xFFFF).expect("masked to 16 bits");
    Facet { normal, vertices, attribute }
}
