// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! FFI bindings for VROOM BLS12-381 field arithmetic (Fp and Fr).

use std::ffi::c_void;

extern "C" {
    // ----- Fp (base field, 48 bytes) -----
    pub fn vroom_fp_ctx_new() -> *mut c_void;
    pub fn vroom_fp_ctx_free(ctx: *mut c_void);
    pub fn vroom_fp_load_a(ctx: *mut c_void, bytes: *const u8);
    pub fn vroom_fp_load_b(ctx: *mut c_void, bytes: *const u8);
    pub fn vroom_fp_store_result(ctx: *mut c_void, bytes: *mut u8);
    pub fn vroom_fp_add(ctx: *mut c_void);
    pub fn vroom_fp_sub(ctx: *mut c_void);
    pub fn vroom_fp_mul(ctx: *mut c_void);
    pub fn vroom_fp_square(ctx: *mut c_void);
    pub fn vroom_fp_neg(ctx: *mut c_void);
    pub fn vroom_fp_double(ctx: *mut c_void);
    pub fn vroom_fp_invert(ctx: *mut c_void);

    // ----- Fr (scalar field, 32 bytes) -----
    pub fn vroom_fr_ctx_new() -> *mut c_void;
    pub fn vroom_fr_ctx_free(ctx: *mut c_void);
    pub fn vroom_fr_load_a(ctx: *mut c_void, bytes: *const u8);
    pub fn vroom_fr_load_b(ctx: *mut c_void, bytes: *const u8);
    pub fn vroom_fr_store_result(ctx: *mut c_void, bytes: *mut u8);
    pub fn vroom_fr_add(ctx: *mut c_void);
    pub fn vroom_fr_sub(ctx: *mut c_void);
    pub fn vroom_fr_mul(ctx: *mut c_void);
    pub fn vroom_fr_square(ctx: *mut c_void);
    pub fn vroom_fr_neg(ctx: *mut c_void);
    pub fn vroom_fr_double(ctx: *mut c_void);
    pub fn vroom_fr_invert(ctx: *mut c_void);
}

#[cfg(test)]
mod tests {
    use super::*;

    // RAII wrapper for Fp context
    struct FpCtx(*mut c_void);
    impl FpCtx {
        fn new() -> Self {
            let ctx = unsafe { vroom_fp_ctx_new() };
            assert!(!ctx.is_null());
            Self(ctx)
        }
        fn ptr(&self) -> *mut c_void {
            self.0
        }
    }
    impl Drop for FpCtx {
        fn drop(&mut self) {
            unsafe { vroom_fp_ctx_free(self.0) }
        }
    }

    // RAII wrapper for Fr context
    struct FrCtx(*mut c_void);
    impl FrCtx {
        fn new() -> Self {
            let ctx = unsafe { vroom_fr_ctx_new() };
            assert!(!ctx.is_null());
            Self(ctx)
        }
        fn ptr(&self) -> *mut c_void {
            self.0
        }
    }
    impl Drop for FrCtx {
        fn drop(&mut self) {
            unsafe { vroom_fr_ctx_free(self.0) }
        }
    }

    // Helper: convert blst Fp to LE bytes
    fn blst_fp_to_le(fp: &blst::blst_fp) -> [u8; 48] {
        let mut out = [0u8; 48];
        unsafe { blst::blst_lendian_from_fp(out.as_mut_ptr(), fp) };
        out
    }

    // Helper: convert LE bytes to blst Fp
    fn blst_fp_from_le(bytes: &[u8; 48]) -> blst::blst_fp {
        let mut fp = blst::blst_fp::default();
        unsafe { blst::blst_fp_from_lendian(&mut fp, bytes.as_ptr()) };
        fp
    }

