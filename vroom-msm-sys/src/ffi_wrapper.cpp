// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// FFI wrapper exposing VROOM's BLS12-381 MSM to Rust via extern "C" functions.
// Converts between BLST Montgomery format (used by the blst Rust crate) and
// VROOM's RNS representation.

#include "../vroom/cpu/precompute/gmp_wrapper.hpp"
extern "C" {
#include "../vroom/blst/vect.h"
#include "../vroom/blst/fields.h"
#include "../vroom/blst/point.h"
}
#include "../vroom/src/msm.hpp"
#include "../vroom/src/bounded_ring.hpp"
#include "../vroom/src/conversion_inversion.hpp"
#include <cstring>
#include <vector>

// Ring type for BLS12-381 Fp
using RingType = BoundedRing<381, 8, 52, -1932, 2377, 12>;

// Context holding pre-initialized ring and curve
struct VroomBls12381Context {
    BigInt q;
    RingType ring;
    G1<RingType> g1_curve;

    VroomBls12381Context()
        : q("1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab", 16)
        , ring(q)
        , g1_curve()
    {}
};

// ---------- Fast fixed-width Montgomery conversion ----------
//
// Uses BLST's assembly-optimized from_mont_384 / mul_mont_384 instead of
// GMP arbitrary-precision modular arithmetic. Combined with mpz_import/export
// for zero-copy BigInt construction, this eliminates the O(n) GMP bottleneck.

// Read a vec384 (6 little-endian 64-bit limbs) into a BigInt via mpz_import.
// No arithmetic — just a memcpy into GMP's internal representation.
static BigInt bigint_from_vec384(const vec384 a) {
    mpz_class result;
    mpz_import(result.get_mpz_t(), 6, -1, sizeof(limb_t), 0, 0, a);
    return BigInt(result);
}

// Write a BigInt to a vec384 (6 little-endian 64-bit limbs) via mpz_export.
static void bigint_to_vec384(vec384 out, const BigInt& a) {
    memset(out, 0, sizeof(vec384));
    size_t count = 0;
    mpz_export(out, &count, -1, sizeof(limb_t), 0, 0, a.get_mpz().get_mpz_t());
}

// Convert BLST Montgomery vec384 → normal-form BigInt.
// Uses BLST assembly for Montgomery reduction (nanoseconds), then mpz_import.
static BigInt vec384_mont_to_bigint(const vec384 a) {
    vec384 normal;
    from_fp(normal, a);  // BLST assembly: a * R^{-1} mod P
    return bigint_from_vec384(normal);
}

// Convert a normal-form BigInt → BLST Montgomery vec384.
// Uses mpz_export then BLST assembly Montgomery multiplication with R².
static void bigint_to_vec384_mont(vec384 out, const BigInt& a) {
    vec384 normal;
    bigint_to_vec384(normal, a);
    mul_mont_384(out, normal, BLS12_381_RR, BLS12_381_P, p0);  // BLST assembly
}

// Convert a BLST affine point (Montgomery) to a VROOM AffinePoint (RNS)
static AffinePoint<RingType::StandardElement> blst_affine_to_vroom(
    const POINTonE1_affine& p,
    const RingType& ring
) {
    BigInt x = vec384_mont_to_bigint(p.X);
    BigInt y = vec384_mont_to_bigint(p.Y);
    AffinePoint<RingType::StandardElement> result;
    result.x = ring.from_bigint(x);
    result.y = ring.from_bigint(y);
    return result;
}

// Convert a VROOM ProjectivePoint to BLST projective (Montgomery/Jacobian) format.
// The output is written as a blst_p1: 3 x vec384 (X, Y, Z) = 144 bytes.
//
// IMPORTANT: VROOM uses standard projective coordinates where (X:Y:Z) means
// affine (X/Z, Y/Z). BLST uses Jacobian coordinates where (X:Y:Z) means
// affine (X/Z², Y/Z³). We must convert:
//   X_jac = X_proj * Z_proj,  Y_jac = Y_proj * Z_proj²,  Z_jac = Z_proj
static void vroom_proj_to_blst(
    uint8_t* out,
    const ProjectivePoint<RingType::StandardElement>& point,
    const RingType& ring,
    const BigInt& q
) {
    BigInt x_bi = ring.to_bigint(point.x);
    BigInt y_bi = ring.to_bigint(point.y);
    BigInt z_bi = ring.to_bigint(point.z);

    // Handle point at infinity: BLST uses Z=0 representation
    POINTonE1 blst_point;
    memset(&blst_point, 0, sizeof(blst_point));

    if (z_bi == BigInt(0)) {
        memcpy(out, &blst_point, sizeof(POINTonE1));
        return;
    }

    // Convert standard projective → Jacobian using BLST Montgomery arithmetic.
    // First convert x, y, z to Montgomery form, then use mul_fp for modular mult.
    vec384 x_mont, y_mont, z_mont;
    bigint_to_vec384_mont(x_mont, x_bi);
    bigint_to_vec384_mont(y_mont, y_bi);
    bigint_to_vec384_mont(z_mont, z_bi);

    // X_jac = X_proj * Z_proj (in Montgomery: mul_fp handles R factor)
    mul_fp(blst_point.X, x_mont, z_mont);
    // Y_jac = Y_proj * Z_proj²
    vec384 z_sq;
    mul_fp(z_sq, z_mont, z_mont);
    mul_fp(blst_point.Y, y_mont, z_sq);
    // Z_jac = Z_proj
    memcpy(blst_point.Z, z_mont, sizeof(vec384));

    memcpy(out, &blst_point, sizeof(POINTonE1));
}

