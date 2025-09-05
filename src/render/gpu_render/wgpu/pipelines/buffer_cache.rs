use std::marker::PhantomData;

use bytemuck::{Pod, cast_slice};
use log::info;
use wgpu;

use crate::render::next_power_of_2;
use crate::render::renderables::{BufferCacheId, BufferChunk};

pub struct BufferCache<V, I> {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    vertex_buffer_len: usize,
    index_buffer_len: usize,
    phantom_data_v: PhantomData<V>,
    phantom_data_i: PhantomData<I>,
}

impl<V: Default + Pod, I: Default + Pod> BufferCache<V, I> {
    pub fn new(device: &wgpu::Device) -> Self {
        let initial_buffer_size = 32;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<V>() * initial_buffer_size) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<I>() * initial_buffer_size) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            vertex_buffer_len: initial_buffer_size,
            index_buffer_len: initial_buffer_size,
            phantom_data_v: PhantomData,
            phantom_data_i: PhantomData,
        }
    }

    pub fn sync_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &crate::render::renderables::BufferCache<V, I>,
    ) {
        if cache.vertex_data.len() > self.vertex_buffer_len {
            self.vertex_buffer_len = next_power_of_2(cache.vertex_data.len());
            info!(
                "Resizing BufferCache vertex buffer to {}",
                self.vertex_buffer_len
            );

            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<V>() * self.vertex_buffer_len) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if cache.index_data.len() > self.index_buffer_len {
            self.index_buffer_len = next_power_of_2(cache.index_data.len());
            info!(
                "Resizing BufferCache index buffer to {}",
                self.index_buffer_len
            );
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<I>() * self.index_buffer_len) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, cast_slice(&cache.vertex_data));
        queue.write_buffer(&self.index_buffer, 0, cast_slice(&cache.index_data));
    }

    pub fn get_chunks(
        &self,
        buffer_cache_id: BufferCacheId,
        cache: &crate::render::renderables::BufferCache<V, I>,
    ) -> (BufferChunk, BufferChunk) {
        cache.get_chunks(buffer_cache_id)
    }

    // pub fn register(&mut self, chunk: BufferCacheId) {
    //     self.cache.write().unwrap().register(chunk);
    // }

    // pub fn alloc_chunk(&mut self, n_vertex: usize, n_index: usize) -> BufferCacheId {
    //     self.cache.write().unwrap().alloc_chunk(n_vertex, n_index)
    // }

    // pub fn alloc_or_reuse_chunk(
    //     &mut self,
    //     buffer_cache: BufferCacheId,
    //     n_vertex: usize,
    //     n_index: usize,
    // ) -> BufferCacheId {
    //     self.cache
    //         .write()
    //         .unwrap()
    //         .alloc_or_reuse_chunk(buffer_cache, n_vertex, n_index)
    // }

    // pub fn set_n_indices(&mut self, buffer_cache: BufferCacheId, n_index: usize) {
    //     self.cache
    //         .write()
    //         .unwrap()
    //         .set_n_indices(buffer_cache, n_index)
    // }

    // pub fn fill_chunks(&mut self, buffer_cache: BufferCacheId) {
    //     self.cache.write().unwrap().fill_chunks(buffer_cache);
    // }
}
