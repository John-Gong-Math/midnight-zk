// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// Shared type definitions for the VROOM field arithmetic FFI wrapper.

#pragma once

#include "../vroom/src/bounded_ring.hpp"
#include "../vroom/src/fr.hpp"
#include "../vroom/src/inversion.hpp"

#include <cstdint>

// ----- BLS12-381 field types -----

// Base field Fp (381 bits, 8 limbs)
using FpRing = BoundedRing<381, 8, 52, -1932, 2377, 12>;

// Scalar field Fr (255 bits, 6 limbs)
// FrRing is already defined in fr.hpp

// ----- Modulus hex strings -----

static const char* fp_modulus_hex =
    "1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf"
    "6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab";

static const char* fr_modulus_hex =
    "73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001";

// ----- Context structs -----

struct VroomFpContext {
    FpRing ring;
    BLS381AddChainInversion<FpRing> inverter;
    FpRing::StandardElement a;
    FpRing::StandardElement b;
    FpRing::StandardElement result;

    VroomFpContext()
        : ring(BigInt(fp_modulus_hex, 16))
        , inverter(ring)
        , a(FpRing::zero())
        , b(FpRing::zero())
        , result(FpRing::zero())
    {}
};

struct VroomFrContext {
    FrRing ring;
    FrRing::StandardElement a;
    FrRing::StandardElement b;
    FrRing::StandardElement result;

    VroomFrContext()
        : ring(BigInt(fr_modulus_hex, 16))
        , a(FrRing::zero())
        , b(FrRing::zero())
        , result(FrRing::zero())
    {}
};
