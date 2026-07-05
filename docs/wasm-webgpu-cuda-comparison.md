# Browser WebGPU vs Native CUDA Proving

> **Note:** `.recursive/...` evidence paths referenced in this document are preserved on the `wasm` archive branch; the presentation branch omits that process tree.

Status: **SP11 closeout 2026-05-15, with follow-on evidence through
2026-05-18.** Cumulative phase work
(SP3–SP9) brings xgboost wall to 102.7 s (18.0× CUDA). SP6 closed
2026-05-15: KeccakUnion(3) eval_check fully on GPU
(`gpu_dispatches=107 cpu_fallbacks=0`) and completes in 303 s vs
prior 3600 s timeout. SP9 phase 1 (layout cache, `e129c9728`) +
phase 2 take 3 (`min_binding_size=0` collapse, `b29e58d67`) landed.
SP8 iter 3 (parallel FRI prove_batch_async via try_join_all,
`e2540dcc5`) landed. SP7 iter 6d-a/b/c (`5d4a37d7c`, `332972022`,
`9de42bdd1`, `3e154206e`) landed: vendored exec_TopChunk0 WGSL,
Tint-validated kernel, probe-mode dispatch behind a process-global
flag. Later SP7 iter 6d-g proved the generated WGSL writes are
bit-exact for every cell they touch (`8,150,063` matches,
`mismatches=0`), but replacement remains blocked because the GPU
path leaves `25,213,024` CPU-written cells uncovered. SP6d iter 10
made the dependency-graph scheduler the default pool route; iter 12
measured xgboost through that path at 102.82 s, effectively flat
against the single-slot anchor. SP6f then cached HAL-created compute
pipelines and measured xgboost at 103.35 s with 36 pipeline creations
and 1,592 cache hits, so pipeline construction is not a material
xgboost wall-time lever in Chrome. SP6g then changed the Poseidon2
fold-chain Merkle path to one dynamic-offset bind group per chain,
dropping xgboost bind-group creations from 12,510 to 9,022 and buffer
allocations from 15,323 to 11,835; wall stayed flat at 102.86 s.
SP6h applied the same dynamic-offset pattern to forward and inverse NTT
steps, dropping bind-group creations to 3,358 and buffer allocations to
6,171; xgboost measured 102.25 s.
SP6i then removed a redundant `hash_rows` output-buffer upload, cutting
xgboost host-to-GPU upload bytes from 15.28 GB to 10.91 GB and measuring
101.93 s.
SP6j cleaned up empty RV32IM scatter handling so xgboost reports
`cpu_fallbacks=0`; wall stayed flat/noisy at 103.22 s.
SP6k then attributed the remaining 7.22 GB of xgboost device-copy
traffic: 7,222,591,488 bytes are copies into `coeffs`, while only
32,768 bytes are `final_coeffs`. This identifies coefficient
materialization as the next target but does not claim a wall-time win.
SP6l added an explicit in-place commit path for dead-after-commit groups,
reducing xgboost device copies from 128 to 85 and device-copy bytes from
7.22 GB to 5.76 GB; wall stayed flat at 101.97 s.
SP6m then fused copy-preserving coefficient materialization into the first
inverse NTT pass, reducing xgboost device copies to only `final_coeffs`
traffic: 32 copies / 32,768 bytes. Wall stayed flat/noisy at 102.26 s, which
closes device-copy cleanup as a material wall-time lever.
SP7a profiling measured the current xgboost RV32IM accumulation surface at
22.13 s across 11 segments: 20.55 s in generated `step_TopAccum`, 1.57 s in
machine-column carry, and 0.01 s in terminal ExtVal prefix.
SP7b moved the machine-column carry scan to WebGPU. The focused GPU/CPU test
caught the required column-major trace layout, and xgboost verified at 101.38 s
with RV32IM accumulation down to 21.03 s.
SP7c profiled recursion accumulation and found no cheap prefix-scan analogue:
prefix products are only 59 ms across xgboost, while generated
`compute_accum`/`verify_accum` total 5.66 s.

## Final per-fixture matrix (2026-05-15)

Hardware: RTX 5090 (32 GB, SM120/Blackwell, CUDA 13.0), Chrome 148.

