// Contract: bunny-u32-div-round-even/1
// Portable WGSL helper (zero entry points; adoptable directly by downstream shaders).
//
// Inputs:
//   n, d: u32 in 0..=4,294,967,295
// Output:
//   vec2<u32>(value, status)
//     status == 0u -> Success (value is ties-to-even rounded quotient of n / d)
//     status == 1u -> DivisionByZero (value is always 0u)
//
// Overflow-safe over the full u32 domain:
//   - Real control-flow branch checks d == 0u before `/` or `%`.
//   - s = d - r lies in 1..=d; comparing r with s avoids 2 * r overflow.
//   - When d == 1u, r == 0u so inc is false; when d >= 2u, q <= 2^31 - 1 so q + 1 fits in u32.
fn bunny_u32_div_round_even(n: u32, d: u32) -> vec2<u32> {
    if (d == 0u) {
        return vec2<u32>(0u, 1u);
    }
    let q: u32 = n / d;
    let r: u32 = n % d;
    let s: u32 = d - r;
    let inc: bool = (r > s) || ((r == s) && ((q & 1u) != 0u));
    return vec2<u32>(q + select(0u, 1u, inc), 0u);
}
