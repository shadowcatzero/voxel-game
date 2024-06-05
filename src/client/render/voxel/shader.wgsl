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
    offset: u32,
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

const X_AXIS = vec3<f32>(1.0, 0.0, 0.0);
const Y_AXIS = vec3<f32>(0.0, 1.0, 0.0);
const Z_AXIS = vec3<f32>(0.0, 0.0, 1.0);
const AXIS = mat3x3<f32>(X_AXIS, Y_AXIS, Z_AXIS);

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const NO_COLOR = vec4<f32>(0.0);

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    // get position of the pixel; eye at origin, pixel on plane z = 1
    let win_dim = vec2<f32>(f32(view.width), f32(view.height));
    let aspect = win_dim.y / win_dim.x;
    let pixel_pos = vec3<f32>(
        (in.clip_position.xy / win_dim - vec2<f32>(0.5)) * vec2<f32>(2.0, -2.0 * aspect),
        1.0
    );

    let group = voxel_groups[0];
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform position so that group is at 0,0 & find direction
    let transform = group.transform * view.transform;
    let dir = (transform * vec4<f32>(normalize(pixel_pos), 0.0)).xyz;
    var pos = (transform * vec4<f32>(pixel_pos, 1.0)).xyz;



    // find where ray intersects with group
    let plane_point = (vec3<f32>(1.0) - sign(dir)) / 2.0 * dim_f;
    if outside3f(pos, ZERO3F, dim_f) {
        // x = td + p, solve for t
        let t = (plane_point - pos) / dir;
        // points of intersection
        let px = pos + t.x * dir;
        let py = pos + t.y * dir;
        let pz = pos + t.z * dir;

        // check if point is in bounds
        let hit = vec3<bool>(
            inside2f(px.yz, ZERO2F, dim_f.yz),
            inside2f(py.xz, ZERO2F, dim_f.xz),
            inside2f(pz.xy, ZERO2F, dim_f.xy),
        ) && (t > ZERO3F);
        if !any(hit) {
            return NO_COLOR;
        }
        pos = select(select(pz, py, hit.y), px, hit.x);
    }
    var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



    let dir_if = sign(dir) * ceil(abs(dir));
    let dir_i = vec3<i32>(dir_if);
    // time to move 1 unit using dir
    let inc_t = abs(1.0 / dir);
    let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5, 0.5, 0.5) + dir_if / 2.0;

    // time of next plane hit for each direction
    var next_t = inc_t * abs(pos - corner);
    var color = NO_COLOR;
    var safety = 0;
    loop {
        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
        let vcolor = unpack4x8unorm(voxels[i]);
        color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
        if color.a >= 1.0 {
            return color;
        }

        // select next voxel to move to next based on least time
        if next_t.x < next_t.y && next_t.x < next_t.z {
            vox_pos.x += dir_i.x;
            next_t.x += inc_t.x;
            if vox_pos.x < 0 || vox_pos.x >= dim_i.x {
                return color;
            }
        } else if next_t.y < next_t.z {
            vox_pos.y += dir_i.y;
            next_t.y += inc_t.y;
            if vox_pos.y < 0 || vox_pos.y >= dim_i.y {
                return color;
            }
        } else {
            vox_pos.z += dir_i.z;
            next_t.z += inc_t.z;
            if vox_pos.z < 0 || vox_pos.z >= dim_i.z {
                return color;
            }
        }

        safety += 1;
        if safety > 1000 {
            break;
        }
    }
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

fn outside3f(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return any(v < low) || any(v > high);
}

fn inside2f(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return all(v >= low) && all(v <= high);
}
