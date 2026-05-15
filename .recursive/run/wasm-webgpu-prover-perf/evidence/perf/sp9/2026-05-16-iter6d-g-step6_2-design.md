Run: `/.recursive/run/wasm-webgpu-prover-perf/`
Phase: SP7 iter 6d-g step 6.2 -- corrected design after per-arm back_Reg audit
Date: 2026-05-16

## Premise

Per the 2026-05-16 correction in
`2026-05-15-iter6d-g-step6_2-architectural-block.md`, 8 of 13 arm
deltas have ZERO internal `back_Reg` calls:

- MISC0, MISC1, MISC2 (3)
- MUL0, DIV0 (2)
- MEM0, MEM1 (2)
- ECALL0 (1)

For these arms, GPU witgen replacement is tractable. Only the
outer 5 cells in `exec_TopChunk0` need shadow-init from preflight.

## Concrete steps

### Step 6.2.0: shadow-init kernel + per-cycle preflight buffer

Add a new GPU kernel `iter6d_g_shadow_init` that, given a side
buffer of `(pc, state, machine_mode, minor)` per cycle, writes the
5 outer Top layout cells at column offsets 14-18:

```wgsl
@group(0) @binding(0) var<storage, read_write> data_buf: array<u32>;
@group(0) @binding(1) var<uniform> params: WitgenParams;
@group(0) @binding(2) var<storage, read> preflight_meta: array<u32>;
// preflight_meta layout: 4 u32 per cycle: [pc, state, mode, minor]

@compute @workgroup_size(64)
fn iter6d_g_shadow_init_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let cycle = gid.x;
  if (cycle >= params.data_rows) { return; }
  let base = cycle * 4u;
  let pc = preflight_meta[base + 0u];
  let state = preflight_meta[base + 1u];
  let mode = preflight_meta[base + 2u];
  // shadow-init: cycle N's outer next* cells get cycle (N+1)'s values
  // back_Reg(1, ...) at cycle (N+1) reads cycle N's stored values
  // Special-case last cycle: (N+1) wraps to 0 via the % rows in back_NondetReg
  let next_cycle = (cycle + 1u) % params.data_rows;
  let next_base = next_cycle * 4u;
  let next_pc = preflight_meta[next_base + 0u];
  let next_state = preflight_meta[next_base + 1u];
  let next_mode = preflight_meta[next_base + 2u];
  let pc_low = next_pc & 0xFFFFu;
  let pc_high = (next_pc >> 16u) & 0xFFFFu;
  // Write Montgomery-encoded values to data_buf[col*rows + cycle]
  data_buf[14u * params.data_rows + cycle] = encode(pc_low);    // nextPcLow
  data_buf[15u * params.data_rows + cycle] = encode(pc_high);   // nextPcHigh
  data_buf[16u * params.data_rows + cycle] = encode(next_state); // nextState_0
  data_buf[17u * params.data_rows + cycle] = encode(next_mode);  // nextMachineMode
  // isFirstCycle: 1 at cycle 0, 0 elsewhere
  let is_first = select(0u, 1u, cycle == 0u);
  data_buf[18u * params.data_rows + cycle] = encode(is_first);
}
```

Rust side: build `preflight_meta` Vec from `preflight.cycles`,
upload once at start of `dispatch_witgen_per_arm_probe`,
dispatch shadow_init kernel before per-arm dispatches.

### Step 6.2.1: InstInputStruct synthesis in per-arm wrapper

For each of 8 zero-back_Reg arms, replace the no-op wrapper with
real synthesis that calls the arm sub-fn:

