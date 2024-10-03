@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<storage, read> chunks: array<Chunk>;
@group(0) @binding(2)
var<storage, read> voxel_data: array<u32>;
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
    chunk_scale: u32,
    chunk_dist: u32,
};

struct Chunk {
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
    let pixel_pos = vec2<f32>(
        (vec2<f32>(cell.xy) / view_dim_f - vec2<f32>(0.5)) * vec2<f32>(2.0, -2.0 * aspect)
    );
    let offset = vec3<f32>(f32(1u << (view.chunk_scale - 1)));
    let pos = view.transform * vec4<f32>(pixel_pos, 1.0, 1.0) + vec4<f32>(offset, 0.0);
    let dir = view.transform * vec4<f32>(normalize(vec3<f32>(pixel_pos, view.zoom)), 0.0);

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
const MAX_HITS = 10;

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const FULL_ALPHA = 0.999;
const EPSILON = 0.00000000001;
const MAX_ITERS = 10000;
// NOTE: CANNOT GO HIGHER THAN 23 due to how floating point
// numbers are stored and the bit manipulation used
const MAX_SCALE: u32 = 13;
fn trace_full(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    if arrayLength(&voxel_data) == 1 {
        return vec4<f32>(0.0);
    }
    let gi = 0;
    let chunk = chunks[gi];
    let side_len = 1u << view.chunk_scale;
    let dimensions = vec3<u32>(side_len);
    let dim_f = vec3<f32>(dimensions);

    let pos_start = pos_view.xyz;
    var dir = dir_view.xyz;
    if dir.x == 0 { dir.x = EPSILON; }
    if dir.y == 0 { dir.y = EPSILON; }
    if dir.z == 0 { dir.z = EPSILON; }

    let dir_if = sign(dir);
    let dir_uf = max(dir_if, vec3<f32>(0.0));

    // find where ray intersects with group
    // closest (min) and furthest (max) corners of cube relative to direction
    let pos_min = (vec3<f32>(1.0) - dir_uf) * dim_f;
    let pos_max = dir_uf * dim_f;
    // time of intersection; x = td + p, solve for t
    let t_min = (pos_min - pos_start) / dir;
    let t_max = (pos_max - pos_start) / dir;
    // time of entrance and exit of the cube
    let t_start = max(max(t_min.x, t_min.y), t_min.z);
    let t_end = min(min(t_max.x, t_max.y), t_max.z);
    if t_end < t_start { return vec4<f32>(0.0); }
    // axis of intersection
    let axis = select(select(2u, 1u, t_start == t_min.y), 0u, t_start == t_min.x);
    // time to move entire side length in each direction
    let inc_t = abs(1.0 / dir) * f32(side_len);
    let t = max(0.0, t_start);

    let inv_dir_bits = 7 - vec_to_dir(vec3<u32>(dir_uf));
    let corner_adj = t_min - inc_t;

    // calculate normals
    var normals = mat3x3<f32>(
        vec3<f32>(dir_if.x, 0.0, 0.0),
        vec3<f32>(0.0, dir_if.y, 0.0),
        vec3<f32>(0.0, 0.0, dir_if.z),
    );

    let result = cast_ray(chunk.offset, t, axis, inv_dir_bits, inc_t, corner_adj);
    return shade_ray(result, pos_start, dir_view.xyz, t_end, normals);
}

fn shade_ray(result: RayResult, pos_start: vec3<f32>, dir: vec3<f32>, t_end: f32, normals: mat3x3<f32>) -> vec4<f32> {
    var hits = result.hits;

    var color = vec4<f32>(0.0);
    for (var i = 0u; i < result.len; i += 1u) {
        let hit = hits[i];
        let id = hit.id;
        let t = hit.t;
        let axis = hit.axis;

        let next_t = select(hits[i + 1].t, t_end, i == result.len - 1);

        var pos = pos_start + dir * t;
        pos[axis] = round(pos[axis]) - f32(dir[axis] < 0.0);
        let normal = select(select(normals[0], normals[1], axis == 1), normals[2], axis == 2);
        let vcolor = shade(id, pos, normal, dir, next_t - t);
        color += vcolor * (1.0 - color.a);
        if color.a > FULL_ALPHA { break; }
    }
    return color;
}

struct RayHit {
    t: f32,
    id: u32,
    axis: u32,
}

struct RayResult {
    hits: array<RayHit, MAX_HITS>,
    len: u32,
}

fn cast_ray(
    data_offset: u32, t_start: f32, axis_start: u32,
    inv_dir_bits: u32, inc_t: vec3<f32>, corner_adj: vec3<f32>
) -> RayResult {
    var hits = array<RayHit, MAX_HITS>();
    var depth = 0u;
    var min_alpha = 0.0;

    var t = t_start;
    var axis = axis_start;
    var node_start = 0u;
    var scale = MAX_SCALE;
    var scale_exp2 = 1.0;
    var parents = array<u32, MAX_SCALE>();
    var child = inv_dir_bits;
    var vox_pos = vec3<f32>(1.0);
    var prev = 0u;

    var iters = 0;
    loop {
        if iters == MAX_ITERS { break; }
        iters += 1;
        let t_corner = vox_pos * inc_t + corner_adj;
        let node = voxel_data[data_offset + node_start + (child ^ inv_dir_bits)];
        if node >= LEAF_BIT {
            // ignore consecutive identical leaves
            if node != prev {
                let id = node & LEAF_MASK;
                hits[depth] = RayHit(t, id, axis);
                min_alpha += min_alpha(id) * (1.0 - min_alpha);
                depth += 1u;
                prev = node;
                if depth == 10 || min_alpha >= FULL_ALPHA { break; }
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
    return RayResult(hits, depth);
}

fn trace_chunk(
    offset: u32,
    inv_dir_bits: u32,
    t: f32, t_mult: f32, inc_t: vec3<f32>,
    min_adj: vec3<f32>
) {
}

fn dir_to_vec(bits: u32) -> vec3<u32> {
    return vec3<u32>(bits >> 2, (bits & 2) >> 1, bits & 1);
}

fn vec_to_dir(vec: vec3<u32>) -> u32 {
    return vec.x * 4 + vec.y * 2 + vec.z * 1;
}

fn min_alpha(id: u32) -> f32 {
    switch id {
        case 0u: {return 0.0;}
        case 3u: {return 0.5;}
        default: {return 1.0;}
    }
}

const AMBIENT: f32 = 0.2;
const SPECULAR: f32 = 0.5;

// returns premultiplied
fn shade(id: u32, pos: vec3<f32>, normal: vec3<f32>, dir_view: vec3<f32>, dist: f32) -> vec4<f32> {
    var color = vec4<f32>(0.0);
    if id == 0 {
        return color;
    }
    let random = random(floor(pos));
    let random2 = random(floor(pos) + vec3<f32>(0.0001));
    switch id {
        case 0u: {
            color = vec4<f32>(0.0);
        }
        case 1u: {
            color = vec4<f32>(vec3<f32>(0.5, 0.5, 0.5 + random * 0.2) * (random2 * 0.4 + 0.8), 1.0);
        }
        case 2u: {
            color = vec4<f32>(vec3<f32>(0.4 + random * 0.2, 0.9, 0.4 + random * 0.2) * (random2 * 0.2 + 0.9), 1.0);
        }
        case 3u: {
            let fog = min(dist / 64.0, 1.0);
            let a = 0.5;
            let rgb = vec3<f32>(0.5, 0.5, 1.0) * (random2 * 0.2 + 0.8);
            color = vec4<f32>(rgb * (1.0 - fog * a), a + fog * (1.0 - a));
        }
        default: {}
    }
    let light_color = vec3<f32>(1.0);
    let light_dir = global_lights[0].dir;

    let diffuse = max(dot(light_dir, normal), 0.0) * light_color;
    let ambient = AMBIENT * light_color;
    let spec_val = pow(max(dot(dir_view.xyz, reflect(-light_dir, normal)), 0.0), 32.0) * SPECULAR;
    let specular = spec_val * light_color;
    let new_color = (ambient + diffuse + specular) * color.xyz;
    let new_a = min(color.a + spec_val, 1.0);
    return vec4<f32>(new_color * new_a, new_a);
}

fn random(pos: vec3<f32>) -> f32 {
    return fract(sin(dot(pos, vec3<f32>(12.9898, 78.233, 25.1279))) * 43758.5453123);
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
