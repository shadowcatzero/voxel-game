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
    out.tex_coords = pos;
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
        1.0
    );

    // move to position in world
    let pos = view.transform * vec4<f32>(pixel_pos, 1.0);
    let dir = view.transform * vec4<f32>(normalize(pixel_pos), 0.0);

    var color = trace_full(pos, dir);
    let light_mult = clamp((-dot(dir.xyz, normalize(GLOBAL_LIGHT)) - 0.99) * 200.0, 0.0, 1.0);
    let sky_color = light_mult * vec3<f32>(1.0, 1.0, 1.0);
    color += vec4<f32>(sky_color * (1.0 - color.a), 1.0 - color.a);
    color.a = 1.0;
    return color;
}

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const DEPTH = 16u;
const FULL_ALPHA = 0.9999;
const GLOBAL_LIGHT = vec3<f32>(-0.5, -4.0, 2.0);

fn trace_full(pos: vec4<f32>, dir: vec4<f32>) -> vec4<f32> {
    // GPUs hate this
    var depths = array<f32,DEPTH>();
    var colors = array<u32,DEPTH>();

    var hits = 0u;
    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        apply_group(gi, pos, dir, &depths, &colors, &hits);
    }
    var color = vec4<f32>(0.0);
    for (var di = 0u; di < DEPTH; di += 1u) {
        let vcolor = unpack4x8unorm(colors[di]);
        color += vec4<f32>(vcolor.xyz * vcolor.a, vcolor.a) * (1.0 - color.a);
        if vcolor.a == 0.0 || color.a >= FULL_ALPHA {
            break;
        }
    }
    return color;
}

// apparently GPUs don't like dynamic indexing cause they just have
// a ton of registers instead of fast memory access; should probably
// try to optimize for that where I can

fn apply_group(
    gi: u32, pos_view: vec4<f32>, dir_view: vec4<f32>,
    depths: ptr<function, array<f32, DEPTH>>,
    colors: ptr<function, array<u32,DEPTH>>,
    hit_len: ptr<function, u32>,
) {
    let group = voxel_groups[gi];
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform so that group is at 0,0
    var pos = (group.transform_inv * pos_view).xyz;
    let dir = (group.transform_inv * dir_view).xyz;

    let dir_if = sign(dir);



    // calculate normals; maybe should do this on cpu?
    let normals = mat3x3<f32>(
        (group.transform * vec4<f32>(dir_if.x, 0.0, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, dir_if.y, 0.0, 0.0)).xyz,
        (group.transform * vec4<f32>(0.0, 0.0, dir_if.z, 0.0)).xyz,
    );
    var next_normal = vec3<f32>(0.0, 0.0, 0.0);
    let norm_light = normalize(GLOBAL_LIGHT);

    // find where ray intersects with group
    let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
    var t_offset = 0.0;
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
            return;
        }
        pos = select(select(pz, py, hit.y), px, hit.x);
        t_offset = select(select(t_i.z, t_i.y, hit.y), t_i.x, hit.x);
        next_normal = select(select(normals[2], normals[1], hit.y), normals[0], hit.x);
    }
    var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



    let dir_i = vec3<i32>(dir_if);
    // time to move 1 unit using dir
    let inc_t = abs(1.0 / dir);
    let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

    // time of next plane hit for each direction
    var next_t = inc_t * abs(pos - corner);
    var alpha = 0.0;
    var t = 0.0;
    var prev_t = t;
    var depth = 0u;
    var prev_a = 0.0;
    loop {
        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
        var vcolor = unpack4x8unorm(voxels[i]);
        let normal = next_normal;

        // select next voxel to move to next based on least time
        let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
        next_normal = select(select(normals[2], normals[1], axis == 1), normals[0], axis == 0);
        prev_t = t;
        // might want to make multiplication mask w select instead of dynamically indexing
        t = next_t[axis];
        vox_pos[axis] += dir_i[axis];
        next_t[axis] += inc_t[axis];

        // hit a voxel
        if vcolor.a > 0.0 {
            let full_t = t_offset + prev_t;
            // skip closer depth hits, or finish if behind opaque
            var a = unpack4x8unorm((*colors)[depth]).a;
            while (*depths)[depth] < full_t && a != 0.0 {
                alpha += (1.0 - alpha) * a;
                if depth + 1 >= DEPTH || alpha >= FULL_ALPHA {
                    return;
                }
                depth += 1u;
                a = unpack4x8unorm((*colors)[depth]).a;
            }
            var move_d = min(*hit_len, DEPTH - 1);
            // move further depth hits back (top 10 efficient algorithms)
            while move_d > depth {
                (*colors)[move_d] = (*colors)[move_d - 1];
                (*depths)[move_d] = (*depths)[move_d - 1];
                move_d -= 1u;
            }
            let full_pos = pos_view + dir_view * full_t;

            // lighting
            let light = trace_light(full_pos);
            let diffuse = max(dot(norm_light, normal) * 1.3 + 0.1, 0.0);
            let ambient = 0.2;
            let specular = (exp(max(
                -(dot(reflect(dir_view.xyz, normal), norm_light) + 0.90) * 4.0, 0.0
            )) - 1.0) * light;
            let lighting = max(diffuse * light.a, ambient);
            let new_rgb = min(vcolor.xyz * lighting + specular.xyz + light.xyz * vcolor.xyz, vec3<f32>(1.0));
            let new_a = min(vcolor.a + specular.a, 1.0);
            let color = vec4<f32>(new_rgb, new_a);
            // let color = vec4<f32>((cos(full_t * 0.1) + 1.0) * 0.5, 0.0, 0.0, new_a);

            // add hit
            (*depths)[depth] = full_t;
            (*colors)[depth] = pack4x8unorm(color);
            prev_a = vcolor.a;
            depth += 1u;
            *hit_len += 1u;
            alpha += (1.0 - alpha) * vcolor.a;
        }

        if alpha >= FULL_ALPHA || depth >= DEPTH || vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis] {
            return;
        }
    }
}

