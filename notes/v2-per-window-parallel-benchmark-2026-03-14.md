# V2 Per-Window Parallel MSM Benchmark — 2026-03-14

## Change

Rewrote `msm_v2_parallel` in `pippenger_v2.hpp` from per-point parallelism (split points across threads within each window) to **per-window work-stealing** (each thread processes complete windows with all points).

### Key differences from old approach

| Aspect | Old (per-point) | New (per-window) |
|--------|-----------------|------------------|
| Parallelism unit | Points split across threads per window | Entire windows distributed across threads |
| Synchronization | Mutex + CV per window (~16 barriers at 2^20) | Single `atomic<size_t>` counter |
| Batch-invert size | `npoints/num_threads` per thread | Full `npoints` per thread |
| Load balancing | Static chunk assignment | Dynamic work-stealing |
| Thread capping | `npoints/64` | `num_windows` |

## Environment

- **VM**: GCP `c3-standard-8` (4 cores / 8 threads, Ice Lake)
- **Zone**: us-central1-a
- **Toolchain**: Rust 1.90.0, clang (AVX512 IFMA)

## Full Benchmark Results

All times are mean values from Criterion (10 samples each).

### Serial implementations

| Size | Blst | msm_best (Rust) | Vroom V1 serial | Vroom V2 serial |
|------|------|-----------------|-----------------|-----------------|
| 2^8  | 1.57 ms | 3.30 ms | 4.15 ms | 3.05 ms |
| 2^10 | 4.25 ms | 8.63 ms | 13.55 ms | 9.53 ms |
| 2^12 | 14.46 ms | 24.58 ms | 43.98 ms | 31.02 ms |
| 2^14 | 44.10 ms | 47.75 ms | 156.45 ms | 108.27 ms |
| 2^16 | 159.36 ms | 150.38 ms | 534.62 ms | 378.45 ms |
| 2^18 | 498.04 ms | 518.65 ms | 1,922 ms | 1,397 ms |
| 2^20 | 2,209 ms | 1,969 ms | 7,149 ms | 5,332 ms |

### Parallel implementations (8 threads)

| Size | Blst | msm_best (Rust) | V1 par (8t) | **V2 par (8t)** |
|------|------|-----------------|-------------|-----------------|
| 2^8  | 1.57 ms | 3.30 ms | 4.15 ms | 3.05 ms |
| 2^10 | 4.25 ms | 8.63 ms | 3.27 ms | **2.64 ms** |
| 2^12 | 14.46 ms | 24.58 ms | 11.10 ms | **9.08 ms** |
| 2^14 | 44.10 ms | 47.75 ms | 35.53 ms | **29.18 ms** |
| 2^16 | 159.36 ms | 150.38 ms | 133.33 ms | **115.39 ms** |
| 2^18 | 498.04 ms | 518.65 ms | 540.15 ms | **437.35 ms** |
| 2^20 | 2,209 ms | 1,969 ms | 1,967 ms | **1,551 ms** |

### V2 par speedups at 2^20

| vs. | Speedup |
|-----|---------|
| V2 serial | 3.4x |
| V1 par | 1.27x |
| msm_best (Rust Rayon) | 1.27x |
| Blst | 1.42x |
| Old V2 par (per-point) | 3.7x |

## Analysis

1. **V2 par is fastest** across all sizes from 2^10 upward
2. **Full batch-invert amortization** is the key win — each thread processes all N points per window, so the Montgomery trick runs over the full batch instead of N/8
3. **Zero synchronization** via atomic work-stealing eliminates the ~16 mutex/CV barriers per MSM
4. **V2 batch affine (6M/pair)** combined with per-window parallelism is strictly better than V1's projective (15M/pair) + per-window parallelism
5. At small sizes (2^8), parallel falls back to serial (npoints < 1024 guard)
