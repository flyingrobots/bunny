//! Oriented boxes (`docs/topics/shape-library/test-plan.md`, SHAPE-TP-006 to -008).
//!
//! The oracle is `shape_oracle::obb_axes_verdict`: lengths and dot products of the raw columns
//! formed in 256-bit two's complement for any entries, sharing no code with the kernel, which
//! squares in `u128` and forms dot products in `i128` only after every length has passed.

mod shape_oracle;

use bunny_geom::{
    FixedObb3, ShapeError, OBB_AXIS_DOT_TOLERANCE_RAW, OBB_AXIS_LENGTH_TOLERANCE_RAW,
};
use bunny_linalg::{FixedMat3, FixedUnitVec3, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};
use shape_oracle::{obb_axes_verdict, AxisFault, Rng};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const LENGTH: i64 = 11;
const DOT: i64 = 12;
const ROTATION_SEED: u64 = 0x5EED_0115_0000_0007;
const ROTATION_CASES: usize = 3_000;
const OBB_SEED: u64 = 0x5EED_0115_0000_0008;
const OBB_CASES: usize = 30_000;

type Columns = [[i64; 3]; 3];

fn pt(v: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(v[0]),
        FixedQ32_32::from_raw(v[1]),
        FixedQ32_32::from_raw(v[2]),
    )
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn matrix(columns: Columns) -> FixedMat3 {
    FixedMat3::from_rows(pt(columns[0]), pt(columns[1]), pt(columns[2])).transpose()
}

fn columns(m: FixedMat3) -> Columns {
    [raw3(m.column0()), raw3(m.column1()), raw3(m.column2())]
}

fn identity() -> Columns {
    [[ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]]
}

fn build(columns: Columns) -> Result<FixedObb3, ShapeError> {
    FixedObb3::try_new(pt([1, 2, 3]), matrix(columns), pt([ONE, 0, 5]))
}

