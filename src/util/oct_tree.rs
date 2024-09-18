use std::fmt::Debug;

use nalgebra::Vector3;
use ndarray::ArrayView3;

const LEAF_BIT: u32 = 1 << 31;
const DATA_OFFSET: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
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

#[derive(Debug, Clone)]
pub struct OctTree {
    data: Vec<OctNode>,
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
        let mut data = Vec::new();
        data.push(OctNode::new_node(0));
        Self::from_fn_offset_inner(f_leaf, f_node, &mut data, levels, offset);
        if data.len() == 2 {
            data.remove(0);
        }
        Self {
            data,
            side_length: 2usize.pow(levels),
            levels,
        }
    }
    fn from_fn_offset_inner(
        f_leaf: &mut impl FnMut(Vector3<usize>) -> u32,
        f_node: &mut impl FnMut(Vector3<usize>, u32) -> Option<u32>,
        accumulator: &mut Vec<OctNode>,
        level: u32,
        offset: Vector3<usize>,
    ) {
        if level == 0 {
            accumulator.push(OctNode::new_leaf(f_leaf(offset)));
            return;
        } else if level == 1 {
            let leaves: [OctNode; 8] =
                core::array::from_fn(|i| OctNode::new_leaf(f_leaf(offset + CORNERS[i])));
            if leaves[1..].iter().all(|l| *l == leaves[0]) {
                accumulator.push(leaves[0]);
            } else {
                accumulator.extend_from_slice(&leaves);
            }
            return;
        }
        let i = accumulator.len();
        accumulator.resize(i + 8, OctNode::new_node(0));
        let mut data_start = 0;
        for (j, corner_offset) in CORNERS.iter().enumerate() {
            let lvl = level - 1;
            let pos = offset + corner_offset * 2usize.pow(lvl);
            if let Some(node) = f_node(pos, lvl) {
                accumulator[i + j] = OctNode::new_leaf(node);
            } else {
                let sub_start = accumulator.len();
                Self::from_fn_offset_inner(f_leaf, f_node, accumulator, lvl, pos);
                let len = accumulator.len() - sub_start;
                if len == 1 {
                    accumulator[i + j] = accumulator[sub_start];
                    accumulator.pop();
                } else {
                    accumulator[i + j] = OctNode::new_node(data_start as u32);
                    data_start += len;
                }
            }
        }
        if data_start == 0 {
            let first = accumulator[i];
            if accumulator[i + 1..i + 8].iter().all(|l| *l == first) {
                accumulator.truncate(i);
                accumulator.push(first);
            }
        }
    }

    pub fn from_fn_iter(f: &mut impl FnMut(Vector3<usize>) -> u32, levels: u32) -> Self {
        let mut data = vec![OctNode::new_node(0)];
        let mut level: usize = 1;
        let mut children = Vec::new();
        let mut child = vec![0; levels as usize + 1];
        let pows: Vec<_> = (0..levels).map(|l| 2usize.pow(l)).collect();
        while level < levels as usize {
            if child[level] == 8 {
                let i = children.len() - 8;
                let first = children[i];
                if children[i + 1..].iter().all(|l| *l == first) {
                    children.truncate(i);
                    children.push(first);
                } else {
                    data.extend_from_slice(&children[i..]);
                    children.truncate(i);
                    children.push(OctNode::new_node(data.len() as u32 - 8));
                }
                child[level] = 0;
                level += 1;
                child[level] += 1;
            } else if level == 1 {
                let offset: Vector3<usize> = (level..8).map(|l| CORNERS[child[l]] * pows[l]).sum();
                let leaves: [OctNode; 8] =
                    core::array::from_fn(|i| OctNode::new_leaf(f(offset + CORNERS[i])));
                if leaves[1..].iter().all(|l| *l == leaves[0]) {
                    children.push(leaves[0]);
                } else {
                    children.push(OctNode::new_node(data.len() as u32));
                    data.extend_from_slice(&leaves);
                }
                child[level] += 1;
            } else {
                level -= 1;
            }
        }
        data[0] = children[0];
        Self {
            data,
            side_length: 2usize.pow(levels),
            levels,
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
