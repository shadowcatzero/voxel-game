// Vertex shader

struct GlobalLight {
    dir: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct View {
    transform: mat4x4<f32>,
    width: u32,
    height: u32,
    zoom: f32,
};

struct VoxelGroup {
    transform: mat4x4<f32>,
    transform_inv: mat4x4<f32>,
    dimensions: vec3<u32>,
    offset: u32,
};

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<storage, read> voxels: array<u32>;
@group(0) @binding(2)
var<storage, read> voxel_groups: array<VoxelGroup>;
@group(0) @binding(3)
var<storage, read> global_lights: array<GlobalLight>;

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;

    var pos = vec2<f32>(
        f32(vi % 2u) * 2.0 - 1.0,
        f32(vi / 2u) * 2.0 - 1.0,
    ) ;
    out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    // get position of the pixel; eye at origin, pixel on plane z = 1
    let win_dim = vec2<f32>(f32(view.width), f32(view.height));
    let aspect = win_dim.y / win_dim.x;
    let pixel_pos = vec3<f32>(
        (in.clip_position.xy / win_dim - vec2<f32>(0.5)) * vec2<f32>(2.0, -2.0 * aspect),
        view.zoom
    );

    // move to position in world
    let pos = view.transform * vec4<f32>(pixel_pos, 1.0);
    let dir = view.transform * vec4<f32>(normalize(pixel_pos), 0.0);

    var color = trace_full(pos, dir);
    let light_mult = clamp((-dot(dir.xyz, global_lights[0].dir) - 0.99) * 200.0, 0.0, 1.0);
    let sky_color = light_mult * vec3<f32>(1.0, 1.0, 1.0);
    color += vec4<f32>(sky_color * (1.0 - color.a), 1.0 - color.a);
    color.a = 1.0;
    return color;
}

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const DEPTH = 16u;
const FULL_ALPHA = 0.9999;

fn trace_full(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    let gi = 0;
    let group = voxel_groups[gi];
    if group.dimensions.x == 0 {
        return vec4<f32>(0.0);
    }
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform so that group is at 0,0
    let pos_start = (group.transform_inv * pos_view).xyz;
    let dir = (group.transform_inv * dir_view).xyz;

    let dir_if = sign(dir);
    let dir_uf = max(dir_if, vec3<f32>(0.0));



    // calculate normals
    var normals = mat3x3<f32>(
        (group.transform * vec4<f32>(dir_if.x, 0.0, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, dir_if.y, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, 0.0, dir_if.z, 0.0)).xyz,
    );
    var next_normal = vec3<f32>(0.0, 0.0, 0.0);

    // find where ray intersects with group
    let pos_min = (vec3<f32>(1.0) - dir_uf) * dim_f;
    let pos_max = dir_uf * dim_f;
    var pos = pos_start;
    // time of intersection; x = td + p, solve for t
    let t_min = (pos_min - pos) / dir;
    let t_max = (pos_max - pos) / dir;
    var t = 0.0;
    if outside3f(pos, ZERO3F, dim_f) {
        // points of intersection
        let px = pos + t_min.x * dir;
        let py = pos + t_min.y * dir;
        let pz = pos + t_min.z * dir;

        // check if point is in bounds
        let hit = vec3<bool>(
            inside2f(px.yz, ZERO2F, dim_f.yz),
            inside2f(py.xz, ZERO2F, dim_f.xz),
            inside2f(pz.xy, ZERO2F, dim_f.xy),
        ) && (t_min > ZERO3F);
        if !any(hit) {
            return vec4<f32>(0.0);
        }
        pos = select(select(pz, py, hit.y), px, hit.x);
        t = select(select(t_min.z, t_min.y, hit.y), t_min.x, hit.x);
        next_normal = select(select(normals[2], normals[1], hit.y), normals[0], hit.x);
    }
    let inc_t = abs(1.0 / dir);
    let dir_i = vec3<i32>(dir_if);
    let dir_u = vec3<u32>((dir_i + vec3<i32>(1)) / 2);
    var i = 0u;
    var data_start = 1u;
    var t_center = (t_max + t_min) / 2.0;
    var half_t_span = f32(256 / 2) * inc_t;
    for (var safety = 0; safety < 9; safety += 1) {
        let node = voxels[group.offset + i];
        if node >= LEAF_BIT {
            let vcolor = get_color(node & LEAF_MASK);
            if vcolor.a > 0.0 {
                // let diffuse = max(dot(global_lights[0].dir, next_normal) + 0.1, 0.0);
                // let ambient = 0.2;
                // let lighting = max(diffuse, ambient);
                // let new_color = min(vcolor.xyz * lighting, vec3<f32>(1.0));
                // color += vec4<f32>(new_color.xyz * vcolor.a, vcolor.a) * (1.0 - color.a);
                if vcolor.a > .999 {
                    return vcolor;
                }
            }
            return vcolor;
        } else {
            half_t_span *= 0.5;
            let dir_idx = vec3<u32>(vec3<f32>(t) < t_center);
            t_center += half_t_span * (1.0 - vec3<f32>(dir_idx * 2));

            let child_i = vec_to_dir(dir_idx ^ dir_u);
            let node_pos = data_start + node;
            i = node_pos + child_i;
            data_start = node_pos + 8;
            continue;
        }
    }
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}

const LEAF_BIT = 1u << 31u;
const LEAF_MASK = ~LEAF_BIT;

// there's no way this is efficient, mod is faster for all I know
fn dir_to_vec(bits: u32) -> vec3<u32> {
    return vec3<u32>(extractBits(bits, 2u, 1u), extractBits(bits, 1u, 1u), extractBits(bits, 0u, 1u));
}

fn vec_to_dir(vec: vec3<u32>) -> u32 {
    return vec.x * 4 + vec.y * 2 + vec.z * 1;
}

fn get_color(id: u32) -> vec4<f32> {
    switch id {
        case 0u: {
            return vec4<f32>(0.0);
        }
        case 1u: {
            return vec4<f32>(0.5, 0.5, 0.5, 1.0);
        }
        case 2u: {
            return vec4<f32>(0.5, 1.0, 0.5, 1.0);
        }
        case 3u: {
            return vec4<f32>(0.5, 0.5, 1.0, 0.5);
        }
        default: {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    }
}

fn outside3f(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return any(v < low) || any(v > high);
}

fn inside2f(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return all(v >= low) && all(v <= high);
}

fn inside3i(v: vec3<i32>, low: vec3<i32>, high: vec3<i32>) -> bool {
    return all(v >= low) && all(v <= high);
}
