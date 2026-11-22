//! The three rounding rules of `bunny-diffuse-shading/1` that round an exact `i128` value once:
//! the light cosine (`round_shift` by 32), the hemisphere direction's frame combination
//! (`round_shift` by 64) and the frame's division by `s + z`. Each is checked on exact ties
//! whose truncation is even and odd, in both signs where the sign is observable, and on the
//! nearest inputs either side of a tie.
//!
//! Oracle: each expected raw value is derived from the exact rational result, written next to
//! it as `exact -> result`, and checked by a separate exact Python model of the chapter's
//! definitions. Ties go to the even neighbour; negative results use the same rule on the signed
//! value. `U = 2^32` is one unit and `H = 2^31` half of it.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{DirectionalLight, ShadingFrame, TriangleSurface};
use bunny_query::TraceTriangle;
use wasm_bindgen_test::wasm_bindgen_test;

const U: i64 = ONE_RAW;
const H: i64 = U / 2;
const EIGHTH: i64 = U / 8;

type Raw3 = [i64; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// A vector that is a unit vector as it stands (its length within one raw unit of 1.0).
fn unit(raw: Raw3) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(vec3(raw)).expect("a unit vector")
}

fn light(toward: Raw3) -> DirectionalLight {
    DirectionalLight::new(vec3(toward), [FixedQ32_32::ONE; 3]).expect("a normalized light")
}

fn surface(v: [Raw3; 3]) -> TriangleSurface {
    let triangle = TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the domain");
    TriangleSurface::new(&triangle).expect("not degenerate")
}

