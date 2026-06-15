use std::fmt::Write as _;

use bunny_codec::{
    decode_compressed_mesh, CompressedIndexWidth, CompressedMeshError, CompressedTriangle,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{QuantizedVertex, Triangle16, Triangle32};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const CANONICAL_HEX: &str = include_str!("fixtures/canonical_compressed_triangle.bunny.hex");
const HEADER_LEN: usize = 76;
const VERSION_OFFSET: usize = 8;
const INDEX_WIDTH_OFFSET: usize = 9;
const FLAGS_OFFSET: usize = 10;
const VERTEX_COUNT_OFFSET: usize = 12;
const TRIANGLE_COUNT_OFFSET: usize = 16;
const PAYLOAD_LEN_OFFSET: usize = 20;
const MIN_X_OFFSET: usize = 28;
const TRIANGLE16_OFFSET: usize = HEADER_LEN + 18;
const ONE_RAW: i64 = 1_i64 << 32;

fn canonical_triangle16() -> Vec<u8> {
    parse_hex(CANONICAL_HEX)
}

fn parse_hex(input: &str) -> Vec<u8> {
    let nybbles: Vec<_> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    assert_eq!(nybbles.len() % 2, 0, "hex fixture must contain byte pairs");
    nybbles
        .chunks_exact(2)
        .map(|pair| (hex_value(pair[0]) << 4) | hex_value(pair[1]))
        .collect()
}

fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("fixture contains non-hex byte"),
    }
}

fn compact_hex(input: &str) -> String {
    input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(char::from)
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String should not fail");
    }
    output
}

fn expected_bounds() -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ONE, FixedQ32_32::ONE),
    )
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i64(bytes: &mut [u8], offset: usize, value: i64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn canonical_triangle32() -> Vec<u8> {
    let mut bytes = canonical_triangle16();
    bytes[INDEX_WIDTH_OFFSET] = 32;
    write_u64(&mut bytes, PAYLOAD_LEN_OFFSET, 30);
    bytes.truncate(TRIANGLE16_OFFSET);
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes
}

fn assert_decode_error(bytes: Vec<u8>, expected: CompressedMeshError) {
    assert_eq!(decode_compressed_mesh(&bytes), Err(expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn canonical_fixture_is_stable_and_borrowed() {
    let bytes = canonical_triangle16();
    assert_eq!(bytes.len(), 100);
    assert_eq!(bytes_to_hex(&bytes), compact_hex(CANONICAL_HEX));

    let mesh = decode_compressed_mesh(&bytes).expect("canonical compressed mesh should parse");

    assert_eq!(mesh.bounds(), expected_bounds());
    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.triangle_count(), 1);
    assert_eq!(mesh.index_width(), CompressedIndexWidth::Width16);
    assert_eq!(
        mesh.vertex_bytes().as_ptr(),
        bytes.as_ptr().wrapping_add(HEADER_LEN)
    );
    assert_eq!(
        mesh.triangle_bytes().as_ptr(),
        bytes.as_ptr().wrapping_add(TRIANGLE16_OFFSET)
    );
    assert_eq!(mesh.vertex(1), Ok(QuantizedVertex::new(u16::MAX, 0, 0)));
    assert_eq!(
        mesh.triangle(0),
        Ok(CompressedTriangle::Width16(Triangle16::new(0, 1, 2)))
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn canonical_width32_payload_decodes_to_triangle32() {
    let bytes = canonical_triangle32();
    let mesh = decode_compressed_mesh(&bytes).expect("width32 compressed mesh should parse");

    assert_eq!(mesh.index_width(), CompressedIndexWidth::Width32);
    assert_eq!(mesh.triangle_bytes().len(), 12);
    assert_eq!(
        mesh.triangle(0),
        Ok(CompressedTriangle::Width32(Triangle32::new(0, 1, 2)))
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn accessors_reject_out_of_range_records() {
    let bytes = canonical_triangle16();
    let mesh = decode_compressed_mesh(&bytes).expect("canonical compressed mesh should parse");

    assert_eq!(mesh.vertex(3), Err(CompressedMeshError::IndexOutOfBounds));
    assert_eq!(mesh.triangle(1), Err(CompressedMeshError::IndexOutOfBounds));
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_malformed_header_corpus() {
    assert_eq!(
        decode_compressed_mesh(&[]),
        Err(CompressedMeshError::PayloadTooShort)
    );

    let mut bad_magic = canonical_triangle16();
    bad_magic[0] = b'x';
    assert_decode_error(bad_magic, CompressedMeshError::InvalidMagic);

    let mut bad_version = canonical_triangle16();
    bad_version[VERSION_OFFSET] = 2;
    assert_decode_error(bad_version, CompressedMeshError::UnsupportedVersion);

    let mut bad_width = canonical_triangle16();
    bad_width[INDEX_WIDTH_OFFSET] = 24;
    assert_decode_error(bad_width, CompressedMeshError::InvalidIndexWidth);

    let mut bad_flags = canonical_triangle16();
    write_u16(&mut bad_flags, FLAGS_OFFSET, 1);
    assert_decode_error(bad_flags, CompressedMeshError::UnsupportedFlags);
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_malformed_count_bounds_and_length_corpus() {
    let mut zero_vertices = canonical_triangle16();
    write_u32(&mut zero_vertices, VERTEX_COUNT_OFFSET, 0);
    assert_decode_error(zero_vertices, CompressedMeshError::InvalidCount);

    let mut zero_triangles = canonical_triangle16();
    write_u32(&mut zero_triangles, TRIANGLE_COUNT_OFFSET, 0);
    assert_decode_error(zero_triangles, CompressedMeshError::InvalidCount);

    let mut too_many_width16_vertices = canonical_triangle16();
    write_u32(&mut too_many_width16_vertices, VERTEX_COUNT_OFFSET, 65_537);
    assert_decode_error(too_many_width16_vertices, CompressedMeshError::InvalidCount);

    let mut inverted_bounds = canonical_triangle16();
    write_i64(&mut inverted_bounds, MIN_X_OFFSET, ONE_RAW * 2);
    assert_decode_error(inverted_bounds, CompressedMeshError::InvalidBounds);

    let mut invalid_payload_len = canonical_triangle16();
    write_u64(&mut invalid_payload_len, PAYLOAD_LEN_OFFSET, 23);
    invalid_payload_len.truncate(HEADER_LEN + 23);
    assert_decode_error(
        invalid_payload_len,
        CompressedMeshError::InvalidPayloadLength,
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn rejects_malformed_payload_corpus() {
    let mut short_payload = canonical_triangle16();
    short_payload.pop();
    assert_decode_error(short_payload, CompressedMeshError::PayloadTooShort);

    let mut trailing_payload = canonical_triangle16();
    trailing_payload.push(0);
    assert_decode_error(trailing_payload, CompressedMeshError::TrailingData);

    let mut invalid_index = canonical_triangle16();
    write_u16(&mut invalid_index, TRIANGLE16_OFFSET + 4, 3);
    assert_decode_error(invalid_index, CompressedMeshError::IndexOutOfBounds);

    let mut overflowing_len = canonical_triangle16();
    write_u64(&mut overflowing_len, PAYLOAD_LEN_OFFSET, u64::MAX);
    assert_decode_error(overflowing_len, CompressedMeshError::IntegerOverflow);
}