| Fixture | Native CUDA | WebGPU (single-slot) | Ratio | gpu_idle_ratio | Notes |
|---|---:|---:|---:|---:|---|
| poseidon2_basic succinct | 437 ms | 3156 ms | 7.2× | 0.36 | refreshed 2026-05-15 (SP8 iter 3) |
| libm succinct | 510 ms | 3178 ms | 6.2× | 0.35 | refreshed 2026-05-15 (SP10) |
| keccak_union_small succinct (4 seg + 9 keccak) | 7.7 s | 106.1 s | 13.7× | 0.35 | SP6 closed: `op=eval_check cpu_fallbacks=0` |
| keccak_union succinct (KeccakUnion(3), 11 seg + 25 keccak) | 20.7 s | 303.2 s | 14.7× | 0.37 | SP6 closed (was SIGKILL > 3600 s) |
| **xgboost succinct (multi-segment)** | **5.7 s** | **102.7 s** | **18.0×** | **0.44** | refreshed 2026-05-15; 2-slot default scheduled pool smoke 102.82 s; SP6f pipeline cache 103.35 s; SP6g fold-chain dynamic bind group 102.86 s; SP6h NTT dynamic bind groups 102.25 s; SP6i hash_rows output upload removal 101.93 s; SP6j empty scatter fallback cleanup 103.22 s; SP6k device-copy attribution 101.80 s; SP6l in-place dead commit groups 101.97 s; SP6m fused copy-preserving interpolate 102.26 s |

The xgboost ratio 18.0× is the closest current proxy for real-world
workloads. R1 single-segment fixtures sit at 6.2-7.2× because their
per-prove fixed costs dominate.

## 2-slot multi-device measurement (SP6d iter 3)

Smoke `webgpu_pool_two_concurrent_succinct_proves_smoke` runs two independent succinct proves on a 2-slot WebGpuProverPool:

| Metric | Single-slot baseline | 2-slot concurrent |
|---|---:|---:|
| Mean GPU util | 12.6% | **52.7%** |
| Peak GPU util | 30% | **100%** |
| Mean power | 53.7 W | 104.4 W |
| Peak power | 76.9 W | 215.7 W |
| Per-prove throughput | 3231 ms | 2935 ms (1.10× faster) |
| Per-slot gpu_idle_ratio | 0.34 | 0.15-0.19 |

This confirms that two `web_sys::GpuDevice` instances can feed independent command queues and raise utilization. Follow-on SP6d evidence narrows the performance claim: homogeneous same-GPU work and lift/join-heavy xgboost stay flat, while heterogeneous KeccakUnion-style dependency graphs can see a modest scheduler win (~8% in iter 9).

## Structural residuals (still > 1.0× CUDA)

Every measured fixture remains above 1.0× CUDA. Reasons:

- **Per-active-second density gap (~2×)**: CUDA's nvcc compiles to native PTX; WGSL→SPIR-V→Vulkan via Chrome/Dawn produces less-efficient code (SP3 staged eval_check retro: ~30× per-dispatch; SP6a Poseidon2 merkle: 50-150× per-dispatch). This applies to every kernel.
- **Single-device queue serialization (~3-4×)**: One `GpuDevice` has one `GpuQueue`. CUDA streams can pipeline; Chrome WebGPU on a single device cannot beyond what the queue absorbs.
- **CPU-bound stages outside finalize (~15-25%)**: `recursion_witgen` (~200 ms), `recursion_accumulate` (~200 ms), `rv32im_witgen` (~250 ms), and IOP bookkeeping run on CPU. SP7 (GPU-resident witness) is the lever; not yet landed.

## Closing roadmap (post 2026-05-15)

