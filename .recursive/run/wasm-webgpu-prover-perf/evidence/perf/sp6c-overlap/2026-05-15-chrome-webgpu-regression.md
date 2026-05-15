Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `Environmental — Chrome WebGPU smoke baseline regression`
Date: 2026-05-15
Status: BLOCKING. R1 smoke fails with `GPUAdapter is not available`
even with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json`,
fresh wasm builds, and webgpu.rs at HEAD (no diffs from main). Goal-hook
verification cannot proceed until resolved.

## Symptom

Repeated invocations of `hello_world_succinct_receipt_verifies` end in:

```
panicked at browser-prove/src/lib.rs:45:44:
called `Result::unwrap()` on an `Err` value: GPUAdapter is not available
test tests::hello_world_succinct_receipt_verifies ... FAIL
```

Reproducer (from worktree root):

```bash
WASM_BINDGEN_TEST_TIMEOUT=600 \
  CHROMEDRIVER=/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  /home/rami/.cache/.wasm-pack/wasm-bindgen-c59d5019a2b42393/wasm-bindgen-test-runner \
    --nocapture \
    examples/target/wasm32-unknown-unknown/release/deps/browser_prove-d0e433bffc6b8cc6.wasm \
    hello_world_succinct_receipt_verifies
```

## Diagnostics

- GPU healthy: `nvidia-smi` reports 145 MiB used / 32607 MiB total /
  5% util (vLLM idle on the side, no contention).
- Vulkan ICD intact: `/usr/share/vulkan/icd.d/nvidia_icd.json` exists.
- chromedriver intact: `/home/rami/.cache/.wasm-pack/chromedriver-75649e7ca5ae435b/chromedriver`
  starts and accepts WebDriver commands.
- Tried with and without `VK_ICD_FILENAMES`: both fail identically.
- webgpu.rs at HEAD `61d3163c9` -- no diff vs the prior known-good
  state in iter-5a baseline (`90804ecea`).

## Root cause hypothesis

Chrome auto-updated 148.0.7778.96 -> 148.0.7778.167 since the iter-5a
baseline (2026-05-14, when the smoke was passing). ChromeDriver is
still 148.0.7778.97 (matches the prior Chrome patch). The .96 -> .167
Chrome bump may have changed WebGPU/Vulkan adapter selection in
headless mode -- the symptom is consistent with that class of
regression (no adapter exposed despite a healthy GPU + ICD).

The per-iter-5a fix `VK_ICD_FILENAMES=...nvidia_icd.json` no longer
helps -- Chrome `.167` may be ignoring the env var, or the WebGPU
init path doesn't reach the ICD selection at all in this build.

## Resolution paths (require user action)

1. **Downgrade Chrome to .96** (matches the iter-5a baseline) and pin
   it via `apt-mark hold google-chrome-stable` to prevent auto-update.
   Lowest-risk, fastest-recovery option.
2. **Update chromedriver to match** Chrome `.167` (download
   chromedriver-148.0.7778.167) and re-test. May or may not help --
   the issue is Chrome-side WebGPU init, not the WebDriver protocol.
3. **Audit Chrome `.167` flags** to find a new flag that re-enables
   the WebGPU adapter in headless+vulkan mode. The current flags
   (`enable-unsafe-webgpu enable-features=Vulkan use-angle=vulkan
   enable-dawn-features=allow_unsafe_apis,disable_robustness`) worked
   in `.96`.

## Impact on this session

- iter-6a (zirgen MuxChunk pass) -- LANDED (zirgen `b956e80`). Naga
  validates without browser involvement.
- iter-6b probe + analysis -- LANDED (risc0 `e5bc7e7ee`). Per-leaf
  module sizing measured with naga (no browser).
- SP9 attempts -- ATTEMPTED + REVERTED + LANDED-AS-LEDGER (risc0
  `61d3163c9`). The first attempt failure was a real bug in cache
  semantics (WebGPU layout-instance identity, observed via the smoke
  output BEFORE the Chrome regression manifested). The second
  attempt's RefCell panic was also observed via the smoke. So the
  smoke was working until at least the first hello_world dispatch
  inside that run; the GPUAdapter regression appeared between then and
  the next clean test cycle.
- xgboost / KeccakUnion(3) baseline re-measurement -- BLOCKED on smoke
  recovery.
- Goal hook completion -- BLOCKED. Cannot demonstrate "as close to
  native CUDA as practically possible" without running smokes.
