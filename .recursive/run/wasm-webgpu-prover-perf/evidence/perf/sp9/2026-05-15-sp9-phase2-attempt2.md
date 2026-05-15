Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `SP9 phase 2 — pipeline cache (attempt 2, fails for same reason as 61d3163c9)`
Date: 2026-05-15

## Question

Can a `(wgsl.as_ptr(), entry_point.as_ptr())`-keyed pipeline cache
correctly reuse cached pipelines now that phase 1 (layout cache) is
committed (`e129c9728`)?

The hypothesis: phase 1 guarantees the layouts passed to
`create_compute_kernel` are now *stable instances* per shape. If WGSL
+ entry are identical (always true for static `const &str` callers)
and layouts are stable instances, the cached pipeline is reusable.

## Attempt

```rust
compute_pipeline_cache: RefCell<BTreeMap<(usize, usize), WebGpuKernel>>,

pub fn create_compute_kernel(&self, label, wgsl, entry_point, layouts) -> Result<WebGpuKernel> {
    let key = (wgsl.as_ptr() as usize, entry_point.as_ptr() as usize);
    if let Some(k) = self.compute_pipeline_cache.borrow().get(&key).cloned() {
        return Ok(k);
    }
    // ... compile pipeline as before ...
    self.compute_pipeline_cache.borrow_mut().insert(key, kernel.clone());
    Ok(kernel)
}
```

## Result

Same exact failure mode as the original SP9 attempt at `61d3163c9`:

```
browser-prove:webgpu-uncaptured-error type=GPUValidationError msg=Bind group layout
  [BindGroupLayout "webgpu_zeroize_elem_layout"]
of pipeline layout [PipelineLayout (unlabeled)] does not match layout
  [BindGroupLayout "webgpu_zeroize_elem_layout"]
of bind group [BindGroup "webgpu_zeroize_elem_bind_group"]
set at group index 0.
```

`hello_world_succinct_receipt_verifies` panics on "verify segment".

## Why the hypothesis was wrong

Phase 1's layout cache is keyed by `(label_ptr, shape-hash)` where the
shape hash includes `min_binding_size`. The `dispatch_zeroize_elem`
call site (and several similar sites) constructs its layout with
`WebGpuBindingLayout::storage(0, byte_len)`. **`byte_len` varies per
call** because zeroize_elem operates on differently-sized buffers
across the prover lifecycle.

Layout-cache trace for zeroize_elem:
- Call 1 (byte_len=A) → layout cache MISS → instance L_A inserted at key K_A
- Pipeline cache MISS → compile pipeline P, references L_A → insert at (WGSL, entry)
- Bind group BG_A created with L_A → dispatch OK

- Call 2 (byte_len=B ≠ A) → layout cache MISS → instance L_B inserted at key K_B (≠ K_A)
- Pipeline cache HIT → returns cached pipeline P (still references L_A)
- Bind group BG_B created with L_B
- Dispatch: pipeline layout uses L_A, bind group uses L_B → MISMATCH

The error message confirms: same *label* ("webgpu_zeroize_elem_layout")
on both sides, but they are different *instances* of layouts with the
same label. Identical to the `61d3163c9` failure trace.

## Reverted

Restored `webgpu.rs` to match the phase 1 commit (`e129c9728`). Phase 1
(layout cache) remains landed; it's correct in isolation because
without a downstream pipeline cache, a varying layout instance just
forces a new pipeline compile every time — which is identical to the
pre-SP9 behavior.

## What a correct SP9 phase 2 needs

Two viable paths, both multi-hour refactors:

1. **Thread layout cache keys.** Change `create_bind_group_layout` to
   return `(layout, cache_key: u64)`. Change `create_compute_kernel`
   signature to accept layout cache keys (or a side struct carrying
   them). Pipeline cache key becomes `(wgsl_ptr, entry_ptr,
   [layout_key]*)` so any layout-shape variance forces a pipeline
   recompile.

2. **Eliminate min_binding_size variance.** Audit the 31
   `create_bind_group_layout` call sites and change each that uses a
   variable `byte_len` to pass `min_binding_size = 0`. WebGPU's runtime
   bind-validation still checks the actual binding's size — the layout
   `min_binding_size` is purely a static hint. After this change, the
   shape-hash collapses across byte_lens and the simple `(wgsl_ptr,
   entry_ptr)` pipeline cache key works.

Option 2 is the simpler one and worth attempting next. The risk is
that some call sites *rely* on the static binding-size check (e.g.,
to catch shader-vs-host buffer-size desync at pipeline-creation time
instead of dispatch time); audit needed before changing.

## Failed-experiment ledger entry

No code shipped. The pipeline cache field, its initialization, and
the cache lookup logic were all removed from `webgpu.rs`. The
in-tree `bind_group_layout_cache` (phase 1) is unaffected.

## Outstanding levers (priority order)

1. **iter-6d**: GPU witgen for exec_Top chunks via WGSL pruner.
   Multi-day; biggest expected impact per SP10 (~5-10% wall).
2. **SP9 phase 2 take 3** (option 2 above): audit & collapse
   min_binding_size variance, then re-add pipeline cache. ~1 day.
3. **SP6e**: bind-group cache for the 62 `create_bind_group` sites
   that vary buffer instances. Multi-day, ~2-3% expected.