| Phase | Target win | Status |
|---|---:|---|
| SP6 KeccakUnion eval_check GPU | -keccak fallback | **CLOSED 2026-05-15**: KU(1) 13.7×, KU(3) 14.7× |
| SP6d multi-device pool | -25% on segment∥anything | Hit po2_18 wasm32 ceiling; dependency-graph scheduler iter 9 lands a ~8% heterogeneous win; iter 10 makes the scheduler the default pool route; iter 12 xgboost pool smoke is flat at 102.82 s |
| SP7 iter 6d-a/b/c (probe-mode GPU witgen) | infrastructure | **LANDED 2026-05-15** -- vendored exec_TopChunk0 WGSL, Tint-valid kernel, probe behind static flag |
| SP7 iter 6d-g (replace rust_steps witgen) | -6 s of 103 s wall (~6%) | **BLOCKED**: generated WGSL is bit-exact for GPU-written cells, but only covers 8.15M of 33.36M CPU-written cells; whole-`step_Top` short-circuit leaves ~25.2M cpu_only cells invalid |
| SP7 iter 6d-deeper (TopAccum split + GPU accumulate) | -22 s of 103 s wall (~21%) | Blocked: TopAccumChunk0 closure 1.7 MB is 4× over Chrome's 0.4 MB reachable-closure cliff; needs straight-line-arithmetic chunking pass MuxChunk doesn't provide |
| SP7a/SP7b RV32IM accumulation substage profile + carry offload | -0.8 s observed | xgboost profile: 22.13 s RV32IM accumulation total; 20.55 s generated `step_TopAccum`, 1.57 s machine-column carry, 0.01 s terminal ExtVal prefix. SP7b offloads carry to WebGPU: CPU carry 1.57 s -> 0, GPU carry 0.274 s, RV32IM accumulation total 21.03 s, xgboost wall 101.38 s. Full TopAccum offload remains the larger blocked lever |
| SP7c recursion accumulation profile | no simple scan win | xgboost verifies at 101.25 s. Recursion accumulation totals 5.72 s: `compute_accum` 3.36 s, `verify_accum` 2.30 s, prefix products 0.059 s. Next recursion win also requires generated circuit work on GPU |
| SP8 iter 1 + iter 3 (parallel readbacks) | -wall on FRI | **LANDED 2026-05-15** -- noise on xgboost (FRI is small), larger fixtures TBD |
| SP9 phase 1 + phase 2 take 3 (layout cache + min_binding_size collapse) | infrastructure | **LANDED 2026-05-15** -- layout identity is now stable enough for higher-level caches |
| SP6e-SP6m object churn, upload trimming, and copy reduction | -2-3% smokes | Bind-group diagnostics showed 12,510 bind groups for 5,365 dispatches. Compute-pipeline cache landed with 1,592 hits on xgboost but wall stayed flat at 103.35 s. Poseidon2 fold-chain dynamic bind groups cut xgboost bind groups to 9,022 and buffers to 11,835, still flat at 102.86 s. NTT dynamic bind groups cut bind groups to 3,358 and buffers to 6,171; xgboost measured 102.25 s. Removing redundant hash_rows output uploads cut xgboost upload bytes from 15.28 GB to 10.91 GB and measured 101.93 s. Empty scatter cleanup removed the last reported xgboost CPU fallbacks but wall stayed flat/noisy. Device-copy attribution showed xgboost's 7.22 GB copy traffic was almost entirely `coeffs`; in-place dead commit groups reduced that to 5.76 GB and 85 copies; fused copy-preserving interpolate reduced remaining device-copy traffic to only `final_coeffs`, 32 copies / 32,768 bytes. Wall remained flat at 102.26 s, so this line is now treated as correctness/infrastructure cleanup rather than the next wall-time lever. Global bind-group caching remains deferred until buffer lifetime/invalidation is explicit |
| Per-kernel WGSL improvements (SP3/SP6a revisits) | -5% per kernel | Parked (low priority per SP6c submission-bound diagnosis) |

Composed expectation (per `project_sp7_witgen_savings_ceiling`):
- Today (2026-05-15): xgboost **18.0× CUDA**
- + SP7 iter 6d-d/e (rv32im witgen GPU): **17.0× CUDA** (-6 s)
- + SP7 iter 6d-deeper (rv32im accum GPU): **13.5× CUDA** (-22 s)
- + recursion circuit witgen+accum on GPU: **12.0× CUDA** (-12 s)
- + safe bind-group cache + readback coalescing: **11× CUDA** remains
  speculative; SP6f showed compute-pipeline caching alone is flat on xgboost
- Floor estimate (single-tab single-GPU Chrome, per SP6c
  submission-bound diagnosis): **5-8× CUDA**

