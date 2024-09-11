// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct View {
    transform: mat4x4<f32>,
    width: u32,
    height: u32,
    zoom: f32,
};

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;

    var pos = vec2<f32>(
        f32(vi % 2u) * 2.0 - 1.0,
        f32(vi / 2u) * 2.0 - 1.0,
    );
    out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let win_dim = vec2<f32>(f32(view.width), f32(view.height));
    var pos = in.clip_position.xy / win_dim;
    pos.y = 1.0 - pos.y;
    return textureSample(t_diffuse, s_diffuse, pos);
}

