use bytemuck::Pod;

use crate::render::next_power_of_2;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BufferCacheId {
    index: usize,
    vertex: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferChunk {
    pub n: usize,
    pub start: usize,
    pub max_size: usize,
    // Is this chunk written to the buffer? Used to determine if the wgpu::Buffer needs to be recreated
    pub filled: bool,
    // Chunks are unmarked at the start of a render pass and marked as each renderable renders to them
    // Chunks that remain unmarked at the end of the pass are free to be claimed for new renderables
    pub marked: bool,
}

pub struct BufferCache<V, I> {
    pub vertex_buffer_chunks: Vec<BufferChunk>,
    pub index_buffer_chunks: Vec<BufferChunk>,
    pub vertex_data: Vec<V>,
    pub index_data: Vec<I>,
}

impl<T: Default + Pod, I: Default + Pod> BufferCache<T, I> {
    pub fn new() -> Self {
        Self {
            vertex_buffer_chunks: Default::default(),
            index_buffer_chunks: Default::default(),
            vertex_data: Default::default(),
            index_data: Default::default(),
        }
    }

    pub fn unmark(&mut self) {
        for c in self.vertex_buffer_chunks.iter_mut() {
            c.marked = false;
        }
        for c in self.index_buffer_chunks.iter_mut() {
            c.marked = false;
        }
    }

    pub fn register(&mut self, chunk: BufferCacheId) {
        self.vertex_buffer_chunks[chunk.vertex].marked = true;
        self.index_buffer_chunks[chunk.index].marked = true;
    }

    fn _alloc_chunk<U: Default>(
        buffer_chunks: &mut Vec<BufferChunk>,
        data: &mut Vec<U>,
        n: usize,
    ) -> usize {
        let target_size = next_power_of_2(n);

        if let Some(i) = buffer_chunks
            .iter()
            .position(|i| !i.marked && i.max_size == target_size)
        {
            let chunk = &mut buffer_chunks[i];
            chunk.n = n;
            chunk.filled = false;
            chunk.marked = true;
            i
        } else {
            let length = data.len() + target_size;
            data.resize_with(length, Default::default);
            buffer_chunks.push(BufferChunk {
                n,
                max_size: target_size,
                start: buffer_chunks
                    .last()
                    .map(|c| c.start + c.max_size)
                    .unwrap_or(0),
                filled: false,
                marked: true,
            });

            buffer_chunks.len() - 1
        }
    }

    pub fn alloc_chunk(&mut self, n_vertex: usize, n_index: usize) -> BufferCacheId {
        let vertex = Self::_alloc_chunk(
            &mut self.vertex_buffer_chunks,
            &mut self.vertex_data,
            n_vertex,
        );
        let index =
            Self::_alloc_chunk(&mut self.index_buffer_chunks, &mut self.index_data, n_index);
        BufferCacheId { vertex, index }
    }

    pub fn alloc_or_reuse_chunk(
        &mut self,
        buffer_cache: BufferCacheId,
        n_vertex: usize,
        n_index: usize,
    ) -> BufferCacheId {
        if n_vertex <= self.vertex_buffer_chunks[buffer_cache.vertex].max_size
            && n_index <= self.index_buffer_chunks[buffer_cache.index].max_size
        {
            self.vertex_buffer_chunks[buffer_cache.vertex].filled = false;
            self.vertex_buffer_chunks[buffer_cache.vertex].n = n_vertex;
            self.index_buffer_chunks[buffer_cache.index].filled = false;
            self.index_buffer_chunks[buffer_cache.index].n = n_index;
            buffer_cache
        } else {
            self.alloc_chunk(n_vertex, n_index)
        }
    }

    pub fn set_n_indices(&mut self, buffer_cache: BufferCacheId, n_index: usize) {
        self.index_buffer_chunks[buffer_cache.index].n = n_index;
    }

    pub fn get_chunks(&self, buffer_cache: BufferCacheId) -> (BufferChunk, BufferChunk) {
        (
            self.vertex_buffer_chunks[buffer_cache.vertex],
            self.index_buffer_chunks[buffer_cache.index],
        )
    }

    pub fn fill_chunks(&mut self, buffer_cache: BufferCacheId) {
        self.vertex_buffer_chunks[buffer_cache.vertex].filled = true;
        self.index_buffer_chunks[buffer_cache.index].filled = true;
    }
}
