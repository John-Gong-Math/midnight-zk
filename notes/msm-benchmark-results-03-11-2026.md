# MSM Benchmark Results — March 11, 2026

BLS12-381 G1, 256-bit scalars, 2^20 (1,048,576) points.
GCP VMs: Intel Ice Lake @ 2.60GHz, AVX-512 IFMA supported.

## 4-core VM (n2-standard-4)

| Implementation | Time | Notes |
|:---|:---:|:---|
| VROOM C++ (parallel) | **381 ms** | Native, auto threads, clang++ -O3 -mavx512ifma |
| VROOM C++ (1 thread) | **605 ms** | Native, AVX512-IFMA |
| BLST C (Pippenger) | **635 ms** | Native single-threaded via C API |
| Blst Rust (multi_exp) | 3,744 ms | Rayon multi-threaded (4 cores) |
| msm_best Rust | 4,080 ms | Rayon multi-threaded (4 cores) |
| VROOM FFI (4t parallel) | 10,246 ms | Via Rust FFI wrapper |
| VROOM FFI (1 thread) | 17,917 ms | Via Rust FFI wrapper |

## 16-core VM (n2-standard-16)

| Implementation | Time | Notes |
|:---|:---:|:---|
| VROOM C++ (parallel) | **394 ms** | Native, auto threads |
| VROOM C++ (1 thread) | **570 ms** | Native, AVX512-IFMA |
| BLST C (Pippenger) | **616 ms** | Native single-threaded via C API |
| Blst Rust (multi_exp) | 952 ms | Rayon multi-threaded (16 cores) |
| msm_best Rust | 1,373 ms | Rayon multi-threaded (16 cores) |
| VROOM FFI (16t parallel) | 11,353 ms | Via Rust FFI wrapper |
| VROOM FFI (1 thread) | 15,952 ms | Via Rust FFI wrapper |

## Full Range Results (16-core VM)

| Size | Blst Rust | msm_best Rust | Vroom FFI (1t) | Vroom FFI (16t) |
|:---:|:---:|:---:|:---:|:---:|
| 2^8 | **1.03 ms** | 2.25 ms | 11.05 ms | 10.06 ms |
| 2^10 | **2.51 ms** | 5.35 ms | 30.39 ms | 39.92 ms |
| 2^12 | **8.45 ms** | 14.67 ms | 100.96 ms | 93.80 ms |
| 2^14 | 29.59 ms | **28.03 ms** | 364.73 ms | 264.74 ms |
| 2^16 | **96.79 ms** | 98.49 ms | 1,181 ms | 904.04 ms |
| 2^18 | **264.96 ms** | 359.00 ms | 4,178 ms | 3,232 ms |
| 2^20 | **939.54 ms** | 1,366 ms | 16,017 ms | 11,505 ms |

## Key Findings

1. **VROOM C++ native is fastest single-threaded MSM**: 570ms vs BLST 616ms (8% faster)
2. **VROOM C++ parallel is overall winner**: 394ms on 16 cores
3. **FFI wrapper has ~28x overhead**: VROOM native 570ms vs FFI 15,952ms — critical bug
4. **Rust Blst multi_exp scales well**: 3.7s (4 cores) → 0.95s (16 cores) = 3.9x
5. **VROOM parallel saturates early**: 381ms (4 cores) → 394ms (16 cores), no improvement

## FFI Wrapper Overhead Investigation

The 28x overhead from C++ native to Rust FFI wrapper needs investigation.
Likely causes: point generation method, data conversion, or compilation flags.
See: `vroom-msm-sys/src/wrapper.cpp` vs `VROOM/src/bench_msm.cpp`.

## Update — Investigation Findings (March 12, 2026)

The large VROOM FFI slowdown was caused by a scalar-generation mismatch in the wrapper.

- Native VROOM benchmark (`vroom/src/bench_msm.cpp`) generates scalars as `BigInt::random(255) % r` (canonical field scalars, `< r`).
- Wrapper (`vroom-msm-sys/src/wrapper.cpp`) was generating arbitrary 256-bit byte strings (not reduced mod `r`).

This input mismatch forced a much slower MSM path. After changing wrapper scalar generation to match native semantics (`BigInt::random(255) % r` + LE serialization), measured performance moved into the native benchmark range in our VM runs.

### Revalidated VM Results After Fix (n2-standard-16, 2^20 points)

| Implementation | Time | Notes |
|:---|:---:|:---|
| VROOM native C++ (1 thread) | **613 ms** | `bench_msm_avx`, same VM session |
| VROOM native C++ (parallel) | **400 ms** | `bench_msm_avx`, same VM session |
| VROOM wrapper-path direct driver (1 thread) | 6,829 ms | Before scalar fix |
| VROOM wrapper-path direct driver (parallel) | 5,639 ms | Before scalar fix |
| VROOM FFI direct bench (1 thread) | **560 ms** | After scalar fix |
| VROOM FFI direct bench (parallel) | **395 ms** | After scalar fix |

### Current Takeaway

Current evidence indicates the dominant regression was scalar format/semantics mismatch between wrapper and native benchmark inputs, rather than Rust FFI boundary cost alone.

## Post-Fix Full-Range Results (March 12, 2026)

Full rerun on VM with:

```bash
cargo bench -p midnight-curves --bench msm -- --noplot
```

### Vroom + Vroom_par (16-core VM)

| Size | Vroom FFI (1t) | Vroom FFI (16t) |
|:---:|:---:|:---:|
| 2^8 | 0.239 ms | 0.240 ms |
| 2^10 | 0.689 ms | 10.96 ms |
| 2^12 | 2.73 ms | 10.17 ms |
| 2^14 | 8.65 ms | 11.87 ms |
| 2^16 | 33.45 ms | 31.91 ms |
| 2^18 | 133.69 ms | 105.01 ms |
| 2^20 | 557.24 ms | 394.10 ms |

### Summary

- Post-fix VROOM FFI is in the native-scale performance range at 2^20 (`~557 ms` single-thread, `~394 ms` parallel) in these VM runs.
- If discrepancies persist in other environments, continue validation with the same scalar semantics and measurement setup before concluding root cause is fully closed.
