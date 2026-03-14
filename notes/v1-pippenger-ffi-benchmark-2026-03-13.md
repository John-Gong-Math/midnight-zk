# V1 Pippenger FFI Benchmark Results

**Date:** 2026-03-13
**Machine:** c3-standard-8 (Sapphire Rapids, 4 cores / 8 threads)
**Points:** BLS12-381 G1, 256-bit scalars
**Benchmark:** `cargo bench -p midnight-curves --bench msm -- --noplot`

## Summary

Added V1 pippenger (`pippenger_msm` / `pippenger_msm_parallel`) FFI wrappers and Rust benchmarks alongside existing V2 (`msm` / `msm_parallel`) and Rust MSM implementations.

## Full Results

| Size | Blst Rust | msm_best | Vroom V2 (1t) | Vroom V2 par (8t) | Vroom V1 (1t) | Vroom V1 par (8t) |
|:---:|---:|---:|---:|---:|---:|---:|
| 2^8 | 1.56 ms | 3.26 ms | 3.05 ms | 3.05 ms | 4.15 ms | 4.15 ms |
| 2^10 | 4.25 ms | 8.62 ms | 9.54 ms | 21.41 ms | 13.54 ms | 3.24 ms |
| 2^12 | 14.23 ms | 24.63 ms | 30.98 ms | 56.31 ms | 43.99 ms | 11.10 ms |
| 2^14 | 43.97 ms | 47.38 ms | 108.26 ms | 142.44 ms | 156.57 ms | 35.50 ms |
| 2^16 | 158.62 ms | 147.31 ms | 376.89 ms | 461.78 ms | 535.44 ms | 133.72 ms |
| 2^18 | 497.74 ms | 512.55 ms | 1,391 ms | 1,599 ms | 1,924 ms | 541 ms |
| 2^20 | 2,207 ms | 1,961 ms | 5,386 ms | 5,780 ms | 7,209 ms | 1,978 ms |

## V1 Parallel Speedup over V1 Serial

| Size | V1 serial | V1 parallel (8t) | Speedup |
|:---:|---:|---:|---:|
| 2^8 | 4.15 ms | 4.15 ms | 1.0x |
| 2^10 | 13.54 ms | 3.24 ms | 4.2x |
| 2^12 | 43.99 ms | 11.10 ms | 4.0x |
| 2^14 | 156.57 ms | 35.50 ms | 4.4x |
| 2^16 | 535.44 ms | 133.72 ms | 4.0x |
| 2^18 | 1,924 ms | 541 ms | 3.6x |
| 2^20 | 7,209 ms | 1,978 ms | 3.6x |

## V1 Parallel vs msm_best Rust

| Size | V1 parallel | msm_best | Winner |
|:---:|---:|---:|:---|
| 2^8 | 4.15 ms | 3.26 ms | msm_best |
| 2^10 | 3.24 ms | 8.62 ms | **V1 par (2.7x)** |
| 2^12 | 11.10 ms | 24.63 ms | **V1 par (2.2x)** |
| 2^14 | 35.50 ms | 47.38 ms | **V1 par (1.3x)** |
| 2^16 | 133.72 ms | 147.31 ms | **V1 par (1.1x)** |
| 2^18 | 541 ms | 513 ms | msm_best |
| 2^20 | 1,978 ms | 1,961 ms | ~tie |

## Key Findings

1. **V1 parallel confirms 3.6x speedup** over V1 serial at 2^20 via FFI — matches native C++ benchmark
2. **V1 parallel is the fastest VROOM variant** at large sizes, beating V2 serial (5.4s) and V2 parallel (5.8s) at 2^20
3. **V1 parallel beats msm_best Rust** at 2^10 through 2^16, essentially ties at 2^20
4. **V2 parallel is slower than V2 serial** — same per-point parallelism issue that V1 had before the per-window rewrite
5. Parallelism kicks in at 2^10+ with ~4x speedup on 4 cores / 8 threads

## Next Steps

- Apply per-window parallelism to V2 batch affine backend — should combine parallel speedup with better per-point arithmetic
- This could bring 2^20 down to ~1.4-1.5s range, clearly beating msm_best
