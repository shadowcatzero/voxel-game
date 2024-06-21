// Vertex shader

struct InstanceInput {
    @location(0) index: u32,
    @location(1) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

struct VoxelFace {
    index: u32,
    color: u32,
}

struct View {
    transform: mat4x4<f32>,
    width: u32,
    height: u32,
    zoom: f32,
};

struct VoxelGroup {
    transform: mat4x4<f32>,
    dimensions: vec3<u32>,
    face: u32,
};

struct GlobalLight {
    dir: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> group: VoxelGroup;
@group(0) @binding(3)
var<storage, read> global_lights: array<GlobalLight>;

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    in: InstanceInput
) -> VertexOutput {
    var out: VertexOutput;

    let invert = select(0.0, 1.0, group.face / 3 == 1);
    let invert_mult = 1.0 - invert * 2.0;
    let face_axis = group.face % 3;
    var square_pos = vec2<f32>(
        f32(vi % 2u),
        invert + invert_mult * f32(vi / 2u),
    );
    var cube_pos = vec3<f32>(invert);
    square_pos *= invert_mult;
    cube_pos[(group.face + 1) % 3] += square_pos.x;
    cube_pos[(group.face + 2) % 3] += square_pos.y;

    let cube_normal = invert_mult * vec3<f32>(
        f32(face_axis == 0),
        f32(face_axis % 2),
        f32(face_axis / 2),
    );
    out.normal = (group.transform * vec4<f32>(cube_normal, 0.0)).xyz;

    var pos = vec4<f32>(
        cube_pos,
        1.0,
    );
    pos += vec4<f32>(
        f32(in.index / (group.dimensions.z * group.dimensions.y)),
        f32((in.index / group.dimensions.z) % group.dimensions.y),
        f32(in.index % group.dimensions.z),
        0.0,
    );
    pos = view.transform * group.transform * pos;
    out.clip_position = pos;
    out.color = unpack4x8unorm(in.color);
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let diffuse = max(dot(global_lights[0].dir, in.normal) + 0.1, 0.0);
    let ambient = 0.2;
    let lighting = max(diffuse, ambient);
    let new_rgb = min(in.color.xyz * lighting, vec3<f32>(1.0));
    return vec4<f32>(new_rgb, in.color.a);
}
