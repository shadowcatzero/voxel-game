#![allow(asm_sub_register)]

use spirv_std::glam::{vec3, Mat3, Vec3};

pub fn sign(vec: Vec3) -> Vec3 {
    vec3(
        if vec.x > 0.0 { 1.0 } else { -1.0 },
        if vec.y > 0.0 { 1.0 } else { -1.0 },
        if vec.z > 0.0 { 1.0 } else { -1.0 },
    )
}

pub fn idx_vec(vec: Vec3, i: u32) -> f32 {
    match i {
        0 => vec.x,
        1 => vec.y,
        2 => vec.z,
        _ => 0.0,
    }
}

pub fn idx_mat(mat: Mat3, i: u32) -> Vec3 {
    match i {
        0 => mat.x_axis,
        1 => mat.y_axis,
        2 => mat.z_axis,
        _ => Vec3::ZERO,
    }
}

pub fn add_idx_vec(vec: &mut Vec3, i: u32, v: f32) {
    match i {
        0 => vec.x += v,
        1 => vec.y += v,
        2 => vec.z += v,
        _ => (),
    }
}

pub fn set_idx_vec(vec: &mut Vec3, i: u32, v: f32) {
    match i {
        0 => vec.x = v,
        1 => vec.y = v,
        2 => vec.z = v,
        _ => (),
    }
}

macro_rules! ext_import {
    () => {
        "%glsl = OpExtInstImport \"GLSL.std.450\""
    };
}

macro_rules! float_op {
    ($name: ident, $id: expr) => {
        pub fn $name(x: f32) -> f32 {
            let mut res: f32;
            unsafe {
                core::arch::asm!(
                    ext_import!(),
                    "%float = OpTypeFloat 32",
                    concat!("{res} = OpExtInst %float %glsl ", $id, " {x}"),
                    x = in(reg) x,
                    res = out(reg) res,
                );
            }
            res
        }
    };
}

float_op!(fract, 10);
float_op!(sin, 13);
float_op!(round, 1);

pub fn reflect(x: Vec3, y: Vec3) -> Vec3 {
    let mut o: Vec3 = Default::default();
    unsafe {
        core::arch::asm!(
            ext_import!(),
            "%float = OpTypeFloat 32",
            "%vec = OpTypeVector %float 3",
            "%x = OpLoad _ {x}",
            "%y = OpLoad _ {y}",
            "%o = OpExtInst %vec %glsl 71 %x %y",
            "OpStore {o} %o",
            x = in(reg) &x,
            y = in(reg) &y,
            o = in(reg) &mut o,
        );
    }
    o
}
