// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_pos: vec2<f32>,
};

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var sample: sampler;

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = vec2<f32>(
        f32(vi % 2u),
        f32(vi / 2u),
    );
    out.clip_position = vec4<f32>(pos * 2.0 - 1.0, 0.0, 1.0);
    out.tex_pos = pos;
    out.tex_pos.y = 1.0 - out.tex_pos.y;
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    return textureSample(tex, sample, in.tex_pos);
}

