# SP7fe - Recursion exec selector plan

## Question

After SP7fa rejected monolithic recursion `exec` WGSL and SP7fb showed selector chunking is browser-capacity safe, choose the smallest runtime slice that can plausibly reduce proving wall time without repeating the SP7fd CPU-exec + GPU-verify_mem upload regression.

## Static selector facts

Source inspected: `/tmp/sp7fa-recursion-wgsl-exec/step_exec.wgsl`.

| Selector | Meaning | WGSL bytes | Exec externs | Data backrefs | Immediate interpretation |
|---|---:|---:|---|---|---|
| `code[1]` / `x5` | `micro_ops` | 88,135 | `womRead=45`, `womWrite=36`, `readIOPHeader=3`, `readIOPBody=3` | one `back=1` | large share, but needs real IOP and WOM rows |
| `code[2]` / `x429` | `macro_ops` | 298,323 | `womRead=25`, `womWrite=5` | many `back` values up to 68 | capacity-safe as a chunk, but small selector share and WOM-heavy |
| `code[3]` / `x2594` | `poseidon2_load` | 19,214 | `womRead=8` | 24 state `back=1` loads | part of a larger Poseidon2 chain |
| `code[4]` | `poseidon2_full` | 37,490 | none except `noop` | 24 state `back=1` loads | attractive in isolation, but depends on previous Poseidon2 state and feeds later Poseidon2 rows |
| `code[5]` | `poseidon2_partial` | 49,896 | none except `noop` | 24 state `back=1` loads | same chain dependency as `poseidon2_full` |
| `code[6]` / `x3780` | `poseidon2_store` | 7,114 | `womWrite=8` | 24 state `back=1` loads | end of the Poseidon2 chain; produces WOM/plonk rows |
| `code[7]` / `x3819` | checked bytes | 40,590 | `readCoefficients=1`, `womRead=1`, `womWrite=1` | 24 state `back=1` loads | needs byte-read preflight buffer plus WOM |

SP7cz's xgboost recursion selector histogram remains the best representative share data:

- `micro_ops`: 62.00%
- `poseidon2_full`: 17.43%
- `poseidon2_load`: 8.75%
- `poseidon2_partial`: 4.36%
- `poseidon2_store`: 3.49%
- `macro_ops`: 3.97%

The Poseidon2 chain therefore covers about 34.03% of recursion rows, while `macro_ops` covers only about 3.97%.

## Decision

Do not wire a standalone `poseidon2_full` or `x429` runtime replacement as the next candidate.

`poseidon2_full` has no exec-time WOM externs, but it is not independently promotable: CPU `poseidon2_store` depends on state produced by the preceding Poseidon2 rows. Skipping only `poseidon2_full` would require syncing GPU-produced state back to the CPU before `poseidon2_store`, which is the wrong dataflow for wall-time reduction.

`x429` is capacity-safe, but it is WOM-heavy and small-share. It would force the real WOM row problem without enough ceiling to justify being first.

The smallest candidate with a meaningful ceiling is the chunk-complete Poseidon2 chain:

`poseidon2_load -> poseidon2_full -> poseidon2_partial -> poseidon2_store`

That candidate only becomes viable if WOM row production stays GPU-resident through `verify_mem`. Otherwise it repeats SP7fd's rejected shape, where correctness was achieved but wall time regressed because the path uploaded full `recursion_data` and about 1 GiB of WOM rows.

## Next runtime requirement

Before promoting any recursion exec selector slice, implement or prove a GPU-resident WOM-row path:

- generated exec chunks write unsorted WOM rows on GPU;
- the row path does not upload full sorted WOM rows from CPU;
- generated `verify_mem` consumes GPU-resident rows;
- the e2e gate is BusyLoop + KeccakUnion first, then xgboost if the representative gate is correctness-clean and not obviously slower.

Accepted wall-time gain for this checkpoint: 0. This is a narrowing decision to avoid another upload-heavy false start.
