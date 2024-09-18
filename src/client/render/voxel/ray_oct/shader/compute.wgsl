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
    zoom: f32,
};

struct VoxelGroup {
    transform: mat4x4<f32>,
    transform_inv: mat4x4<f32>,
    scale: u32,
    offset: u32,
};

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) cell: vec3<u32>) {
    let view_dim = textureDimensions(output);
    // get position of the pixel; eye at origin, pixel on plane z = 1
    if cell.x >= view_dim.x || cell.y >= view_dim.y {
        return;
    }
    let view_dim_f = vec2<f32>(view_dim);
    let aspect = view_dim_f.y / view_dim_f.x;
    let pixel_pos = vec3<f32>(
        (vec2<f32>(cell.xy) / view_dim_f - vec2<f32>(0.5)) * vec2<f32>(2.0, -2.0 * aspect),
        view.zoom
    );
    let pos = view.transform * vec4<f32>(pixel_pos, 1.0);
    let dir = view.transform * vec4<f32>(normalize(pixel_pos), 0.0);

    var color = trace_full(pos, dir);
    let light_mult = clamp((-dot(dir.xyz, global_lights[0].dir) - 0.99) * 200.0, 0.0, 1.0);
    let sun_color = light_mult * vec3<f32>(1.0, 1.0, 1.0);
    let sky_bg = vec3<f32>(0.3, 0.6, 1.0);
    let sky_color = sun_color + sky_bg * (1.0 - light_mult);
    color += vec4<f32>(sky_color * (1.0 - color.a), 1.0 - color.a);
    color.a = 1.0;
    textureStore(output, cell.xy, color);
}

const LEAF_BIT = 1u << 31u;
const LEAF_MASK = ~LEAF_BIT;

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const FULL_ALPHA = 0.999;
const EPSILON = 0.00000000001;
const MAX_ITERS = 2000;
// NOTE: CANNOT GO HIGHER THAN 23 due to how floating point
// numbers are stored and the bit manipulation used
const MAX_SCALE: u32 = 13;