```wgsl
@compute @workgroup_size(64)
fn iter6d_g_misc0_chunk0_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let lane = gid.x;
  if (lane >= arrayLength(&cycle_list)) { return; }
  cycle = cycle_list[lane];
  if (cycle >= params.data_rows) { return; }
  let bound_top = BoundLayout_TopLayout(kLayout_Top, buf_data);
  // Read shadow values just initialized (pre-populated, equivalent to
  // what step_exec at cycle N-1 would have written to nextPc/State/Mode).
  let nextPcLow = back_Reg(1, lookup_TopLayout_nextPcLow(bound_top));
  let nextPcHigh = back_Reg(1, lookup_TopLayout_nextPcHigh(bound_top));
  let nextState_0 = back_Reg(1, lookup_TopLayout_nextState_0(bound_top));
  let nextMachineMode = back_Reg(1, lookup_TopLayout_nextMachineMode(bound_top));
  let isFirstCycle = back_NondetReg(0, lookup_TopLayout_isFirstCycle(bound_top));
  let x4 = sub(MONT_268435454, isFirstCycle._super);  // (1 - isFirstCycle)
  // Read major/minor from preflight_meta
  let base = cycle * 4u;
  let major = encode(preflight_meta[base + 3u] >> 16u);  // upper 16 = major
  let minor = encode(preflight_meta[base + 3u] & 0xFFFFu);  // lower 16 = minor
  let pcU32 = ValU32Struct(mul(x4, nextPcLow._super), mul(x4, nextPcHigh._super));
  let state = mul(x4, nextState_0._super);
  let mode = add(mul(x4, nextMachineMode._super), isFirstCycle._super);
  let inst_input = exec_InstInput(major, minor, pcU32, state, mode,
                                  lookup_TopLayout_instInput(bound_top));
  let nondet0 = back_Reg(0, lookup_TopCycleLayout__super(lookup_TopLayout_cycleRedef(bound_top)));
  let _result = exec_Misc0Chunk0(nondet0, inst_input,
                                 lookup_TopInstResultLayout_arm0(lookup_TopLayout_instResult(bound_top)));
}
```

### Step 6.2.2: Validate bit-exact against rust_steps

Smoke test: run xgboost with probe enabled, dump data_buf
checksum after GPU witgen + before rust_steps overwrite. Compare
to rust_steps-only output checksum. For cells touched by Misc0/etc
arms, output should match.

### Step 6.2.3: Short-circuit rust_steps for the 8 arms

Once 6.2.2 passes, gate `step_exec` to skip cycles whose major
opcode is in {0, 1, 2, 3, 4, 5, 6, 8} (MISC0/1/2, MUL0, DIV0,
MEM0/1, ECALL0).

Expected wall savings on xgboost: ~80% of 5.7 s = **~4.5 s**
(102.6 s → 98 s ≈ 17.4× CUDA).

### Step 6.2.4: Repeat for chunk1 arms

The 8 arms also have chunk1 variants. Repeat the synthesis for
chunk1 wrappers. Should be straightforward given the chunk0
template.

## Complications

- **Encoding**: All Vals stored in data_buf are Montgomery-encoded
  (R = 268435454 in BabyBear). The shadow_init kernel must use
  `encode()` (defined in witgen_baseline.wgsl line 138).
- **major→FE mapping**: u8 major (0..12) doesn't trivially map to
  the 13 onehot values (0, 134217711, ...). Verify by checking
  what `extern_getMajorMinor()` returns in rust_steps:
  `(Val::new(major as u32), Val::new(minor as u32))` (rust_steps.rs:370-372).
  So Val::new(major) is the field element. Val::new is just
  Montgomery encoding of the u32 input. So `encode(major)` in
  WGSL gives the same value. Good.
- **Wrap-around**: cycle (N+1) % rows for the last cycle. The
  back_Reg(1, ...) computes (cycle - 1 + rows) % rows. So we
  shadow-init at cycle N with cycle (N+1)'s values. For cycle 0,
  we shadow-init with cycle 1's values; for the last cycle, with
  cycle 0's values. But the actual back_Reg(1) at cycle 0 reads
  the LAST cycle's stored values. To be consistent: at cycle
  index i, write cell value that the cycle (i+1) % rows back_Reg(1)
  call should retrieve. So `data_buf[col*rows + i] =
  preflight.cycles[(i+1) % rows].field`. That matches the WGSL above.

## Estimated effort

- 6.2.0 (shadow-init kernel + buffer): ~150 lines, ~2 hours
- 6.2.1 (InstInputStruct synthesis × 8 arms): ~50 lines/arm × 8 = ~400 lines, ~4 hours
- 6.2.2 (bit-exact validation): smoke test + checksum compare, ~2 hours
- 6.2.3 (rust_steps short-circuit): trivial gate, ~1 hour
- 6.2.4 (chunk1 repeat): copy of 6.2.1 with chunk1 variants, ~3 hours

**Total: 12 hours** (multi-day if iterating with full xgboost test cycle).

Path is genuinely tractable. Most likely failure mode: something
about Montgomery encoding I'm missing, or a back_Reg(N>1) read
hidden somewhere I missed.
