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

struct VoxelGroup {
    transform: mat4x4<f32>,
    dimensions: vec3<u32>,
};

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<storage, read> voxels: array<u32>;
@group(0) @binding(2)
var<storage, read> voxel_groups: array<VoxelGroup>;

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

    let group = voxel_groups[0];
    let dim_f = vec3<f32>(group.dimensions);

    // this should definitely be done per pixel trust me guys
    let transform = group.transform * view.transform;
    pixel_pos = (transform * vec4<f32>(pixel_pos, 1.0)).xyz;
    let origin = (transform * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
    let dir = normalize(pixel_pos - origin);



    var t = 0;
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    for(t = 0; t < 1000; t += 1) {
        let pos = pixel_pos + f32(t) * 0.1 * dir;
        if pos.x < 0.0 || pos.y < 0.0 || pos.z < 0.0 || pos.x > dim_f.x || pos.y > dim_f.y || pos.z > dim_f.z {
            continue;
        } else {
            let rel_coords = vec3<u32>(pos.xyz);
            let i = u32(rel_coords.x + rel_coords.y * group.dimensions.x + rel_coords.z * group.dimensions.x * group.dimensions.y);
            let vcolor = unpack4x8unorm(voxels[i]);
            // now I understand premultiplied alpha lmao
            color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
            if color.a == 1.0 {
                break;
            }
        }
    }

    return color;
}
