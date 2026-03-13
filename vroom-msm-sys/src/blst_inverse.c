// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// Minimal wrapper providing ct_inverse_mod_384_wrapper for the VROOM
// batch affine MSM (pippenger_v2). All assembly symbols (ctx_inverse_mod_384,
// redcx_mont_384, fromx_mont_384) and constants (BLS12_381_P, p0) are
// resolved from the Rust `blst` crate's libblst.a at link time.

#include "../vroom/blst/vect.h"
#include "../vroom/blst/consts.h"

void ct_inverse_mod_384_wrapper(vec384 out, const vec384 inp)
{
    static const vec384 Px8 = {    /* left-aligned value of the modulus */
        TO_LIMB_T(0xcff7fffffffd5558), TO_LIMB_T(0xf55ffff58a9ffffd),
        TO_LIMB_T(0x39869507b587b120), TO_LIMB_T(0x23ba5c279c2895fb),
        TO_LIMB_T(0x58dd3db21a5d66bb), TO_LIMB_T(0xd0088f51cbff34d2)
    };
    union { vec768 x; vec384 r[2]; } temp;

    ct_inverse_mod_384(temp.x, inp, BLS12_381_P, Px8);
    redc_mont_384(temp.r[0], temp.x, BLS12_381_P, p0);
    from_mont_384(out, temp.r[0], BLS12_381_P, p0);
}
