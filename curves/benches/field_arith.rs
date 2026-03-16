//! Benchmark field arithmetic operations.
//! It measures the base field `Fp` and scalar field `Scalar` from the Bls12-381
//! curve. Note: The bencharks are generic and can be easily extended for Jubjub
//! scalar field and G2 base field Fp2.
//!
//! To run this benchmark:
//!
//!     cargo bench --bench field_arith

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ff::Field;
use midnight_curves::*;
use rand_core::{RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use vroom_fields_sys::*;

const SEED: [u8; 16] = [
    0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06, 0xbc, 0xe5,
];

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

// ----- VROOM benchmarks -----

fn bench_vroom_fp_arithmetic(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);
    let a = Fp::random(&mut rng);
    let b = Fp::random(&mut rng);

    let a_bytes = a.to_bytes_le();
    let b_bytes = b.to_bytes_le();

    let ctx = unsafe { vroom_fp_ctx_new() };
    assert!(!ctx.is_null());
    unsafe {
        vroom_fp_load_a(ctx, a_bytes.as_ptr());
        vroom_fp_load_b(ctx, b_bytes.as_ptr());
    }

    let mut group = c.benchmark_group("base-field arithmetic");
    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    group.bench_function("vroom_base-field_add", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_add(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_sub", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_sub(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_double", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_double(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_neg", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_neg(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_mul", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_mul(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_square", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_square(black_box(ctx)) })
    });

    group.bench_function("vroom_base-field_invert", |bencher| {
        bencher.iter(|| unsafe { vroom_fp_invert(black_box(ctx)) })
    });

    group.finish();
    unsafe { vroom_fp_ctx_free(ctx) };
}

fn bench_vroom_fr_arithmetic(c: &mut Criterion) {
    let mut rng = XorShiftRng::from_seed(SEED);
    let a = Fq::random(&mut rng);
    let b = Fq::random(&mut rng);

    let a_bytes = a.to_bytes_le();
    let b_bytes = b.to_bytes_le();

    let ctx = unsafe { vroom_fr_ctx_new() };
    assert!(!ctx.is_null());
    unsafe {
        vroom_fr_load_a(ctx, a_bytes.as_ptr());
        vroom_fr_load_b(ctx, b_bytes.as_ptr());
    }

    let mut group = c.benchmark_group("scalar-field arithmetic");
    group.significance_level(0.1).sample_size(1000);
    group.throughput(Throughput::Elements(1));

    group.bench_function("vroom_scalar-field_add", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_add(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_sub", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_sub(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_double", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_double(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_neg", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_neg(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_mul", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_mul(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_square", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_square(black_box(ctx)) })
    });

    group.bench_function("vroom_scalar-field_invert", |bencher| {
        bencher.iter(|| unsafe { vroom_fr_invert(black_box(ctx)) })
    });

    group.finish();
    unsafe { vroom_fr_ctx_free(ctx) };
}

criterion_group!(
    benches,
    bench_bls_base_field,
    bench_bls_scalar_field,
    bench_vroom_fp_arithmetic,
    bench_vroom_fr_arithmetic,
);
criterion_main!(benches);
