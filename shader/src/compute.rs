use core::mem::transmute;

use super::fns::*;
use spirv_std::glam::{
    vec2, vec3, vec4, Mat3, Mat4, UVec2, UVec3, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles,
};
use spirv_std::num_traits::{clamp, Pow};
use spirv_std::{spirv, Image};

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct View {
    transform: Mat4,
    zoom: f32,
    chunk_scale: u32,
    chunk_dist: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GlobalLight {
    dir: Vec3,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Chunk {
    offset: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PushConstants {
    time: u32,
}

#[spirv(compute(threads(8, 8)))]
pub fn main(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] view: &View,
    #[spirv(push_constant)] consts: &PushConstants,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] chunks: &[Chunk],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] voxel_data: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] global_lights: &[GlobalLight],
    #[spirv(descriptor_set = 0, binding = 4)] output: &Image!(2D, format = rgba8, sampled = false),
    #[spirv(global_invocation_id)] cell: UVec3,
) {
    let view_dim: UVec2 = output.query_size();
    // get position of the pixel; eye at origin, pixel on plane z = 1
    if cell.x >= view_dim.x || cell.y >= view_dim.y {
        return;
    }
    let view_dim_f = view_dim.as_vec2();
    let aspect = view_dim_f.y / view_dim_f.x;
    let pixel_pos = (cell.xy().as_vec2() / view_dim_f - vec2(0.5, 0.5)) * vec2(2.0, -2.0 * aspect);
    let offset = Vec3::from_array([(1 << (view.chunk_scale - 1)) as f32; 3]);
    let pos = view.transform * vec4(pixel_pos.x, pixel_pos.y, 1.0, 1.0) + offset.extend(0.0);
    let dir = view.transform * pixel_pos.extend(view.zoom).normalize().extend(0.0);

    let mut color = trace_full(
        global_lights,
        consts.time,
        view,
        voxel_data,
        chunks,
        pos,
        dir,
    );
    let light_mult = clamp(
        (-dir.xyz().dot(global_lights[0].dir) - 0.99) * 200.0,
        0.0,
        1.0,
    );
    let sun_color = light_mult * vec3(1.0, 1.0, 1.0);
    let sky_bg = vec3(0.3, 0.6, 1.0);
    let sky_color = sun_color + sky_bg * (1.0 - light_mult);
    color += (sky_color * (1.0 - color.w)).extend(1.0 - color.w);
    color.w = 1.0;
    unsafe {
        output.write(cell.xy(), color);
    }
}

const LEAF_BIT: u32 = 1u32 << 31;
const LEAF_MASK: u32 = !LEAF_BIT;
const MAX_HITS: u32 = 10;

const FULL_ALPHA: f32 = 0.999;
const EPSILON: f32 = 0.0000000001;
const MAX_ITERS: u32 = 10000;
// NOTE: CANNOT GO HIGHER THAN 23 due to how floating point
// numbers are stored and the bit manipulation used
const MAX_SCALE: u32 = 23;

fn trace_full(
    global_lights: &[GlobalLight],
    time: u32,
    view: &View,
    voxel_data: &[u32],
    chunks: &[Chunk],
    pos_view: Vec4,
    dir_view: Vec4,
) -> Vec4 {
    if voxel_data.len() == 1 {
        return Vec4::ZERO;
    }
    let gi = 0;
    let chunk = chunks[gi];
    let side_len = 1u32 << view.chunk_scale;
    let dimensions = UVec3::from_array([side_len; 3]);
    let dim_f = dimensions.as_vec3();

    let pos_start = pos_view.xyz();
    let mut dir = dir_view.xyz();
    if dir.x == 0.0 {
        dir.x = EPSILON;
    }
    if dir.y == 0.0 {
        dir.y = EPSILON;
    }
    if dir.z == 0.0 {
        dir.z = EPSILON;
    }

    let dir_if = sign(dir);
    let dir_uf = dir_if.max(Vec3::ZERO);

    // find where ray intersects with group
    // closest (min) and furthest (max) corners of cube relative to direction
    let pos_min = (Vec3::ONE - dir_uf) * dim_f;
    let pos_max = dir_uf * dim_f;
    // time of intersection; x = td + p, solve for t
    let t_min = (pos_min - pos_start) / dir;
    let t_max = (pos_max - pos_start) / dir;
    // time of entrance and exit of the cube
    let t_start = t_min.max_element();
    let t_end = t_max.min_element();
    if t_end < t_start {
        return Vec4::ZERO;
    }
    // axis of intersection
    let axis = if t_start == t_min.x {
        0
    } else if t_start == t_min.y {
        1
    } else {
        2
    };
    // time to move entire side length in each direction
    let inc_t = (1.0 / dir).abs() * (side_len as f32);
    let t = t_start.max(0.0);

    let inv_dir_bits = 7 - vec_to_dir(dir_uf.as_uvec3());
    let corner_adj = t_min - inc_t;

    // calculate normals
    let normals = Mat3::from_diagonal(dir_if);

    let result = cast_ray(
        voxel_data,
        chunk.offset,
        t,
        axis,
        inv_dir_bits,
        inc_t,
        corner_adj,
    );

    shade_ray(
        global_lights,
        time,
        result,
        pos_start,
        dir_view.xyz(),
        t_end,
        normals,
    )
}

fn shade_ray(
    global_lights: &[GlobalLight],
    time: u32,
    result: RayResult,
    pos_start: Vec3,
    dir: Vec3,
    t_end: f32,
    normals: Mat3,
) -> Vec4 {
    let hits = result.hits;

    let mut color = Vec4::ZERO;
    for i in 0..result.len {
        let hit = hits[i];
        let id = hit.id;
        let t = hit.t;
        let axis = hit.axis;

        let next_t = if i == result.len - 1 {
            t_end
        } else {
            hits[i + 1].t
        };

        let mut pos = pos_start + dir * t;
        let val = round(idx_vec(pos, axis)) - (idx_vec(dir, axis) < 0.0) as u32 as f32;
        set_idx_vec(&mut pos, axis, val);
        let normal = idx_mat(normals, axis);
        let vcolor = shade(global_lights, time, id, pos, normal, dir, next_t - t);
        color += vcolor * (1.0 - color.w);
        if color.w > FULL_ALPHA {
            break;
        }
    }
    color
}

const AMBIENT: f32 = 0.2;
const SPECULAR: f32 = 0.5;

// returns premultiplied
fn shade(
    global_lights: &[GlobalLight],
    time: u32,
    id: u32,
    pos: Vec3,
    normal: Vec3,
    dir_view: Vec3,
    dist: f32,
) -> Vec4 {
    let mut color = Vec4::ZERO;
    if id == 0 {
        return color;
    }
    let random = randomf(pos.floor());
    let random2 = randomf(pos.floor() + vec3(0.0001, 0.0001, 0.0001));
    match id {
        0 => {
            color = Vec4::ZERO;
        }
        1 => {
            color = (vec3(0.5, 0.5, 0.5 + random * 0.2) * (random2 * 0.4 + 0.8)).extend(1.0);
        }
        2 => {
            color = (vec3(0.4 + random * 0.2, 0.9, 0.4 + random * 0.2) * (random2 * 0.2 + 0.9))
                .extend(1.0);
        }
        3 => {
            let fog = (dist / 64.0).min(1.0);
            let a = 0.5;
            let rand =
                (sin(((time as f32) / 2000.0 + random) * core::f32::consts::TAU) + 1.0) / 2.0;
            let rgb = vec3(0.5, 0.5, 1.0) * (rand * 0.2 + 0.8);
            color = (rgb * (1.0 - fog * a)).extend(a + fog * (1.0 - a));
        }
        _ => (),
    }
    let light_color = Vec3::ONE;
    let light_dir = global_lights[0].dir;

    let diffuse = light_dir.dot(normal).max(0.0) * light_color;
    let ambient = AMBIENT * light_color;
    let spec_val = dir_view
        .xyz()
        .dot(reflect(-light_dir, normal))
        .max(0.0)
        .pow(32.0)
        * SPECULAR;
    let specular = spec_val * light_color;
    let new_color = (ambient + diffuse + specular) * color.xyz();
    let new_a = (color.w + spec_val).min(1.0);
    (new_color * new_a).extend(new_a)
}

fn randomf(pos: Vec3) -> f32 {
    fract(sin(pos.dot(vec3(12.9898, 78.233, 25.1279))) * 43758.545)
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct RayHit {
    t: f32,
    id: u32,
    axis: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct RayResult {
    hits: [RayHit; MAX_HITS as usize],
    len: usize,
}

fn vec_to_dir(vec: UVec3) -> u32 {
    vec.x * 4 + vec.y * 2 + vec.z
}

fn cast_ray(
    voxel_data: &[u32],
    data_offset: u32,
    t_start: f32,
    axis_start: u32,
    inv_dir_bits: u32,
    inc_t: Vec3,
    corner_adj: Vec3,
) -> RayResult {
    let mut hits: [RayHit; MAX_HITS as usize] = Default::default();
    let mut depth = 0;
    let mut min_alpha = 0.0;

    let mut t = t_start;
    let mut axis = axis_start;
    let mut node_start = 0;
    let mut scale = MAX_SCALE;
    let mut scale_exp2 = 1.0;
    let mut parents = [0u32; MAX_SCALE as usize];
    let mut child = inv_dir_bits;
    let mut vox_pos = Vec3::ONE;
    let mut prev = 0;

    for _ in 0..MAX_ITERS {
        let t_corner = vox_pos * inc_t + corner_adj;
        let node = voxel_data[(data_offset + node_start + (child ^ inv_dir_bits)) as usize];
        if node >= LEAF_BIT {
            // ignore consecutive identical leaves
            if node != prev {
                let id = node & LEAF_MASK;
                hits[depth] = RayHit { t, id, axis };
                min_alpha += min_alpha_for(id) * (1.0 - min_alpha);
                depth += 1;
                prev = node;
                if depth == 10 || min_alpha >= FULL_ALPHA {
                    break;
                }
            }

            // move to next time point and determine which axis to move along
            let t_next = t_corner + scale_exp2 * inc_t;
            t = t_next.min_element();
            axis = if t == t_next.x {
                0
            } else if t == t_next.y {
                1
            } else {
                2
            };
            let move_dir = 4 >> axis;

            // check if need to pop stack
            if (child & move_dir) > 0 {
                // calculate new scale; first differing bit after adding
                let axis_pos = idx_vec(vox_pos, axis);
                // AWARE
                let differing = axis_pos.to_bits() ^ (axis_pos + scale_exp2).to_bits();
                scale = ((differing as f32).to_bits() >> 23) - 127 - (23 - MAX_SCALE);
                scale_exp2 = f32::from_bits((scale + 127 - MAX_SCALE) << 23);
                if scale >= MAX_SCALE {
                    break;
                }

                // restore & recalculate parent
                let parent_info = parents[scale as usize - 1];
                node_start = parent_info >> 3;
                child = parent_info & 7;
                let scale_vec = UVec3::from_array([scale + 23 - MAX_SCALE; 3]);
                // remove bits lower than current scale
                unsafe {
                    vox_pos = transmute::<UVec3, Vec3>(
                        (transmute::<Vec3, UVec3>(vox_pos) >> scale_vec) << scale_vec,
                    );
                }
            }
            // move to next child and voxel position
            child += move_dir;
            add_idx_vec(&mut vox_pos, axis, scale_exp2);
        } else {
            // push current node to stack
            parents[scale as usize - 1] = (node_start << 3) + child;
            scale -= 1;

            // calculate child node vars
            scale_exp2 *= 0.5;
            child = 0;
            let t_center = t_corner + scale_exp2 * inc_t;
            if t > t_center.x {
                vox_pos.x += scale_exp2;
                child |= 4;
            }
            if t > t_center.y {
                vox_pos.y += scale_exp2;
                child |= 2;
            }
            if t > t_center.z {
                vox_pos.z += scale_exp2;
                child |= 1;
            }
            node_start = node;
        }
    }
    RayResult { hits, len: depth }
}

fn min_alpha_for(id: u32) -> f32 {
    match id {
        0 => 0.0,
        3 => 0.5,
        _ => 1.0,
    }
}
