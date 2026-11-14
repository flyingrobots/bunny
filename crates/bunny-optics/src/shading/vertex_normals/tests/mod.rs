//! The triangle-count limit of `VertexNormals::new`, which no test can reach through the public
//! API: a mesh over the limit needs more than `2^32` triangles (48 GiB of indices).

use super::{check_triangle_count, VertexNormalError, MAX_NORMAL_TRIANGLES};

#[test]
fn optics_tp_069_the_triangle_count_limit_is_inclusive() {
    assert_eq!(MAX_NORMAL_TRIANGLES, 1 << 32);
    assert_eq!(check_triangle_count(0), Ok(()));
    assert_eq!(check_triangle_count(MAX_NORMAL_TRIANGLES), Ok(()));
    let over = MAX_NORMAL_TRIANGLES + 1;
    let refused = check_triangle_count(over);
    assert_eq!(refused, Err(VertexNormalError::TooManyTriangles { count: over }));
    let text = refused.err().map(|e| e.to_string());
    assert_eq!(text.as_deref(), Some("mesh has 4294967297 triangles, more than 4294967296"));
    let most = check_triangle_count(u64::MAX).err();
    assert_eq!(most, Some(VertexNormalError::TooManyTriangles { count: u64::MAX }));
}
