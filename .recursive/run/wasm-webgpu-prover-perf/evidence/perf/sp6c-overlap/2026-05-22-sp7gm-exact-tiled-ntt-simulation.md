# SP7gm Exact Tiled NTT Simulation

Date: 2026-05-22

## Purpose

Extend SP7gd/SP7gj static arithmetic evidence to the exact guarded tiled strided-local NTT shapes, including the largest `n_bits=20` hot path.

No production runtime code changed. Current accepted working state remains SP7fr. Accepted wall-time gain: 0.

## Scratch Program

Scratch source:

```text
/tmp/sp7gm_exact_tiled_ntt.rs
```

Compiled binary:

```text
/tmp/sp7gm_exact_tiled_ntt
```

The scratch program implements BabyBear arithmetic over `P = 2013265921`, uses the repo `ROU_FWD` constants, and compares three paths:

1. expanded input followed by the full forward staged NTT from `expand_bits + 1` through `n_bits`;
2. the current WebGPU shape: local fused prefix through `fused_bits=10`, then global staged NTT;
3. the SP7gj tiled strided-local shape: same local prefix, then tiled block-index butterflies with `s_original = block_s * 1024 + intra`.

The data pattern is deterministic and non-constant:

```text
input[i] = (i * 1103515245 + 12345) mod P
```

## Commands

Compile:

```text
rustc -O /tmp/sp7gm_exact_tiled_ntt.rs -o /tmp/sp7gm_exact_tiled_ntt
```

Run:

```text
/tmp/sp7gm_exact_tiled_ntt
```

Result:

```text
n_bits=16 fused=10 expand_bits=2 offsets_per_tile=16 current_match=true tiled_match=true checksum=5164251640937000354
n_bits=18 fused=10 expand_bits=2 offsets_per_tile=4 current_match=true tiled_match=true checksum=15514549705846934455
n_bits=20 fused=10 expand_bits=2 offsets_per_tile=1 current_match=true tiled_match=true checksum=1212977098799693622
```

## Interpretation

This strengthens the SP7gj static evidence:

- `n_bits=16`, `offsets_per_tile=16` covers the KeccakUnion `domain=65536` check bucket.
- `n_bits=18`, `offsets_per_tile=4` covers the intermediate full-block extension if it appears in future logs.
- `n_bits=20`, `offsets_per_tile=1` covers the largest xgboost and KeccakUnion FRI/check hot path.

All exact guarded shapes matched the current staged WebGPU arithmetic and the full staged forward NTT model.

## Remaining Risk

This is still not runtime acceptance evidence. Browser-specific risks remain:

- strided global memory may underperform despite fewer passes;
- Dawn/Chrome validation, pipeline compilation, and queue behavior may differ from static arithmetic;
- correctness must still be proven by focused browser HAL parity and representative proof generation.

Do not retain runtime performance code until SP7gf browser gates are available and pass.
