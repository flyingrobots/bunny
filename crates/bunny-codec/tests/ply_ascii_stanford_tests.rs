//! Integration test: the Stanford bunny asset checked in to Geordi, read as ASCII PLY.
//!
//! The expected bounds are the manifest's decimal bounds converted with the same exact
//! rule; no floating point is used.

use bunny_codec::{parse_ascii_ply, parse_decimal_q32_32};
use wasm_bindgen_test::wasm_bindgen_test;

const ASSET: &[u8] = include_bytes!("fixtures/stanford-bunny-ascii/bun_zipper_res3.ply");
/// Byte length of the asset whose SHA-256 is 975e7f9b...5cd6.
const ASSET_LEN: usize = 142_784;
const MANIFEST_MIN: [&str; 3] = ["-0.0943643", "0.0334143", "-0.0616721"];
const MANIFEST_MAX: [&str; 3] = ["0.0609346", "0.184813", "0.0584651"];

fn convert(texts: [&str; 3]) -> [i64; 3] {
    texts.map(|text| parse_decimal_q32_32(text.as_bytes()).expect("manifest bound").raw())
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_051_geordi_stanford_bunny_asset_reads_with_exact_bounds() {
    assert_eq!(ASSET.len(), ASSET_LEN, "fixture bytes were rewritten");
    let mesh = parse_ascii_ply(ASSET).expect("asset should parse");
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (1889, 3851));

    let (mut min, mut max, mut count) = ([i64::MAX; 3], [i64::MIN; 3], 0);
    for vertex in mesh.vertices() {
        let vertex = vertex.expect("validated vertex");
        for (axis, value) in
            [vertex.x.raw(), vertex.y.raw(), vertex.z.raw()].into_iter().enumerate()
        {
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
        count += 1;
    }
    assert_eq!(count, 1889);
    let mut triangles = 0;
    for triangle in mesh.triangles() {
        triangle.expect("validated triangle");
        triangles += 1;
    }
    assert_eq!(triangles, 3851);
    assert_eq!(min, convert(MANIFEST_MIN));
    assert_eq!(max, convert(MANIFEST_MAX));
}
