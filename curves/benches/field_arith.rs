//! Benchmark field arithmetic operations.
//!
//! Benchmarks:
//! - Single field ops (add, sub, mul, square, neg, double, invert, pow, sqrt)
//! - VROOM single ops (same set, via AVX512-IFMA RNS Montgomery)
//! - 2^20 multiplication chain: sequential a = a*b (serial latency)
//! - 2^20 batched multiplications: batch_modmul<6> (latency hiding)
//! - Sum of 2 products: a*b + c*d (shared reduction)
//! - VROOM inversion via GMP (convert-invert-convert optimization)
//!
//! To run: cargo bench --bench field_arith -p midnight-curves

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ff::Field;
use midnight_curves::*;
use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use vroom_fields_sys::*;

const SEED: [u8; 16] = [
    0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06, 0xbc,
    0xe5,
];

const N_MULS: u64 = 1 << 20; // 2^20 multiplications
const BATCH_SIZE: u64 = 6; // Must match FP_BATCH/FR_BATCH in C++

fn bench_field_arithmetic<F: Field>(c: &mut Criterion, name: &'static str) {
    let mut rng = XorShiftRng::from_seed(SEED);

    let a = <F as Field>::random(&mut rng);
    let b = <F as Field>::random(&mut rng);
    let mut ret = a;
    let exp = rng.next_u64();

    let mut group = c.benchmark_group(format!("{} arithmetic", name));

    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    group.bench_function(format!("{}_add", name), |bencher| {
        bencher.iter(|| black_box(&a).add(black_box(&b)))
    });

    group.bench_function(format!("{}_add_assign", name), |bencher| {
        bencher.iter(|| black_box(&mut ret).add_assign(black_box(&b)))
    });

    group.bench_function(format!("{}_sub", name), |bencher| {
        bencher.iter(|| black_box(&a).sub(black_box(&b)))
    });

    group.bench_function(format!("{}_sub_assign", name), |bencher| {
        bencher.iter(|| black_box(&mut ret).sub_assign(black_box(&b)))
    });

    group.bench_function(format!("{}_double", name), |bencher| {
        bencher.iter(|| black_box(&a).double())
    });

    group.bench_function(format!("{}_neg", name), |bencher| {
        bencher.iter(|| black_box(&a).neg())
    });

    group.bench_function(format!("{}_mul", name), |bencher| {
        bencher.iter(|| black_box(&a).mul(black_box(&b)))
    });

    group.bench_function(format!("{}_mul_assign", name), |bencher| {
        bencher.iter(|| black_box(&mut ret).mul_assign(black_box(&b)))
    });

    group.bench_function(format!("{}_square", name), |bencher| {
        bencher.iter(|| black_box(&a).square())
    });

    group.bench_function(format!("{}_pow_vartime", name), |bencher| {
        bencher.iter(|| black_box(&a).pow_vartime(black_box(&[exp])))
    });

    group.bench_function(format!("{}_invert", name), |bencher| {
        bencher.iter(|| black_box(&a).invert())
    });

    group.bench_function(format!("{}_sqrt", name), |bencher| {
        bencher.iter(|| black_box(&a).sqrt())
    });
    group.finish()
}

fn bench_bls_base_field(c: &mut Criterion) {
    bench_field_arithmetic::<Fp>(c, "base-field")
}

fn bench_bls_scalar_field(c: &mut Criterion) {
    bench_field_arithmetic::<Fq>(c, "scalar-field")
}

// =========================================================================
//  VROOM single-op benchmarks (same groups as native for side-by-side)
// =========================================================================