The 5-8× floor reflects the structural per-active-second density gap
(~2×: nvcc-compiled CUDA PTX vs Tint-compiled SPIR-V) and the single
JS thread + single GpuQueue submission overhead. Closing further
than that requires either (a) multi-device exposed to a single proof
(WebGPU API restriction), (b) substantial Chrome/Dawn WGSL→SPIR-V
quality improvements, or (c) running the witness generator out of
the JS event loop entirely (Web Workers with SharedArrayBuffer +
async device coordination -- not currently supported in
wasm-bindgen-test). Document this as a structural residual at SP11
close.

See `docs/wasm-webgpu-prover-learnings.md` for the pause handoff and `.recursive/run/wasm-webgpu-prover-perf/evidence/perf/r1-baselines/` for the canonical capture commands plus the refreshed summary table.

## R1 Smoke Matrix (refreshed 2026-05-12 on RTX 5090 + Chrome 148)

Hardware: RTX 5090 (32 GB, SM120/Blackwell, CUDA 13.0), Chrome 148.0.7778.96, ChromeDriver 148.0.7778.97, headless `enable-unsafe-webgpu enable-features=Vulkan use-angle=vulkan`. Negotiated WebGPU limits: `max_buffer_size=max_storage_buffer_binding_size=1073741824`, `max_compute_workgroup_storage_size=49152`.

| Fixture | Native CUDA | Chrome WebGPU | Ratio | Receipt | Segments | User cycles | gpu_dispatches | cpu_fallbacks | cpu_only_ops |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| `risc0-zkvm-methods/cfg` | 499.5 ms | 3.82 s | **7.6×** | Succinct, verified | 1 | 2269 | 306 | 1 (scatter) | 0 |
| `hello-world` | 483.5 ms | 3.80 s | **7.9×** | Succinct, verified | 1 | 3560 | 306 | 1 (scatter) | 0 |
| `json` | 521.1 ms | 4.58 s | **8.8×** | Succinct, verified | 1 | 13319 | 312 | 1 (scatter) | 0 |
| `multi_test/poseidon2_basic` (cold CUDA) | 545.3 ms | 3.95 s | **7.2×** | Succinct, verified | 1 | 3553 | 306 | 1 (scatter) | 0 |
| `multi_test/poseidon2_basic` (warm CUDA) | 244.9 ms | 3.95 s | **16.1×** | Succinct, verified | 1 | 3553 | 306 | 1 (scatter) | 0 |
| `multi_test/libm` | 510.0 ms | 3.80 s | **7.5×** | Succinct, verified | 1 | 3328 | 306 | 1 (scatter) | 0 |
| `multi_test/keccak_union_small` | 7.748 s | 121.99 s | **15.7×** | Succinct, verified | 4 | 747265 | 6022 | 4 (scatter + 3 keccak eval_check) | 0 |

The libm ratio is no longer 491×; the async/GPU-authoritative recursion path applies. The KeccakUnion(1) ratio is no longer 59×; same path applies plus async Keccak union. The remaining residual is shape-specific: small fixtures are upload + readback + interpreter-startup dominated; KeccakUnion(1) is dominated by Keccak `eval_check` (3 fallbacks because 6741 > 1536 FP slot cap) and Keccak `scatter`.

## Pre-2026-05-12 Anchor (retained for delta visibility)

The pre-2026-05-12 figures below are retained because they document the path that landed the dramatic libm + Keccak improvements. They are NOT the current measurement anchors.

This note compares the current browser WebGPU proving path with native CUDA
proving as implemented in this branch. It is intentionally focused on the
parts that explain correctness parity and the current order-of-magnitude
runtime gap.

## Historical Measurement Anchor (pre-2026-05-12; superseded by R1 Smoke Matrix above)

All native baselines should be measured before browser runs with:

```bash
RECURSION_SRC_PATH=examples/target/release/build/risc0-circuit-recursion-4e96382f0d1db440/out/recursion_zkr.zip \
RISC0_PROVER=local RISC0_EXECUTOR=local RISC0_INFO=1 RUST_LOG=info RISC0_PRINT_SEGMENTS=1
```

