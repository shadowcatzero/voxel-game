mod command;
pub mod voxel;
use std::sync::Arc;

pub use command::*;
use vulkano::{
    device::{Device, DeviceCreateInfo, QueueCreateInfo, QueueFlags}, instance::{Instance, InstanceCreateInfo}, memory::allocator::StandardMemoryAllocator, VulkanLibrary
};

use super::camera::Camera;
use crate::client::rsc::CLEAR_COLOR;
use nalgebra::Vector2;
use voxel::VoxelPipeline;
use winit::{dpi::PhysicalSize, window::Window};

pub struct Renderer {
    camera: Camera,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let instance = Instance::new(library, InstanceCreateInfo::default())
            .expect("failed to create instance");
        let physical_device = instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .next()
            .expect("no devices available");
        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(_queue_family_index, queue_family_properties)| {
                queue_family_properties
                    .queue_flags
                    .contains(QueueFlags::GRAPHICS)
            })
            .expect("couldn't find a graphical queue family")
            as u32;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        Self {
            camera: Camera::default(),
            size: Vector2::new(size.width, size.height),
            voxel_pipeline: VoxelPipeline::new(&device, &config),
        }
    }

    pub fn reset_shader(&mut self) {
        todo!()
    }

    pub fn update_shader(&mut self) {
        todo!()
    }

    pub fn draw(&mut self) {}

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = Vector2::new(size.width, size.height);
        todo!();
    }
}