fn trace_full(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    let gi = 0;
    let group = voxel_groups[gi];
    if group.scale == 0 {
        return vec4<f32>(0.0);
    }
    let dimensions = vec3<u32>(1u << group.scale);
    let dim_f = vec3<f32>(dimensions);
    let dim_i = vec3<i32>(dimensions);

    // transform so that group is at 0,0
    let pos_start = (group.transform_inv * pos_view).xyz;
    var dir = (group.transform_inv * dir_view).xyz;
    if dir.x == 0 {dir.x = EPSILON;}
    if dir.y == 0 {dir.y = EPSILON;}
    if dir.z == 0 {dir.z = EPSILON;}

    let dir_if = sign(dir);
    let dir_uf = max(dir_if, vec3<f32>(0.0));



    // calculate normals
    var normals = mat3x3<f32>(
        (group.transform * vec4<f32>(dir_if.x, 0.0, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, dir_if.y, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, 0.0, dir_if.z, 0.0)).xyz,
    );
    var axis = 0u;

    // find where ray intersects with group
    let pos_min = (vec3<f32>(1.0) - dir_uf) * dim_f;
    // time of intersection; x = td + p, solve for t
    var t_min = (pos_min - pos_start) / dir;
    if outside3f(pos_start, ZERO3F, dim_f) {
        // points of intersection
        let px = pos_start + t_min.x * dir;
        let py = pos_start + t_min.y * dir;
        let pz = pos_start + t_min.z * dir;

        // check if point is in bounds
        let hit = vec3<bool>(
            inside2f(px.yz, ZERO2F, dim_f.yz),
            inside2f(py.xz, ZERO2F, dim_f.xz),
            inside2f(pz.xy, ZERO2F, dim_f.xy),
        ) && (t_min > ZERO3F);
        if !any(hit) {
            return vec4<f32>(0.0);
        }
        axis = select(select(2u, 1u, hit.y), 0u, hit.x);
    }
    let t_mult =f32(1u << (MAX_SCALE - group.scale));
    t_min *= t_mult;
    // time to move 1 unit in each direction
    let full = f32(1u << MAX_SCALE);
    let inc_t = abs(1.0 / dir) * full;
    let t_offset = max(max(t_min.x, t_min.y), t_min.z);
    var t = max(0.0, t_offset);

    let dir_i = vec3<i32>(dir_if);
    let dir_u = vec3<u32>(dir_uf);
    let dir_bits = vec_to_dir(dir_u);
    let inv_dir_bits = 7 - dir_bits;

    var node_start = 1u;
    var scale = MAX_SCALE - 1;
    var scale_exp2 = 0.5;
    var color = vec4<f32>(0.0);
    var parents = array<u32, MAX_SCALE>();
    var prev = LEAF_BIT;
    var old_t = t / t_mult;

    var child = 0u;
    var vox_pos = vec3<f32>(1.0);
    let t_center = t_min + scale_exp2 * inc_t;
    if t > t_center.x { vox_pos.x = 1.5; child |= 4u; }
    if t > t_center.y { vox_pos.y = 1.5; child |= 2u; }
    if t > t_center.z { vox_pos.z = 1.5; child |= 1u; }
    let min_adj = t_min - inc_t;

    var iters = 0;
    loop {
        if iters == MAX_ITERS {
            return vec4<f32>(1.0, 0.0, 1.0, 1.0);
        }
        iters += 1;
        let t_corner = vox_pos * inc_t + min_adj;
        let node = voxels[group.offset + node_start + (child ^ inv_dir_bits)];
        if node >= LEAF_BIT {
            if node != prev {
                if node != LEAF_BIT {
                    let real_t = t / t_mult;
                    let dist = real_t - old_t;
                    old_t = real_t;
                    let filt = min(dist / 64.0, 1.0);
                    if prev == LEAF_BIT + 3 {
                        color.a += filt * (1.0 - color.a);
                        if color.a > FULL_ALPHA { break; }
                    }
                    var pos = (pos_view + dir_view * real_t).xyz;
                    pos[axis] = round(pos[axis]) - (1.0 - dir_uf[axis]);
                    // if true {return vec4<f32>(floor(pos) / 16.0, 1.0);}
                    // let pos = (vox_pos - 1.5) * (dir_if) + 0.5 - scale_exp2 * (1.0 - dir_uf);
                    // let pos = t / t_mult;
                    // if true {return vec4<f32>(pos, 1.0);}
                    let vcolor = get_color(node & LEAF_MASK, pos);
                    let diffuse = max(dot(global_lights[0].dir, normals[axis]) + 0.1, 0.0);
                    let ambient = 0.2;
                    let lighting = max(diffuse, ambient);
                    let new_color = min(vcolor.xyz * lighting, vec3<f32>(1.0));
                    color += vec4<f32>(new_color.xyz * vcolor.a, vcolor.a) * (1.0 - color.a);
                    if color.a > FULL_ALPHA { break; }
                }
                prev = node;
            }

            // move to next time point and determine which axis to move along
            let t_next = t_corner + scale_exp2 * inc_t;
            t = min(min(t_next.x, t_next.y), t_next.z);
            axis = select(select(0u, 1u, t == t_next.y), 2u, t == t_next.z);
            let move_dir = 4u >> axis;

            // check if need to pop stack
            if (child & move_dir) > 0 {
                // calculate new scale; first differing bit after adding
                let axis_pos = vox_pos[axis];
                // AWARE
                let differing = bitcast<u32>(axis_pos) ^ bitcast<u32>(axis_pos + scale_exp2);
                scale = (bitcast<u32>(f32(differing)) >> 23) - 127 - (23 - MAX_SCALE);
                scale_exp2 = bitcast<f32>((scale + 127 - MAX_SCALE) << 23);
                if scale >= MAX_SCALE { break; }

                // restore & recalculate parent
                let parent_info = parents[scale];
                node_start = parent_info >> 3;
                child = parent_info & 7;
                let scale_vec = vec3<u32>(scale + 23 - MAX_SCALE);
                // remove bits lower than current scale
                vox_pos = bitcast<vec3<f32>>((bitcast<vec3<u32>>(vox_pos) >> scale_vec) << scale_vec);
            }
            // move to next child and voxel position
            child += move_dir;
            vox_pos[axis] += scale_exp2;
        } else {
            // push current node to stack
            parents[scale] = (node_start << 3) + child;
            scale -= 1u;

            // calculate child node vars
            scale_exp2 *= 0.5;
            child = 0u;
            let t_center = t_corner + scale_exp2 * inc_t;
            if t > t_center.x { vox_pos.x += scale_exp2; child |= 4u; }
            if t > t_center.y { vox_pos.y += scale_exp2; child |= 2u; }
            if t > t_center.z { vox_pos.z += scale_exp2; child |= 1u; }
            node_start = node;
        }
    }
    // let fog = min(t / t_mult / 1000.0, 1.0);
    // return vec4<f32>(color.xyz * (1.0 - fog) + vec3<f32>(fog), color.a * (1.0 - fog) + fog);
    // return vec4<f32>(f32(iters) / f32(MAX_ITERS), 0.0, 0.0, 1.0);
    return color;
}

fn dir_to_vec(bits: u32) -> vec3<u32> {
    return vec3<u32>(bits >> 2, (bits & 2) >> 1, bits & 1);
}

fn vec_to_dir(vec: vec3<u32>) -> u32 {
    return vec.x * 4 + vec.y * 2 + vec.z * 1;
}

fn get_color(id: u32, pos: vec3<f32>) -> vec4<f32> {
    let random = random(floor(pos));
    let random2 = random(floor(pos) + vec3<f32>(0.0001));
    switch id {
        case 0u: {
            return vec4<f32>(0.0);
        }
        case 1u: {
            let color = vec3<f32>(0.5, 0.5, 0.5 + random * 0.2) * (random2 * 0.4 + 0.8);
            return vec4<f32>(color, 1.0);
        }
        case 2u: {
            let color = vec3<f32>(0.4 + random * 0.2, 0.9, 0.4 + random * 0.2) * (random2 * 0.2 + 0.9);
            return vec4<f32>(color, 1.0);
        }
        case 3u: {
            let color = vec3<f32>(0.5, 0.5, 1.0) * (random2 * 0.2 + 0.8);
            return vec4<f32>(color, 0.5);
        }
        default: {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    }
}

fn random(pos: vec3<f32>) -> f32 {
    return fract(sin(dot(pos,vec3<f32>(12.9898,78.233,25.1279)))*43758.5453123);
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
