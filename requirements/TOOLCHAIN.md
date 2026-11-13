---
id: TOOLCHAIN
status: floor
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
edition: "2024"
crates: none
checked: "2026-11-13"
release: "https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/"
---

# Toolchain floor

The floor is the latest stable on the day this file was checked. It only moves up.

| | Floor |
| --- | --- |
| rustc | 1.100.0 (0ef1bc386, 2026-11-09), announced 2026-11-12 |
| edition | 2024 |
| crates | none |

1.100.0 stabilized the never type (`!`), the core of `std::alloc::Allocator` (`Vec` and `Box` with a custom allocator), and `core::mem::DropGuard`. This tree does not need them yet. Do not pull a crate in just to use one.

## Session ritual

After `scripts/verify-toolchain.sh` prints `gucci`:

1. Open [the latest Rust release](https://blog.rust-lang.org/releases/latest) and the detailed notes linked from it.
2. If stable is newer than `toolchain_floor`, stop. Bump `rust-toolchain.toml`, `Cargo.toml` `rust-version`, this file, and the mast line in the same change. Do not compile the session on the old compiler.
3. A new crate needs a requirement id, the crates.io stable version that day as its floor, and a sentence in the PR naming the release note that justified it. No git deps. No yanked versions. No version below the recorded floor.

`x-profile` is not a dependency and not a credential. See `requirements/FR-GATE.md`.
