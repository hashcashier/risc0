// Copyright 2026 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tiled multi-buffer source representation for gather operations.
//!
//! For recursion-sized data groups the logical 2D matrix (`cols × stride`
//! elements, column-major) can exceed the WebGPU
//! `maxStorageBufferBindingSize`. A single `WebGpuBuffer` can't bind for
//! kernel reads in that case. `BufferPool` splits the matrix across N
//! GPU buffers — each holding a contiguous slab of columns — and
//! `dispatch_gather_sample_tiled` iterates over
//! the tiles, binding one buffer at a time, to replace the locked CPU
//! fallback that previously fired on these sizes.
//!
//! `TileLayout` carries the pure addressing math (chosen `tile_cols` for
//! a given `stride` + `total_cols` + `max_binding_bytes`) and has no
//! GPU dependency — it can be unit-tested without a WebGPU device. The
//! actual `BufferPool` struct wraps a
//! `Vec<web_sys::GpuBuffer>` keyed by `TileLayout`.

use std::mem;

use anyhow::{anyhow, ensure, Result};

use super::{dispatch::byte_len_for, WebGpuHal};
// `size` and `view` are on the `super::super::Buffer` trait, not
// inherent methods on `WebGpuBuffer`. Bring the trait into scope.
use super::super::Buffer as _;

/// Layout describing how a logical 2D matrix (`total_cols × stride`
/// elements, column-major) is split across tile buffers. Each tile
/// holds `tile_cols` consecutive columns; the last tile may hold fewer
/// if `total_cols` doesn't divide evenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileLayout {
    /// Number of elements in one column (the runtime-interpreter's
    /// `stride` parameter for `gather_sample`).
    pub stride: usize,
    /// Number of columns packed into each tile buffer.
    pub tile_cols: usize,
    /// Total number of columns across all tiles.
    pub total_cols: usize,
}

impl TileLayout {
    /// Pick `tile_cols` so each tile's byte size (`tile_cols * stride *
    /// elem_size`) fits in `max_binding_bytes`. Returns `Err` if even a
    /// single column doesn't fit.
    ///
    /// Empty inputs (`stride == 0` or `total_cols == 0`) produce a
    /// degenerate one-tile layout — the caller can short-circuit on
    /// `total_cols == 0` and skip the pool entirely.
    pub fn new(
        stride: usize,
        total_cols: usize,
        elem_size: usize,
        max_binding_bytes: u64,
    ) -> Result<Self> {
        if elem_size == 0 {
            return Err(anyhow!("TileLayout: elem_size must be non-zero"));
        }
        if max_binding_bytes == 0 {
            return Err(anyhow!("TileLayout: max_binding_bytes must be non-zero"));
        }
        if stride == 0 || total_cols == 0 {
            return Ok(Self {
                stride,
                tile_cols: total_cols.max(1),
                total_cols,
            });
        }
        let col_bytes = (stride as u64)
            .checked_mul(elem_size as u64)
            .ok_or_else(|| anyhow!("TileLayout: column byte count overflows u64"))?;
        if col_bytes > max_binding_bytes {
            return Err(anyhow!(
                "TileLayout: single column ({col_bytes} B at stride={stride}) exceeds max binding ({max_binding_bytes} B)"
            ));
        }
        // Largest tile_cols such that tile_cols * col_bytes <= max_binding_bytes.
        let tile_cols = (max_binding_bytes / col_bytes) as usize;
        let tile_cols = tile_cols.min(total_cols).max(1);
        Ok(Self {
            stride,
            tile_cols,
            total_cols,
        })
    }

    /// Number of tile buffers needed to cover all columns.
    pub fn num_tiles(&self) -> usize {
        if self.total_cols == 0 {
            return 0;
        }
        self.total_cols.div_ceil(self.tile_cols)
    }

