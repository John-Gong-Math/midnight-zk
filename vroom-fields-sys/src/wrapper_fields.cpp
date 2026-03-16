// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// VROOM FFI wrapper for BLS12-381 field arithmetic (Fp and Fr).
//
// Design notes:
// - VROOM's addition/subtraction/doubling produce non-StandardElement types
//   (bounds grow after +/-). The benchmark functions store to a scratch buffer
//   to prevent dead-code elimination. Separate *_result functions use
//   check_bounds to extract the BigInt for correctness verification.
// - mul/square/invert/neg produce StandardElement directly.

#include "../vroom/cpu/precompute/gmp_wrapper.hpp"
#include "wrapper_fields_types.hpp"

#include <cstring>

// ----- Helpers: LE bytes <-> BigInt -----

static BigInt bytes_le_to_bigint(const uint8_t* bytes, size_t len) {
    BigInt result(0);
    for (size_t i = len; i > 0; i--) {
        result = (result << 8) | BigInt(static_cast<unsigned long>(bytes[i - 1]));
    }
    return result;
}

static void bigint_to_bytes_le(uint8_t* bytes, const BigInt& value, size_t len) {
    BigInt temp = value;
    for (size_t i = 0; i < len; i++) {
        bytes[i] = static_cast<uint8_t>(temp.to_ulong() & 0xff);
        temp = temp >> 8;
    }
}

// =========================================================================
//  Fp (base field, 381 bits, 48 bytes)
// =========================================================================

extern "C" {

void* vroom_fp_ctx_new() {
    return new VroomFpContext();
}

void vroom_fp_ctx_free(void* ctx) {
    delete static_cast<VroomFpContext*>(ctx);
}

void vroom_fp_load_a(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    BigInt val = bytes_le_to_bigint(bytes, 48);
    ctx->a = ctx->ring.from_bigint(val);
}

void vroom_fp_load_b(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    BigInt val = bytes_le_to_bigint(bytes, 48);
    ctx->b = ctx->ring.from_bigint(val);
}

void vroom_fp_store_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    BigInt val = ctx->ring.to_bigint(ctx->result);
    bigint_to_bytes_le(bytes, val, 48);
}

// --- Benchmark functions: add/sub/double store to scratch ---

void vroom_fp_add(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->b;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

void vroom_fp_sub(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto neg_b = ctx->ring.standard_negate(ctx->b);
    auto sum = ctx->a + neg_b;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

void vroom_fp_double(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->a;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

// --- Correctness: extract add/sub/double results as bytes ---

void vroom_fp_add_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->b;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 48);
}

void vroom_fp_sub_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto neg_b = ctx->ring.standard_negate(ctx->b);
    auto sum = ctx->a + neg_b;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 48);
}

void vroom_fp_double_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->a;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 48);
}

// --- Operations producing StandardElement directly ---

void vroom_fp_mul(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->result = ctx->ring.modmul(ctx->a, ctx->b);
}

void vroom_fp_square(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->result = ctx->ring.modmul(ctx->a, ctx->a);
}

void vroom_fp_neg(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->result = ctx->ring.standard_negate(ctx->a);
}

void vroom_fp_invert(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->result = ctx->inverter.invert(ctx->a, ctx->ring);
}

} // extern "C" (Fp)

// =========================================================================
//  Fr (scalar field, 255 bits, 32 bytes)
// =========================================================================

// Fr inversion via square-and-multiply: a^(r-2) mod r
static FrRing::StandardElement fr_invert_sqm(
    const FrRing::StandardElement& a,
    const FrRing& ring)
{
    BigInt r(fr_modulus_hex, 16);
    BigInt exp = r - 2;

    size_t bits = exp.bit_length();
    auto result = ring.one();
    auto base = a;

    for (size_t i = bits; i > 0; i--) {
        result = ring.modmul(result, result);
        BigInt bit_mask = BigInt(1) << static_cast<int>(i - 1);
        if ((exp & bit_mask) != BigInt(0)) {
            result = ring.modmul(result, base);
        }
    }
    return result;
}

extern "C" {

void* vroom_fr_ctx_new() {
    return new VroomFrContext();
}

void vroom_fr_ctx_free(void* ctx) {
    delete static_cast<VroomFrContext*>(ctx);
}

void vroom_fr_load_a(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    BigInt val = bytes_le_to_bigint(bytes, 32);
    ctx->a = ctx->ring.from_bigint(val);
}

void vroom_fr_load_b(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    BigInt val = bytes_le_to_bigint(bytes, 32);
    ctx->b = ctx->ring.from_bigint(val);
}

void vroom_fr_store_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    BigInt val = ctx->ring.to_bigint(ctx->result);
    bigint_to_bytes_le(bytes, val, 32);
}

// --- Benchmark functions: add/sub/double store to scratch ---

void vroom_fr_add(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->b;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

void vroom_fr_sub(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto neg_b = ctx->ring.standard_negate(ctx->b);
    auto sum = ctx->a + neg_b;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

void vroom_fr_double(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->a;
    std::memcpy(ctx->scratch, &sum, sizeof(sum));
}

// --- Correctness: extract add/sub/double results as bytes ---

void vroom_fr_add_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->b;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 32);
}

void vroom_fr_sub_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto neg_b = ctx->ring.standard_negate(ctx->b);
    auto sum = ctx->a + neg_b;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 32);
}

void vroom_fr_double_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto sum = ctx->a + ctx->a;
    auto [val, ok] = ctx->ring.check_bounds(sum, "");
    bigint_to_bytes_le(bytes, val, 32);
}

// --- Operations producing StandardElement directly ---

void vroom_fr_mul(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->result = ctx->ring.modmul(ctx->a, ctx->b);
}

void vroom_fr_square(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->result = ctx->ring.modmul(ctx->a, ctx->a);
}

void vroom_fr_neg(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->result = ctx->ring.standard_negate(ctx->a);
}

void vroom_fr_invert(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->result = fr_invert_sqm(ctx->a, ctx->ring);
}

} // extern "C" (Fr)
