# VROOM MSM Integration Notes

## Current State (2026-03-07)

Branch: `msm-vroom`
Latest commit: `b8d72cd` (Fix projective coordinate mismatch: VROOM standard -> BLST Jacobian)

### Uncommitted Changes (tested & verified on GCP VM)

1. **`curves/src/msm.rs`** — Filter identity/infinity points before passing to VROOM (VROOM can't handle them). Added `test_msm_vroom_with_identity` regression test.

2. **`curves/benches/msm.rs`** — Fixed clippy `let_and_return` lint.

3. **`vroom-msm-sys/src/ffi_wrapper.cpp`** — Replaced GMP-based Montgomery conversion with BLST assembly (`from_fp`, `mul_mont_384`, `mul_fp`) + `mpz_import`/`mpz_export` for zero-copy BigInt construction. ~13-34% speedup on conversion overhead.

### Test Results (GCP VM: vroom-bench, Ice Lake, AVX-512 IFMA)

- midnight-curves: 260/260 passed
- midnight-proofs (including plonk_api): all passed
- clippy: clean

### Benchmark Results (after Option 2 fix)

| Size | BLST (ms) | msm_best (ms) | VROOM (ms) | VROOM/BLST |
|------|-----------|---------------|------------|------------|
| 2^8  | 0.22      | 0.35          | 0.97       | 4.4x       |
| 2^10 | 0.65      | 1.15          | 2.49       | 3.8x       |
| 2^12 | 2.20      | 3.95          | 7.72       | 3.5x       |
| 2^14 | 8.38      | 14.13         | 25.94      | 3.1x       |
| 2^16 | 31.92     | 46.68         | 88.89      | 2.8x       |
| 2^18 | 121.8     | 158.3         | 325.3      | 2.7x       |
| 2^20 | 463.2     | 557.1         | 1118       | 2.4x       |

### Remaining Bottleneck

The dominant cost is `ring.from_bigint()` — per-point RNS base conversion inside VROOM. This is O(n) and runs on every MSM call, converting each affine point from binary to RNS representation.

### Option 1: SRS Pre-caching (Not Yet Implemented)

Pre-convert SRS bases to VROOM's RNS format once at setup time, cache them, and reuse across MSM calls. This would eliminate the per-call conversion overhead entirely and is the path to making VROOM competitive with BLST.

Approach:
- Add a `vroom_precompute_bases()` FFI function that converts and stores affine points in RNS format
- Add a `vroom_g1_msm_precomputed()` that takes the cached RNS bases directly
- On the Rust side, store pre-converted bases in the KZG params or a lazy-static cache

### Minor Cleanup

- Unused parameter `const BigInt& q` in `vroom_proj_to_blst` (kept in signature for now, could be removed)
