use spirv_std::glam::{vec2, Vec2, Vec4};
use spirv_std::{spirv, Image, Sampler};

#[spirv(vertex)]
pub fn vs_main(
    #[spirv(vertex_index)] vi: u32,
    #[spirv(position)] clip_position: &mut Vec4,
    tex_pos: &mut Vec2,
) {
    let pos = vec2((vi % 2) as f32, (vi / 2) as f32);
    *clip_position = (pos * 2.0 - 1.0).extend(0.0).extend(1.0);
    *tex_pos = pos;
    tex_pos.y = 1.0 - tex_pos.y;
}

#[spirv(fragment)]
pub fn fs_main(
    #[spirv(descriptor_set = 0, binding = 0)] texture: &Image!(2D, type=f32, sampled=true),
    #[spirv(descriptor_set = 0, binding = 1)] sampler: &Sampler,
    tex_pos: Vec2,
    output: &mut Vec4,
) {
    *output = texture.sample(*sampler, tex_pos);
}
