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

const ZERO3F = vec3<f32>(0.0);
const ZERO2F = vec2<f32>(0.0);
const DEPTH = 20;

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
    let dir_view = view.transform * vec4<f32>(normalize(pixel_pos), 0.0);
    let pos_view = view.transform * vec4<f32>(pixel_pos, 1.0);

    var depths = array<f32,DEPTH>();
    var colors = array<vec4<f32>,DEPTH>();

    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        draw_group(gi, pos_view, dir_view, &depths, &colors);
    }
    var color = vec4<f32>(0.0);
    for(var di = 0; di < DEPTH; di += 1) {
        // p sure if it can't unroll colors the performance dies; switch to buffer
        let vcolor = colors[di];
        color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
        if vcolor.a == 0.0 || color.a >= 0.99999 {
            return color;
        }
    }
    return color;
}

fn draw_group(
    gi: u32, pos_view: vec4<f32>, dir_view: vec4<f32>,
    depths: ptr<function, array<f32, DEPTH>>,
    colors: ptr<function, array<vec4<f32>,DEPTH>>,
) {
    let group = voxel_groups[gi];
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform so that group is at 0,0
    var pos = (group.transform * pos_view).xyz;
    let dir = (group.transform * dir_view).xyz;

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
            return;
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
    var alpha = 0.0;
    var safety = 0;
    var t = 0.0;
    var prev_t = t;
    var depth = 0;
    loop {
        // should prolly remove when gaming
        safety += 1;

        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
        var vcolor = unpack4x8unorm(voxels[i]);

        // select next voxel to move to next based on least time
        let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
        prev_t = t;
        t = next_t[axis];
        vox_pos[axis] += dir_i[axis];
        next_t[axis] += inc_t[axis];

        // hit a voxel
        if vcolor.a > 0.0 {
            let full_t = t_offset + prev_t;
            // skip closer depth hits, or completely if behind opaque
            while (*depths)[depth] < full_t && (*colors)[depth].a != 0.0 {
                depth += 1;
                if depth >= DEPTH || (*colors)[depth].a == 1.0 {
                    return;
                }
            }
            var move_d = depth;
            // move further depth hits back
            while move_d < DEPTH - 1 && (*colors)[move_d].a != 0.0 {
                (*colors)[move_d + 1] = (*colors)[move_d];
                (*depths)[move_d + 1] = (*depths)[move_d];
                move_d += 1;
            }
            // add hit
            (*depths)[depth] = full_t;
            (*colors)[depth] = vcolor;
            depth += 1;
            alpha += (1.0 - alpha) * vcolor.a;
        }

        if alpha >= 0.9999 || depth >= DEPTH
            || vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis]
            || safety > 1000 {
            return;
        }
    }
}

fn outside3f(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return any(v < low) || any(v > high);
}

fn inside2f(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return all(v >= low) && all(v <= high);
}