extern "C" {

void* vroom_bls12_381_init() {
    return new VroomBls12381Context();
}

void vroom_bls12_381_free(void* ctx) {
    delete static_cast<VroomBls12381Context*>(ctx);
}

void vroom_g1_msm(
    void* ctx,
    uint8_t* out,
    const uint8_t* points,
    const uint8_t* scalars,
    size_t npoints
) {
    auto* c = static_cast<VroomBls12381Context*>(ctx);

    if (npoints == 0) {
        // Return point at infinity
        memset(out, 0, 144);
        return;
    }

    // Convert BLST affine points to VROOM format
    const POINTonE1_affine* blst_pts = reinterpret_cast<const POINTonE1_affine*>(points);
    std::vector<AffinePoint<RingType::StandardElement>> vroom_points(npoints);
    for (size_t i = 0; i < npoints; i++) {
        vroom_points[i] = blst_affine_to_vroom(blst_pts[i], c->ring);
    }

    // Build scalar pointer array (each scalar is 32 bytes LE)
    std::vector<const uint8_t*> scalar_ptrs(npoints);
    for (size_t i = 0; i < npoints; i++) {
        scalar_ptrs[i] = scalars + i * 32;
    }

    // Run single-threaded MSM
    auto result = msm(c->g1_curve, c->ring, vroom_points.data(),
                      scalar_ptrs.data(), npoints, 255);

    // Convert result back to BLST format
    vroom_proj_to_blst(out, result, c->ring, c->q);
}

void vroom_g1_msm_parallel(
    void* ctx,
    uint8_t* out,
    const uint8_t* points,
    const uint8_t* scalars,
    size_t npoints,
    size_t num_threads
) {
    auto* c = static_cast<VroomBls12381Context*>(ctx);

    if (npoints == 0) {
        memset(out, 0, 144);
        return;
    }

    // Convert BLST affine points to VROOM format
    const POINTonE1_affine* blst_pts = reinterpret_cast<const POINTonE1_affine*>(points);
    std::vector<AffinePoint<RingType::StandardElement>> vroom_points(npoints);
    for (size_t i = 0; i < npoints; i++) {
        vroom_points[i] = blst_affine_to_vroom(blst_pts[i], c->ring);
    }

    // Build scalar pointer array
    std::vector<const uint8_t*> scalar_ptrs(npoints);
    for (size_t i = 0; i < npoints; i++) {
        scalar_ptrs[i] = scalars + i * 32;
    }

    // Run parallel MSM
    auto result = msm_parallel(c->g1_curve, c->ring, vroom_points.data(),
                                scalar_ptrs.data(), npoints, 255, num_threads);

    // Convert result back to BLST format
    vroom_proj_to_blst(out, result, c->ring, c->q);
}

// Debug: roundtrip affine point conversion (BLST → VROOM → BLST projective with Z=1)
// Returns 144 bytes (blst_p1) with Z = R mod P (Montgomery one).
void vroom_g1_roundtrip_affine(
    void* ctx,
    uint8_t* out,
    const uint8_t* point_in
) {
    auto* c = static_cast<VroomBls12381Context*>(ctx);
    const POINTonE1_affine* blst_pt = reinterpret_cast<const POINTonE1_affine*>(point_in);

    // Convert BLST affine → VROOM affine
    auto vroom_pt = blst_affine_to_vroom(*blst_pt, c->ring);

    // Build a projective point with Z=1
    auto z_one = c->ring.one();
    ProjectivePoint<RingType::StandardElement> proj(vroom_pt.x, vroom_pt.y, z_one);

    // Convert back to BLST projective
    vroom_proj_to_blst(out, proj, c->ring, c->q);
}

} // extern "C"
