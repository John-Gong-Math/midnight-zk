// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Benchmark for `msm_specific` which is the hybrid MSM used in KZG.
//!
//! To run this benchmark:
//!
//!     cargo bench -p midnight-proofs --bench msm_specific

#[macro_use]
extern crate criterion;

use std::time::SystemTime;

use criterion::{BenchmarkId, Criterion};
use ff::Field;
use group::Group;
use midnight_proofs::poly::kzg::msm::msm_specific;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use rayon::{
    current_thread_index,
    prelude::{IntoParallelIterator, ParallelIterator},
};

const SAMPLE_SIZE: usize = 10;
const SEED: [u8; 16] = [
    0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06, 0xbc,
    0xe5,
];

const MULTICORE_RANGE: &[u8] = &[20];

fn generate_bases(k: u8) -> Vec<midnight_curves::G1Projective> {
    let n: u64 = 1 << k;
    println!("Generating 2^{k} = {n} projective curve points..");

    let timer = SystemTime::now();
    let bases = (0..n)
        .into_par_iter()
        .map_init(
            || {
                let mut thread_seed = SEED;
                let uniq = current_thread_index().unwrap().to_ne_bytes();
                assert!(std::mem::size_of::<usize>() == 8);
                for i in 0..uniq.len() {
                    thread_seed[i] += uniq[i];
                    thread_seed[i + 8] += uniq[i];
                }
                XorShiftRng::from_seed(thread_seed)
            },
            |rng, _| midnight_curves::G1Projective::random(rng),
        )
        .collect();
    let end = timer.elapsed().unwrap();
    println!(
        "Generating 2^{k} = {n} projective curve points took: {} sec.\n",
        end.as_secs()
    );
    bases
}

fn generate_coefficients(k: u8) -> Vec<midnight_curves::Fq> {
    let n: u64 = 1 << k;

    (0..n)
        .into_par_iter()
        .map_init(
            || {
                let mut thread_seed = SEED;
                let uniq = current_thread_index().unwrap().to_ne_bytes();
                assert!(std::mem::size_of::<usize>() == 8);
                for i in 0..uniq.len() {
                    thread_seed[i] += uniq[i];
                    thread_seed[i + 8] += uniq[i];
                }
                XorShiftRng::from_seed(thread_seed)
            },
            |rng, _| midnight_curves::Fq::random(rng),
        )
        .collect()
}

fn bench_msm_specific(c: &mut Criterion) {
    let mut group = c.benchmark_group("MsmSpecific");
    group.significance_level(0.1).sample_size(SAMPLE_SIZE);

    let max_k = *MULTICORE_RANGE.iter().max().unwrap();
    let bases = generate_bases(max_k);
    let coeffs = generate_coefficients(max_k);

    for k in MULTICORE_RANGE {
        let n: usize = 1 << k;
        let id = format!("msm_specific_256b_{k}");
        group.bench_function(BenchmarkId::new("msm_specific", id), |b| {
            b.iter(|| {
                msm_specific::<midnight_curves::G1Affine>(&coeffs[..n], &bases[..n])
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_msm_specific);
criterion_main!(benches);