fn bench_vroom_fp_arithmetic(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);
    let a = Fp::random(&mut rng);
    let b = Fp::random(&mut rng);

    let ctx = unsafe { vroom_fp_ctx_new() };
    assert!(!ctx.is_null());
    unsafe {
        vroom_fp_load_a(ctx, a.to_bytes_le().as_ptr());
        vroom_fp_load_b(ctx, b.to_bytes_le().as_ptr());
    }

    let mut group = c.benchmark_group("base-field arithmetic");
    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    group.bench_function("vroom_base-field_add", |b| {
        b.iter(|| unsafe { vroom_fp_add(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_sub", |b| {
        b.iter(|| unsafe { vroom_fp_sub(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_double", |b| {
        b.iter(|| unsafe { vroom_fp_double(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_neg", |b| {
        b.iter(|| unsafe { vroom_fp_neg(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_mul", |b| {
        b.iter(|| unsafe { vroom_fp_mul(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_square", |b| {
        b.iter(|| unsafe { vroom_fp_square(black_box(ctx)) })
    });
    group.bench_function("vroom_base-field_invert", |b| {
        b.iter(|| unsafe { vroom_fp_invert(black_box(ctx)) })
    });

    group.finish();
    unsafe { vroom_fp_ctx_free(ctx) };
}

fn bench_vroom_fr_arithmetic(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);
    let a = Fq::random(&mut rng);
    let b = Fq::random(&mut rng);

    let ctx = unsafe { vroom_fr_ctx_new() };
    assert!(!ctx.is_null());
    unsafe {
        vroom_fr_load_a(ctx, a.to_bytes_le().as_ptr());
        vroom_fr_load_b(ctx, b.to_bytes_le().as_ptr());
    }

    let mut group = c.benchmark_group("scalar-field arithmetic");
    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    group.bench_function("vroom_scalar-field_add", |b| {
        b.iter(|| unsafe { vroom_fr_add(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_sub", |b| {
        b.iter(|| unsafe { vroom_fr_sub(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_double", |b| {
        b.iter(|| unsafe { vroom_fr_double(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_neg", |b| {
        b.iter(|| unsafe { vroom_fr_neg(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_mul", |b| {
        b.iter(|| unsafe { vroom_fr_mul(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_square", |b| {
        b.iter(|| unsafe { vroom_fr_square(black_box(ctx)) })
    });
    group.bench_function("vroom_scalar-field_invert", |b| {
        b.iter(|| unsafe { vroom_fr_invert(black_box(ctx)) })
    });

    group.finish();
    unsafe { vroom_fr_ctx_free(ctx) };
}

// =========================================================================
//  2^20 multiplication chain benchmarks
// =========================================================================

fn bench_mul_chain(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);

    // --- Fp ---
    let fp_a = Fp::random(&mut rng);
    let fp_b = Fp::random(&mut rng);
    let fq_a = Fq::random(&mut rng);
    let fq_b = Fq::random(&mut rng);

    let mut group = c.benchmark_group("mul_chain_2^20");
    group.significance_level(0.1).sample_size(10);
    group.throughput(Throughput::Elements(N_MULS));

    // Native blst Fp chain
    group.bench_function("blst_Fp_chain", |bencher| {
        bencher.iter(|| {
            let mut a = fp_a;
            for _ in 0..N_MULS {
                a = a * fp_b;
            }
            black_box(a)
        })
    });

    // VROOM Fp sequential chain
    {
        let ctx = unsafe { vroom_fp_ctx_new() };
        unsafe {
            vroom_fp_load_a(ctx, fp_a.to_bytes_le().as_ptr());
            vroom_fp_load_b(ctx, fp_b.to_bytes_le().as_ptr());
        }
        group.bench_function("vroom_Fp_chain", |bencher| {
            bencher.iter(|| {
                // Reload a for each iteration
                unsafe {
                    vroom_fp_load_a(ctx, fp_a.to_bytes_le().as_ptr());
                    vroom_fp_mul_chain(black_box(ctx), N_MULS);
                }
            })
        });
        unsafe { vroom_fp_ctx_free(ctx) };
    }

    // VROOM Fp batched chain (6 parallel chains, same total muls)
    {
        let batch_ctx = unsafe { vroom_fp_batch_ctx_new() };
        let mut rng2 = XorShiftRng::from_seed(SEED);
        let fp_pairs: Vec<(Fp, Fp)> = (0..BATCH_SIZE)
            .map(|_| (Fp::random(&mut rng2), Fp::random(&mut rng2)))
            .collect();
        for (i, (a, b)) in fp_pairs.iter().enumerate() {
            unsafe {
                vroom_fp_batch_load(
                    batch_ctx,
                    i as i32,
                    a.to_bytes_le().as_ptr(),
                    b.to_bytes_le().as_ptr(),
                );
            }
        }
        let n_iters = N_MULS / BATCH_SIZE;
        group.bench_function("vroom_Fp_batch6_chain", |bencher| {
            bencher.iter(|| {
                // Reload for each criterion iteration
                for (i, (a, b)) in fp_pairs.iter().enumerate() {
                    unsafe {
                        vroom_fp_batch_load(
                            batch_ctx,
                            i as i32,
                            a.to_bytes_le().as_ptr(),
                            b.to_bytes_le().as_ptr(),
                        );
                    }
                }
                unsafe { vroom_fp_batch_mul_chain(black_box(batch_ctx), n_iters) }
            })
        });
        unsafe { vroom_fp_batch_ctx_free(batch_ctx) };
    }

    // Native blst Fr chain
    group.bench_function("blst_Fr_chain", |bencher| {
        bencher.iter(|| {
            let mut a = fq_a;
            for _ in 0..N_MULS {
                a = a * fq_b;
            }
            black_box(a)
        })
    });

    // VROOM Fr sequential chain
    {
        let ctx = unsafe { vroom_fr_ctx_new() };
        unsafe {
            vroom_fr_load_a(ctx, fq_a.to_bytes_le().as_ptr());
            vroom_fr_load_b(ctx, fq_b.to_bytes_le().as_ptr());
        }
        group.bench_function("vroom_Fr_chain", |bencher| {
            bencher.iter(|| {
                unsafe {
                    vroom_fr_load_a(ctx, fq_a.to_bytes_le().as_ptr());
                    vroom_fr_mul_chain(black_box(ctx), N_MULS);
                }
            })
        });
        unsafe { vroom_fr_ctx_free(ctx) };
    }

    // VROOM Fr batched chain
    {
        let batch_ctx = unsafe { vroom_fr_batch_ctx_new() };
        let mut rng2 = XorShiftRng::from_seed(SEED);
        let fr_pairs: Vec<(Fq, Fq)> = (0..BATCH_SIZE)
            .map(|_| (Fq::random(&mut rng2), Fq::random(&mut rng2)))
            .collect();
        for (i, (a, b)) in fr_pairs.iter().enumerate() {
            unsafe {
                vroom_fr_batch_load(
                    batch_ctx,
                    i as i32,
                    a.to_bytes_le().as_ptr(),
                    b.to_bytes_le().as_ptr(),
                );
            }
        }
        let n_iters = N_MULS / BATCH_SIZE;
        group.bench_function("vroom_Fr_batch6_chain", |bencher| {
            bencher.iter(|| {
                for (i, (a, b)) in fr_pairs.iter().enumerate() {
                    unsafe {
                        vroom_fr_batch_load(
                            batch_ctx,
                            i as i32,
                            a.to_bytes_le().as_ptr(),
                            b.to_bytes_le().as_ptr(),
                        );
                    }
                }
                unsafe { vroom_fr_batch_mul_chain(black_box(batch_ctx), n_iters) }
            })
        });
        unsafe { vroom_fr_batch_ctx_free(batch_ctx) };
    }

    group.finish();
}

// =========================================================================
//  Sum of 2 products benchmark: a*b + c*d
// =========================================================================

fn bench_sum_of_products(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);
    let fp_a = Fp::random(&mut rng);
    let fp_b = Fp::random(&mut rng);
    let fp_c = Fp::random(&mut rng);
    let fp_d = Fp::random(&mut rng);
    let fq_a = Fq::random(&mut rng);
    let fq_b = Fq::random(&mut rng);
    let fq_c = Fq::random(&mut rng);
    let fq_d = Fq::random(&mut rng);

    let mut group = c.benchmark_group("sum_of_2_products");
    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    // blst Fp: 2 muls + 1 add
    group.bench_function("blst_Fp_2muls_add", |bencher| {
        bencher.iter(|| {
            let ab = black_box(fp_a) * black_box(fp_b);
            let cd = black_box(fp_c) * black_box(fp_d);
            black_box(ab + cd)
        })
    });

    // VROOM Fp: single reduction
    {
        let ctx = unsafe { vroom_fp_ctx_new() };
        unsafe {
            vroom_fp_load_a(ctx, fp_a.to_bytes_le().as_ptr());
            vroom_fp_load_b(ctx, fp_b.to_bytes_le().as_ptr());
            vroom_fp_load_c(ctx, fp_c.to_bytes_le().as_ptr());
            vroom_fp_load_d(ctx, fp_d.to_bytes_le().as_ptr());
        }
        group.bench_function("vroom_Fp_sum2prod", |bencher| {
            bencher.iter(|| unsafe { vroom_fp_sum_of_2_products(black_box(ctx)) })
        });
        unsafe { vroom_fp_ctx_free(ctx) };
    }

    // blst Fr: 2 muls + 1 add
    group.bench_function("blst_Fr_2muls_add", |bencher| {
        bencher.iter(|| {
            let ab = black_box(fq_a) * black_box(fq_b);
            let cd = black_box(fq_c) * black_box(fq_d);
            black_box(ab + cd)
        })
    });

    // VROOM Fr: single reduction
    {
        let ctx = unsafe { vroom_fr_ctx_new() };
        unsafe {
            vroom_fr_load_a(ctx, fq_a.to_bytes_le().as_ptr());
            vroom_fr_load_b(ctx, fq_b.to_bytes_le().as_ptr());
            vroom_fr_load_c(ctx, fq_c.to_bytes_le().as_ptr());
            vroom_fr_load_d(ctx, fq_d.to_bytes_le().as_ptr());
        }
        group.bench_function("vroom_Fr_sum2prod", |bencher| {
            bencher.iter(|| unsafe { vroom_fr_sum_of_2_products(black_box(ctx)) })
        });
        unsafe { vroom_fr_ctx_free(ctx) };
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_bls_base_field,
    bench_bls_scalar_field,
    bench_vroom_fp_arithmetic,
    bench_vroom_fr_arithmetic,
    bench_mul_chain,
    bench_sum_of_products,
);
criterion_main!(benches);
