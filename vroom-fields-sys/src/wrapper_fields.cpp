// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// VROOM FFI wrapper for BLS12-381 field arithmetic (Fp and Fr).
//
// Benchmarks:
// - Single ops: add/sub/mul/square/neg/double/invert
// - mul_chain: N sequential a = a*b (tests serial latency)
// - batch_mul_chain: N iters of batch_modmul<6> (tests latency hiding)
// - sum_of_2_products: a*b + c*d with single reduction

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
    ctx->a = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 48));
}

void vroom_fp_load_b(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->b = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 48));
}

void vroom_fp_load_c(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->c = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 48));
}

void vroom_fp_load_d(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    ctx->d = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 48));
}

void vroom_fp_store_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->result), 48);
}

// --- Single-op benchmarks ---

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

// --- Correctness: extract add/sub/double results as bytes ---

void vroom_fp_add_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->b, "");
    bigint_to_bytes_le(bytes, val, 48);
}

void vroom_fp_sub_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->ring.standard_negate(ctx->b), "");
    bigint_to_bytes_le(bytes, val, 48);
}

void vroom_fp_double_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->a, "");
    bigint_to_bytes_le(bytes, val, 48);
}

// --- Chain of N sequential multiplications: a = a * b ---

void vroom_fp_mul_chain(void* ctx_ptr, uint64_t n) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    for (uint64_t i = 0; i < n; i++) {
        ctx->a = ctx->ring.modmul(ctx->a, ctx->b);
    }
    ctx->result = ctx->a;
}

// --- Sum of 2 products: result = a*b + c*d with single reduction ---

void vroom_fp_sum_of_2_products(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    auto p1 = ctx->a * ctx->b;
    auto p2 = ctx->c * ctx->d;
    auto sum = p1 + p2;
    // Single reduction for both products
    auto ready_sum = ctx->ring.template ready<FpRing::MAX_ADD>(sum);
    std::array<decltype(ready_sum), 1> arr = {ready_sum};
    ctx->result = ctx->ring.template batch_reduce_expand<1>(arr)[0];
}

void vroom_fp_sum_of_2_products_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpContext*>(ctx_ptr);
    vroom_fp_sum_of_2_products(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->result), 48);
}

// --- Batch context: 6 parallel multiplication chains ---

void* vroom_fp_batch_ctx_new() {
    return new VroomFpBatchContext();
}

void vroom_fp_batch_ctx_free(void* ctx) {
    delete static_cast<VroomFpBatchContext*>(ctx);
}

void vroom_fp_batch_load(void* ctx_ptr, int index, const uint8_t* a_bytes, const uint8_t* b_bytes) {
    auto* ctx = static_cast<VroomFpBatchContext*>(ctx_ptr);
    ctx->a[index] = ctx->ring.from_bigint(bytes_le_to_bigint(a_bytes, 48));
    ctx->b[index] = ctx->ring.from_bigint(bytes_le_to_bigint(b_bytes, 48));
}

// One iteration of batch_modmul<6>: a[i] = a[i] * b[i] for all i
void vroom_fp_batch_mul(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFpBatchContext*>(ctx_ptr);
    ctx->a = ctx->ring.batch_modmul<FP_BATCH>(ctx->a, ctx->b);
}

// N iterations of batch_modmul<6>: total muls = N * 6
void vroom_fp_batch_mul_chain(void* ctx_ptr, uint64_t n_iters) {
    auto* ctx = static_cast<VroomFpBatchContext*>(ctx_ptr);
    for (uint64_t i = 0; i < n_iters; i++) {
        ctx->a = ctx->ring.batch_modmul<FP_BATCH>(ctx->a, ctx->b);
    }
}

void vroom_fp_batch_store(void* ctx_ptr, int index, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFpBatchContext*>(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->a[index]), 48);
}

} // extern "C" (Fp)

// =========================================================================
//  Fr (scalar field, 255 bits, 32 bytes)
// =========================================================================

extern "C" {

void* vroom_fr_ctx_new() {
    return new VroomFrContext();
}

void vroom_fr_ctx_free(void* ctx) {
    delete static_cast<VroomFrContext*>(ctx);
}

void vroom_fr_load_a(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->a = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 32));
}

