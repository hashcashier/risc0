# M5a+M5b: wasm threads — bring-up, flags baseline, and parallel rv32im witgen

Date: 2026-07-03. Status: ACCEPTED (M5a bring-up + M5b first parallel kernel
landed together). xgboost 23004 → 21093 ms (−8.3%).

## Why

Post-M4, every ranked lever is wasm-thread CPU (the single thread serializes
ALL CPU work — [[project-webgpu-queue-concurrency-limits]]). Real threads
convert that physics constraint into an engineering problem. User asked
directly: "Can we get more wasm threads?"

## The recipe (all pieces verified in-repo before building)

1. **Build**: `-C target-feature=+atomics,+bulk-memory,+mutable-globals`
   + `-C link-arg=--max-memory=4294967296` + `-Z build-std=panic_abort,std`.
   Runs on the PINNED stable 1.89 via `RUSTC_BOOTSTRAP=1` — `rust-src` is
   already a pinned component in rust-toolchain.toml (guest-std precedent),
   so no toolchain change and codegen stays comparable to all baselines.
2. **SharedArrayBuffer**: Chrome flag merged into the EXISTING enable-features
   arg: `--enable-features=Vulkan,SharedArrayBuffer` (repeated
   `--enable-features` flags override each other — appending a second one
   silently disables Vulkan). Deployments without the flag need COOP/COEP
   headers (standard for threaded wasm).
3. **Thread pool**: `wasm-bindgen-rayon` 1.3.0 with the **`no-bundler`**
   feature (default bundler mode hangs `init_thread_pool` forever under
   wasm-bindgen-test-runner — nested module workers can't resolve the
   helper; no-bundler resolves at runtime). rayon 1.12 compatible.
