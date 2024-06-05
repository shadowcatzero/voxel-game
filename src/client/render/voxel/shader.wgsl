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

const n0 = vec3<f32>(1.0, 0.0, 0.0);
const n1 = -n0;
const n2 = vec3<f32>(0.0, 1.0, 0.0);
const n3 = -n2;
const n4 = vec3<f32>(0.0, 0.0, 1.0);
const n5 = -n4;

const ORIGIN = vec3<f32>(0.0, 0.0, 0.0);
const ORIGIN2 = vec2<f32>(0.0, 0.0);
const NO_COLOR = vec4<f32>(0.0, 0.0, 0.0, 0.0);

@fragment
fn fs_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    let aspect = f32(view.height) / f32(view.width);
    var pos = vec3<f32>(in.clip_position.x / f32(view.width), 1.0 - in.clip_position.y / f32(view.height), 1.0);
    pos.x -= 0.5;
    pos.y -= 0.5;
    pos.x *= 2.0;
    pos.y *= 2.0;
    pos.y *= aspect;

    let group = voxel_groups[0];
    let dim_f = vec3<f32>(group.dimensions);

    // this should definitely be done per pixel trust me guys
    let transform = group.transform * view.transform;
    pos = (transform * vec4<f32>(pos, 1.0)).xyz;
    let origin = (transform * vec4<f32>(ORIGIN, 1.0)).xyz;
    let dir = normalize(pos - origin);



    var p = ORIGIN;

    var na: vec3<f32>;
    var nb: vec3<f32>;
    var nc: vec3<f32>;
    if dot(dir, n0) < 0.0 {
        na = n0;
        p.x = 1.0;
    } else {
        na = n1;
    }
    if dot(dir, n2) < 0.0 {
        nb = n2;
        p.y = 1.0;
    } else {
        nb = n3;
    }
    if dot(dir, n4) < 0.0 {
        nc = n4;
        p.z = 1.0;
    } else {
        nc = n5;
    }
    p *= dim_f;

    var vox_pos: vec3<i32>;
    var offset = ORIGIN;
    let dir_if = sign(dir) * ceil(abs(dir));
    if outside(pos, ORIGIN, dim_f) {
        let ta = intersect(pos, dir, p, na);
        let tb = intersect(pos, dir, p, nb);
        let tc = intersect(pos, dir, p, nc);
        let pa = pos + ta * dir;
        let pb = pos + tb * dir;
        let pc = pos + tc * dir;

        if inside2(pa.yz, ORIGIN2, dim_f.yz) && ta > 0.0 {
            pos = pa;
        } else if inside2(pb.xz, ORIGIN2, dim_f.xz) && tb > 0.0 {
            pos = pb;
        } else if inside2(pc.xy, ORIGIN2, dim_f.xy) && tc > 0.0 {
            pos = pc;
        } else {
            return NO_COLOR;
        }
    }
    vox_pos = vec3<i32>(pos);
    let dim_i = vec3<i32>(group.dimensions);
    vox_pos = clamp(vox_pos, vec3<i32>(0, 0, 0), dim_i - vec3<i32>(1, 1, 1));



    let dir_i = vec3<i32>(dir_if);
    let inc_t = abs(1.0 / dir);
    let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5, 0.5, 0.5) + dir_if / 2.0;
    var next_t = inc_t * abs(pos - corner);
    var color = NO_COLOR;
    var t = 0;
    loop {
        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y);
        let vcolor = unpack4x8unorm(voxels[i]);
        color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
        if color.a >= 1.0 {
            return color;
        }

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

        t += 1;
        if t > 1000 {
            break;
        }
    }
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

fn intersect(lp: vec3<f32>, ld: vec3<f32>, pp: vec3<f32>, pn: vec3<f32>) -> f32 {
    let v = pn * (lp - pp);
    let a = v.x + v.y + v.z;
    let u = pn * ld;
    let b = u.x + u.y + u.z;
    return -a / b;
}

fn outside(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return v.x < low.x || v.y < low.y || v.z < low.z || v.x > high.x || v.y > high.y || v.z > high.z;
}

fn inside(v: vec3<f32>, low: vec3<f32>, high: vec3<f32>) -> bool {
    return !outside(v, low, high);
}

fn outside2(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return v.x < low.x || v.y < low.y || v.x > high.x || v.y > high.y;
}

fn inside2(v: vec2<f32>, low: vec2<f32>, high: vec2<f32>) -> bool {
    return !outside2(v, low, high);
}
