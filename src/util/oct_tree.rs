use std::{fmt::Debug, hash::Hash};

use nalgebra::Vector3;
use ndarray::ArrayView3;
use rustc_hash::FxHashMap;

const LEAF_BIT: u32 = 1 << 31;
const DATA_OFFSET: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OctNode(u32);
impl OctNode {
    pub const fn new_node(addr: u32) -> Self {
        Self(addr)
    }
    pub const fn new_leaf(data: u32) -> Self {
        Self(data | LEAF_BIT)
    }
    pub const fn is_leaf(&self) -> bool {
        self.0 >= LEAF_BIT
    }
    pub const fn is_node(&self) -> bool {
        self.0 < LEAF_BIT
    }
    pub const fn node_data(&self) -> u32 {
        self.0
    }
    pub const fn leaf_data(&self) -> u32 {
        self.0 & !LEAF_BIT
    }
}

type OctNodeMap = FxHashMap<[OctNode; 8], OctNode>;

#[derive(Debug, Clone)]
pub struct OctTree {
    data: Vec<OctNode>,
    map: OctNodeMap,
    levels: u32,
    side_length: usize,
}

const CORNERS: [Vector3<usize>; 8] = [
    Vector3::new(0, 0, 0),
    Vector3::new(0, 0, 1),
    Vector3::new(0, 1, 0),
    Vector3::new(0, 1, 1),
    Vector3::new(1, 0, 0),
    Vector3::new(1, 0, 1),
    Vector3::new(1, 1, 0),
    Vector3::new(1, 1, 1),
];

impl OctTree {
    pub fn from_leaf(val: u32, levels: u32) -> Self {
        Self {
            data: vec![OctNode::new_leaf(val)],
            map: FxHashMap::default(),
            side_length: 2usize.pow(levels),
            levels,
        }
    }
    pub fn from_fn_rec(
        f_leaf: &mut impl FnMut(Vector3<usize>) -> u32,
        f_node: &mut impl FnMut(Vector3<usize>, u32) -> Option<u32>,
        levels: u32,
    ) -> OctTree {
        Self::from_fn_offset(f_leaf, f_node, levels, Vector3::from_element(0))
    }
    pub fn from_fn_offset(
        f_leaf: &mut impl FnMut(Vector3<usize>) -> u32,
        f_node: &mut impl FnMut(Vector3<usize>, u32) -> Option<u32>,
        levels: u32,
        offset: Vector3<usize>,
    ) -> Self {
        assert!(levels > 0);
        let mut data = Vec::new();
        let mut map = OctNodeMap::default();
        data.push(OctNode::new_node(0));
        Self::from_fn_offset_inner(f_leaf, f_node, &mut data, levels, offset, &mut map);
        if data.len() == 2 {
            data.remove(0);
        }
        Self {
            data,
            map,
            side_length: 2usize.pow(levels),
            levels,
        }
    }
    fn from_fn_offset_inner(
        f_leaf: &mut impl FnMut(Vector3<usize>) -> u32,
        f_node: &mut impl FnMut(Vector3<usize>, u32) -> Option<u32>,
        data: &mut Vec<OctNode>,
        level: u32,
        offset: Vector3<usize>,
        map: &mut OctNodeMap,
    ) {
        if level == 1 {
            let leaves: [OctNode; 8] =
                core::array::from_fn(|i| OctNode::new_leaf(f_leaf(offset + CORNERS[i])));
            if leaves[1..].iter().all(|l| *l == leaves[0]) {
                data.push(leaves[0]);
            } else if let Some(node) = map.get(&leaves) {
                data.push(*node);
            } else {
                data.extend_from_slice(&leaves);
            }
            return;
        }
        let i = data.len();
        data.resize(i + 8, OctNode::new_node(0));
        let mut data_start = 0;
        for (j, corner_offset) in CORNERS.iter().enumerate() {
            let lvl = level - 1;
            let pos = offset + corner_offset * 2usize.pow(lvl);
            if let Some(leaf) = f_node(pos, lvl) {
                data[i + j] = OctNode::new_leaf(leaf);
            } else {
                let sub_start = data.len();
                Self::from_fn_offset_inner(f_leaf, f_node, data, lvl, pos, map);
                let len = data.len() - sub_start;
                if len == 1 {
                    data[i + j] = data[sub_start];
                    data.pop();
                } else {
                    let node = OctNode::new_node(sub_start as u32);
                    data[i + j] = node;
                    data_start += len;
                    map.insert(data[sub_start..sub_start+8].try_into().unwrap(), node);
                }
            }
        }
        if data_start == 0 {
            let first = data[i];
            if first.is_leaf() && data[i + 1..i + 8].iter().all(|l| *l == first) {
                data.truncate(i);
                data.push(first);
            } else if let Some(node) = map.get(&data[i..i + 8]) {
                data.truncate(i);
                data.push(*node);
            }
        }
    }
    pub fn from_arr(arr: ArrayView3<u32>, levels: u32) -> Self {
        Self::from_fn_rec(&mut |p| arr[(p.x, p.y, p.z)], &mut |_, _| None, levels)
    }
    pub fn get(&self, mut pos: Vector3<usize>) -> u32 {
        let mut data_start = 1;
        let mut i = 0;
        let mut half_len = self.side_length / 2;
        while self.data[i].is_node() {
            let node_pos = data_start + self.data[i].node_data() as usize;
            let corner = pos / half_len;
            pos -= corner * half_len;
            half_len /= 2;
            let j = corner.x * 4 + corner.y * 2 + corner.z;
            i = node_pos + j;
            data_start = node_pos + DATA_OFFSET;
        }
        self.data[i].leaf_data()
    }
    pub fn raw(&self) -> &[OctNode] {
        &self.data
    }
    pub fn mesh(&self) {}
}

pub struct OctTreeIter<'a> {
    queue: Vec<OctNode>,
    levels: Vec<u32>,
    cur: u32,
    run: usize,
    data: &'a [OctNode],
}

impl<'a> Iterator for OctTreeIter<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.run != 0 {
            self.run -= 1;
            return Some(self.cur);
        }
        let node = self.queue.pop()?;
        let level = self.levels.pop()?;
        if node.is_leaf() {
            self.run = 8usize.pow(level);
            self.cur = node.leaf_data();
        } else {
            let add = &self.data[..8];
            self.data = &self.data[DATA_OFFSET..];
            self.queue.extend(add.iter().rev());
            self.levels.resize(self.levels.len() + 8, level - 1);
        }
        self.next()
    }
}

impl<'a> IntoIterator for &'a OctTree {
    type Item = u32;
    type IntoIter = OctTreeIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        OctTreeIter {
            data: &self.data[1..],
            cur: 0,
            levels: vec![self.levels],
            run: 0,
            queue: vec![self.data[0]],
        }
    }
}
