# V2 Point-Parallel MSM Benchmark (16 threads) -- 2026-03-15

## Overview

New **point-parallel** strategy: partitions points across threads (each thread runs independent sub-MSM on N/T points) instead of distributing windows. This reduces per-thread working set from 16MB to 1MB (fits in L2 cache) and total memory traffic from ~4GB to ~256MB.

## Environment

- **VM**: GCP `c3-standard-22`
- **CPU**: Intel Xeon Platinum 8481C (Sapphire Rapids) @ 2.70GHz, AVX512 IFMA
- **Threads**: 16 (pinned via `taskset -c 0-15`)
- **Zone**: us-central1-a
- **Toolchain**: Rust 1.90.0, clang (AVX512 IFMA)

## Benchmark Results

All times are mean values from Criterion (10 samples each).

### Full comparison (16 threads)

| Size | Blst | V2 serial | V2 par (per-window) | **V2 pp (point-parallel)** | pp vs par |
|------|------|-----------|---------------------|---------------------------|-----------|
| 2^8  | 0.81 ms | 3.05 ms | 3.05 ms | 3.05 ms | 1.00x (fallback) |
| 2^10 | 2.28 ms | 9.54 ms | 1.61 ms | 3.15 ms | 0.51x |
| 2^12 | 6.77 ms | 31.1 ms | 5.01 ms | 6.63 ms | 0.76x |
| 2^14 | 22.9 ms | 108 ms | 18.2 ms | 20.4 ms | 0.89x |
| 2^16 | 68.3 ms | 379 ms | 57.3 ms | 62.7 ms | 0.91x |
| 2^18 | 246 ms | 1,402 ms | 256 ms | **210 ms** | **1.22x** |
| 2^20 | 794 ms | 5,349 ms | 917 ms | **621 ms** | **1.48x** |

### Point-parallel vs Blst (large sizes)

| Size | Blst | V2 point-parallel | pp vs Blst |
|------|------|-------------------|------------|
| 2^18 | 246 ms | 210 ms | **1.17x faster** |
| 2^20 | 794 ms | 621 ms | **1.28x faster** |

## Analysis

- **At 2^20**: 621ms vs 917ms per-window = **1.48x faster**, beats Blst's 794ms by **28%**
- **At 2^18**: 210ms vs 256ms per-window = **1.22x faster**, beats Blst's 246ms by **17%**
- **At 2^16 and below**: per-window parallelism still wins because working sets fit in cache and Horner evaluation runs in parallel
- **Crossover**: point-parallel becomes advantageous around k=17-18 where point array exceeds L3 cache (~33MB)

### Why point-parallel wins at large N

Per-window parallel (16t, 2^20 points):
- Each thread reads ALL 2^20 points (256MB total reads across 16 threads = 4GB)
- Per-thread bucket working set: 16MB (exceeds L2, thrashes L3)

Point-parallel (16t, 2^20 points):
- Each thread reads only 2^20/16 = 2^16 points (16MB total reads = 256MB)
- Per-thread bucket working set: ~1MB (fits in L2 cache)
- Trade-off: T-1 = 15 extra PointAdd operations for final summation (negligible)

### Why per-window wins at small N

- Working sets fit in L2 regardless of strategy
- Per-window allows Horner evaluation in parallel (no sequential reduction)
- Point-parallel creates sub-optimal smaller Pippenger instances (fewer points per bucket)

## Implementation

- `msm_v2_point_parallel()` in `pippenger_v2.hpp`: partitions points into contiguous chunks, each thread calls `msm_v2()`, results summed
- Falls back to `msm_v2()` for npoints < 1024 or single thread
- MIN_CHUNK = 256 to keep Pippenger efficient per thread

## Next Steps

- Hybrid strategy: auto-select per-window vs point-parallel based on npoints
- Potential crossover heuristic: `if npoints > 2^17 { point_parallel } else { per_window }`
