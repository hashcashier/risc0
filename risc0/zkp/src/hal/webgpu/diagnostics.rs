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

//! Backend usage counters exposed to the browser harness.

#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use super::{device::*, dispatch::*, eval_check::*, kernels_wgsl::*, ops::*, resources::*};

/// Snapshot of WebGPU HAL backend usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuDiagnostics {
    pub buffers_allocated: u64,
    pub bytes_allocated: u64,
    pub host_to_gpu_uploads: u64,
    pub host_to_gpu_bytes: u64,
    pub device_copies: u64,
    pub device_copy_bytes: u64,
    pub readbacks: u64,
    pub readback_bytes: u64,
    pub bind_group_layout_creations: u64,
    pub bind_group_layout_cache_hits: u64,
    pub bind_group_creations: u64,
    pub compute_pipeline_creations: u64,
    pub compute_pipeline_cache_hits: u64,
    pub gpu_dispatches: u64,
    pub raw_compute_dispatches: u64,
    pub queue_submits: u64,
    pub cpu_mirrors: u64,
    pub cpu_fallbacks: u64,
    pub cpu_only_ops: u64,
    pub stages: Vec<WebGpuStageDiagnostics>,
    pub ops: Vec<WebGpuOpDiagnostics>,
    pub upload_sources: Vec<WebGpuUploadDiagnostics>,
    pub device_copy_sources: Vec<WebGpuDeviceCopyDiagnostics>,
    pub readback_sources: Vec<WebGpuReadbackDiagnostics>,
}

/// Per-stage browser WebGPU timing diagnostics.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuStageDiagnostics {
    pub label: String,
    pub elapsed_us: u64,
    pub gpu_active: bool,
}

/// Per-operation WebGPU HAL backend usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuOpDiagnostics {
    pub name: &'static str,
    pub gpu_dispatches: u64,
    pub cpu_mirrors: u64,
    pub cpu_fallbacks: u64,
    pub cpu_only_ops: u64,
}

/// Per-buffer WebGPU host-to-device upload usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuUploadDiagnostics {
    pub name: &'static str,
    pub uploads: u64,
    pub upload_bytes: u64,
}

/// Per-buffer WebGPU device-to-device copy usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuDeviceCopyDiagnostics {
    pub name: &'static str,
    pub device_copies: u64,
    pub device_copy_bytes: u64,
}