The focused async/GPU-authoritative correctness anchor is
`multi_test/poseidon2_basic`. Both paths prove one segment with the same cycle
count and produce a succinct receipt that verifies with the existing verifier:

| Path | API | Receipt | Segments | User cycles | Total cycles | Wall time |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Native CUDA | native local prover | Succinct | 1 | 3598 | 32768 | 426.880871ms |
| Chrome/WASM/WebGPU | `WebGpuProver::prove_with_opts_async` | Succinct | 1 | 3598 | 32768 | 4.03s |

Browser stage telemetry for that run:

| Browser stage | Time |
| --- | ---: |
| `execute` | 57ms |
| `segment_preflight` | 24ms |
| `rv32im_witgen` | 86ms |
| `rv32im_accumulate` | 241ms |
| `rv32im eval_check_interpreter_submit` | 1ms |
| `segment_prove_core_async` | 969ms |
| `verify_segment` | 14ms |
| `recursion_witgen` | 203ms |
| `recursion_accumulate` | 200ms |
| `recursion eval_check_interpreter_submit` | 1 dispatch |
| `lift_prove_async` | 2.878s |
| `verify_lift` | 15ms |
| `prove_session_async` | 3.913s |

The same proof recorded mostly GPU-authoritative ZKP dispatches, with explicit
readbacks and CPU fallbacks at the remaining compatibility boundaries:

```text
gpu_dispatches=306 cpu_mirrors=11 cpu_fallbacks=1 cpu_only_ops=0
uploads=750 upload_bytes=444187604
device_copies=8 device_copy_bytes=212208640
readbacks=62 readback_bytes=431840
buffers=818 buffer_bytes=1738130508
```

This latest run is correctness-positive for the interpreted circuit
`eval_check` path: the proof still verifies, and the HAL operation counters
now record `eval_check` as 2 WebGPU dispatches and 0 CPU fallbacks. Chrome
granted 1 GiB `maxBufferSize` and `maxStorageBufferBindingSize`, so the
recursion data group is now allocated and bound directly instead of being
uploaded through bounded chunks. The first
dispatch is the rv32im check (`domain=131072`, `instructions=20202`,
`fp_slots=927`, `mix_slots=29`).

For the older `multi_test/libm` performance anchor, both paths also prove one
segment with the same cycle count:

| Path | Receipt | Segments | User cycles | Total cycles | Wall time |
| --- | --- | ---: | ---: | ---: | ---: |
| Native CUDA | Succinct | 1 | 3373 | 32768 | 436.441617ms |
| Chrome/WASM/WebGPU | Succinct | 1 | 3373 | 32768 | 214.08s |

Browser stage telemetry for that run:

| Browser stage | Time |
| --- | ---: |
| `execute` | 57ms |
| `segment_preflight` | 21ms |
| `rv32im_witgen` | 85ms |
| `rv32im_accumulate` | 309ms |
| `rv32im_eval_check` | 32.546s |
| `segment_prove_core` | 43.592s |
| `verify_segment` | 14ms |
| `recursion_witgen` | 207ms |
| `recursion_accumulate` | 207ms |
| `recursion_eval_check` | 119.937s |
| `lift_prove` | 170.293s |
| `verify_lift` | 15ms |
| `prove_session` | 213.951s |

That older browser proof recorded WebGPU dispatches, but it predated the full
async/GPU-authoritative recursion path and still used the synchronous CPU-shadow
path, so every dispatch had a CPU mirror update:

```text
gpu_dispatches=950 cpu_mirrors=950 cpu_fallbacks=1 cpu_only_ops=0
uploads=2015 upload_bytes=643307516
device_copies=8 device_copy_bytes=212208640
readbacks=0 readback_bytes=0
buffers=2032 buffer_bytes=1736647420
```

## Side-By-Side Proving Path

