# Auto-Parallel MSM Benchmark Results

**Date**: 2026-03-15
**VM**: vroom-c3-22 (GCP c3, us-central1-a), 16 cores pinned via `taskset -c 0-15`
**Branch**: vroom-bench

## Changes

- Added `msm_v2_auto_parallel` in `pippenger_v2.hpp`: auto-dispatches between per-window parallel (N < 2^17) and point-parallel (N >= 2^17)
- Added `msm_auto_parallel` public API wrapper in `msm.hpp`
- Added FFI wrappers `vroom_g1_msm_auto_parallel` and `vroom_g1_msm_auto_parallel_matches_serial` in `wrapper_msm.cpp`
- Added Rust FFI declarations and correctness test `vroom_auto_parallel_matches_serial_sweep` in `lib.rs`
- Added `Vroom_pp_sweep` (thread count sweep T=4,8,12) and `Vroom_auto` benchmark groups in `curves/benches/msm.rs`

## Full Benchmark Results

### All algorithms, all sizes

| k | N | Blst | msm_best | Vroom 1t | Vroom_v1 1t | Vroom_v1_par 16t | Vroom_par 16t | Vroom_pp 16t | Vroom_auto 16t |
|---|---|------|----------|----------|-------------|------------------|---------------|--------------|----------------|
| 8 | 256 | 0.81ms | 2.20ms | 3.05ms | 4.16ms | 4.15ms | 3.06ms | 3.05ms | 3.05ms |
| 10 | 1K | 2.28ms | 5.30ms | 9.54ms | 13.56ms | 1.98ms | 1.62ms | 3.15ms | 1.62ms |
| 12 | 4K | 6.73ms | 14.96ms | 31.12ms | 44.02ms | 6.55ms | 5.04ms | 6.57ms | 5.08ms |
| 14 | 16K | 23.07ms | 25.00ms | 108.37ms | 156.71ms | 21.86ms | 18.25ms | 20.21ms | 18.26ms |
| 16 | 64K | 68.32ms | 68.32ms | 377.46ms | 535.51ms | 71.84ms | 56.57ms | 64.20ms | 57.02ms |
| 18 | 256K | 245.36ms | 253.46ms | 1391.7ms | 1914.4ms | 318.79ms | 259.44ms | 206.37ms | 207.06ms |
| 20 | 1M | 792.61ms | 1003.6ms | 5361.9ms | 7226.0ms | 1131.0ms | 921.03ms | 629.63ms | 638.99ms |

### Thread sweep (point-parallel only, k >= 14)

| k | N | pp 4t | pp 8t | pp 12t | pp 16t |
|---|---|-------|-------|--------|--------|
| 14 | 16K | 31.24ms | **17.53ms** | 24.67ms | 20.21ms |
| 16 | 64K | 108.55ms | **59.15ms** | 64.21ms | 64.20ms |
| 18 | 256K | 379.76ms | **203.64ms** | 239.07ms | 206.37ms |
| 20 | 1M | 1448.2ms | 758.37ms | 763.66ms | **629.63ms** |

## Key Findings

1. **Auto-dispatch works correctly** — picks per-window for k<=16, point-parallel for k>=18 (threshold at 2^17). Matches the best strategy at every size.

2. **8 threads is surprisingly competitive** — at k=14 (17.5ms vs 20.2ms), k=16 (59ms vs 64ms), and k=18 (204ms vs 206ms). Only at k=20 does 16t clearly win (630ms vs 758ms).

3. **12 threads is generally worse than 8t** — likely due to NUMA/L3 contention on this CPU.

4. **Point-parallel at 16t/2^20 = 630ms** remains the best overall, beating Blst (793ms) by 21%.

5. **No speedup to point-parallel itself** — the algorithm is unchanged. The auto-dispatch eliminates the manual strategy choice.

6. **Future optimization**: auto-dispatch could also select thread count (8t for mid-range, 16t for 2^20+).