/// Per-buffer WebGPU readback usage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WebGpuReadbackDiagnostics {
    pub name: &'static str,
    pub readbacks: u64,
    pub readback_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WebGpuOpStats {
    pub(crate) gpu_dispatches: u64,
    pub(crate) cpu_mirrors: u64,
    pub(crate) cpu_fallbacks: u64,
    pub(crate) cpu_only_ops: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WebGpuUploadStats {
    pub(crate) uploads: u64,
    pub(crate) upload_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WebGpuDeviceCopyStats {
    pub(crate) device_copies: u64,
    pub(crate) device_copy_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WebGpuReadbackStats {
    pub(crate) readbacks: u64,
    pub(crate) readback_bytes: u64,
}

#[derive(Debug, Default)]
pub(crate) struct WebGpuDiagnosticsState {
    pub(crate) buffers_allocated: Cell<u64>,
    pub(crate) bytes_allocated: Cell<u64>,
    pub(crate) host_to_gpu_uploads: Cell<u64>,
    pub(crate) host_to_gpu_bytes: Cell<u64>,
    pub(crate) device_copies: Cell<u64>,
    pub(crate) device_copy_bytes: Cell<u64>,
    pub(crate) readbacks: Cell<u64>,
    pub(crate) readback_bytes: Cell<u64>,
    pub(crate) bind_group_layout_creations: Cell<u64>,
    pub(crate) bind_group_layout_cache_hits: Cell<u64>,
    pub(crate) bind_group_creations: Cell<u64>,
    pub(crate) compute_pipeline_creations: Cell<u64>,
    pub(crate) compute_pipeline_cache_hits: Cell<u64>,
    pub(crate) gpu_dispatches: Cell<u64>,
    pub(crate) raw_compute_dispatches: Cell<u64>,
    pub(crate) queue_submits: Cell<u64>,
    pub(crate) cpu_mirrors: Cell<u64>,
    pub(crate) cpu_fallbacks: Cell<u64>,
    pub(crate) cpu_only_ops: Cell<u64>,
    pub(crate) stages: Rc<RefCell<Vec<WebGpuStageDiagnostics>>>,
    pub(crate) ops: RefCell<BTreeMap<&'static str, WebGpuOpStats>>,
    pub(crate) upload_sources: RefCell<BTreeMap<&'static str, WebGpuUploadStats>>,
    pub(crate) device_copy_sources: RefCell<BTreeMap<&'static str, WebGpuDeviceCopyStats>>,
    pub(crate) readback_sources: RefCell<BTreeMap<&'static str, WebGpuReadbackStats>>,
}

impl WebGpuDiagnosticsState {
    pub(crate) fn snapshot(&self) -> WebGpuDiagnostics {
        WebGpuDiagnostics {
            buffers_allocated: self.buffers_allocated.get(),
            bytes_allocated: self.bytes_allocated.get(),
            host_to_gpu_uploads: self.host_to_gpu_uploads.get(),
            host_to_gpu_bytes: self.host_to_gpu_bytes.get(),
            device_copies: self.device_copies.get(),
            device_copy_bytes: self.device_copy_bytes.get(),
            readbacks: self.readbacks.get(),
            readback_bytes: self.readback_bytes.get(),
            bind_group_layout_creations: self.bind_group_layout_creations.get(),
            bind_group_layout_cache_hits: self.bind_group_layout_cache_hits.get(),
            bind_group_creations: self.bind_group_creations.get(),
            compute_pipeline_creations: self.compute_pipeline_creations.get(),
            compute_pipeline_cache_hits: self.compute_pipeline_cache_hits.get(),
            gpu_dispatches: self.gpu_dispatches.get(),
            raw_compute_dispatches: self.raw_compute_dispatches.get(),
            queue_submits: self.queue_submits.get(),
            cpu_mirrors: self.cpu_mirrors.get(),
            cpu_fallbacks: self.cpu_fallbacks.get(),
            cpu_only_ops: self.cpu_only_ops.get(),
            stages: self.stages.borrow().clone(),
            ops: self
                .ops
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuOpDiagnostics {
                    name,
                    gpu_dispatches: stats.gpu_dispatches,
                    cpu_mirrors: stats.cpu_mirrors,
                    cpu_fallbacks: stats.cpu_fallbacks,
                    cpu_only_ops: stats.cpu_only_ops,
                })
                .collect(),
            upload_sources: self
                .upload_sources
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuUploadDiagnostics {
                    name,
                    uploads: stats.uploads,
                    upload_bytes: stats.upload_bytes,
                })
                .collect(),
            device_copy_sources: self
                .device_copy_sources
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuDeviceCopyDiagnostics {
                    name,
                    device_copies: stats.device_copies,
                    device_copy_bytes: stats.device_copy_bytes,
                })
                .collect(),
            readback_sources: self
                .readback_sources
                .borrow()
                .iter()
                .map(|(&name, stats)| WebGpuReadbackDiagnostics {
                    name,
                    readbacks: stats.readbacks,
                    readback_bytes: stats.readback_bytes,
                })
                .collect(),
        }
    }

    pub(crate) fn reset(&self) {
        self.buffers_allocated.set(0);
        self.bytes_allocated.set(0);
        self.host_to_gpu_uploads.set(0);
        self.host_to_gpu_bytes.set(0);
        self.device_copies.set(0);
        self.device_copy_bytes.set(0);
        self.readbacks.set(0);
        self.readback_bytes.set(0);
        self.bind_group_layout_creations.set(0);
        self.bind_group_layout_cache_hits.set(0);
        self.bind_group_creations.set(0);
        self.compute_pipeline_creations.set(0);
        self.compute_pipeline_cache_hits.set(0);
        self.gpu_dispatches.set(0);
        self.raw_compute_dispatches.set(0);
        self.queue_submits.set(0);
        self.cpu_mirrors.set(0);
        self.cpu_fallbacks.set(0);
        self.cpu_only_ops.set(0);
        self.stages.borrow_mut().clear();
        self.ops.borrow_mut().clear();
        self.upload_sources.borrow_mut().clear();
        self.device_copy_sources.borrow_mut().clear();
        self.readback_sources.borrow_mut().clear();
    }

    pub(crate) fn add(cell: &Cell<u64>, value: u64) {
        cell.set(cell.get().saturating_add(value));
    }

    pub(crate) fn record_op(&self, name: &'static str, update: impl FnOnce(&mut WebGpuOpStats)) {
        let mut ops = self.ops.borrow_mut();
        update(ops.entry(name).or_default());
    }

    pub(crate) fn record_buffer_allocated(&self, byte_len: u64) {
        Self::add(&self.buffers_allocated, 1);
        Self::add(&self.bytes_allocated, byte_len);
    }

    pub(crate) fn record_upload(&self, name: &'static str, byte_len: u64) {
        Self::add(&self.host_to_gpu_uploads, 1);
        Self::add(&self.host_to_gpu_bytes, byte_len);
        let mut uploads = self.upload_sources.borrow_mut();
        let stats = uploads.entry(name).or_default();
        stats.uploads = stats.uploads.saturating_add(1);
        stats.upload_bytes = stats.upload_bytes.saturating_add(byte_len);
    }

    pub(crate) fn record_device_copy(&self, name: &'static str, byte_len: u64) {
        Self::add(&self.device_copies, 1);
        Self::add(&self.device_copy_bytes, byte_len);
        let mut copies = self.device_copy_sources.borrow_mut();
        let stats = copies.entry(name).or_default();
        stats.device_copies = stats.device_copies.saturating_add(1);
        stats.device_copy_bytes = stats.device_copy_bytes.saturating_add(byte_len);
    }

    pub(crate) fn record_readback(&self, name: &'static str, byte_len: u64) {
        Self::add(&self.readbacks, 1);
        Self::add(&self.readback_bytes, byte_len);
        let mut readbacks = self.readback_sources.borrow_mut();
        let stats = readbacks.entry(name).or_default();
        stats.readbacks = stats.readbacks.saturating_add(1);
        stats.readback_bytes = stats.readback_bytes.saturating_add(byte_len);
    }

    pub(crate) fn record_bind_group_layout_creation(&self) {
        Self::add(&self.bind_group_layout_creations, 1);
    }

    pub(crate) fn record_bind_group_layout_cache_hit(&self) {
        Self::add(&self.bind_group_layout_cache_hits, 1);
    }

    pub(crate) fn record_bind_group_creation(&self) {
        Self::add(&self.bind_group_creations, 1);
    }

    pub(crate) fn record_compute_pipeline_creation(&self) {
        Self::add(&self.compute_pipeline_creations, 1);
    }

    pub(crate) fn record_compute_pipeline_cache_hit(&self) {
        Self::add(&self.compute_pipeline_cache_hits, 1);
    }

    pub(crate) fn record_gpu_dispatch(&self, name: &'static str) {
        Self::add(&self.gpu_dispatches, 1);
        self.record_op(name, |stats| {
            stats.gpu_dispatches = stats.gpu_dispatches.saturating_add(1);
        });
    }

    pub(crate) fn record_raw_compute_dispatch(&self) {
        Self::add(&self.raw_compute_dispatches, 1);
    }

    pub(crate) fn record_queue_submit(&self) {
        Self::add(&self.queue_submits, 1);
    }

    pub(crate) fn stage_diagnostics_handle(&self) -> Rc<RefCell<Vec<WebGpuStageDiagnostics>>> {
        self.stages.clone()
    }

    pub(crate) fn record_cpu_mirror(&self, name: &'static str) {
        Self::add(&self.cpu_mirrors, 1);
        self.record_op(name, |stats| {
            stats.cpu_mirrors = stats.cpu_mirrors.saturating_add(1);
        });
    }

    pub(crate) fn record_cpu_fallback(&self, name: &'static str) {
        Self::add(&self.cpu_fallbacks, 1);
        self.record_op(name, |stats| {
            stats.cpu_fallbacks = stats.cpu_fallbacks.saturating_add(1);
        });
    }
}
