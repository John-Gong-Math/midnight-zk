// Direct wrapper-path benchmark driver.
// Compiled and run on VM to isolate wrapper C++ path from Rust Criterion/runtime.

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

extern "C" {
void* vroom_ctx_new();
void vroom_ctx_free(void* ctx);
void* vroom_generate_points(void* ctx, size_t npoints, uint64_t seed);
void vroom_free_points(void* points);
void* vroom_generate_scalars(size_t npoints, uint64_t seed);
void vroom_free_scalars(void* scalars);
uint64_t vroom_g1_msm(void* ctx, const void* points, const void* scalars, size_t npoints);
uint64_t vroom_g1_msm_parallel(
    void* ctx,
    const void* points,
    const void* scalars,
    size_t npoints,
    size_t num_threads
);
}

static double median_ms(std::vector<double>& values) {
    std::sort(values.begin(), values.end());
    return values[values.size() / 2];
}

template <typename Fn>
static void run_case(const std::string& label, size_t repeats, Fn fn) {
    std::vector<double> times;
    times.reserve(repeats);

    for (size_t i = 0; i < repeats; i++) {
        auto t0 = std::chrono::steady_clock::now();
        volatile uint64_t sink = fn();
        (void)sink;
        auto t1 = std::chrono::steady_clock::now();
        double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
        times.push_back(ms);
    }

    double best = *std::min_element(times.begin(), times.end());
    double med = median_ms(times);
    double avg = 0.0;
    for (double t : times) avg += t;
    avg /= static_cast<double>(times.size());

    std::cout << label << ": best=" << best << " ms median=" << med << " ms avg=" << avg
              << " ms samples=" << repeats << "\n";
}

int main(int argc, char** argv) {
    size_t n = (argc > 1) ? static_cast<size_t>(std::strtoull(argv[1], nullptr, 10)) : (1ULL << 20);
    size_t repeats = (argc > 2) ? static_cast<size_t>(std::strtoull(argv[2], nullptr, 10)) : 3;
    size_t num_threads = std::thread::hardware_concurrency();
    if (num_threads == 0) num_threads = 4;

    std::cout << "vroom_wrapper_driver npoints=" << n << " repeats=" << repeats
              << " threads=" << num_threads << "\n";

    auto setup_t0 = std::chrono::steady_clock::now();
    void* ctx = vroom_ctx_new();
    void* points = vroom_generate_points(ctx, n, 0x5962be5d763d318dULL);
    void* scalars = vroom_generate_scalars(n, 0x5962be5d763d318dULL);
    auto setup_t1 = std::chrono::steady_clock::now();

    std::cout << "setup: "
              << std::chrono::duration<double, std::milli>(setup_t1 - setup_t0).count() << " ms\n";

    run_case("wrapper_single", repeats, [&]() { return vroom_g1_msm(ctx, points, scalars, n); });
    run_case("wrapper_parallel", repeats, [&]() {
        return vroom_g1_msm_parallel(ctx, points, scalars, n, num_threads);
    });

    vroom_free_scalars(scalars);
    vroom_free_points(points);
    vroom_ctx_free(ctx);
    return 0;
}
