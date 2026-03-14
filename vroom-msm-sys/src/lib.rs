use std::ffi::c_void;

extern "C" {
    pub fn vroom_ctx_new() -> *mut c_void;
    pub fn vroom_ctx_free(ctx: *mut c_void);
    pub fn vroom_generate_points(ctx: *mut c_void, npoints: usize, seed: u64) -> *mut c_void;
    pub fn vroom_free_points(points: *mut c_void);
    pub fn vroom_generate_scalars(npoints: usize, seed: u64) -> *mut c_void;
    pub fn vroom_free_scalars(scalars: *mut c_void);
    pub fn vroom_g1_msm(
        ctx: *mut c_void,
        points: *const c_void,
        scalars: *const c_void,
        npoints: usize,
    );
    pub fn vroom_g1_msm_parallel(
        ctx: *mut c_void,
        points: *const c_void,
        scalars: *const c_void,
        npoints: usize,
        num_threads: usize,
    );
    pub fn vroom_g1_pippenger_v1(
        ctx: *mut c_void,
        points: *const c_void,
        scalars: *const c_void,
        npoints: usize,
    );
    pub fn vroom_g1_pippenger_v1_parallel(
        ctx: *mut c_void,
        points: *const c_void,
        scalars: *const c_void,
        npoints: usize,
        num_threads: usize,
    );
    pub fn vroom_g1_msm_parallel_matches_serial(
        ctx: *mut c_void,
        points: *const c_void,
        scalars: *const c_void,
        npoints: usize,
        num_threads: usize,
    ) -> bool;
}

#[cfg(test)]
mod tests {
    use blst::blst_p1;
    use super::*;

    const TEST_SEED: u64 = 0x5962be5d763d318d;

    struct TestFixture {
        ctx: *mut c_void,
        points: *mut c_void,
        scalars: *mut c_void,
    }

    impl TestFixture {
        fn new(npoints: usize) -> Self {
            let ctx = unsafe { vroom_ctx_new() };
            assert!(!ctx.is_null(), "failed to allocate VROOM context");

            let points = unsafe { vroom_generate_points(ctx, npoints, TEST_SEED) };
            assert!(!points.is_null(), "failed to generate points");

            let scalars = unsafe { vroom_generate_scalars(npoints, TEST_SEED) };
            assert!(!scalars.is_null(), "failed to generate scalars");

            Self {
                ctx,
                points,
                scalars,
            }
        }

        fn msm_parallel_matches_serial(&self, npoints: usize, num_threads: usize) -> bool {
            unsafe {
                vroom_g1_msm_parallel_matches_serial(
                    self.ctx,
                    self.points,
                    self.scalars,
                    npoints,
                    num_threads,
                )
            }
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            unsafe {
                if !self.scalars.is_null() {
                    vroom_free_scalars(self.scalars);
                }
                if !self.points.is_null() {
                    vroom_free_points(self.points);
                }
                if !self.ctx.is_null() {
                    vroom_ctx_free(self.ctx);
                }
            }
        }
    }

    #[test]
    fn vroom_parallel_matches_serial_output() {
        let _ = std::mem::size_of::<blst_p1>();
        let n_max = 1 << 12;
        let fixture = TestFixture::new(n_max);
        let cases = [1 << 8, 1 << 10, 1 << 12];
        let thread_counts = [1, 2, 4];

        for &n in &cases {
            for &threads in &thread_counts {
                let matches = fixture.msm_parallel_matches_serial(n, threads);
                assert!(
                    matches,
                    "parallel msm mismatch for npoints={n}, num_threads={threads}"
                );
            }
        }
    }

    #[test]
    fn vroom_parallel_matches_serial_sweep() {
        let _ = std::mem::size_of::<blst_p1>();

        let k_values = [8usize, 10, 12, 14, 16];
        let n_max = 1 << k_values[k_values.len() - 1];
        let fixture = TestFixture::new(n_max);

        let hw_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let thread_candidates = [1usize, 2, 4, 8, 16, hw_threads];

        for &k in &k_values {
            let n = 1 << k;
            for &threads in &thread_candidates {
                // Skip obviously invalid over-subscription in this coverage test.
                if threads == 0 || threads > n {
                    continue;
                }

                let matches = fixture.msm_parallel_matches_serial(n, threads);
                assert!(
                    matches,
                    "parallel msm mismatch for npoints={n}, num_threads={threads}"
                );
            }
        }
    }
}
