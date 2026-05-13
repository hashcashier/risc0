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

//! SP4 (R8): tiled multi-buffer source representation for gather operations.
//!
//! For recursion-sized data groups the logical 2D matrix (`cols × stride`
//! elements, column-major) can exceed the WebGPU
//! `maxStorageBufferBindingSize`. A single `WebGpuBuffer` can't bind for
//! kernel reads in that case. `BufferPool` splits the matrix across N
//! GPU buffers — each holding a contiguous slab of columns — and
//! `dispatch_gather_sample_tiled` (added in SP4 iter 2) iterates over
//! the tiles, binding one buffer at a time, to replace the locked CPU
//! fallback that previously fired on these sizes.
//!
//! `TileLayout` carries the pure addressing math (chosen `tile_cols` for
//! a given `stride` + `total_cols` + `max_binding_bytes`) and has no
//! GPU dependency — it can be unit-tested without a WebGPU device. The
//! actual `BufferPool` struct (added in SP4 iter 2) wraps a
//! `Vec<web_sys::GpuBuffer>` keyed by `TileLayout`.

use anyhow::{anyhow, Result};

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
        debug_assert!(col < self.total_cols, "col {col} >= total_cols {}", self.total_cols);
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
        assert!(result.is_err(), "expected single-col-too-large to be rejected");
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
