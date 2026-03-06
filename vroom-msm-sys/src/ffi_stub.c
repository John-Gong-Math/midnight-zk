// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// Stub for non-x86-64 platforms. All functions abort at runtime.

#include <stdlib.h>
#include <stdint.h>
#include <stddef.h>

void* vroom_bls12_381_init(void) { abort(); }
void vroom_bls12_381_free(void* ctx) { (void)ctx; abort(); }
void vroom_g1_msm(void* ctx, uint8_t* out, const uint8_t* points,
                   const uint8_t* scalars, size_t npoints) {
    (void)ctx; (void)out; (void)points; (void)scalars; (void)npoints;
    abort();
}
void vroom_g1_msm_parallel(void* ctx, uint8_t* out, const uint8_t* points,
                            const uint8_t* scalars, size_t npoints,
                            size_t num_threads) {
    (void)ctx; (void)out; (void)points; (void)scalars; (void)npoints;
    (void)num_threads;
    abort();
}
