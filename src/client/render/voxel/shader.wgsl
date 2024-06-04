// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct View {
    width: u32,
    height: u32,
    zoom: f32,
    padding: u32,
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;
@group(0) @binding(3)
var<storage, read> voxels: array<u32>;

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
    out.tex_coords = pos;
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let aspect = f32(view.height) / f32(view.width);
    var pixel_pos = vec3<f32>(in.clip_position.x / f32(view.width), 1.0 - in.clip_position.y / f32(view.height), 1.0);
    pixel_pos.x -= 0.5;
    pixel_pos.y -= 0.5;
    pixel_pos.x *= 2.0;
    pixel_pos.y *= 2.0;
    pixel_pos.y *= aspect;

    pixel_pos = (view.transform * vec4<f32>(pixel_pos, 1.0)).xyz;
    let origin = (view.transform * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    let dir = normalize(pixel_pos - origin);



    let voxel_pos = vec3<f32>(-5.0, -5.0, 30.0);
    var t = 0;
    for(t = 0; t < 1000; t += 1) {
        let pos = pixel_pos + f32(t) * 0.1 * dir - voxel_pos;
        let rel_coords = vec3<i32>(pos.xyz);
        if rel_coords.x < 0 || rel_coords.y < 0 || rel_coords.z < 0 || rel_coords.x > 10 || rel_coords.y > 10 || rel_coords.z > 10 {
            continue;
        } else {
            let i = rel_coords.x + rel_coords.y * 10 + rel_coords.z * 100;
            let color = unpack4x8unorm(voxels[i]);
            if voxels[i] != 0 {
                return vec4<f32>(1.0);
            } else {
                let pos = vec3<f32>(rel_coords);
                return vec4<f32>(pos.x / 10.0, pos.y / 10.0, pos.z / 10.0, 1.0);
            }
        }
    }

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
