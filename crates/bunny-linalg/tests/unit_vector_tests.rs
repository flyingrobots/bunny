use bunny_linalg::{FixedUnitVec2, FixedUnitVec3, FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const NEG_ONE: FixedQ32_32 = FixedQ32_32::from_raw(-bunny_num::fixed_q32_32::ONE_RAW);
const CONST_UNIT_X_2: Option<FixedUnitVec2> =
    FixedUnitVec2::try_from_unit(FixedVec2::new(FixedQ32_32::ONE, FixedQ32_32::ZERO));
const CONST_NOT_UNIT_2: Option<FixedUnitVec2> =
    FixedUnitVec2::try_from_unit(FixedVec2::new(FixedQ32_32::ONE, FixedQ32_32::ONE));
const CONST_NEG_Z_3: Option<FixedUnitVec3> = FixedUnitVec3::try_from_unit(FixedVec3::new(
    FixedQ32_32::ZERO,
    FixedQ32_32::ZERO,
    NEG_ONE,
));
const CONST_NOT_UNIT_3: Option<FixedUnitVec3> = FixedUnitVec3::try_from_unit(FixedVec3::new(
    FixedQ32_32::from_raw(1),
    FixedQ32_32::from_raw(1),
    FixedQ32_32::ZERO,
));

#[wasm_bindgen_test(unsupported = test)]
fn test_unit_vector_compile_time_proofs() {
    assert_eq!(
        CONST_UNIT_X_2.expect("const unit x should validate"),
        FixedUnitVec2::UNIT_X
    );
    assert_eq!(CONST_NOT_UNIT_2, None);

    assert_eq!(
        CONST_NEG_Z_3.expect("const negative z should validate"),
        FixedUnitVec3::NEG_UNIT_Z
    );
    assert_eq!(CONST_NOT_UNIT_3, None);

    assert_eq!(
        FixedUnitVec2::NEG_UNIT_Y.into_inner(),
        FixedVec2::new(FixedQ32_32::ZERO, NEG_ONE)
    );
    assert_eq!(
        FixedUnitVec3::UNIT_Y.into_inner(),
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO)
    );
}