    /// Which tile contains the given column index.
    pub fn tile_for_col(&self, col: usize) -> usize {
        debug_assert!(
            col < self.total_cols,
            "col {col} >= total_cols {}",
            self.total_cols
        );
        col / self.tile_cols
    }

    /// Column index relative to the start of its tile.
    pub fn col_within_tile(&self, col: usize) -> usize {
        col % self.tile_cols
    }

    /// Number of columns actually stored in tile `tile_idx`. The last
    /// tile may be smaller than `tile_cols` if `total_cols` doesn't
    /// divide evenly.
    pub fn cols_in_tile(&self, tile_idx: usize) -> usize {
        let start = tile_idx.saturating_mul(self.tile_cols);
        if start >= self.total_cols {
            return 0;
        }
        (self.total_cols - start).min(self.tile_cols)
    }

    /// Number of elements in tile `tile_idx`'s backing buffer.
    pub fn elems_in_tile(&self, tile_idx: usize) -> usize {
        self.cols_in_tile(tile_idx).saturating_mul(self.stride)
    }

    /// Number of elements in the source the FIRST `col_within_tile`
    /// columns of tile `tile_idx` skip over. Useful for computing the
    /// `src_base` u32 a per-tile gather kernel writes into its params
    /// UBO (interpreter-side `dispatch_gather_sample_chunked` does the
    /// analogous `col_start * stride` calculation).
    pub fn col_offset_within_tile(&self, col_within_tile: usize) -> usize {
        col_within_tile.saturating_mul(self.stride)
    }
}

/// Tiled multi-buffer source representation. Each tile
/// buffer holds `cols_in_tile(tile_idx) * layout.stride` elements
/// (column-major, contiguous). Used by
/// `WebGpuHal::dispatch_gather_sample_tiled` to gather a sample row
/// across the full logical matrix when the matrix itself can't fit in
/// a single `maxStorageBufferBindingSize` binding.
pub struct BufferPool {
    pub layout: TileLayout,
    pub buffers: Vec<web_sys::GpuBuffer>,
    pub name: &'static str,
}

impl BufferPool {
    /// Allocate `layout.num_tiles()` per-tile GPU storage buffers
    /// sized to `cols_in_tile(t) * stride * elem_size` bytes each.
    /// Buffers are zero-initialized; the caller populates them via
    /// `upload_tile_from_cpu_slice` (test helper) or by dispatching
    /// kernels that write per-tile output (production: a future
    /// `BufferPool`-aware `make_coeffs_async` variant).
    pub fn new<T>(hal: &WebGpuHal, name: &'static str, layout: TileLayout) -> Result<Self> {
        let elem_size = mem::size_of::<T>();
        ensure!(elem_size > 0, "BufferPool::new: T must be sized");
        let mut buffers = Vec::with_capacity(layout.num_tiles());
        for tile_idx in 0..layout.num_tiles() {
            let elems = layout.elems_in_tile(tile_idx).max(1);
            let buffer = hal.create_storage_buffer(name, byte_len_for::<T>(elems))?;
            buffers.push(buffer);
        }
        Ok(Self {
            layout,
            buffers,
            name,
        })
    }

    /// Number of tile buffers backing this pool.
    pub fn num_tiles(&self) -> usize {
        self.buffers.len()
    }

    /// Borrow tile buffer `tile_idx`. Panics if out of range.
    pub fn tile_buffer(&self, tile_idx: usize) -> &web_sys::GpuBuffer {
        &self.buffers[tile_idx]
    }

