// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_arch != "x86_64" {
        // Non-x86-64: compile stub that aborts at runtime
        cc::Build::new()
            .file("src/ffi_stub.c")
            .compile("vroom_msm");
        return;
    }

    // BLST assembly and C code are NOT compiled here — those symbols are
    // provided by the `blst` crate (dependency of midnight-curves).
    // We only need the BLST headers for type definitions in ffi_wrapper.cpp.

    // Compile C++ FFI wrapper (links against blst symbols at final link time)
    cc::Build::new()
        .cpp(true)
        .file("src/ffi_wrapper.cpp")
        .include("vroom")
        .include("vroom/blst")
        .include("vroom/src")
        .include("vroom/cpu/vector")
        .include("vroom/cpu/precompute")
        .include("vroom/cpu/reduction")
        .flag("-std=c++20")
        .flag("-mavx512ifma")
        .flag("-D__ADX__")
        .flag("-O2")
        .compile("vroom_msm");

    // VROOM's RNS precomputation requires GMP
    println!("cargo:rustc-link-lib=dylib=gmp");
    println!("cargo:rustc-link-lib=dylib=gmpxx");

    // On macOS, link C++ standard library
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
