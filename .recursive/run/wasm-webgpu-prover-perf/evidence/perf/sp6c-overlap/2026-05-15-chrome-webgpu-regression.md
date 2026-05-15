Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: `Environmental — TWO layered issues; second one requires user action`
Date: 2026-05-15
Status: **PARTIALLY RESOLVED -- second-layer BLOCKED on system action.**

## Layer 1: cwd / webdriver.json discovery — RESOLVED

The initial "GPUAdapter is not available" was operator error: running
`wasm-bindgen-test-runner` from the worktree ROOT instead of from
`examples/browser-prove/`. The runner searches for `webdriver.json`
relative to its cwd; from the wrong dir it falls back to default Chrome
capabilities (no WebGPU flags), so Chrome launches without
`enable-unsafe-webgpu`, `enable-features=Vulkan`, `use-angle=vulkan`,
`enable-dawn-features=allow_unsafe_apis,disable_robustness`, and no
WebGPU adapter is exposed. From `examples/browser-prove/` the runner
correctly emits `Try find webdriver.json... Ok` and Chrome launches
with all flags. **Canonical command at
`evidence/perf/r1-baselines/README.md` already includes the cd-step --
never run the runner from elsewhere.**

## Layer 2: Chrome `vkCreateInstance` fails -7 on the GPU — BLOCKING

Even with the correct flags applied, Chrome's GPU subprocess fails
Vulkan instance creation:

```
[ERROR:gpu/vulkan/vulkan_instance.cc:200] vkCreateInstance() failed: -7
[ERROR:gpu/ipc/service/gpu_init.cc:1408] Failed to create and initialize Vulkan implementation.
```

`-7` = `VK_ERROR_FEATURE_NOT_PRESENT`. Chrome falls back to SwiftShader
(CPU rasterizer), making smokes ~140× slower than the iter-5a
baseline (3.93s hello_world hit the 10-min WASM_BINDGEN_TEST_TIMEOUT).

**Confirmed NOT a Chrome version regression.** Both Chrome
148.0.7778.167 (current stable) and 148.0.7778.96 (the iter-5a
known-working chrome-for-testing build) fail IDENTICALLY:

| Chrome | Source | vkCreateInstance |
|---|---|---|
| 148.0.7778.167 | /opt/google/chrome (apt) | failed -7 |
| 148.0.7778.167 | chrome-for-testing /tmp/chrome-linux64 | failed -7 |
| 148.0.7778.96 | chrome-for-testing /tmp/chrome96/chrome-linux64 | failed -7 |

What was tried (none of which fix the chromedriver-spawned Chrome GPU
subprocess):
- `VK_LOADER_LAYERS_DISABLE='*'` (disable implicit MESA_device_select layer)
- `VK_LOADER_DRIVERS_DISABLE='*virtio*,*lvp*,*nouveau*,...'` (force NVIDIA ICD)
- `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json` (already set)
- `--vulkan-api-version=1.3` arg (flag accepted on parent but NOT forwarded to gpu-process)
- `--enable-features=Vulkan,VulkanFromANGLE` (crashes the GPU process with SIGSEGV)
- LD_PRELOAD of system libvulkan.so.1.3.275 in place of Chrome's bundled 1.4.348
- Wrapper script setting env before exec'ing Chrome (Chrome's GPU subprocess strips env)
- Both stable Chrome and chrome-for-testing binaries

System state:
- NVIDIA driver: `580.159.03` (loaded), `Apr 24 13:22` install date
- Vulkan loader (system): `libvulkan1 1.3.275.0-1build1` (very old)
- Chrome bundled Vulkan loader: 1.4.348
- NVIDIA ICD api_version: 1.4.312 (`/usr/share/vulkan/icd.d/nvidia_icd.json`)
- Implicit layers: NVIDIA layers + VkLayer_MESA_device_select
- `apt list --installed` shows `libnvidia-cfg1-580 580.159.03 [upgradable to 580.159.04]`
- Kernel: `6.17.0-23-generic`
- RTX 5090 (visible to nvidia-smi, 32607 MiB available)

## Resolution paths (REQUIRE USER ACTION)

Most likely: NVIDIA 580.159.03 has a Vulkan instance-feature reporting
regression that Chrome trips on. Apt has 580.159.04 ready.

1. **Try NVIDIA driver patch update (most likely fix):**
   ```bash
   sudo apt-get install --only-upgrade 'libnvidia-*' nvidia-driver-580-open
   sudo reboot   # reload kernel module
   ```
2. **Or pin a different driver branch:**
   ```bash
   sudo apt-get install nvidia-driver-565-open  # if available
   ```
3. **Or wait for the next libvulkan1 SRU** (current 1.3.275 from 2024 is very stale).

Until one of these lands, all smoke runs will fall back to SwiftShader
CPU and complete in tens of minutes instead of seconds. SP7 iter-6a/6c
landed work is verified by **naga + cargo test on host** but not by
end-to-end smoke A/B. xgboost perf cannot be re-measured.

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