/// `[normal, tangent, bitangent]` raw.
fn frame_raw(frame: &ShadingFrame) -> [Raw3; 3] {
    [raw3(frame.normal()), raw3(frame.tangent()), raw3(frame.bitangent())]
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_092_the_light_cosine_rounds_to_nearest_with_ties_to_even() {
    // toward (1, 2^-32, 0) normalizes to exactly (U, 1, 0): its prescale is a shift and the
    // length's floored square root is exactly 2^61. So the cosine with (x, y, 0) is
    // x + y / 2^32 before rounding: y = H is a tie, H - 1 and H + 1 its nearest neighbours.
    let fine = light([U, 1, 0]);
    assert_eq!(fine.cosine(vec3([U, 0, 0])).raw(), U); // the light's x is U
    assert_eq!(fine.cosine(vec3([0, U, 0])).raw(), 1); // and its y is 1
    let x_even = 3_719_550_788;
    let x_odd = 3_719_550_787;
    let cases = [
        ([x_even, H - 1, 0], x_even),     // x + 1/2 - 2^-32 -> x
        ([x_even, H, 0], x_even),         // x + 1/2, x even -> x
        ([x_even, H + 1, 0], x_even + 1), // x + 1/2 + 2^-32 -> x + 1
        ([x_odd, H - 1, 0], x_odd),       // x + 1/2 - 2^-32 -> x
        ([x_odd, H, 0], x_odd + 1),       // x + 1/2, x odd -> x + 1
        ([x_odd, H + 1, 0], x_odd + 1),   // x + 1/2 + 2^-32 -> x + 1
    ];
    for (normal, expected) in cases {
        let normal = unit(normal).into_inner();
        assert_eq!(fine.cosine(normal).raw(), expected, "{normal:?}");
    }
    // The review's tie: toward (1, 0, 18 874 368 raw) normalizes to (4 294 925 825, 0,
    // 18 874 186), and with the normal (H, 3 719 550 787, 0) the exact cosine is
    // 4 294 925 825 / 2 = 2 147 462 912.5 -> 2 147 462 912 (ties away from zero: ...913).
    let tilted = light([U, 0, 18_874_368]);
    assert_eq!(tilted.cosine(unit([H, 3_719_550_787, 0]).into_inner()).raw(), 2_147_462_912);
    // The review's triangle under the light toward (1, 1, 0): unit normal (1 952 538 793,
    // 3 693 065 732, -997 798 494), light (3 037 000 500, 3 037 000 500, 0); the exact cosine
    // is 3 992 045 243.556 -> 3 992 045 244 (floored: ...243).
    let review = surface([
        [58_793_695_247, -11_551_779_499, 1_934_831_863],
        [11_783_892_607, 4_722_883_646, -29_820_139_422],
        [-51_477_046_776, 54_907_774_659, 32_132_913_612],
    ]);
    let normal = review.frame().normal();
    assert_eq!(raw3(normal), [1_952_538_793, 3_693_065_732, -997_798_494]);
    assert_eq!(light([U, U, 0]).cosine(normal).raw(), 3_992_045_244);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_093_the_frame_combination_rounds_ties_to_even_in_both_signs() {
    // On the exact +Z frame the direction is (r cos, r sin, h) with each product divided by
    // 2^32 and rounded once. At 1/8 turn sin = cos = 3 037 000 500 (and at 7/8 the sine is
    // negated); r = floor_sqrt(u) is 2^29 at u = 2^-6 and 3 * 2^29 at u = 9 * 2^-6, so the
    // products are r * 3 037 000 500 / 2^32 = 379 625 062.5 and 1 138 875 187.5: ties with an
    // even and an odd truncation. The height h = floor_sqrt(1 - u) multiplies the normal's
    // exact 1.0, so the third component is h with no rounding.
    let plus_z = surface([[0; 3], [U, 0, 0], [0, U, 0]]).frame();
    let (u_even, u_odd) = (1 << 26, 9 << 26);
    let cases = [
        ((u_even, EIGHTH), [379_625_062, 379_625_062, 4_261_280_757]), // 379 625 062.5 -> even
        ((u_even, 7 * EIGHTH), [379_625_062, -379_625_062, 4_261_280_757]), // -...062.5 -> -...062
        ((u_odd, EIGHTH), [1_138_875_188, 1_138_875_188, 3_981_541_245]), // ...187.5 -> ...188
        ((u_odd, 7 * EIGHTH), [1_138_875_188, -1_138_875_188, 3_981_541_245]), // -...187.5 -> -...188
    ];
    for ((u, v), expected) in cases {
        let direction = plus_z.cosine_direction([fixed(u), fixed(v)]).map(raw3);
        assert_eq!(direction, Some(expected), "u = {u}, v = {v}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_094_the_frame_division_rounds_ties_to_even_in_both_signs() {
    // With z = 0 the frame divides by s + z = 1.0 exactly, so q(x y) = x y / 2^32 raw: a tie
    // whenever the 2-adic valuations of x and y add up to 31. The four triangles below (found
    // by search; their plane normal is (2^30 x, 2^30 y, 0)) normalize to such unit normals:
    // x = 9 267 * 2^16 with y = 129 755 * 2^15, and x = 63 645 * 2^16 with y = 31 259 * 2^15.
    // tangent = (1 - q(x x), -q(x y), -x) and bitangent = (-q(x y), 1 - q(y y), -y).
    let up = [0, 0, 1 << 30];
    let triangles: [([Raw3; 3], [Raw3; 3]); 4] = [
        // x y / 2^32 = 601 219 792.5 -> 601 219 792 (even truncation; away from zero: ...793).
        (
            [[0; 3], up, [4_251_811_840, -607_322_112, 0]],
            [
                [607_322_112, 4_251_811_840, 0],
                [4_209_090_007, -601_219_792, -607_322_112],
                [-601_219_792, 85_877_290, -4_251_811_840],
            ],
        ),
        // The mirror image, x < 0: -601 219 792.5 -> -601 219 792 (floored: -...793).
        (
            [[0; 3], up, [4_251_811_840, 607_322_112, 0]],
            [
                [-607_322_112, 4_251_811_840, 0],
                [4_209_090_007, 601_219_792, 607_322_112],
                [601_219_792, 85_877_290, -4_251_811_840],
            ],
        ),
        // 994 739 527.5 -> 994 739 528 (odd truncation; toward zero or floored: ...527).
        (
            [[0; 3], up, [1_024_294_912, -4_171_038_720, 0]],
            [
                [4_171_038_720, 1_024_294_912, 0],
                [244_281_271, -994_739_528, -4_171_038_720],
                [-994_739_528, 4_050_686_026, -1_024_294_912],
            ],
        ),
        // The mirror image: -994 739 527.5 -> -994 739 528 (toward zero or half up: -...527).
        (
            [[0; 3], up, [1_024_294_912, 4_171_038_720, 0]],
            [
                [-4_171_038_720, 1_024_294_912, 0],
                [244_281_271, 994_739_528, 4_171_038_720],
                [994_739_528, 4_050_686_026, -1_024_294_912],
            ],
        ),
    ];
    for (vertices, expected) in triangles {
        assert_eq!(frame_raw(&surface(vertices).frame()), expected, "{vertices:?}");
    }
    // A sliver whose plane normal (490 274 164 023, 849 179 761 779, 0) normalizes to
    // (H, 3 719 550 787, 0), the odd neighbour of the review's tie (found by search; a
    // triangle's unit normal with x = H and z = 0 has y = 3 719 550 786 or ...787, so the
    // review's ...789 needs a unit vector given as it stands). q(x y) = 3 719 550 787 / 2 =
    // 1 859 775 393.5 -> 1 859 775 394 (toward zero or floored: ...393).
    let sliver = [[0; 3], [0, 0, 1], [849_179_761_779, -490_274_164_023, 0]];
    assert_eq!(
        frame_raw(&surface(sliver).frame()),
        [
            [H, 3_719_550_787, 0],
            [3_221_225_472, -1_859_775_394, -H],
            [-1_859_775_394, 1_073_741_824, -3_719_550_787],
        ]
    );
    // Unit normals given as they stand. The review's tie (H, 3 719 550 789, 0): q(x y) =
    // 3 719 550 789 / 2 = 1 859 775 394.5 -> 1 859 775 394 (away from zero: ...395); q(y y) is
    // 3 221 225 475.88 -> 3 221 225 476. With z > 0 the divisor 1 + z is odd, so no tie
    // exists; the two normals after it (found by search) put the remainder of x y one half
    // unit of the divisor below and above the half: 2 r = d - 1 and 2 r = d + 1.
    let given: [[Raw3; 3]; 3] = [
        [
            [H, 3_719_550_789, 0],
            [3_221_225_472, -1_859_775_394, -H],
            [-1_859_775_394, 1_073_741_820, -3_719_550_789],
        ],
        // x y / (U + 14 929) = 3 971 126.5 - 1 / (2 d) -> 3 971 126.
        [
            [3_971_142, 4_294_965_461, 14_929],
            [4_294_963_624, -3_971_126, -3_971_142],
            [-3_971_126, 18_599, -4_294_965_461],
        ],
        // x y / (U + 21 209) = 1 304 119.5 + 1 / (2 d) -> 1 304 120 (floored: ...119).
        [
            [1_304_126, 4_294_967_098, 21_209],
            [4_294_966_900, -1_304_120, -1_304_126],
            [-1_304_120, 21_605, -4_294_967_098],
        ],
    ];
    for expected in given {
        let frame = ShadingFrame::new(unit(expected[0]));
        assert_eq!(frame_raw(&frame), expected, "{:?}", expected[0]);
    }
}
