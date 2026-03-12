// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use midnight_curves::pairing::group::Group;

fn median_ms(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

fn run_case(label: &str, repeats: usize, mut f: impl FnMut()) {
    let mut times_ms = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        let start = Instant::now();
        f();
        times_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let best = times_ms
        .iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
    let median = median_ms(times_ms.clone());
    let avg = times_ms.iter().sum::<f64>() / times_ms.len() as f64;

    println!(
        "{label}: best={best:.3} ms median={median:.3} ms avg={avg:.3} ms samples={repeats}"
    );
}

fn main() {
    // Ensure BLST symbols are linked in this standalone bench binary.
    let dummy_point = midnight_curves::G1Projective::generator();
    let dummy_scalar = midnight_curves::Fq::from(1u64);
    let _ = midnight_curves::G1Projective::multi_exp(&[dummy_point], &[dummy_scalar]);

    let n: usize = std::env::var("NPOINTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1 << 20);
    let repeats: usize = std::env::var("REPEATS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!("vroom_ffi_direct npoints={n} repeats={repeats} threads={num_threads}");

    let setup_start = Instant::now();
    let ctx = unsafe { vroom_msm_sys::vroom_ctx_new() };
    let points = unsafe { vroom_msm_sys::vroom_generate_points(ctx, n, 0x5962be5d763d318d) };
    let scalars = unsafe { vroom_msm_sys::vroom_generate_scalars(n, 0x5962be5d763d318d) };
    println!(
        "setup: {:.3} ms",
        setup_start.elapsed().as_secs_f64() * 1000.0
    );

    run_case("vroom_ffi_single", repeats, || unsafe {
        let _ = vroom_msm_sys::vroom_g1_msm(ctx, points, scalars, n);
    });

    run_case("vroom_ffi_parallel", repeats, || unsafe {
        let _ = vroom_msm_sys::vroom_g1_msm_parallel(ctx, points, scalars, n, num_threads);
    });

    unsafe {
        vroom_msm_sys::vroom_free_scalars(scalars);
        vroom_msm_sys::vroom_free_points(points);
        vroom_msm_sys::vroom_ctx_free(ctx);
    }
}
