# google-chat-but-in-rust

Fork of [k-dot-greyz/rust-app-core](https://github.com/k-dot-greyz/rust-app-core) at `6523b791`. The hull is the frontend and the gate. This tree does not add a card kind, a pipe step, or a crate.

Start with [docs/INDEX.md](docs/INDEX.md). It is the enforced catalog. [docs/BIBLE.md](docs/BIBLE.md) is the map. [docs/LOG.md](docs/LOG.md) is the append-only reasoning record. The tests and the Rust types win if the prose disagrees.

| | State |
| --- | --- |
| Toolchain | Rust 1.100.0 floor, edition 2024, zero crates |
| Gate | Green. Seven profiles, compiled in. |
| FR-1 ingest | Red |
| FR-2 git | Red |
| FR-3 model route | Red |
| n8n | Not in this phase |

`scripts/verify.sh` is the workspace check. It passes only when the green suite passes and the FR suite still fails on `NotImplemented`.
