@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<storage, read> voxels: array<u32>;
@group(0) @binding(2)
var<storage, read> voxel_groups: array<VoxelGroup>;
@group(0) @binding(3)
var<storage, read> global_lights: array<GlobalLight>;
@group(0) @binding(4)
var output: texture_storage_2d<rgba8unorm, write>;

struct GlobalLight {
    dir: vec3<f32>,
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

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) cell: vec3<u32>) {
    // get position of the pixel; eye at origin, pixel on plane z = 1
    let win_dim = vec2<f32>(f32(view.width), f32(view.height));
    let aspect = win_dim.y / win_dim.x;
    let pixel_pos = vec3<f32>(
        (vec2<f32>(cell.xy) / win_dim - vec2<f32>(0.5)) * vec2<f32>(2.0, 2.0 * aspect),
        view.zoom
    );
    let pos = view.transform * vec4<f32>(pixel_pos, 1.0);
    let dir = view.transform * vec4<f32>(normalize(pixel_pos), 0.0);

    var color = trace_full(pos, dir);
    let light_mult = clamp((-dot(dir.xyz, global_lights[0].dir) - 0.99) * 200.0, 0.0, 1.0);
    let sky_color = light_mult * vec3<f32>(1.0, 1.0, 1.0);
    color += vec4<f32>(sky_color * (1.0 - color.a), 1.0 - color.a);
    color.a = 1.0;
    textureStore(output, cell.xy, color);
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



    // calculate normals
    var normals = mat3x3<f32>(
        (group.transform * vec4<f32>(dir_if.x, 0.0, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, dir_if.y, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, 0.0, dir_if.z, 0.0)).xyz,
    );
    var next_normal = vec3<f32>(0.0, 0.0, 0.0);

    // find where ray intersects with group
    let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
    var pos = pos_start;
    var t = 0.0;
    if outside3f(pos, ZERO3F, dim_f) {
        // time of intersection; x = td + p, solve for t
        let t_i = (plane_point - pos) / dir;
        // points of intersection
        let px = pos + t_i.x * dir;
        let py = pos + t_i.y * dir;
        let pz = pos + t_i.z * dir;

        // check if point is in bounds
        let hit = vec3<bool>(
            inside2f(px.yz, ZERO2F, dim_f.yz),
            inside2f(py.xz, ZERO2F, dim_f.xz),
            inside2f(pz.xy, ZERO2F, dim_f.xy),
        ) && (t_i > ZERO3F);
        if !any(hit) {
            return vec4<f32>(0.0);
        }
        pos = select(select(pz, py, hit.y), px, hit.x);
        t = select(select(t_i.z, t_i.y, hit.y), t_i.x, hit.x);
        next_normal = select(select(normals[2], normals[1], hit.y), normals[0], hit.x);
    }
    // voxel position relative to low_corner
    var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



    let dir_i = vec3<i32>(dir_if);
    let dir_u = ((dir_i + vec3<i32>(1)) / 2);
    let dir_bits = u32(dir_u.x * 4 + dir_u.y * 2 + dir_u.z);
    // time to move 1 unit using dir
    let inc_t = abs(1.0 / dir);
    var side_len = 256;
    // "unsigned" minimum cube coords of current tree
    var low_corner = vec3<i32>(0);
    // time of next 1 unit plane hit in each direction
    var color = vec4<f32>(0.0);
    var data_start = 1u;
    var i = 0u;
    var axis = 0;
    var parents = array<u32, 8>();
    var scale = 0;
    for (var safety = 0; safety < 1000; safety += 1) {
        let node = voxels[group.offset + i];
        if node >= LEAF_BIT {
            // leaf
            let leaf = node & LEAF_MASK;
            if leaf != 0 {
                let vcolor = get_color(leaf);
                let diffuse = max(dot(global_lights[0].dir, next_normal) + 0.1, 0.0);
                let ambient = 0.2;
                let lighting = max(diffuse, ambient);
                let new_color = min(vcolor.xyz * lighting, vec3<f32>(1.0));
                color += vec4<f32>(new_color.xyz * vcolor.a, vcolor.a) * (1.0 - color.a);
                if color.a > .999 {
                    // return vec4<f32>(f32(safety) / 1000.0, 0.0, 0.0, 1.0);
                    return color;
                }
            }

            // move to next face of cube
            let half_len = f32(side_len) / 2.0;
            let corner = vec3<f32>(low_corner) + vec3<f32>(half_len) + dir_if * half_len;
            let next_t = inc_t * abs(corner - pos_start);
            axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
            t = next_t[axis];
            next_normal = normals[axis];
            pos = pos_start + t * dir;
            let old = vox_pos[axis];
            vox_pos = vec3<i32>(pos) - low_corner;
            vox_pos = clamp(vox_pos, vec3<i32>(0), side_len - vec3<i32>(1));
            vox_pos[axis] += dir_i[axis];
        } else if inside3i(vox_pos, vec3<i32>(0), vec3<i32>(side_len - 1)) {
            // node
            parents[scale] = (data_start << 3) + (data_start - i - 1);
            scale += 1;

            let children_pos = data_start + node;
            side_len /= 2;
            let vcorner = vox_pos / side_len;
            let child_pos = u32(vcorner.x * 4 + vcorner.y * 2 + vcorner.z);
            i = children_pos + child_pos;
            data_start = children_pos + 8;

            vox_pos -= vcorner * side_len;
            low_corner += vec3<i32>(dir_to_vec(child_pos)) * i32(side_len);

            continue;
        }

        // exit if highest node
        if scale == 0 {
            // return vec4<f32>(f32(safety) / 1000.0, 0.0, 0.0, 1.0);
            return color;
        }

        // get parent info and reset "pointers" to parent
        scale -= 1;
        let parent_info = parents[scale];
        let loc = 8 - (data_start - i);
        data_start = parent_info >> 3;
        i = data_start - ((parent_info & 7) + 1);

        // adjust corner back to parent
        let low_corner_adj = vec3<i32>(dir_to_vec(loc)) * i32(side_len);
        low_corner -= low_corner_adj;

        // update vox pos to be relative to parent
        vox_pos += low_corner_adj;

        side_len *= 2;
    }
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}

const LEAF_BIT = 1u << 31u;
const LEAF_MASK = ~LEAF_BIT;

// there's no way this is efficient, mod is faster for all I know
fn dir_to_vec(bits: u32) -> vec3<u32> {
    return vec3<u32>(extractBits(bits, 2u, 1u), extractBits(bits, 1u, 1u), extractBits(bits, 0u, 1u));
}

fn get_voxel(offset: u32, pos_: vec3<u32>) -> u32 {
    var data_start = 1u;
    var i = 0u;
    var pos = pos_;
    var side_len: u32 = 256;
    var safety = 0;
    while voxels[offset + i] < LEAF_BIT {
        let node_pos = data_start + voxels[offset + i];
        side_len /= 2u;
        let corner = pos / side_len;
        pos -= corner * side_len;
        let j = corner.x * 4 + corner.y * 2 + corner.z;
        i = node_pos + j;
        data_start = node_pos + 8;
        if safety == 10 {
            return 10u;
        }
        safety += 1;
    }
    return voxels[offset + i] & LEAF_MASK;
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
            return vec4<f32>(0.8, 0.2, 0.2, 1.0);
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