4. **Blocking legality**: gates already run `run_in_worker` — `Atomics.wait`
   (rayon's fork-join block) is legal in a dedicated worker, trap on main.
5. **No C in the graph**: browser wasm skips the sys-crate C kernels
   (`target_is_browser_wasm()` guard) — the whole CPU path is Rust, so no
   foreign objects need matching `-matomics` flags at link.

## Probe result (scratch crate, exact gate harness: wasm-bindgen-test-runner + chromedriver-149 + headless Chrome, run_in_worker)

```
probe: memory_shared=true
probe: hardware_concurrency=32
probe: init_thread_pool(8) ok in 11ms
probe: serial=274ms par=53ms speedup=5.17x distinct_workers=8 sums_match=true
```

Probe crate + runner script: scratchpad/wasm-threads-probe/ (session scratch).

## Kernel adoption map (M5b+)

| Kernel | CPU (xgboost) | Parallel structure |
|---|---:|---|
| rv32im `run_witness_steps` (rust_steps) | ~320ms/seg | two forward loops split at `table_split_cycle` → par(0..split), barrier, par(split..last). `LookupTables` is `Vec<Cell<u32>>` histograms → relaxed `AtomicU32` (associative counts). CUDA implements the same split → parallel semantics proven. `lookup_current` (order-dependent read) callers must be post-split — assert it. |
| recursion `rust_kernels::generate_witness` | 3045ms total | per-cycle exec loop is parallel (WOM reads pre-resolved by preflight) BUT plonk read/write queues use `&mut self` sequential passes — needs per-cycle seeding restructure. M5c. |
| keccak rust_steps + preflight | ~3.4s/proof | own rust_steps; preflight (keccak-f permutations) likely dominates; per-preimage parallel. M5c/M5d. |
| rv32im accum steppers | 224ms/seg + carry | `step_TopAccum` per-cycle loop parallelizable; machine-column carry is a separate sequential prefix pass (block-scan candidate, M1b pattern). |

rayon is ALREADY an active dep of rv32im on wasm (under `prove`) — kernels
need zero manifest changes. zkp gets `wasm-bindgen-rayon` (optional, wasm32
target section, `dep:` under webgpu feature); pool init awaited once in
`WebGpuHal::new` behind a process-wide flag.

## Sequencing note (atomics tax)

The atomics build changes single-thread runtime behavior (thread-safe
dlmalloc locks every alloc, shared-memory codegen). So M5a measures the
FLAGS BASELINE — same code, atomics flags, no pool — before any parallelism
lands. The threads win must clear whatever tax shows up here.

## M5a full-graph build + flags-baseline gates (atomics flags, NO code changes)

Whole browser-prove graph (std + all circuits + WGSL-embedding crates)
compiles and links as a shared-memory module: exit 0, first try, 25 min cold
in a separate `target-m5-atomics/` dir (baseline cache left warm).

| Gate | M4 baseline | Atomics flags-baseline |
|---|---:|---:|
| Parity suite | 4/4 | 4/4 |
| BusyLoop | ~2594 ms | 2607 ms (flat) |
| KeccakUnion(1) | 43.7–43.9 s band | 43863 ms (in-band) |
| xgboost | 22850–23299 ms | 23004 ms (in-band) |

**The atomics tax (thread-safe dlmalloc, shared-memory codegen) is zero at
gate granularity.**

## M5b: pool init + parallel rv32im witgen

- `wasm-bindgen-rayon` 1.3 (`no-bundler`) optional dep of risc0-zkp under
  `webgpu`; pool awaited once in `WebGpuHal::new` (`wasm_thread_pool
  workers=8 init_ms=14`; default min(hardware_concurrency, 8), knob
  `set_wasm_thread_pool_workers`, no silent serial fallback on failure).
- rv32im `rust_steps`: `LookupTables` `Cell<u32>` → relaxed `AtomicU32`
  (C reference uses `std::vector<std::atomic_uint32_t>`); `run_witness_steps`
  `StepMode::Parallel` arm → `into_par_iter().try_for_each` over each side of
  `table_split_cycle` — a 1:1 port of `risc0_circuit_rv32im_cpu_witgen`'s
  `poolstl::par` structure; `unsafe impl Send/Sync for BufferRow` justified by
  the identical C cross-thread capture.

| Gate | Flags-baseline | M5b | Movement |
|---|---:|---:|---:|
| Parity suite | 4/4 | 4/4 | ✓ |
| BusyLoop | 2607 ms | 2610 ms | flat (witgen span 510→476) |
| KeccakUnion(1) | 43863 ms | 44094 ms | in-band |
| **xgboost** | **23004 ms** | **21093 ms** | **−8.3%** |

xgboost `rv32im_witgen` span sum: **3571 → 1616 ms** (per-segment 274 →
~110 ms; first segment 340 ms carries pool warm-up). All receipts verified.

## Observation: KeccakUnion-phase witgen spans don't shrink (pre-existing)

The 4 rv32im segments proven during the KeccakUnion phase show ~1000 ms
witgen spans in BOTH m5a and m5b runs, despite identical cycles=262144 and
comparable txns to xgboost's 274 ms segments. Whatever inflates those spans
(~3.6× vs the same work in a quiet phase) predates M5b and is immune to the
stepper parallelization — leading hypothesis is absorption of concurrent
keccak-pipeline activity (M1/M2b label-lie family). Attribution deferred; the
KeccakUnion wall is in-band either way.

## Notes

- `cargo check -p risc0-circuit-rv32im --features prove` (native) fails
  pre-existing (29 errors on the unchanged tree — native `prove` wiring is
  not maintained in this fork). zkp native `--features prove` remains the
  landing gate: 36/36.
- Both build AND run must pass the atomics config: the gate scripts carry
  `RUSTC_BOOTSTRAP=1`, `-Z build-std=panic_abort,std`, and the `--config`
  target rustflags (atomics, bulk-memory, mutable-globals,
  max-memory=4GiB). A non-atomics build of `webgpu`-featured zkp now fails
  loudly at wasm-bindgen-rayon's compile_error gate.
- webdriver.json: SAB flag MERGED into the existing enable-features arg
  (`--enable-features=Vulkan,SharedArrayBuffer`) — repeated flags override.

## M5c: parallel rv32im accum steppers (same-day follow-on)

All three per-cycle `step_TopAccum` loops (`run_accum_raw_steps_skip_major`,
`run_accum_raw_steps_skip_replaced_misc0_or_majors`, `run_accum_steps`) go
`into_par_iter().try_for_each` — the C reference (`cpu_accum` phase1) runs
`stepAccum` under `poolstl::par` identically; the cross-cycle recurrences
already live in separate prefix passes (GPU-offloaded in production:
`terminal_ext_prefix_gpu` 122 ms, `machine_column_carry_gpu` ~0). The
`direct_misc0_rows` counter became a local `AtomicUsize`.

| Gate | M5b | M5c | Movement |
|---|---:|---:|---:|
| Parity suite | 4/4 | 4/4 | ✓ |
| BusyLoop | 2610 ms | **2330 ms** | **−10.7%** |
| KeccakUnion(1) | 44094 ms | **42180 ms** | **−4.3% — below the 42.8–44.5 s all-time band** |
| **xgboost** | **21093 ms** | **19454 ms** | **−7.8%** |

Span proof (xgboost): `step_top_accum_cpu_skip_replaced` **2281 → 365 ms**
(6.2×); `rv32im_witgen_accum` 2474 → 560 ms; `rv32im_accumulate` 2423 → 513 ms.

Remaining CPU ranking after M5c: recursion_witgen_generate 3107 ms (M5d —
per-worker MachineContext over shared raw-ptr fields + `is_par_safe` group
leadership, mirroring C++ `parStepExec`; generated inc calls methods only,
so the struct internals are free to change), rv32im_witgen residual 1560 ms
(materialize + Amdahl tail), keccak witgen/preflight (~3.4 s per proof,
KeccakUnion phase).

## M5d: parallel recursion witgen (per-worker MachineContext)

The generated steps require `&mut MachineContext`, so the restructure gives
every rayon worker its own COPY of a raw-ptr view (no aliased Rust
references): driver-owned `MachineStorage` holds the cycles/wom_rows/
wom_index Vecs; `MachineContext` becomes a Copy view over them (the
generated inc calls methods only — 10 accessors — so struct internals were
free to change). Exec phase = 1:1 port of C++ `doStepExec` (par over ALL
cycles, each task runs its dependency group only if leader: `cycle == 0 ||
is_par_safe`); verify phase = plain per-cycle par (C++ `doStepVerifyWom`);
`prepare_wom_for_verify` keeps its serial prefix but the WOM sort went
`par_sort` (stable, identical order). `KernelArgs` (already raw ptrs) became
Copy with documented Send/Sync.

| Gate | M5c | M5d | Movement |
|---|---:|---:|---:|
| Parity suite | 4/4 | 4/4 | ✓ |
| BusyLoop | 2330 ms | **2236 ms** | −4.0% |
| KeccakUnion(1) | 42180 ms | **40057 ms** | **−5.0%** (union chain = recursion proofs) |
| **xgboost** | **19454 ms** | **17489 ms** | **−10.1%** |

Span proof (xgboost): `recursion_witgen_generate` **3107 → 635 ms** (4.9×),
`recursion_witgen_new` 4300 → 1920 ms. Honest note: `rv32im_witgen`
stretched 1560 → 2037 ms — lookahead lifts now put recursion witgen on the
worker pool concurrently with segment witgen; per-span contention stretch,
net wall −10.1%. Native `cargo check -p risc0-circuit-recursion` (default
features) fails PRE-EXISTING at HEAD (dead-code `GpuAuthoritative` under
-Dwarnings), unchanged by this diff; zkp native suite 36/36. fmt reflow
after gating produced a byte-identical binary (debuginfo=0), so the gated
bytes are the landing bytes.

## M5e: parallel keccak witgen

`run_witness_steps` Parallel arm → `into_par_iter` (C reference keccak-sys
ffi.cpp is plain `poolstl::par` per cycle — no split, no leadership);
keccak's own `BufferRow` gets the documented Send/Sync impls; the concrete
preimage slices are captured outside the closure so the generic preflight
order type stays off it.

| Gate | M5d | M5e | Movement |
|---|---:|---:|---:|
| Parity suite | 4/4 | 4/4 | ✓ |
| BusyLoop | 2236 ms | 2255 ms | flat |
| KeccakUnion(1) | 40057 ms | **38945 ms** | **−2.8%** |
| xgboost (no keccaks) | 17489 ms | 17052 ms | flat-to-better ✓ |
| **Heavy 25-keccak fixture** | 119.5 s (M4e) | **107.7 s** | **−9.9%** |

Span proof: `keccak_witgen` **2436 → 369 ms** (6.6×). Receipts verified.

## M6 probes: the current config is locally optimal under threads

**M6a — worker count.** 16 workers regressed EVERY gate ~20-25% (xgboost
17052→21159 ms, KeccakUnion 38945→49034 ms, BusyLoop +18%): oversubscribed
rayon idle-spinning on shared memory taxes the busy workers. 31 not worth
probing after that. `min(hardware_concurrency, 8)` stands (documented at the
default site).

**M6b — succinct width 4.** No OOM under the 4 GiB atomics ceiling (the M3-era
2 GiB wall is genuinely gone) but dead flat on xgboost (17050 vs 17052 ms)
and slightly worse on KeccakUnion (39623 vs 38945). Width 3 stands; the
succinct phase is no longer limited by proof-level concurrency.

**M6c — SP7 witgen/accum GPU acceleration re-check.** Full-disable probe
blocked by design: the e2e tests are acceptance tests of the acceleration
config (assert flag masks + dispatch row counters). Timeline analysis
answered the underlying question instead: the 2311 ms
`iter6d_d_witgen_prewarm_async` span starts 1.66 s BEFORE `prove_session`
(during prover construction) and overlaps segment 0 for only ~650 ms —
segment 0 stretches ~500 ms vs later segments, so the real critical-path
cost is ~300-500 ms, not 2.3 s (absorbed-span illusion, M1 label-lie
family). The acceleration actively dispatches in production; expected
|delta| from removal is sub-500 ms either way — deprioritized versus
segment-level pipelining.

**Next ranked avenue (M6d candidate): segment-phase pipelining.** With
gpu_active at 66% of the xgboost wall, the ~5.8 s idle is mostly per-segment
CPU-only windows (witgen ~110-190 ms + accum stepper + preflight + executor
slice, ×11) during which the GPU drains. Segments were kept strictly serial
by the wasm32 ~2 GiB ceiling ("segment∥anything impossible at po2_18") —
the 4 GiB atomics ceiling reopens overlapping segment N+1's
preflight/witgen with segment N's GPU tail, M4e-style.
