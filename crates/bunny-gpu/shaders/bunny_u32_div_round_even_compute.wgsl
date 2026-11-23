// Compute conformance wrapper for bunny-u32-div-round-even/1.
// Concatenated after bunny_u32_div_round_even.wgsl by bunny-gpu.

@group(0) @binding(0)
var<storage, read> bunny_div_inputs: array<vec2<u32>>;

@group(0) @binding(1)
var<storage, read_write> bunny_div_outputs: array<vec2<u32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx: u32 = gid.x;
    if (idx >= arrayLength(&bunny_div_inputs)) {
        return;
    }
    let pair: vec2<u32> = bunny_div_inputs[idx];
    bunny_div_outputs[idx] = bunny_u32_div_round_even(pair.x, pair.y);
}