| Phase | Browser WASM/WebGPU | Native CUDA |
| --- | --- | --- |
| API entry | `webgpu_prover().await` initializes one browser `WebGpuHal` and returns `WebGpuProver`. `webgpu_prover_pool(slots).await` initializes a `WebGpuProverPool` whose async proving entrypoints use the SP6d dependency scheduler by default. `default_prover()` is intentionally unavailable for browser WebGPU because adapter and device creation are async. | Native callers use the normal local prover path. With CUDA enabled and `RISC0_PROVER=local`, the existing prover selectors create CUDA-backed segment and recursion provers. |
| Backend binding | `WebGpuProver` runs proving and compression inside scoped `with_webgpu_hal` bindings for `rv32im`, `keccak`, and `recursion`, so synchronous prover code resolves to the initialized browser HAL. | Native prover construction resolves directly to CUDA circuit HALs when CUDA support is compiled and selected. |
| Guest execution | The guest executes in the browser WASM host executor. Execution is not the current bottleneck for small fixtures; `libm` execution measured 56ms. | The guest executes on the native host executor, then proof work is handed to CUDA-backed proving. |
| Segment proof core | Uses the browser `rv32im` circuit HAL plus WebGPU ZKP HAL operations. The circuit path is browser-compatible Rust/WASM for witness and accumulation, with portable check evaluation where WebGPU circuit kernels are not yet present. | Uses generated CUDA circuit kernels for `rv32im` witness generation, accumulation, and `eval_check`, plus CUDA ZKP HAL kernels. |
| Keccak proof core | Uses the browser `keccak` circuit HAL plus WebGPU ZKP HAL operations. Like `rv32im`, correctness is brought up through browser-compatible circuit code rather than generated CUDA kernels. | Uses generated CUDA Keccak witness and `eval_check` kernels plus CUDA ZKP HAL operations. |
| Recursion proof core | Uses the browser recursion circuit HAL for lift, join, resolve, union, and identity. The async `poseidon2_basic` run now keeps ZKP commit/finalize and combo prepare/divide GPU-authoritative. The interpreted WebGPU `eval_check` path runs for both rv32im and recursion; Chrome currently grants 1 GiB buffer and storage-binding limits, so the focused recursion data group binds directly. | Uses generated CUDA recursion witness, accumulation, and `eval_check` kernels. The focused native CUDA proof, including compression to succinct, completes in 426.880871ms in the latest run. |
| ZKP HAL buffers | `WebGpuBuffer` owns a real `GPUBuffer` and tracks CPU-dirty vs CPU-stale state. The async public path can keep successful WebGPU outputs GPU-owned and materialize CPU shadows with explicit async readback. | CUDA buffers are device-resident. Circuit and ZKP kernels operate on device pointers; host visibility is explicit and much less frequent. |
| ZKP HAL kernels | WebGPU kernels cover proof-critical bulk operations such as NTT, hashing, FRI fold, gather, scatter, mixing, copies, and prefix products. Current async diagnostics assert `cpu_only_ops == 0` and reduce CPU mirrors sharply, but some large storage-binding cases still fall back to CPU after explicit readback. | CUDA kernels are the authoritative execution path for bulk ZKP work, with no browser-style synchronous CPU shadow. |
| Transcript and verification | Existing receipt assembly and verifier logic are reused. Internal `verify_segment`, `verify_lift`, and `verify_composite` timings are milliseconds and are not the blocker. | Existing receipt assembly and verifier logic are reused. Verification work is also not the dominant cost. |
| Receipt compatibility | Produces standard succinct receipts that verify with the existing verifier for the passing browser cases. | Produces the native reference receipts used as the parity baseline. |

## Why The Browser Path Is CPU-Bound Today

The public async browser proof path is now GPU-authoritative for STARK
commit/finalize in the focused segment and recursion paths. The remaining gap
is no longer blanket CPU mirroring; it is explicit compatibility work:
batched transcript/Merkle query readbacks, fallback readbacks for buffers that
exceed the conservative WebGPU storage-binding limit, and portable WASM circuit
`eval_check`.

That explains the current diagnostics: `gpu_dispatches` is high,
`cpu_only_ops` is zero, and `cpu_mirrors` is much lower than dispatches. The
batched Merkle query path plus bounded oversized `batch_evaluate_any` and
chunked `mix_poly_coeffs` paths reduced the focused proof to 61 readbacks,
2870560 readback bytes, and 6 CPU fallbacks. Circuit checks and oversized
recursion-data uploads still dominate wall time.