void vroom_fr_load_b(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->b = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 32));
}

void vroom_fr_load_c(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->c = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 32));
}

void vroom_fr_load_d(void* ctx_ptr, const uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    ctx->d = ctx->ring.from_bigint(bytes_le_to_bigint(bytes, 32));
}

void vroom_fr_store_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->result), 32);
}

// --- Single-op benchmarks ---

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
    // Convert to BigInt, use GMP modular inverse, convert back.
    // This is the paper's recommended approach for inversion
    // (much faster than square-and-multiply via modmul chain).
    BigInt r(fr_modulus_hex, 16);
    BigInt a_val = ctx->ring.to_bigint(ctx->a);
    BigInt inv_val = a_val.mod_inverse(r);
    ctx->result = ctx->ring.from_bigint(inv_val);
}

// --- Correctness extraction ---

void vroom_fr_add_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->b, "");
    bigint_to_bytes_le(bytes, val, 32);
}

void vroom_fr_sub_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->ring.standard_negate(ctx->b), "");
    bigint_to_bytes_le(bytes, val, 32);
}

void vroom_fr_double_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto [val, ok] = ctx->ring.check_bounds(ctx->a + ctx->a, "");
    bigint_to_bytes_le(bytes, val, 32);
}

// --- Chain of N sequential multiplications: a = a * b ---

void vroom_fr_mul_chain(void* ctx_ptr, uint64_t n) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    for (uint64_t i = 0; i < n; i++) {
        ctx->a = ctx->ring.modmul(ctx->a, ctx->b);
    }
    ctx->result = ctx->a;
}

// --- Sum of 2 products: result = a*b + c*d with single reduction ---

void vroom_fr_sum_of_2_products(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    auto p1 = ctx->a * ctx->b;
    auto p2 = ctx->c * ctx->d;
    auto sum = p1 + p2;
    auto ready_sum = ctx->ring.template ready<FrRing::MAX_ADD>(sum);
    std::array<decltype(ready_sum), 1> arr = {ready_sum};
    ctx->result = ctx->ring.template batch_reduce_expand<1>(arr)[0];
}

void vroom_fr_sum_of_2_products_result(void* ctx_ptr, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrContext*>(ctx_ptr);
    vroom_fr_sum_of_2_products(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->result), 32);
}

// --- Batch context: 6 parallel multiplication chains ---

void* vroom_fr_batch_ctx_new() {
    return new VroomFrBatchContext();
}

void vroom_fr_batch_ctx_free(void* ctx) {
    delete static_cast<VroomFrBatchContext*>(ctx);
}

void vroom_fr_batch_load(void* ctx_ptr, int index, const uint8_t* a_bytes, const uint8_t* b_bytes) {
    auto* ctx = static_cast<VroomFrBatchContext*>(ctx_ptr);
    ctx->a[index] = ctx->ring.from_bigint(bytes_le_to_bigint(a_bytes, 32));
    ctx->b[index] = ctx->ring.from_bigint(bytes_le_to_bigint(b_bytes, 32));
}

void vroom_fr_batch_mul(void* ctx_ptr) {
    auto* ctx = static_cast<VroomFrBatchContext*>(ctx_ptr);
    ctx->a = ctx->ring.batch_modmul<FR_BATCH>(ctx->a, ctx->b);
}

void vroom_fr_batch_mul_chain(void* ctx_ptr, uint64_t n_iters) {
    auto* ctx = static_cast<VroomFrBatchContext*>(ctx_ptr);
    for (uint64_t i = 0; i < n_iters; i++) {
        ctx->a = ctx->ring.batch_modmul<FR_BATCH>(ctx->a, ctx->b);
    }
}

void vroom_fr_batch_store(void* ctx_ptr, int index, uint8_t* bytes) {
    auto* ctx = static_cast<VroomFrBatchContext*>(ctx_ptr);
    bigint_to_bytes_le(bytes, ctx->ring.to_bigint(ctx->a[index]), 32);
}

} // extern "C" (Fr)
