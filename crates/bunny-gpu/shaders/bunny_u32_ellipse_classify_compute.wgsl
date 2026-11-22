// Compute conformance wrapper for bunny-u32-ellipse-classify/1.
// Concatenated after bunny_u32_ellipse_classify.wgsl by bunny-gpu.

@group(0) @binding(0)
var<storage, read> bunny_ellipse_inputs: array<vec4<u32>>;

@group(0) @binding(1)
var<storage, read_write> bunny_ellipse_outputs: array<vec2<u32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx: u32 = gid.x;
    if (idx >= arrayLength(&bunny_ellipse_inputs)) {
        return;
    }
    let v: vec4<u32> = bunny_ellipse_inputs[idx];
    bunny_ellipse_outputs[idx] = bunny_u32_ellipse_classify(v.x, v.y, v.z, v.w);
}
