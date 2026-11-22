// Bunny deterministic GPU math primitive: bunny-u32-ellipse-classify/1
//
// Portable WGSL helper (zero entry points) implementing exact unsigned 64-bit
// ellipse trichotomy via 2-limb u32 arithmetic over 0..=46,340:
//
//   (dx * semi_y)^2 + (dy * semi_x)^2  <=>  (semi_x * semi_y)^2
//
// Returns vec2<u32>(class, status):
//   - status == 0u (Ok): class is 0u (Inside), 1u (OnBoundary), or 2u (Outside)
//   - status == 1u (OutOfDomain): any operand > 46340u; class is 0u
//
// Explicit degenerate rule precedes the raw polynomial:
//   - if semi_x == 0u || semi_y == 0u:
//       (0u, 0u) -> OnBoundary (1u), all other points -> Outside (2u).
//
// Zero floats, zero 64-bit extensions, zero divisions, zero subtractions.
// Makes no claim of Q32.32 GPU conformance.

const BUNNY_U32_ELLIPSE_MAX_COORD: u32 = 46340u;

const BUNNY_U32_ELLIPSE_CLASS_INSIDE: u32 = 0u;
const BUNNY_U32_ELLIPSE_CLASS_ON_BOUNDARY: u32 = 1u;
const BUNNY_U32_ELLIPSE_CLASS_OUTSIDE: u32 = 2u;

const BUNNY_U32_ELLIPSE_STATUS_OK: u32 = 0u;
const BUNNY_U32_ELLIPSE_STATUS_OUT_OF_DOMAIN: u32 = 1u;

fn bunny_u32_mul_wide(lhs: u32, rhs: u32) -> vec2<u32> {
    let lhs_lo = lhs & 0xffffu;
    let lhs_hi = lhs >> 16u;
    let rhs_lo = rhs & 0xffffu;
    let rhs_hi = rhs >> 16u;

    let p00 = lhs_lo * rhs_lo;
    let p01 = lhs_lo * rhs_hi;
    let p10 = lhs_hi * rhs_lo;
    let p11 = lhs_hi * rhs_hi;

    let mid = (p00 >> 16u) + (p01 & 0xffffu) + (p10 & 0xffffu);
    let lo = (p00 & 0xffffu) | ((mid & 0xffffu) << 16u);
    let hi = p11 + (p01 >> 16u) + (p10 >> 16u) + (mid >> 16u);
    return vec2<u32>(lo, hi);
}

fn bunny_u64_add(lhs: vec2<u32>, rhs: vec2<u32>) -> vec2<u32> {
    let lo = lhs.x + rhs.x;
    let carry = select(0u, 1u, lo < lhs.x);
    let hi = lhs.y + rhs.y + carry;
    return vec2<u32>(lo, hi);
}

fn bunny_u32_ellipse_classify(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> vec2<u32> {
    if (dx > BUNNY_U32_ELLIPSE_MAX_COORD
        || dy > BUNNY_U32_ELLIPSE_MAX_COORD
        || semi_x > BUNNY_U32_ELLIPSE_MAX_COORD
        || semi_y > BUNNY_U32_ELLIPSE_MAX_COORD) {
        return vec2<u32>(0u, BUNNY_U32_ELLIPSE_STATUS_OUT_OF_DOMAIN);
    }

    if (semi_x == 0u || semi_y == 0u) {
        if (dx == 0u && dy == 0u) {
            return vec2<u32>(BUNNY_U32_ELLIPSE_CLASS_ON_BOUNDARY, BUNNY_U32_ELLIPSE_STATUS_OK);
        }
        return vec2<u32>(BUNNY_U32_ELLIPSE_CLASS_OUTSIDE, BUNNY_U32_ELLIPSE_STATUS_OK);
    }

    let term_x = dx * semi_y;
    let term_y = dy * semi_x;
    let radius = semi_x * semi_y;

    let lhs = bunny_u64_add(
        bunny_u32_mul_wide(term_x, term_x),
        bunny_u32_mul_wide(term_y, term_y),
    );
    let rhs = bunny_u32_mul_wide(radius, radius);

    if (lhs.y < rhs.y || (lhs.y == rhs.y && lhs.x < rhs.x)) {
        return vec2<u32>(BUNNY_U32_ELLIPSE_CLASS_INSIDE, BUNNY_U32_ELLIPSE_STATUS_OK);
    }
    if (lhs.y == rhs.y && lhs.x == rhs.x) {
        return vec2<u32>(BUNNY_U32_ELLIPSE_CLASS_ON_BOUNDARY, BUNNY_U32_ELLIPSE_STATUS_OK);
    }
    return vec2<u32>(BUNNY_U32_ELLIPSE_CLASS_OUTSIDE, BUNNY_U32_ELLIPSE_STATUS_OK);
}
