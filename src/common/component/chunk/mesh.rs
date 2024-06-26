use bevy_ecs::component::Component;
use block_mesh::{ndshape::RuntimeShape, UnitQuadBuffer, RIGHT_HANDED_Y_UP_CONFIG};
use nalgebra::Vector3;
use ndarray::{ArrayView3, Axis};

use crate::client::render::voxel::{VoxelColor, /*VoxelFace*/};

#[derive(Debug, Clone, Component)]
pub struct ChunkMesh {
    // pub faces: [Vec<VoxelFace>; 6],
}

impl ChunkMesh {
    pub fn from_data(data: ArrayView3<VoxelColor>) -> Self {
        // let dim_pad = Vector3::new(
        //     data.len_of(Axis(0)) as u32,
        //     data.len_of(Axis(1)) as u32,
        //     data.len_of(Axis(2)) as u32,
        // );
        // let dim = dim_pad - Vector3::from_element(2);
        // let mut buffer = UnitQuadBuffer::new();
        // let shape = RuntimeShape::<u32, 3>::new(dim_pad.into());
        // let slice = data.as_slice().unwrap();
        // block_mesh::visible_block_faces(
        //     slice,
        //     &shape,
        //     [0; 3],
        //     (dim_pad - Vector3::new(1, 1, 1)).into(),
        //     &RIGHT_HANDED_Y_UP_CONFIG.faces,
        //     &mut buffer,
        // );
        // let faces = [2, 1, 0, 5, 4, 3].map(|f| {
        //     buffer.groups[f]
        //         .iter()
        //         .map(|a| {
        //             let i = (a.minimum[0] - 1)
        //                 + (a.minimum[1] - 1) * dim.y
        //                 + (a.minimum[2] - 1) * dim.y * dim.x;
        //             let i_pad = a.minimum[0]
        //                 + a.minimum[1] * dim_pad.y
        //                 + a.minimum[2] * dim_pad.y * dim_pad.x;
        //             VoxelFace {
        //                 index: i,
        //                 color: slice[i_pad as usize],
        //             }
        //         })
        //         .collect()
        // });
        Self { /*faces*/ }
    }
}

