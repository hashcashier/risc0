Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP9 minimal — cache compute pipelines in WebGpuHal::create_compute_kernel`
Date: 2026-05-15
Status: **FAILED EXPERIMENT — REVERTED.** Pipeline cache produced runtime
"layout doesn't match" errors on the very first hello_world smoke. The
naive design assumed bind-group-layout shape was enough; WebGPU actually
requires bind-group-layout INSTANCE identity to match between the
pipeline and any bind groups dispatched against it. SP9 needs to cache
BOTH the layout AND the pipeline together.

## Attempt

Added `compute_pipeline_cache: RefCell<BTreeMap<(usize, usize), WebGpuKernel>>`
to `WebGpuHal`, keyed by `(wgsl.as_ptr(), entry_point.as_ptr())`. Cache
hit returned a clone of the pre-compiled `WebGpuKernel`.

Build: clean (cargo check + wasm32 target compile both succeeded).

R1 hello_world smoke: failed in `dispatch_zeroize_elem`:
```
type=GPUValidationError msg=Bind group layout
[BindGroupLayout "webgpu_zeroize_elem_layout"] of pipeline layout
[PipelineLayout (unlabeled)] does not match layout
[BindGroupLayout "webgpu_zeroize_elem_layout"] of bind group
[BindGroup "webgpu_zeroize_elem_bind_group"] set at group index 0.
```

The two `BindGroupLayout` instances had the same LABEL but were
different JS objects — created by separate `create_bind_group_layout`
calls per dispatch. WebGPU's pipeline-layout binding is by object
identity; the cached pipeline still referenced the FIRST layout
instance, while the new bind groups for subsequent dispatches were
built from FRESH layout instances. Mismatch.

## Lesson

WebGPU bind-group-layout matching is by object identity, not shape.
Caching `create_compute_kernel` alone is unsound — the cached pipeline
captures a specific layout instance that won't match later bind groups.
A correct SP9 must:
1. Cache `create_bind_group_layout(label, &[entries]) -> GpuBindGroupLayout`
   keyed by (label_ptr, entries-shape). All callers reuse the cached
   instance.
2. THEN cache `create_compute_kernel` keyed by (wgsl, entry, layout
   instances). All callers reuse the cached pipeline.

Step 1 is a bigger refactor — 31 `create_bind_group_layout` sites in
this file, each with a slightly different `&[WebGpuBindingLayout]`
shape. Worth doing if the perf gain is real, but needs careful per-site
audit (some layouts depend on per-call sizes).

## Decision

Reverted both attempts in this commit. Logged here as a failed-experiment
ledger entry per [[feedback_codebase_quality]] / R11 (do not re-enable
without dedicated evidence). Recorded in code as "SP9 was tried
2026-05-15 with (a) pipeline caching — FAILED, layout-instance identity
required; (b) shader-module-only caching — FAILED, RefCell already
borrowed at runtime, root cause not pinned down (current source line is
not a borrow point — debug-info skew vs runtime, or some re-entry path
through `create_shader_module`); revisit only after a careful audit of
the borrow chains and a layout-instance cache on top."

## Second attempt: shader-module-only cache (also failed)

After reverting (a), I tried a narrower scope: cache only the
`GpuShaderModule` returned by `create_shader_module(wgsl)`, leaving
pipeline assembly untouched. Shader modules are pure functions of WGSL
source -- no layout coupling -- so this is theoretically sound.

```rust
let shader_key = wgsl.as_ptr() as usize;
let cached = self.shader_module_cache.borrow().get(&shader_key).cloned();
let shader = if let Some(module) = cached {
    module
} else {
    let shader_desc = web_sys::GpuShaderModuleDescriptor::new(wgsl);
    shader_desc.set_label(label);
    let module = self.device.create_shader_module(&shader_desc);
    self.shader_module_cache.borrow_mut().insert(shader_key, module.clone());
    module
};
```

The `cloned()` pattern is supposed to drop the immutable borrow before
any potential `borrow_mut`. wasm32 build clean. R1 hello_world smoke:
**`panicked at risc0/zkp/src/hal/webgpu.rs:7937:18: RefCell already
borrowed`**. The line that the wasm panic points to is no longer a
borrow point in the current source (it's the `let shader_desc = ...`
literal), suggesting either (i) the wasm wasn't actually rebuilt with
the latest source despite the touch+rebuild dance, OR (ii) some chain
through `create_shader_module` re-enters `create_compute_kernel` while
the `borrow_mut` is held. I couldn't pin the root cause down in the
session-time available; reverted.

Future SP9 work: needs a careful borrow-chain audit before any
caching attempt. Possibly the cache should use `OnceCell`/`Rc<...>`
patterns instead of `RefCell` to avoid borrow conflicts entirely.
