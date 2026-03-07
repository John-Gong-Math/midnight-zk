# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Midnight ZK implements the zero-knowledge proof system used in **Midnight**, along with tooling for building ZK circuits. It is a Cargo workspace based on a modified fork of Halo2 v0.3.0 (PSE variant) with KZG commitments over BLS12-381.

## Build & Test Commands

**Rust toolchain:** 1.90.0 (enforced via `rust-toolchain.toml`)

```bash
# Build entire workspace
cargo build

# Test a specific crate (always use --release for circuits/proofs/zk_stdlib)
cargo test -p midnight-proofs --release
cargo test -p midnight-curves --release --all-features
cargo test -p midnight-circuits --release
cargo test -p midnight-zk-stdlib --release -- --skip serialization
cargo test -p midnight-aggregator --release --all-features
cargo test -p midnight-zkir --release --all-features

# Run a single test
cargo test -p midnight-circuits --release -- test_name

# Serialization tests must run single-threaded
cargo test -p midnight-zk-stdlib --release --test serialization -- --test-threads=1

# VK consistency / golden file tests (run ignored tests)
cargo test -p midnight-zk-stdlib --release -- --ignored

# Lint
cargo clippy --all-targets --all-features -- -Dwarnings
cargo clippy -p midnight-aggregator --all-targets --all-features -- -Dwarnings

# Format (requires nightly)
cargo +nightly fmt --all -- --check

# Check documentation links
cargo doc --workspace --document-private-items --no-deps

# Benchmarks (uses criterion)
cargo bench -p midnight-curves -- msm
cargo bench -p midnight-proofs -- plonk
cargo bench -p midnight-circuits -- poseidon
```

**Parallelism:** Control thread count with `RAYON_NUM_THREADS` env var.

**SRS file:** zk_stdlib tests/examples require `zk_stdlib/examples/assets/bls_filecoin_2p19` (download from CI config URL if missing).

## Workspace Crates

| Crate | Package Name | Purpose |
|-------|-------------|---------|
| `proofs/` | midnight-proofs | PLONK proof system with KZG commitments |
| `curves/` | midnight-curves | BLS12-381, JubJub, secp256k1, bn256, curve25519 |
| `circuits/` | midnight-circuits | Halo2 gadgets: field ops, ECC, hashing, parsing, in-circuit verification |
| `aggregator/` | midnight-aggregator | Proof aggregation toolkit |
| `zkir/` | midnight-zkir | ZKIR circuit parser |
| `zk_stdlib/` | midnight-zk-stdlib | High-level standard library with `Relation` trait abstraction |
| `vroom-msm-sys/` | vroom-msm-sys | FFI crate for VROOM BLS12-381 multi-scalar multiplication (C++ with `cc` build) |

## Architecture

**Layering** (bottom to top):
- `curves` → `proofs` → `circuits` → `zk_stdlib`
- `aggregator` depends on `proofs` and `curves`
- `zkir` depends on `circuits`

**Key patterns:**
- **Chip architecture (Halo2):** Circuit functionality is encapsulated in "chips" that implement instruction traits. Chips are composed together in circuit configurations.
- **Instruction traits** (`circuits/src/instructions/`): Define interfaces for arithmetic, field ops, ECC, hashing, etc. Implementations live in corresponding chip modules.
- **Native vs non-native field operations:** `circuits/src/field/` provides both native field arithmetic and non-native (foreign field) arithmetic for cross-curve operations.
- **`ZkStdLib` and `ZkStdLibArch`** (`zk_stdlib/`): Configurable architecture struct that selects which chips to enable. The `Relation` trait provides a simplified interface over raw Halo2 for building circuits.
- **Committed instances:** Instance values can be committed (hashed) rather than passed in the clear — controlled by the `committed-instances` feature.
- **Truncated challenges:** The `truncated-challenges` feature enables recursive proof verification by truncating Fiat-Shamir challenges to fit in the scalar field. Incompatible with `aggregator`.

**Golden file testing:** `circuits/goldenfiles/` and `zk_stdlib/goldenfiles/` contain regression snapshots. VK consistency is verified by running `cargo test --release -- --ignored` in zk_stdlib — if goldenfiles change, commit the updated files.

## Key Features

- `committed-instances` — commit instance columns
- `truncated-challenges` — enable recursive verification
- `circuit-params` — parameterized circuits
- `testing` — test utilities in circuits
- `dev-curves` — use bn256 dev curve for faster testing

## License Header

New files should include this Apache 2.0 header:
```
// This file is part of midnight-zk.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
```

## Workspace Patches

The root `Cargo.toml` patches `crates-io` and the GitHub URL to use local crate paths. This ensures type unification across workspace members and third-party integrations that depend on published versions.

## Taking notes

When required to take notes with regard to the conversation, write the conversation contents into a markdown file with the specified name under the folder `./notes`.