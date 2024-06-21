use std::fmt::Debug;

use nalgebra::Vector3;
use ndarray::{Array3, ArrayView3, Axis};

#[derive(Debug, Clone)]
pub enum OctTree<T> {
    Leaf(T),
    Node(Box<[OctTree<T>; 8]>),
}

impl<T: PartialEq + Clone + Debug> OctTree<T> {
    pub fn from_arr(arr: ArrayView3<T>) -> OctTree<T> {
        let mut node_arr = arr.map(|x| OctTree::Leaf(x.clone()));
        while node_arr.len() > 1 {
            let new_data = node_arr.exact_chunks([2; 3]).into_iter().map(|chunk| {
                let vec: Vec<OctTree<T>> = chunk.iter().cloned().collect();
                let vec: [OctTree<T>; 8] = vec.try_into().unwrap();
                if let OctTree::Leaf(first) = &chunk[[0; 3]] {
                    if vec.iter().all(|n| {
                        if let OctTree::Leaf(d) = n {
                            *d == *first
                        } else {
                            false
                        }
                    }) {
                        return OctTree::Leaf(first.clone())
                    }
                }
                OctTree::Node(Box::new(vec))
            }).collect();
            node_arr = Array3::from_shape_vec([node_arr.len_of(Axis(0)) / 2; 3], new_data).unwrap();
        }
        node_arr[[0; 3]].clone()
    }
}

impl<T> OctTree<T> {
    fn get(i: Vector3<usize>) {

    }
}
