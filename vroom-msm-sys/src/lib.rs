// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Raw FFI bindings to VROOM's BLS12-381 MSM implementation.

extern "C" {
    /// Initialize a VROOM BLS12-381 context (BoundedRing + G1 curve).
    /// Returns an opaque pointer. Must be freed with `vroom_bls12_381_free`.
    pub fn vroom_bls12_381_init() -> *mut std::ffi::c_void;

    /// Free a VROOM BLS12-381 context.
    pub fn vroom_bls12_381_free(ctx: *mut std::ffi::c_void);

    /// Multi-scalar multiplication on G1.
    ///
    /// # Arguments
    /// * `ctx` - Context from `vroom_bls12_381_init`
    /// * `out` - Output buffer, 144 bytes (`blst_p1` projective, Montgomery form)
    /// * `points` - Input affine points, `npoints * 96` bytes (`blst_p1_affine[]`, Montgomery form)
    /// * `scalars` - Input scalars, `npoints * 32` bytes (little-endian scalar field elements)
    /// * `npoints` - Number of points/scalars
    pub fn vroom_g1_msm(
        ctx: *mut std::ffi::c_void,
        out: *mut u8,
        points: *const u8,
        scalars: *const u8,
        npoints: usize,
    );

    /// Parallel multi-scalar multiplication on G1.
    ///
    /// Same as `vroom_g1_msm` but uses multiple threads.
    /// `num_threads = 0` means auto-detect.
    pub fn vroom_g1_msm_parallel(
        ctx: *mut std::ffi::c_void,
        out: *mut u8,
        points: *const u8,
        scalars: *const u8,
        npoints: usize,
        num_threads: usize,
    );

    /// Debug: roundtrip affine point through VROOM conversion.
    /// Input: 96 bytes (blst_p1_affine). Output: 144 bytes (blst_p1 projective).
    pub fn vroom_g1_roundtrip_affine(
        ctx: *mut std::ffi::c_void,
        out: *mut u8,
        point_in: *const u8,
    );
}