/// The kernel's verdict on these columns agrees with the oracle's.
fn check(columns: Columns, context: &str) -> Result<(), AxisFault> {
    let expected = obb_axes_verdict(columns, LENGTH, DOT);
    let wanted = match expected {
        Ok(()) => Ok(()),
        Err(AxisFault::Length(column)) => Err(ShapeError::AxisLengthOutOfTolerance { column }),
        Err(AxisFault::Dot(first, second)) => Err(ShapeError::AxesNotOrthogonal { first, second }),
    };
    assert_eq!(build(columns).map(|_| ()), wanted, "{context}: {columns:?}");
    expected
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_006_hand_built_boxes_on_and_beside_each_threshold() {
    assert_eq!((OBB_AXIS_LENGTH_TOLERANCE_RAW, OBB_AXIS_DOT_TOLERANCE_RAW), (11, 12));
    let center = pt([7, -8, 9]);
    let obb = FixedObb3::try_new(center, matrix(identity()), pt([0, 0, 0])).expect("a point box");
    assert_eq!(
        (obb.center(), columns(obb.axes()), obb.half_extents()),
        (center, identity(), pt([0, 0, 0]))
    );
    // Half extents: one raw unit either side of zero on each axis; the first negative wins.
    for axis in 0..3 {
        let mut extents = [ONE; 3];
        extents[axis] = -1;
        let refused = Err(ShapeError::NegativeHalfExtent { axis });
        assert_eq!(FixedObb3::try_new(center, matrix(identity()), pt(extents)), refused);
        extents[axis] = 0;
        assert!(FixedObb3::try_new(center, matrix(identity()), pt(extents)).is_ok(), "flat");
    }
    let refused = Err(ShapeError::NegativeHalfExtent { axis: 1 });
    assert_eq!(FixedObb3::try_new(center, matrix([[0; 3]; 3]), pt([0, i64::MIN, -1])), refused);
    // Axis-aligned lengths on and one raw unit past each end of the band, per column.
    for column in 0..3 {
        for (entry, ok) in [
            (ONE + LENGTH, true),
            (ONE + LENGTH + 1, false),
            (ONE - LENGTH, true),
            (ONE - LENGTH - 1, false),
        ] {
            let mut c = identity();
            c[column][column] = entry;
            assert_eq!(check(c, "aligned length").is_ok(), ok, "column {column} entry {entry}");
            c[column][column] = -entry;
            assert_eq!(check(c, "aligned length, reflected").is_ok(), ok);
        }
    }
    // A diagonal length threshold: for y = 2^31, x is the largest with x^2 + y^2 in the band.
    let high = u128::try_from(ONE + LENGTH).expect("positive").pow(2);
    let y = 1_i64 << 31;
    let x =
        i64::try_from((high - u128::try_from(y).expect("positive").pow(2)).isqrt()).expect("fits");
    assert!(check([[x, y, 0], [0, 0, ONE], [y, -x, 0]], "diagonal on").is_ok());
    assert_eq!(
        check([[x + 1, y, 0], [0, 0, ONE], [y, -x, 0]], "diagonal past"),
        Err(AxisFault::Length(0))
    );
    // Dot products on and one raw unit past 12 raw units, of either sign, for every pair.
    for d in [DOT, -DOT] {
        assert!(check([[ONE, 0, 0], [d, ONE, 0], [d, 0, ONE]], "dot on, pairs 01 02").is_ok());
        assert!(check([[ONE, 0, 0], [0, ONE, 0], [0, d, ONE]], "dot on, pair 12").is_ok());
        let past = d + d.signum();
        assert_eq!(
            check([[ONE, 0, 0], [past, ONE, 0], [0, 0, ONE]], "01"),
            Err(AxisFault::Dot(0, 1))
        );
        assert_eq!(
            check([[ONE, 0, 0], [0, ONE, 0], [past, 0, ONE]], "02"),
            Err(AxisFault::Dot(0, 2))
        );
        assert_eq!(
            check([[ONE, 0, 0], [0, ONE, 0], [0, past, ONE]], "12"),
            Err(AxisFault::Dot(1, 2))
        );
    }
    // Lengths are checked before dot products: column 2 too long beats pair (0, 1).
    assert_eq!(
        check([[ONE, 0, 0], [ONE, 0, 0], [0, 0, 2 * ONE]], "order"),
        Err(AxisFault::Length(2))
    );
    // The extremes of the raw range are refused by length, without overflow.
    assert_eq!(check([[i64::MIN; 3], [0, ONE, 0], [0, 0, ONE]], "min"), Err(AxisFault::Length(0)));
    assert_eq!(check([[ONE, 0, 0], [0, ONE, 0], [i64::MAX; 3]], "max"), Err(AxisFault::Length(2)));
    assert_eq!(check([[0; 3], [0, ONE, 0], [0, 0, ONE]], "zero axis"), Err(AxisFault::Length(0)));
    // Two equal axes are refused by their dot product. A reflection is accepted.
    assert_eq!(check([[ONE, 0, 0], [ONE, 0, 0], [0, 0, ONE]], "equal"), Err(AxisFault::Dot(0, 1)));
    assert!(check([[ONE, 0, 0], [0, ONE, 0], [0, 0, -ONE]], "reflection").is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_007_every_sampled_rotation_constructor_result_is_accepted() {
    let mut rng = Rng(ROTATION_SEED);
    let accept = |m: FixedMat3, context: &str| {
        assert_eq!(check(columns(m), context), Ok(()), "{context}");
    };
    // The two FixedUnitVec3 extremes that give the largest axis-angle errors.
    let extremes = [[ONE + 1, 92_681, 0], [ONE - 1, 0, 0]]
        .map(|u| FixedUnitVec3::try_from_unit(pt(u)).expect("in the band"));
    let mut look_accepted = 0;
    for case in 0..ROTATION_CASES {
        let context = format!("seed={ROTATION_SEED:#x} case={case}");
        let angle = FixedAngle::from_raw(rng.next_u64());
        accept(FixedMat3::rotation_x(angle), &context);
        accept(FixedMat3::rotation_y(angle), &context);
        accept(FixedMat3::rotation_z(angle), &context);
        let v = pt([(); 3].map(|()| rng.within(1 << 40)));
        if let Some(axis) = FixedUnitVec3::new(v) {
            accept(FixedMat3::rotation_axis_angle(axis, angle), &context);
        }
        accept(FixedMat3::rotation_axis_angle(extremes[case % 2], angle), &context);
        let forward = pt([(); 3].map(|()| rng.within(1 << 40)));
        let up = pt([(); 3].map(|()| rng.within(1 << 40)));
        if let Ok(m) = FixedMat3::try_look_rotation(forward, up) {
            accept(m, &context);
            look_accepted += 1;
        }
    }
    let half = FixedAngle::from_raw(1 << 63);
    accept(FixedMat3::rotation_axis_angle(extremes[0], half), "longest axis, half turn");
    assert!(look_accepted >= 2_900, "seed={ROTATION_SEED:#x}: {look_accepted}");
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_008_seeded_boxes_match_the_exact_oracle() {
    let mut rng = Rng(OBB_SEED);
    let (mut ok, mut length, mut dot) = (0, 0, 0);
    for case in 0..OBB_CASES {
        let context = format!("seed={OBB_SEED:#x} case={case}");
        let base = columns(FixedMat3::rotation_axis_angle(
            FixedUnitVec3::new(pt([(); 3].map(|()| rng.within(ONE))))
                .unwrap_or(FixedUnitVec3::UNIT_Z),
            FixedAngle::from_raw(rng.next_u64()),
        ));
        let spread = [8, 16, 40, 1 << 20][case % 4];
        let c = if case % 10 == 9 {
            [(); 3].map(|()| [(); 3].map(|()| rng.next_i64()))
        } else {
            base.map(|column| column.map(|entry| entry + rng.within(spread)))
        };
        match check(c, &context) {
            Ok(()) => ok += 1,
            Err(AxisFault::Length(_)) => length += 1,
            Err(AxisFault::Dot(..)) => dot += 1,
        }
    }
    assert!(
        ok >= 5_000 && length >= 5_000 && dot >= 500,
        "seed={OBB_SEED:#x}: {ok} {length} {dot}"
    );
}