    // Helper: convert blst Fr to LE bytes
    fn blst_fr_to_le(fr: &blst::blst_fr) -> [u8; 32] {
        let mut out = [0u64; 4];
        unsafe { blst::blst_uint64_from_fr(out.as_mut_ptr(), fr) };
        let mut bytes = [0u8; 32];
        for (i, limb) in out.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_le_bytes());
        }
        bytes
    }

    // Helper: convert LE bytes to blst Fr
    fn blst_fr_from_le(bytes: &[u8; 32]) -> blst::blst_fr {
        let mut limbs = [0u64; 4];
        for (i, limb) in limbs.iter_mut().enumerate() {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            *limb = u64::from_le_bytes(buf);
        }
        let mut fr = blst::blst_fr::default();
        unsafe { blst::blst_fr_from_uint64(&mut fr, limbs.as_ptr()) };
        fr
    }

    // Fixed test values for Fp
    fn test_fp_a() -> blst::blst_fp {
        // A known value < p
        let bytes: [u8; 48] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24,
            0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x00,
        ];
        blst_fp_from_le(&bytes)
    }

    fn test_fp_b() -> blst::blst_fp {
        let bytes: [u8; 48] = [
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
            0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
            0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54,
            0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x00,
        ];
        blst_fp_from_le(&bytes)
    }

    // Fixed test values for Fr
    fn test_fr_a() -> blst::blst_fr {
        let bytes: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ];
        blst_fr_from_le(&bytes)
    }

    fn test_fr_b() -> blst::blst_fr {
        let bytes: [u8; 32] = [
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c,
            0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
            0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
        ];
        blst_fr_from_le(&bytes)
    }

    fn fp_bytes_eq(a: &[u8; 48], b: &[u8; 48]) -> bool {
        a == b
    }

    fn fr_bytes_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
        a == b
    }

    // ---- Fp tests ----

    #[test]
    fn fp_add_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let b = test_fp_b();
        let a_bytes = blst_fp_to_le(&a);
        let b_bytes = blst_fp_to_le(&b);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fp_add(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_add(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp add mismatch");
    }

    #[test]
    fn fp_sub_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let b = test_fp_b();
        let a_bytes = blst_fp_to_le(&a);
        let b_bytes = blst_fp_to_le(&b);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fp_sub(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_sub(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp sub mismatch");
    }

    #[test]
    fn fp_mul_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let b = test_fp_b();
        let a_bytes = blst_fp_to_le(&a);
        let b_bytes = blst_fp_to_le(&b);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fp_mul(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_mul(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp mul mismatch");
    }

    #[test]
    fn fp_square_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let a_bytes = blst_fp_to_le(&a);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_square(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_sqr(&mut blst_result, &a) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp square mismatch");
    }

    #[test]
    fn fp_neg_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let a_bytes = blst_fp_to_le(&a);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_neg(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        // blst doesn't have fp_neg directly, compute 0 - a
        let mut blst_result = blst::blst_fp::default();
        let zero = blst::blst_fp::default();
        unsafe { blst::blst_fp_sub(&mut blst_result, &zero, &a) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp neg mismatch");
    }

    #[test]
    fn fp_double_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let a_bytes = blst_fp_to_le(&a);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_double(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_add(&mut blst_result, &a, &a) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp double mismatch");
    }

    #[test]
    fn fp_invert_matches_blst() {
        let ctx = FpCtx::new();
        let a = test_fp_a();
        let a_bytes = blst_fp_to_le(&a);

        unsafe {
            vroom_fp_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fp_invert(ctx.ptr());
        }

        let mut vroom_result = [0u8; 48];
        unsafe { vroom_fp_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fp::default();
        unsafe { blst::blst_fp_inverse(&mut blst_result, &a) };
        let blst_bytes = blst_fp_to_le(&blst_result);

        assert!(fp_bytes_eq(&vroom_result, &blst_bytes), "Fp invert mismatch");
    }

    // ---- Fr tests ----

    #[test]
    fn fr_add_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let b = test_fr_b();
        let a_bytes = blst_fr_to_le(&a);
        let b_bytes = blst_fr_to_le(&b);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fr_add(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fr::default();
        unsafe { blst::blst_fr_add(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr add mismatch");
    }

    #[test]
    fn fr_sub_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let b = test_fr_b();
        let a_bytes = blst_fr_to_le(&a);
        let b_bytes = blst_fr_to_le(&b);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fr_sub(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fr::default();
        unsafe { blst::blst_fr_sub(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr sub mismatch");
    }

    #[test]
    fn fr_mul_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let b = test_fr_b();
        let a_bytes = blst_fr_to_le(&a);
        let b_bytes = blst_fr_to_le(&b);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_load_b(ctx.ptr(), b_bytes.as_ptr());
            vroom_fr_mul(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fr::default();
        unsafe { blst::blst_fr_mul(&mut blst_result, &a, &b) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr mul mismatch");
    }

    #[test]
    fn fr_square_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let a_bytes = blst_fr_to_le(&a);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_square(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fr::default();
        unsafe { blst::blst_fr_sqr(&mut blst_result, &a) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr square mismatch");
    }

    #[test]
    fn fr_neg_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let a_bytes = blst_fr_to_le(&a);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_neg(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        // Compute 0 - a for negation
        let mut blst_result = blst::blst_fr::default();
        let zero = blst::blst_fr::default();
        unsafe { blst::blst_fr_sub(&mut blst_result, &zero, &a) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr neg mismatch");
    }

    #[test]
    fn fr_double_matches_blst() {
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let a_bytes = blst_fr_to_le(&a);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_double(ctx.ptr());
        }

        let mut vroom_result = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), vroom_result.as_mut_ptr()) };

        let mut blst_result = blst::blst_fr::default();
        unsafe { blst::blst_fr_add(&mut blst_result, &a, &a) };
        let blst_bytes = blst_fr_to_le(&blst_result);

        assert!(fr_bytes_eq(&vroom_result, &blst_bytes), "Fr double mismatch");
    }

    #[test]
    fn fr_invert_correctness() {
        // Fr inversion via square-and-multiply; verify a * a^(-1) = 1
        let ctx = FrCtx::new();
        let a = test_fr_a();
        let a_bytes = blst_fr_to_le(&a);

        unsafe {
            vroom_fr_load_a(ctx.ptr(), a_bytes.as_ptr());
            vroom_fr_invert(ctx.ptr());
        }

        let mut inv_bytes = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), inv_bytes.as_mut_ptr()) };

        // Load inv as b, multiply a * inv
        unsafe {
            vroom_fr_load_b(ctx.ptr(), inv_bytes.as_ptr());
            vroom_fr_mul(ctx.ptr());
        }

        let mut product_bytes = [0u8; 32];
        unsafe { vroom_fr_store_result(ctx.ptr(), product_bytes.as_mut_ptr()) };

        // Should be 1 in LE: [1, 0, 0, ..., 0]
        let mut one_bytes = [0u8; 32];
        one_bytes[0] = 1;
        assert!(
            fr_bytes_eq(&product_bytes, &one_bytes),
            "Fr invert: a * a^(-1) != 1"
        );
    }
}
