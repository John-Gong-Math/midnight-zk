// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// VROOM MSM computation — isolated in its own translation unit.
// Keeping this separate from the data generation code is CRITICAL for
// performance: clang's template optimizer generates 10x+ faster code
// when the msm() template is the only major instantiation in the TU.

#include "../vroom/cpu/precompute/gmp_wrapper.hpp"

#include "../vroom/src/msm.hpp"
#include "../vroom/src/pippenger.hpp"
#include "../vroom/src/bounded_ring.hpp"
#include "../vroom/src/conversion_inversion.hpp"

#include "wrapper_types.hpp"

static std::pair<BigInt, BigInt> proj_to_affine_bigint(
    const ProjPoint& point,
    const RingType& ring
) {
    BigInt x = ring.to_bigint(point.x);
    BigInt y = ring.to_bigint(point.y);
    BigInt z = ring.to_bigint(point.z);
    if (z == BigInt(0)) {
        return {BigInt(0), BigInt(0)};
    }

    static const BigInt modulus(bls12_381_modulus_hex, 16);
    BigInt z_inv = z.mod_inverse(modulus);

    BigInt ax = (x * z_inv) % modulus;
    if (ax < 0) {
        ax = ax + modulus;
    }

    BigInt ay = (y * z_inv) % modulus;
    if (ay < 0) {
        ay = ay + modulus;
    }

    return {ax, ay};
}

extern "C" {

namespace {
volatile uint64_t g_msm_sink = 0;
}

void vroom_g1_msm(void* ctx_ptr, const void* points_ptr,
                  const void* scalars_ptr, size_t npoints) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto result = msm(ctx->curve, ctx->ring,
                      pts->data.data(), sc->ptrs.data(),
                      npoints, 255);

    // Keep an observable side-effect so the optimizer cannot drop MSM work.
    g_msm_sink ^= result.z.m2.to_unsigned_array()[0];
}

void vroom_g1_msm_parallel(void* ctx_ptr, const void* points_ptr,
                           const void* scalars_ptr, size_t npoints,
                           size_t num_threads) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto result = msm_parallel(ctx->curve, ctx->ring,
                               pts->data.data(), sc->ptrs.data(),
                               npoints, 255, num_threads);

    // Keep an observable side-effect so the optimizer cannot drop MSM work.
    g_msm_sink ^= result.z.m2.to_unsigned_array()[0];
}

bool vroom_g1_msm_parallel_matches_serial(
    void* ctx_ptr,
    const void* points_ptr,
    const void* scalars_ptr,
    size_t npoints,
    size_t num_threads
) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto serial = msm(ctx->curve, ctx->ring,
                      pts->data.data(), sc->ptrs.data(),
                      npoints, 255);
    auto parallel = msm_parallel(ctx->curve, ctx->ring,
                                 pts->data.data(), sc->ptrs.data(),
                                 npoints, 255, num_threads);

    auto [sx, sy] = proj_to_affine_bigint(serial, ctx->ring);
    auto [px, py] = proj_to_affine_bigint(parallel, ctx->ring);

    return sx == px && sy == py;
}

void vroom_g1_pippenger_v1(void* ctx_ptr, const void* points_ptr,
                           const void* scalars_ptr, size_t npoints) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto result = pippenger_msm(ctx->curve, ctx->ring,
                                pts->data.data(), sc->ptrs.data(),
                                npoints, 255);

    g_msm_sink ^= result.z.m2.to_unsigned_array()[0];
}

void vroom_g1_pippenger_v1_parallel(void* ctx_ptr, const void* points_ptr,
                                    const void* scalars_ptr, size_t npoints,
                                    size_t num_threads) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto result = pippenger_msm_parallel(ctx->curve, ctx->ring,
                                         pts->data.data(), sc->ptrs.data(),
                                         npoints, 255, num_threads);

    g_msm_sink ^= result.z.m2.to_unsigned_array()[0];
}

void vroom_g1_msm_point_parallel(void* ctx_ptr, const void* points_ptr,
                                  const void* scalars_ptr, size_t npoints,
                                  size_t num_threads) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto result = msm_point_parallel(ctx->curve, ctx->ring,
                                      pts->data.data(), sc->ptrs.data(),
                                      npoints, 255, num_threads);

    g_msm_sink ^= result.z.m2.to_unsigned_array()[0];
}

bool vroom_g1_msm_point_parallel_matches_serial(
    void* ctx_ptr,
    const void* points_ptr,
    const void* scalars_ptr,
    size_t npoints,
    size_t num_threads
) {
    auto* ctx = static_cast<VroomContext*>(ctx_ptr);
    auto* pts = static_cast<const VroomPoints*>(points_ptr);
    auto* sc = static_cast<const VroomScalars*>(scalars_ptr);

    auto serial = msm(ctx->curve, ctx->ring,
                      pts->data.data(), sc->ptrs.data(),
                      npoints, 255);
    auto point_par = msm_point_parallel(ctx->curve, ctx->ring,
                                         pts->data.data(), sc->ptrs.data(),
                                         npoints, 255, num_threads);

    auto [sx, sy] = proj_to_affine_bigint(serial, ctx->ring);
    auto [px, py] = proj_to_affine_bigint(point_par, ctx->ring);

    return sx == px && sy == py;
}

} // extern "C"
