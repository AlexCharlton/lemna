use bytemuck::cast_slice;
use log::info;
use wgpu;

use super::buffer_cache::BufferCache;
use super::shared::{create_pipeline, VBDesc};
use crate::base_types::AABB;
use crate::render::next_power_of_2;
use crate::render::renderables::shape::{Instance, Shape, Vertex};
use crate::render::wgpu::context;

pub struct ShapePipeline {
    pipeline: wgpu::RenderPipeline,
    msaa_pipeline: wgpu::RenderPipeline,
    pub(crate) buffer_cache: BufferCache<Vertex, u16>,
    instance_data: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    num_instances: usize,
}

impl ShapePipeline {
    pub(crate) fn unmark_buffer_cache(&mut self) {
        self.buffer_cache.unmark();
    }

    fn draw_renderables<'a: 'b, 'b>(
        &'a self,
        renderables: &[(&'a Shape, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        msaa: bool,
        instance_offset: usize,
    ) {
        let mut i = 0;
        for (renderable, _) in renderables.iter() {
            let (vertex_chunk, index_chunk) = self.buffer_cache.get_chunks(renderable.buffer_id);

            pass.set_vertex_buffer(
                0,
                self.buffer_cache
                    .vertex_buffer
                    .slice(((vertex_chunk.start * std::mem::size_of::<Vertex>()) as u64)..),
            );
            pass.set_vertex_buffer(
                1,
                self.instance_buffer
                    .slice((((i + instance_offset) * std::mem::size_of::<Instance>()) as u64)..),
            );
            pass.set_index_buffer(
                self.buffer_cache
                    .index_buffer
                    .slice(((index_chunk.start * std::mem::size_of::<u16>()) as u64)..),
                wgpu::IndexFormat::Uint16,
            );
            if renderable.is_filled() {
                pass.draw_indexed(renderable.fill_range.clone(), 0, 0..1);
                i += 1;
            }
            if renderable.is_stroked() {
                // Don't draw stroked lines unless doing the MSAA pass
                if msaa || !cfg!(feature = "msaa_shapes") {
                    let instances = if renderable.is_filled() { 1..2 } else { 0..1 };
                    pass.draw_indexed(renderable.stroke_range.clone(), 0, instances);
                }
                i += 1;
            }
        }
    }

    pub fn alloc_instance_buffer<'a: 'b, 'b>(
        &'a mut self,
        num_instances: usize,
        device: &'b wgpu::Device,
    ) {
        if num_instances > self.num_instances {
            self.num_instances = next_power_of_2(num_instances);
            info!(
                "Resizing ShapePipeline instance buffer to {}",
                self.num_instances
            );
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<Instance>() * self.num_instances) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
    }

    pub fn fill_buffers<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Shape, &'a AABB)],
        device: &'b wgpu::Device,
        queue: &'b mut wgpu::Queue,
    ) {
        self.instance_data.clear();

        let mut cache_changed = false;
        for (renderable, aabb) in renderables {
            self.instance_data
                .extend(renderable.render(aabb, &mut self.buffer_cache.cache.write().unwrap()));
            let (vertex_chunk, _) = self.buffer_cache.get_chunks(renderable.buffer_id);
            cache_changed |= !vertex_chunk.filled;
            // Maybe TODO: Only write chunks that have changed (combining contiguous changes?)
        }

        if cache_changed {
            self.buffer_cache.sync_buffers(device, queue);
        }

        queue.write_buffer(&self.instance_buffer, 0, cast_slice(&self.instance_data));
    }

    pub fn render<'a: 'b, 'b>(
        &'a mut self,
        renderables: &[(&'a Shape, &'a AABB)],
        pass: &'b mut wgpu::RenderPass<'a>,
        instance_offset: usize,
        msaa: bool,
    ) {
        pass.set_pipeline(if msaa {
            &self.msaa_pipeline
        } else {
            &self.pipeline
        });
        self.draw_renderables(renderables, pass, msaa, instance_offset);
    }

    pub fn new(
        context: &context::WGPUContext,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let layout = &context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shape_pipeline_layout"),
                bind_group_layouts: &[uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let num_instances = 32; // Initial allocation
        let instance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<Instance>() * num_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/shape.vert.spv"));
        let fs_module = context
            .device
            .create_shader_module(wgpu::include_spirv!("shaders/vert_color.frag.spv"));

        Self {
            buffer_cache: BufferCache::new(&context.device),
            instance_data: vec![],
            instance_buffer,
            num_instances,
            pipeline: create_pipeline(
                context,
                layout,
                &fs_module,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                false,
                wgpu::ColorWrites::ALL,
            ),
            msaa_pipeline: create_pipeline(
                context,
                layout,
                &fs_module,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                true,
                wgpu::ColorWrites::ALL,
            ),
        }
    }
}
