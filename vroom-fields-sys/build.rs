// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn find_gmp_include() -> Option<PathBuf> {
    for path in &["/opt/homebrew/include", "/usr/local/include", "/usr/include"] {
        let p = PathBuf::from(path);
        if p.join("gmp.h").exists() {
            return Some(p);
        }
    }
    None
}

fn main() {
    let vroom_dir = PathBuf::from("vroom");
    let blst_dir = vroom_dir.join("blst");
    let src_dir = vroom_dir.join("src");
    let cpu_dir = vroom_dir.join("cpu");

    let includes: Vec<PathBuf> = vec![
        src_dir.clone(),
        vroom_dir.clone(),
        cpu_dir.join("precompute"),
    ];

    let gmp_include = find_gmp_include();

    // Use clang++ if available (better SIMD codegen)
    let cxx = std::env::var("VROOM_CXX").unwrap_or_else(|_| {
        if std::process::Command::new("clang++")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "clang++".to_string()
        } else {
            "c++".to_string()
        }
    });

    let mut build = cc::Build::new();
    build
        .compiler(cxx.as_str())
        .cpp(true)
        .std("c++20")
        .flag("-O3")
        .flag("-march=native")
        .flag("-mavx512ifma")
        .define("__ADX__", None)
        .flag("-Wno-unused-function")
        .flag("-Wno-ignored-attributes")
        .flag("-Wno-sign-conversion")
        .flag("-Wno-unknown-pragmas");

    for inc in &includes {
        build.include(inc);
    }
    build.include(&blst_dir);
    if let Some(ref gmp_inc) = gmp_include {
        build.include(gmp_inc);
    }

    build.file("src/wrapper_fields.cpp");
    build.compile("vroom_fields");

    // Add GMP library search paths
    for path in &["/opt/homebrew/lib", "/usr/local/lib"] {
        if std::path::Path::new(path).exists() {
            println!("cargo:rustc-link-search=native={path}");
        }
    }

    // Link libraries
    println!("cargo:rustc-link-lib=gmp");
    println!("cargo:rustc-link-lib=gmpxx");
    println!("cargo:rustc-link-lib=stdc++");

    // Rerun triggers
    println!("cargo:rerun-if-changed=src/wrapper_fields.cpp");
    println!("cargo:rerun-if-changed=src/wrapper_fields_types.hpp");
    println!("cargo:rerun-if-changed=vroom/");
}