The second major gap is circuit-specific work. Native CUDA uses generated CUDA
kernels for `rv32im`, `keccak`, and especially `recursion` witness,
accumulation, and `eval_check`. The browser currently uses browser-compatible
Rust/WASM circuit paths and portable check evaluation where WebGPU circuit
kernels have not yet been implemented. The `libm` measurement points directly
at this: recursion `lift_prove` alone takes about 170.3 seconds in the browser,
with `recursion_eval_check` accounting for about 119.9 seconds. Native CUDA
finishes the entire succinct proof in about 436 milliseconds.

Keccak-heavy fixtures amplify the same gap. The smaller
`KeccakUnion(1)` diagnostic now proves and verifies as a Chrome/WebGPU
succinct receipt after the async Keccak receipt union fix, async WebGPU
Keccak subproof path, and 1 GiB WebGPU limit negotiation. The latest verified
run took 441.14s versus a 7.46676192s native CUDA baseline for the same 4
segments, 9 pending Keccak proofs, and 1 assumption. All ZKP HAL bulk ops
except `scatter` ran on WebGPU with zero CPU fallbacks; the remaining major
cost is the Keccak circuit `eval_check`, which still falls back to portable
WASM 9 times because the generic interpreter shape needs 6741 FP slots. The
full `KeccakUnion(3)` fixture still exceeds the current browser runner budget:
the latest run progressed through all 11 RV32IM segments and into the async
Keccak/union proof tree before a 3600s timeout/SIGKILL.

The portable `eval_check` loop now precomputes the zerofier inverse table for
the `INV_RATE` expanded-domain residues instead of recomputing powers and
inverses for every row. A follow-up reusable scratch path for the interpreted
`poly_ext` executor preserved correctness but stayed within noise by itself.
Precomputing the PolyExt constraint-compression mix powers once per
`eval_check` moved the focused `libm` browser proof from 228.71s to 214.08s.
The remaining cost is still dominated by generated/interpreted constraint
evaluation and CPU-shadow buffer traversal in WASM. A WebGPU circuit
`eval_check` hook now runs before the compatibility readback barrier, and a
tiny generated straight-line WGSL `poly_ext` regression passes in Chrome. The
generator now assigns fixed named scratch slots from `poly_ext` liveness;
measured liveness is about 1000 FP slots for recursion and 926 for rv32im. A
real recursion monolithic shader probe still lost the WebGPU device after
about 128 seconds, so large straight-line shaders remain disabled. The current
prototype instead uses a small interpreted WGSL program for large real
definitions. A recursion CPU-equivalence smoke test passes in Chrome, and the
latest focused proof verifies with `eval_check` recorded as 2 WebGPU dispatches
and 0 CPU fallbacks. The rv32im interpreter path is fast enough for the
focused segment proof. With 1 GiB Chrome WebGPU limits, recursion `group2` now
binds directly in the focused proof instead of being uploaded through bounded
chunks.

A split `eval_check` prototype is also not production-ready. Earlier recursion
experiments showed that one recursion contribution needs 1603 FP dependencies,
and the slot-reusing split shader lost the Chrome WebGPU device after 486.20s
on a `po2 = 0` reference-comparison smoke test. A later Keccak-only split
attempt avoided the CPU fallback but deferred about 148s of GPU work into
`check_group` and then lost the device. Real circuit `eval_check` needs smaller
semantic staging or circuit-specific kernels rather than larger straight-line
WGSL chunks.

`mix_poly_coeffs` now has an async GPU-authoritative wrapper plus a chunked
path for oversized input bindings. The focused `poseidon2_basic` proof now
records `mix_poly_coeffs` as 8 GPU dispatches and 0 CPU fallbacks while still
producing a verified succinct receipt.

## GPU-Authoritative Refactor Status

The async/GPU-authoritative slice is implemented in the HAL, ZKP proof layers,
and focused public browser proving path:

- `WebGpuBuffer` now distinguishes CPU writes that need upload from GPU writes
  that make the CPU shadow stale.
- `WebGpuHal::gpu_authoritative_scope(true)` lets successful WebGPU kernels
  mark outputs GPU-owned without running the CPU mirror.
- `WebGpuBuffer::sync_gpu_to_cpu(...).await` is the explicit readback boundary
  before synchronous transcript or diagnostic reads.