    /// Build a `BufferPool` for `total_cols` columns of
    /// `stride` elements at type `T`, sized per tile to fit
    /// `max_binding_bytes`, and populate it from a CPU-current
    /// `WebGpuBuffer<T>`. The source's CPU shadow is read directly
    /// (the buffer must be CPU-current — call `sync_gpu_to_cpu`
    /// first if it's been touched on GPU). The byte encoding is
    /// taken as-is — `T` must be a plain-old-data type whose
    /// in-memory representation matches what kernels expect to read
    /// from the storage buffer (BabyBearElem stores its raw u32
    /// directly; no Montgomery conversion needed because the kernel
    /// shaders also read raw u32s).
    pub fn from_webgpu_buffer(
        hal: &WebGpuHal,
        name: &'static str,
        src: &super::WebGpuBuffer<risc0_core::field::baby_bear::BabyBearElem>,
        stride: usize,
        total_cols: usize,
        max_binding_bytes: u64,
    ) -> Result<Self> {
        let elem_size = mem::size_of::<risc0_core::field::baby_bear::BabyBearElem>();
        ensure!(
            src.size() == stride * total_cols,
            "BufferPool::from_webgpu_buffer: src.size()={} != stride={stride} * total_cols={total_cols}",
            src.size(),
        );
        let layout = TileLayout::new(stride, total_cols, elem_size, max_binding_bytes)?;
        let pool = Self::new::<risc0_core::field::baby_bear::BabyBearElem>(hal, name, layout)?;
        // `view` takes a closure returning `()`; route any upload
        // errors through an `Option<anyhow::Error>` captured by the
        // closure and surface after `view` returns.
        let mut err: Option<anyhow::Error> = None;
        src.view(|cpu| {
            for tile_idx in 0..pool.num_tiles() {
                let col_start = tile_idx * pool.layout.tile_cols;
                let cols = pool.layout.cols_in_tile(tile_idx);
                let elem_start = col_start * pool.layout.stride;
                let elem_end = elem_start + cols * pool.layout.stride;
                let tile_slice = &cpu[elem_start..elem_end];
                // Cast the BabyBearElem slice to its raw u32 bytes.
                let bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        tile_slice.as_ptr() as *const u8,
                        std::mem::size_of_val(tile_slice),
                    )
                };
                if let Err(e) = hal.write_buffer_named(&pool.buffers[tile_idx], pool.name, 0, bytes)
                {
                    err = Some(e);
                    return;
                }
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        Ok(pool)
    }

    /// Test/setup helper: populate the entire pool from a contiguous
    /// column-major CPU byte slice (`cpu_bytes[col * stride * elem_size
    /// .. (col+1) * stride * elem_size]` for each column). Sliced
    /// into per-tile uploads. Used by the browser-prove regression
    /// test (`webgpu_hal_recursion_sized_gather_sample_uses_buffer_pool`)
    /// and by production callers that build the recursion data
    /// group from a CPU-side staging buffer.
    ///
    /// `elem_size` must match the `T` used when calling `Self::new`.
    /// Callers can build `cpu_bytes` via `bytemuck::cast_slice` over
    /// their elem slice, or any other byte-equivalent encoding.
    #[doc(hidden)]
    pub fn upload_from_cpu_bytes(
        &self,
        hal: &WebGpuHal,
        elem_size: usize,
        cpu_bytes: &[u8],
    ) -> Result<()> {
        let expected_bytes = self
            .layout
            .total_cols
            .checked_mul(self.layout.stride)
            .and_then(|n| n.checked_mul(elem_size))
            .ok_or_else(|| anyhow!("BufferPool::upload: total bytes overflow"))?;
        ensure!(
            cpu_bytes.len() == expected_bytes,
            "BufferPool::upload size mismatch: cpu={} expected={expected_bytes}",
            cpu_bytes.len(),
        );
        for tile_idx in 0..self.num_tiles() {
            let col_start = tile_idx * self.layout.tile_cols;
            let cols = self.layout.cols_in_tile(tile_idx);
            let byte_start = col_start * self.layout.stride * elem_size;
            let byte_end = byte_start + cols * self.layout.stride * elem_size;
            hal.write_buffer_named(
                &self.buffers[tile_idx],
                self.name,
                0,
                &cpu_bytes[byte_start..byte_end],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    const ELEM_SIZE: usize = 4; // BabyBearElem
    const MAX_1G: u64 = 1024 * 1024 * 1024;
    const MAX_128M: u64 = 128 * 1024 * 1024;

    #[test]
    fn single_tile_when_matrix_fits() {
        // 128 cols × 1M rows × 4 B = 512 MiB, fits in 1 GiB binding.
        let layout = TileLayout::new(1 << 20, 128, ELEM_SIZE, MAX_1G).unwrap();
        assert_eq!(layout.tile_cols, 128);
        assert_eq!(layout.num_tiles(), 1);
        assert_eq!(layout.cols_in_tile(0), 128);
        assert_eq!(layout.elems_in_tile(0), 128 << 20);
    }

    #[test]
    fn splits_into_multiple_tiles_when_matrix_exceeds_binding() {
        // 128 cols × 1M rows × 4 B = 512 MiB; 128 MiB binding allows
        // 128 MiB / 4 MiB-per-col = 32 cols per tile → 4 tiles total.
        let layout = TileLayout::new(1 << 20, 128, ELEM_SIZE, MAX_128M).unwrap();
        assert_eq!(layout.tile_cols, 32);
        assert_eq!(layout.num_tiles(), 4);
        for t in 0..4 {
            assert_eq!(layout.cols_in_tile(t), 32);
            assert_eq!(layout.elems_in_tile(t), 32 << 20);
        }
    }

    #[test]
    fn last_tile_shorter_when_total_cols_doesnt_divide() {
        // 100 cols, 32 per tile → 3 full tiles + 1 partial tile of 4.
        let layout = TileLayout::new(1 << 20, 100, ELEM_SIZE, MAX_128M).unwrap();
        assert_eq!(layout.tile_cols, 32);
        assert_eq!(layout.num_tiles(), 4);
        assert_eq!(layout.cols_in_tile(0), 32);
        assert_eq!(layout.cols_in_tile(1), 32);
        assert_eq!(layout.cols_in_tile(2), 32);
        assert_eq!(layout.cols_in_tile(3), 4);
    }

    #[test]
    fn col_to_tile_addressing_roundtrip() {
        let layout = TileLayout::new(1 << 20, 100, ELEM_SIZE, MAX_128M).unwrap();
        // tile_cols = 32. col 0 → tile 0, within=0. col 31 → tile 0, within=31.
        // col 32 → tile 1, within=0. col 99 → tile 3, within=3.
        for col in 0..100 {
            let tile = layout.tile_for_col(col);
            let within = layout.col_within_tile(col);
            assert_eq!(tile * layout.tile_cols + within, col);
        }
    }

    #[test]
    fn single_oversized_column_returns_err() {
        // 1 col × 1G rows × 4 B = 4 GiB > 1 GiB binding limit.
        let result = TileLayout::new(1 << 30, 1, ELEM_SIZE, MAX_1G);
        assert!(
            result.is_err(),
            "expected single-col-too-large to be rejected"
        );
    }

    #[test]
    fn zero_cols_produces_empty_layout() {
        let layout = TileLayout::new(1024, 0, ELEM_SIZE, MAX_1G).unwrap();
        assert_eq!(layout.num_tiles(), 0);
        assert_eq!(layout.cols_in_tile(0), 0);
        assert_eq!(layout.elems_in_tile(0), 0);
    }

    #[test]
    fn zero_stride_clamps_tile_cols_to_at_least_one() {
        let layout = TileLayout::new(0, 5, ELEM_SIZE, MAX_1G).unwrap();
        assert_eq!(layout.tile_cols, 5);
        assert_eq!(layout.num_tiles(), 1);
    }

    #[test]
    fn col_offset_within_tile_is_col_index_times_stride() {
        let layout = TileLayout::new(7, 100, ELEM_SIZE, MAX_1G).unwrap();
        assert_eq!(layout.col_offset_within_tile(0), 0);
        assert_eq!(layout.col_offset_within_tile(5), 35);
        assert_eq!(layout.col_offset_within_tile(99), 693);
    }
}
