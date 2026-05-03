// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use std::sync::Arc;
use tracing::info;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::VulkanLibrary;

pub struct VulkanEngine {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<ComputePipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

mod shaders {
    vulkano_shaders::shader! {
        ty: "compute",
        src: r#"
            #version 450

            layout(local_size_x = 8, local_size_y = 8) in;

            layout(set = 0, binding = 0, rgba8) uniform readonly image2D inputImage;
            layout(set = 0, binding = 1) buffer OutputBuffer {
                float data[];
            } outputBuffer;

            layout(push_constant) uniform PushConstants {
                uint width;
                uint height;
                uint targetWidth;
                uint targetHeight;
            } pcs;

            void main() {
                uint tx = gl_GlobalInvocationID.x;
                uint ty = gl_GlobalInvocationID.y;

                if (tx >= pcs.targetWidth || ty >= pcs.targetHeight) {
                    return;
                }

                // Simple sampling (nearest neighbor)
                float srcX = float(tx) * float(pcs.width) / float(pcs.targetWidth);
                float srcY = float(ty) * float(pcs.height) / float(pcs.targetHeight);

                vec4 color = imageLoad(inputImage, ivec2(int(srcX), int(srcY)));

                // Output is NCHW
                uint baseIdx = ty * pcs.targetWidth + tx;
                uint planeSize = pcs.targetWidth * pcs.targetHeight;

                outputBuffer.data[baseIdx] = color.r;
                outputBuffer.data[planeSize + baseIdx] = color.g;
                outputBuffer.data[planeSize * 2 + baseIdx] = color.b;
            }
        "#
    }
}

impl VulkanEngine {
    pub fn new() -> Option<Self> {
        info!("--- VULKAN ENGINE: Initializing iGPU Parallel Compute ---");

        let library = VulkanLibrary::new().ok()?;
        let instance = Instance::new(library, InstanceCreateInfo::default()).ok()?;

        let physical_device = instance.enumerate_physical_devices().ok()?.find(|p| {
            p.properties().device_type == PhysicalDeviceType::IntegratedGpu
                || p.properties().device_type == PhysicalDeviceType::DiscreteGpu
        })?;

        info!(
            "--- VULKAN ENGINE: Using Device: {} ---",
            physical_device.properties().device_name
        );

        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(_i, q)| q.queue_flags.contains(QueueFlags::COMPUTE))?
            as u32;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .ok()?;

        let queue = queues.next()?;
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let shader = shaders::load(device.clone()).ok()?;
        let entry_point = shader.entry_point("main").unwrap();

        let stage = PipelineShaderStageCreateInfo::new(entry_point);
        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(device.clone())
                .ok()?,
        )
        .ok()?;

        let pipeline = ComputePipeline::new(
            device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .ok()?;

        Some(Self {
            device,
            queue,
            pipeline,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
        })
    }

    pub fn preprocess(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
        target_w: u32,
        target_h: u32,
    ) -> Option<Vec<f32>> {
        let image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .ok()?;

        let staging_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            rgba_data.iter().cloned(),
        )
        .ok()?;

        let view = ImageView::new_default(image.clone()).ok()?;

        let output_size = (target_w * target_h * 3) as usize;
        let output_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            (0..output_size).map(|_| 0.0f32),
        )
        .ok()?;

        let layout = self.pipeline.layout().set_layouts().first().unwrap();
        let descriptor_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [
                WriteDescriptorSet::image_view(0, view),
                WriteDescriptorSet::buffer(1, output_buffer.clone()),
            ],
            [],
        )
        .ok()?;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .ok()?;

        let push_constants = shaders::PushConstants {
            width,
            height,
            targetWidth: target_w,
            targetHeight: target_h,
        };

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging_buffer,
                image.clone(),
            ))
            .ok()?
            .bind_pipeline_compute(self.pipeline.clone())
            .ok()?
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .ok()?
            .push_constants(self.pipeline.layout().clone(), 0, push_constants)
            .ok()?
            .dispatch([target_w.div_ceil(8), target_h.div_ceil(8), 1])
            .ok()?;

        let command_buffer = builder.build().ok()?;

        let future = sync::now(self.device.clone())
            .then_execute(self.queue.clone(), command_buffer)
            .ok()?
            .then_signal_fence_and_flush()
            .ok()?;

        future.wait(None).ok()?;

        let read_content = output_buffer.read().ok()?;
        Some(read_content.to_vec())
    }
}