fn trace_light(
    pos: vec4<f32>
) -> vec4<f32> {
    let dir = vec4<f32>(-normalize(GLOBAL_LIGHT), 0.0);
    var mask = vec4<f32>(0.0);
    let start = pos + dir * .001;
    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        let color = trace_one(gi, start, dir);
        mask += vec4<f32>(color.xyz * color.a * (1.0 - mask.a), (1.0 - mask.a) * color.a);
        if mask.a > FULL_ALPHA {
            break;
        }
    }
    mask.a = 1.0 - mask.a;
    mask = vec4<f32>(mask.a * mask.xyz, mask.a);
    return mask;
}

fn trace_one(gi: u32, pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    let group = voxel_groups[gi];
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform so that group is at 0,0
    var pos = (group.transform_inv * pos_view).xyz;
    let dir = (group.transform_inv * dir_view).xyz;

    let dir_if = sign(dir);



    // find where ray intersects with group
    let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
    var t_offset = 0.0;
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
        t_offset = select(select(t_i.z, t_i.y, hit.y), t_i.x, hit.x);
    }
    var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



    let dir_i = vec3<i32>(dir_if);
    // time to move 1 unit using dir
    let inc_t = abs(1.0 / dir);
    let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

    // time of next plane hit for each direction
    var next_t = inc_t * abs(pos - corner);
    var color = vec4<f32>(0.0);
    loop {
        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
        var vcolor = unpack4x8unorm(voxels[i]);

        // select next voxel to move to next based on least time
        let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
        vox_pos[axis] += dir_i[axis];
        next_t[axis] += inc_t[axis];
        color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);

        if color.a >= FULL_ALPHA || vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis] {
            break;
        }
    }
    return color;
}

fn outside3f(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return any(v < low) || any(v > high);
}

fn inside2f(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return all(v >= low) && all(v <= high);
}