- WebGPU-specific async variants now exist for Merkle tree construction,
  PolyGroup construction, FRI proving, STARK group commit, and STARK finalize.
- The rv32im, recursion, and keccak WebGPU circuit prover wrappers can attempt
  a WebGPU circuit `eval_check` hook before the portable CPU fallback. The
  currently generated real circuit definitions still exceed the conservative
  shader-size gate, but the hook removes the architectural blocker at the
  async finalize boundary.
- The rv32im and recursion WebGPU circuit prover wrappers use the async ZKP
  commit/finalize path while keeping existing CPU witness/accumulation phases
  outside GPU-authoritative mode.
- `WebGpuProver::prove_with_opts_async` has produced a verified succinct
  `multi_test/poseidon2_basic` receipt with both segment and recursion ZKP
  commit/finalize in GPU-authoritative mode.

The remaining refactor work is to extend this across the full parity matrix,
replace oversized-buffer CPU fallbacks with proof-correct tiled buffers or
chunked WebGPU kernels where possible, and make recursion/rv32im/keccak
`eval_check` shaders production-capable so the compatibility readback before
portable `eval_check` is skipped on real proofs. The current interpreted
`eval_check` prototype now proves correctness for rv32im and recursion in the
focused proof. Full GPU-authoritative recursion data still requires keeping
the recursion data group GPU-resident across stages; the current prototype
uses temporary chunk uploads, which is correct but doubles upload volume. A
production `gather_sample` chunking attempt,
including aligned source bindings and per-chunk command submission, passed
isolated HAL tests but failed the focused proof at `verify_lift`. A
recursion-sized production regression then showed all-zero output for a
512 MiB single source buffer (`rows = 2^20`, `cols = 128`). The HAL now gates
GPU allocation with `GPUSupportedLimits.maxBufferSize`, and the proof path
keeps the CPU fallback for oversized gather sources until a tiled multi-buffer
implementation replaces the single-buffer assumption.

## Optimization Priorities

1. Replace temporary recursion `group2` chunk uploads with a tiled/staged
   GPU-resident representation so recursion `eval_check`, gather, and related
   proof stages can share the same tiles without re-uploading 512 MiB per
   proof.
2. Replace CPU fallbacks for oversized storage bindings with proof-correct
   chunked WebGPU kernels or bounded staged/tiled buffers, especially hash
   rows, gather, NTT, and polynomial evaluation. `gather_sample` now has a
   recursion-sized fallback regression; production GPU re-enable requires a
   tiled source representation for matrices that exceed the browser/device
   single-buffer limit.
3. Add or port remaining WebGPU circuit kernels for `rv32im` and `keccak` after
   recursion, preserving the existing `CircuitHal` boundary.
4. Reduce synchronous transcript-driven buffer views and per-round host access
   in FRI, DEEP, and Merkle paths.
5. Batch small WebGPU dispatches and reuse pipelines/buffers aggressively; the
   current `libm` run performs 950 dispatches for a one-segment proof.
6. Keep native CUDA measurements beside every browser measurement so timeouts
   can be interpreted against known segment counts and expected proof scale.

## Relevant Code Paths

- Browser API and scoped HAL binding:
  `risc0/zkvm/src/host/client/prove/webgpu.rs`
- Browser prover orchestration and stage telemetry:
  `risc0/zkvm/src/host/server/prove/prover_impl.rs`
- WebGPU ZKP HAL and buffer diagnostics:
  `risc0/zkp/src/hal/webgpu/`
- Browser circuit HALs:
  `risc0/circuit/rv32im/src/prove/hal/webgpu.rs`,
  `risc0/circuit/keccak/src/prove/hal/webgpu.rs`,
  `risc0/circuit/recursion/src/prove/hal/webgpu.rs`
- Native CUDA circuit HALs:
  `risc0/circuit/rv32im/src/prove/hal/cuda.rs`,
  `risc0/circuit/keccak/src/prove/hal/cuda.rs`,
  `risc0/circuit/recursion/src/prove/hal/cuda.rs`
- Native CUDA ZKP HAL:
  `risc0/zkp/src/hal/cuda.rs`
