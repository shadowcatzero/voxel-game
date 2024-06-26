use std::{collections::VecDeque, fmt::Debug};

use nalgebra::Vector3;
use ndarray::ArrayView3;

const LEAF_BIT: u32 = 1 << 31;
const DATA_OFFSET: usize = 9;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OctNode(u32);
impl OctNode {
    pub fn new_node(addr: u32) -> Self {
        Self(addr)
    }
    pub fn new_leaf(data: u32) -> Self {
        Self(data | LEAF_BIT)
    }
    pub fn new_parent(offset: u32, corner: u32) -> Self {
        Self((offset << 3) + corner)
    }
    pub fn is_leaf(&self) -> bool {
        self.0 >= LEAF_BIT
    }
    pub fn is_node(&self) -> bool {
        self.0 < LEAF_BIT
    }
    pub fn node_data(&self) -> u32 {
        self.0
    }
    pub fn leaf_data(&self) -> u32 {
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
    pub fn from_fn(f: &mut impl FnMut(Vector3<usize>) -> u32, levels: u32) -> OctTree {
        Self::from_fn_offset(f, levels, Vector3::from_element(0))
    }
    pub fn from_fn_offset(
        f: &mut impl FnMut(Vector3<usize>) -> u32,
        levels: u32,
        offset: Vector3<usize>,
    ) -> Self {
        let mut data = Vec::new();
        data.push(OctNode::new_node(0));
        // #######N P SSSSSSSS P
        // --------------------| 17
        // -------| 7
        Self::from_fn_offset_inner(f, &mut data, levels, offset, OctNode::new_parent(17, 7));
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
        f: &mut impl FnMut(Vector3<usize>) -> u32,
        accumulator: &mut Vec<OctNode>,
        level: u32,
        offset: Vector3<usize>,
        parent: OctNode,
    ) {
        if level == 0 {
            accumulator.push(OctNode::new_leaf(f(offset)));
            return;
        } else if level == 1 {
            let leaves: [OctNode; 8] =
                core::array::from_fn(|i| OctNode::new_leaf(f(offset + CORNERS[i])));
            if leaves.iter().all(|l| *l == leaves[0]) {
                accumulator.push(leaves[0]);
            } else {
                accumulator.extend_from_slice(&leaves);
                accumulator.push(parent);
            }
            return;
        }
        let i = accumulator.len();
        accumulator.resize(i + 8, OctNode::new_node(0));
        accumulator.push(parent);
        let mut data_start = 0;
        for (j, corner_offset) in CORNERS.iter().enumerate() {
            let sub_start = accumulator.len();
            let sub_parent_offset = 9 + data_start + 8;
            Self::from_fn_offset_inner(
                f,
                accumulator,
                level - 1,
                offset + corner_offset * 2usize.pow(level - 1),
                OctNode::new_parent(sub_parent_offset as u32, j as u32),
            );
            let len = accumulator.len() - sub_start;
            if len == 1 {
                accumulator[i + j] = accumulator[sub_start];
                accumulator.pop();
            } else {
                accumulator[i + j] = OctNode::new_node(data_start as u32);
                data_start += len;
            }
        }
        if data_start == 0 {
            let first = accumulator[i];
            if accumulator[i..i + 8].iter().all(|l| *l == first) {
                accumulator.truncate(i);
                accumulator.push(first)
            }
        }
    }
    pub fn from_arr(arr: ArrayView3<u32>, levels: u32) -> Self {
        Self::from_fn(&mut |p| arr[(p.x, p.y, p.z)], levels)
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
}

pub struct OctTreeIter<'a> {
    queue: Vec<OctNode>,
    levels: Vec<u32>,
    pos: usize,
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
            let pos = 0;
            let add = &self.data[pos..pos + 8];
            self.data = &self.data[pos + DATA_OFFSET..];
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
            pos: 0,
            cur: 0,
            levels: vec![self.levels],
            run: 0,
            queue: vec![self.data[0]],
        }
    }
}
