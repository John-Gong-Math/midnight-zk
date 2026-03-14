# V2 Per-Window Parallel MSM Benchmark (16 threads) — 2026-03-14

## Environment

- **VM**: GCP `c3-standard-22` (resized from c3-standard-8)
- **CPU**: Intel Xeon Platinum 8481C (Sapphire Rapids) @ 2.70GHz, AVX512 IFMA
- **Threads**: 16 (pinned via `taskset -c 0-15`)
- **Zone**: us-central1-a
- **Toolchain**: Rust 1.90.0, clang (AVX512 IFMA)

## Full Benchmark Results

All times are mean values from Criterion (10 samples each).

### Serial implementations

| Size | Blst | msm_best (Rust Rayon) | Vroom V1 serial | Vroom V2 serial |
|------|------|----------------------|-----------------|-----------------|
| 2^8  | 0.81 ms | 2.25 ms | 4.15 ms | 3.05 ms |
| 2^10 | 2.28 ms | 5.37 ms | 13.54 ms | 9.54 ms |
| 2^12 | 6.71 ms | 15.07 ms | 44.01 ms | 31.03 ms |
| 2^14 | 23.04 ms | 25.04 ms | 156.72 ms | 108.30 ms |
| 2^16 | 68.30 ms | 74.37 ms | 534.00 ms | 379.32 ms |
| 2^18 | 245.63 ms | 256.09 ms | 1,923 ms | 1,423 ms |
| 2^20 | 794.17 ms | 1,010 ms | 7,529 ms | 5,785 ms |

### Parallel implementations (16 threads)

| Size | Blst | msm_best (Rust Rayon) | V1 par (16t) | **V2 par (16t)** |
|------|------|----------------------|--------------|------------------|
| 2^8  | 0.81 ms | 2.25 ms | 4.15 ms | 3.05 ms |
| 2^10 | 2.28 ms | 5.37 ms | 2.01 ms | **1.61 ms** |
| 2^12 | 6.71 ms | 15.07 ms | 6.51 ms | **4.99 ms** |
| 2^14 | 23.04 ms | 25.04 ms | 21.95 ms | **18.48 ms** |
| 2^16 | 68.30 ms | 74.37 ms | 71.56 ms | **57.78 ms** |
| 2^18 | 245.63 ms | 256.09 ms | 322.09 ms | **265.72 ms** |
| 2^20 | 794.17 ms | 1,010 ms | 1,255 ms | **1,023 ms** |

### V2 par speedup vs msm_best (Rust Rayon)

| Size | msm_best | V2 par (16t) | Speedup |
|------|----------|--------------|---------|
| 2^10 | 5.37 ms | 1.61 ms | **3.34x** |
| 2^12 | 15.07 ms | 4.99 ms | **3.02x** |
| 2^14 | 25.04 ms | 18.48 ms | **1.35x** |
| 2^16 | 74.37 ms | 57.78 ms | **1.29x** |
| 2^18 | 256.09 ms | 265.72 ms | **0.96x** |
| 2^20 | 1,010 ms | 1,023 ms | **0.99x** |

### V2 par speedup vs V1 par

| Size | V1 par (16t) | V2 par (16t) | Speedup |
|------|--------------|--------------|---------|
| 2^10 | 2.01 ms | 1.61 ms | **1.25x** |
| 2^12 | 6.51 ms | 4.99 ms | **1.30x** |
| 2^14 | 21.95 ms | 18.48 ms | **1.19x** |
| 2^16 | 71.56 ms | 57.78 ms | **1.24x** |
| 2^18 | 322.09 ms | 265.72 ms | **1.21x** |
| 2^20 | 1,255 ms | 1,023 ms | **1.23x** |

### V2 par: 8t vs 16t scaling

| Size | V2 par (8t) | V2 par (16t) | Scaling |
|------|-------------|--------------|---------|
| 2^10 | 2.64 ms | 1.61 ms | 1.64x |
| 2^12 | 9.08 ms | 4.99 ms | 1.82x |
| 2^14 | 29.18 ms | 18.48 ms | 1.58x |
| 2^16 | 115.39 ms | 57.78 ms | 2.00x |
| 2^18 | 437.35 ms | 265.72 ms | 1.65x |
| 2^20 | 1,551 ms | 1,023 ms | 1.52x |

## Analysis

1. **V2 par (16t) beats everything at 2^10–2^16**, including Blst (single-threaded C with hand-optimized assembly)
2. **At 2^18–2^20**, V2 par and msm_best converge (~equal) — Rayon's thread pool + Rust batch affine MSM is extremely competitive at large sizes
3. **V2 par consistently 1.2–1.3x faster than V1 par** across all sizes, confirming batch affine (6M/pair) advantage over projective (15M/pair)
4. **8t to 16t scaling**: 1.5–2.0x improvement from doubling threads — good parallel efficiency, near-linear at 2^16
5. **Blst dominance at 2^8**: At tiny sizes, Blst's optimized serial assembly wins since parallel overhead isn't amortized. V2 par correctly falls back to serial (npoints < 1024 guard)
