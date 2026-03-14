# V1 Per-Window Parallel MSM Benchmark Results

**Date:** 2026-03-14
**Machine:** c3-standard-8 (Sapphire Rapids, 4 cores / 8 threads)
**CPU:** 8 x 2700 MHz, L3 107520 KiB
**Points:** 2^20 (1,048,576)
**Compiler:** clang++ -std=c++20 -O3 -march=native -mavx512ifma

## Results

| Benchmark | Time (ms) | Notes |
|---|---|---|
| BM_VROOM_V1_Pippenger | 7119 | V1 serial |
| **BM_VROOM_V1_Pippenger_Parallel** | **1962** | **V1 per-window parallel (NEW)** |
| BM_VROOM_MSM | 5313 | V1 serial (via msm.hpp) |
| BM_VROOM_MSM_Parallel | 5799 | V1 old per-point parallel (via msm.hpp) |
| BM_VROOM_MSM_V2 | 5293 | V2 batch affine serial |
| BM_VROOM_MSM_V2_Parallel | 5788 | V2 batch affine parallel |
| BM_BLST_Pippenger | 8120 | BLST reference |

## Key Takeaways

- **3.6x speedup** over V1 serial (7119ms -> 1962ms)
- **2.7x faster** than V1/V2 serial via msm.hpp (5313ms -> 1962ms)
- **4.1x faster** than BLST (8120ms -> 1962ms)
- Old per-point parallel was **slower** than serial (5799ms vs 5313ms); new per-window approach fixes this entirely

## What Changed

Replaced per-point parallelism in `pippenger_msm_parallel` (pippenger.hpp) with per-window parallelism:

- **Old approach:** For each of ~16 windows, spawn threads that each scatter a chunk of points into independent bucket arrays, then sequentially integrate and merge per-thread results. Caused 112 thread create/join ops, sequential integration bottleneck, and L3 cache thrashing from multiple bucket arrays.
- **New approach:** Each thread processes complete windows (scatter all points + integrate) using an `atomic<size_t>` work-stealing counter. Thread-local buckets are reused across windows for cache warmth. Final Horner reduction is sequential (~255 doublings). Zero inter-thread synchronization during compute.

## Correctness

All tests pass including new V1 parallel correctness tests at sizes {256, 1024, 4096, 65536} with thread counts {2, 4, 8}.

## Note on BM_VROOM_V1_Pippenger vs BM_VROOM_MSM difference

`BM_VROOM_V1_Pippenger` (7119ms) calls `pippenger_msm()` directly, while `BM_VROOM_MSM` (5313ms) calls `msm()` which goes through msm.hpp. The ~1800ms difference suggests msm.hpp has additional optimizations (e.g. different wbits selection or integration path) that the raw pippenger.hpp serial path doesn't have. The parallel version benefits from all points being processed per-window regardless.
