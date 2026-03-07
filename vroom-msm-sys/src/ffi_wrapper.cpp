// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// FFI wrapper exposing VROOM's BLS12-381 MSM to Rust via extern "C" functions.
// Converts between BLST Montgomery format (used by the blst Rust crate) and
// VROOM's RNS representation.

#include "../vroom/cpu/precompute/gmp_wrapper.hpp"
extern "C" {
#include "../vroom/blst/vect.h"
#include "../vroom/blst/point.h"
}
#include "../vroom/src/msm.hpp"
#include "../vroom/src/bounded_ring.hpp"
#include "../vroom/src/conversion_inversion.hpp"
#include <cstring>
#include <vector>

// BLS12-381 base field modulus
static const char* BLS12_381_Q_HEX =
    "1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab";

// Montgomery constants for converting between BLST Montgomery form and normal form.
// R = 2^384, R_inv = R^{-1} mod P.
struct MontgomeryConstants {
    BigInt P;
    BigInt R;
    BigInt R_inv;

    MontgomeryConstants()
        : P(BLS12_381_Q_HEX, 16)
        , R(BigInt(1) << 384)
        , R_inv(R.mod_inverse(P))
    {}
};

static const MontgomeryConstants& mont_consts() {
    static MontgomeryConstants mc;
    return mc;
}

// Ring type for BLS12-381 Fp
using RingType = BoundedRing<381, 8, 52, -1932, 2377, 12>;

// Context holding pre-initialized ring and curve
struct VroomBls12381Context {
    BigInt q;
    RingType ring;
    G1<RingType> g1_curve;

    VroomBls12381Context()
        : q(BLS12_381_Q_HEX, 16)
        , ring(q)
        , g1_curve()
    {}
};

// Convert BLST Montgomery vec384 to a normal-form BigInt.
// Uses pure GMP arithmetic instead of BLST functions to avoid symbol conflicts.
static BigInt vec384_mont_to_bigint(const vec384 a) {
    const auto& mc = mont_consts();
    // Read 6 limbs as a 384-bit integer (little-endian limbs)
    BigInt mont_val(0);
    BigInt two_to_64 = BigInt(1) << 64;
    for (int i = 5; i >= 0; i--) {
        mont_val = mont_val * two_to_64 + BigInt(static_cast<unsigned long>(a[i]));
    }
    // Convert from Montgomery: a_normal = a_mont * R^{-1} mod P
    return (mont_val * mc.R_inv) % mc.P;
}

// Convert a normal-form BigInt to BLST Montgomery vec384.
// Uses pure GMP arithmetic instead of BLST functions to avoid symbol conflicts.
static void bigint_to_vec384_mont(vec384 out, const BigInt& a) {
    const auto& mc = mont_consts();
    // Convert to Montgomery: a_mont = a_normal * R mod P
    BigInt mont_val = (a * mc.R) % mc.P;
    // Write to limbs (little-endian)
    memset(out, 0, sizeof(vec384));
    BigInt temp = mont_val;
    BigInt mask64 = (BigInt(1) << 64) - BigInt(1);
    for (int i = 0; i < 6; i++) {
        out[i] = static_cast<limb_t>((temp & mask64).to_ulong());
        temp = temp >> 64;
    }
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

// Convert a VROOM ProjectivePoint to BLST projective (Montgomery) format.
// The output is written as a blst_p1: 3 x vec384 (X, Y, Z) = 144 bytes.
static void vroom_proj_to_blst(
    uint8_t* out,
    const ProjectivePoint<RingType::StandardElement>& point,
    const RingType& ring,
    const BigInt& q
) {
    // Convert each coordinate from RNS to BigInt (normal form),
    // then to BLST Montgomery form.
    BigInt x_bi = ring.to_bigint(point.x);
    BigInt y_bi = ring.to_bigint(point.y);
    BigInt z_bi = ring.to_bigint(point.z);

    // Handle point at infinity: BLST uses Z=0 representation
    POINTonE1 blst_point;
    memset(&blst_point, 0, sizeof(blst_point));

    if (z_bi == BigInt(0)) {
        // Point at infinity
        memcpy(out, &blst_point, sizeof(POINTonE1));
        return;
    }

    bigint_to_vec384_mont(blst_point.X, x_bi);
    bigint_to_vec384_mont(blst_point.Y, y_bi);
    bigint_to_vec384_mont(blst_point.Z, z_bi);

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
