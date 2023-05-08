use bytemuck::{cast_slice, Pod};
use log::info;
use std::sync::{Arc, RwLock};
use wgpu;

use crate::render::next_power_of_2;
use crate::render::renderables::buffer_cache::{BufferCacheId, BufferChunk};

pub struct BufferCache<V, I> {
    pub cache: Arc<RwLock<crate::render::renderables::buffer_cache::BufferCache<V, I>>>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    vertex_buffer_len: usize,
    index_buffer_len: usize,
}

impl<T: Default + Pod, I: Default + Pod> BufferCache<T, I> {
    pub fn new(device: &wgpu::Device) -> Self {
        let cache = Arc::new(RwLock::new(
            crate::render::renderables::buffer_cache::BufferCache::new(),
        ));
        let initial_buffer_size = 32;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<T>() * initial_buffer_size) as u64,
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
            cache,
            vertex_buffer,
            index_buffer,
            vertex_buffer_len: initial_buffer_size,
            index_buffer_len: initial_buffer_size,
        }
    }

    pub fn sync_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.cache.read().unwrap().vertex_data.len() > self.vertex_buffer_len {
            self.vertex_buffer_len = next_power_of_2(self.cache.read().unwrap().vertex_data.len());
            info!(
                "Resizing BufferCache vertex buffer to {}",
                self.vertex_buffer_len
            );

            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (std::mem::size_of::<T>() * self.vertex_buffer_len) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.cache.read().unwrap().index_data.len() > self.index_buffer_len {
            self.index_buffer_len = next_power_of_2(self.cache.read().unwrap().index_data.len());
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
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            cast_slice(&self.cache.read().unwrap().vertex_data),
        );
        queue.write_buffer(
            &self.index_buffer,
            0,
            cast_slice(&self.cache.read().unwrap().index_data),
        );
    }

    pub fn unmark(&mut self) {
        self.cache.write().unwrap().unmark();
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

    pub fn get_chunks(&self, buffer_cache: BufferCacheId) -> (BufferChunk, BufferChunk) {
        self.cache.read().unwrap().get_chunks(buffer_cache)
    }

    // pub fn fill_chunks(&mut self, buffer_cache: BufferCacheId) {
    //     self.cache.write().unwrap().fill_chunks(buffer_cache);
    // }
}
