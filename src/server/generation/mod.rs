use nalgebra::Vector3;
use simdnoise::NoiseBuilder;

use crate::{
    common::component::{chunk, ChunkPos},
    util::oct_tree::OctTree,
};

pub fn generate_tree(pos: ChunkPos) -> OctTree {
    if pos.y > 0 || pos.y < -1 {
        return OctTree::from_leaf(0, 8);
    }
    let posf: Vector3<f32> = pos.cast() * chunk::SIDE_LENGTH as f32;
    let noise1 = generate_noise_map(0, 1.0, posf, chunk::SCALE, &mut |v: f32| {
        (v * 2.0).exp2() * TOP * 0.25
    });
    let noise2 = generate_noise_map(1, 50.0, posf, chunk::SCALE, &mut |v: f32| v * 20.0 + GRASS);
    OctTree::from_fn_rec(
        &mut |p| generate_leaf(p, posf, (&noise1.base, &noise2.base)),
        &mut |p, lvl| generate_node(p, lvl, posf, (&noise1, &noise2)),
        chunk::SCALE,
    )
}

const WATER: f32 = 0.18 * chunk::SIDE_LENGTH as f32;
const GRASS: f32 = 0.35 * chunk::SIDE_LENGTH as f32;
const TOP: f32 = 0.5 * chunk::SIDE_LENGTH as f32;

// 0 air 1 stone 2 grass 3 water
fn generate_leaf(p: Vector3<usize>, posf: Vector3<f32>, noise: (&[f32], &[f32])) -> u32 {
    let y = p.y as f32 + posf.y;
    let n = noise.0[p.x + p.z * chunk::SIDE_LENGTH];
    let n2 = noise.1[p.x + p.z * chunk::SIDE_LENGTH];
    if y < n {
        if y < WATER {
            1
        } else if y < n2 {
            2
        } else {
            1
        }
    } else if y <= WATER {
        3
    } else {
        0
    }
}

// 0 air 1 stone 2 grass 3 water
fn generate_node(
    p: Vector3<usize>,
    scale: u32,
    posf: Vector3<f32>,
    noise: (&NoiseMap, &NoiseMap),
) -> Option<u32> {
    let side_len = 2usize.pow(scale);
    let y = NumRange {
        min: p.y as f32 + posf.y,
        max: (p.y + side_len - 1) as f32 + posf.y,
    };
    let l = scale as usize - 1;
    let i = (p.x >> scale) + (p.z >> scale) * (chunk::SIDE_LENGTH / side_len);
    let n = &noise.0.levels[l][i];
    let n2 = &noise.1.levels[l][i];
    Some(if y.max < n.min {
        if y.max < WATER {
            1
        } else if y.max < n2.min && y.min >= WATER {
            2
        } else if y.min > n2.max {
            1
        } else {
            return None;
        }
    } else if y.max <= WATER && y.min > n.max {
        3
    } else if y.min > WATER && y.min > n.max {
        0
    } else {
        return None;
    })
}

fn generate_noise_map(
    seed: i32,
    freq: f32,
    posf: Vector3<f32>,
    levels: u32,
    adjust: &mut impl FnMut(f32) -> f32,
) -> NoiseMap {
    let mut size = 2usize.pow(levels);
    let (mut base, min, max) = NoiseBuilder::gradient_2d_offset(posf.x, size, posf.z, size)
        .with_seed(seed)
        .with_freq(freq / (size as f32))
        .generate();
    for v in &mut base {
        *v = adjust((*v - min) / (max - min));
    }
    let first_len = base.len() / 4;
    let mut first = Vec::with_capacity(first_len);
    for y in (0..size).step_by(2) {
        for x in (0..size).step_by(2) {
            let a = base[x + y * size];
            let b = base[x + 1 + y * size];
            let c = base[x + (y + 1) * size];
            let d = base[x + 1 + (y + 1) * size];
            first.push(NumRange {
                min: a.min(b).min(c).min(d),
                max: a.max(b).max(c).max(d),
            })
        }
    }
    let mut arr = vec![first];
    for l in 1..levels as usize {
        size /= 2;
        let prev = &arr[l - 1];
        let mut new = Vec::with_capacity(prev.len() / 4);
        for y in (0..size).step_by(2) {
            for x in (0..size).step_by(2) {
                let a = &prev[x + y * size];
                let b = &prev[x + 1 + y * size];
                let c = &prev[x + (y + 1) * size];
                let d = &prev[x + 1 + (y + 1) * size];
                new.push(NumRange {
                    min: a.min.min(b.min).min(c.min).min(d.min),
                    max: a.max.max(b.max).max(c.max).max(d.max),
                })
            }
        }
        arr.push(new);
    }
    NoiseMap { base, levels: arr }
}

#[derive(Debug)]
pub struct NoiseMap {
    levels: Vec<Vec<NumRange>>,
    base: Vec<f32>,
}

#[derive(Debug)]
pub struct NumRange {
    min: f32,
    max: f32,
}
