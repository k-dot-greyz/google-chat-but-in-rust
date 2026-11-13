---
id: INDEX
status: reference
enforced_by: AGENTS.md
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
log: docs/LOG.md
upstream: https://github.com/k-dot-greyz/google-chat-but-in-rust
checked: "2026-11-13T17:36:00+02:00"
---

# Reference index

This file is the catalog. [docs/LOG.md](LOG.md) is the intent and reasoning record. Agents open this index before editing. A `reference` row is not a scratchpad: change it only to fix a stale pointer or to record a new decision, and append that decision to the log in the same change. Do not delete or rewrite a log entry.

If this index, the bible, and a test disagree, the test wins. Then fix the index.

## How status is enforced

| Status | Meaning | Agent rule |
| --- | --- | --- |
| `reference` | Settled. Permanent until a new requirement says otherwise. | Do not weaken it. Append a log line if you touch it. |
| `law` | The compiler and the tests. | Edit the code and the test together. Update the index row if the path changes. |
| `floor` | Minimum version. Moves up only. | Re-read the official notes. Append the log. |
| `green` | Implemented and tested. | Do not regress it to make a red test easier. |
| `red` | Specified by a failing test. | Stay red until the assert passes for the reason it names. No stubs. |
| `accepted-not-built` | A decision, not code. | Do not create the files early. |

`AGENTS.md` is the instruction that makes this binding. `CODEOWNERS` names `@k-dot-greyz` on this index, the log, and the docs.

## Catalog

| Path | Status | What it is |
| --- | --- | --- |
| [docs/LOG.md](LOG.md) | reference | Append-only intent and reasoning log. |
| [docs/BIBLE.md](BIBLE.md) | reference | Big picture, task order, test prose, open changes. |
| [docs/WHITEPAPER.md](WHITEPAPER.md) | reference | Why the hull is the record and Chat is the glass. |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | reference | Tree and what is law versus data. |
| [docs/FLOWS.md](FLOWS.md) | reference | Three role-level scenarios. |
| [docs/PLUGINS.md](PLUGINS.md) | reference | Named sinks. Phase 1 runs none of them. |
| [docs/INTEGRATIONS.md](INTEGRATIONS.md) | reference | Edges. No client in the tree. |
| [SPEC.md](../SPEC.md) | reference | Hull contract. Kinds and routes. |
| [requirements/TOOLCHAIN.md](../requirements/TOOLCHAIN.md) | floor | Rust 1.100.0. Empty crate list. |
| [requirements/FR-GATE.md](../requirements/FR-GATE.md) | green | Profile scope. Tests in `tests/gate.rs`. |
| [requirements/FR-1.md](../requirements/FR-1.md) | red | Ingest and button approve. |
| [requirements/FR-2.md](../requirements/FR-2.md) | red | One installation, one repo. |
| [requirements/FR-3.md](../requirements/FR-3.md) | red | Model route. Change the token shape before implementing. |
| [AGENTS.md](../AGENTS.md) | reference | Session instructions. |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | reference | What a change must do. |
| [CODEOWNERS](../CODEOWNERS) | reference | Review ownership. |

## Artifacts that are law or data

| Path | Status | Notes |
| --- | --- | --- |
| `modules/profiles.json` | law | Compiled in. Not watched. Seven profiles. |
| `modules/policy.json` | law | Egress allowlist. Hull class still wins. |
| `modules/cards/*.json` | data | Deck. Closed kinds. Not the contract. |
| `modules/pipes/*.json` | law | Steps stay append-list, inc-stat, set-gauge, airlock. |
| `src/schema.rs` | law | Card ranges. Do not restate them in markdown. |
| `src/gate.rs` | law | Write gate. |
| `src/airlock.rs` | law | Dry dock. `sent` stays false. |
| `src/radio.rs` | red | Factory is real. Four functions are `NotImplemented`. |
| `tests/flow.rs` `tests/gate.rs` `tests/radio_factory.rs` `tests/serve.rs` | green | Must stay green. |
| `tests/radio_fr.rs` | red | Must keep failing on `NotImplemented` until the requirement is real. |
| `scripts/verify.sh` | reference | Green suite passes and the FR suite still fails. |

Not in the tree, on purpose: `program.json`, `integrations.json`, a SQLite file, a crate, an n8n node.

## Official references checked 2026-11-13

Re-read the latest Rust release at the start of a session. Append [docs/LOG.md](LOG.md) if the floor moves. The other links are the ones this thread actually used. They are not a suggestion to add those crates.

| Topic | Reference | Checked finding |
| --- | --- | --- |
| Rust stable | [1.100.0 announcement](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/) | 2026-11-12. Never type, allocator API, `DropGuard`. Unused here. |
| Rust release index | [releases/latest](https://blog.rust-lang.org/releases/latest) | Session ritual. Floor was 1.99.0 in the sandbox, then pinned to 1.100.0. |
| Previous point | [1.99.0](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/) | 2026-10-01. Not the floor. |
| rusqlite | [0.42.0](https://crates.io/crates/rusqlite/0.42.0) | 2026-11-11. MIT. MSRV 1.89. Investigated, not depended. |
| rusqlite release | [GitHub releases](https://github.com/rusqlite/rusqlite/releases) | 0.42.0 bundles SQLite 3.54.1. |
| rusqlite connection | [docs.rs Connection](https://docs.rs/rusqlite/0.42.0/rusqlite/struct.Connection.html) | `Send`, not `Sync`. `transaction` takes `&mut self`. `query_one` errors on extra rows. |
| SQLite 3.54.1 | [release log](https://www.sqlite.org/releaselog/3_54_1.html) | 2026-10-16. Heap-buffer overflow fix on 32-bit. |
| STRICT tables | [sqlite.org/stricttables](https://www.sqlite.org/stricttables.html) | Required if a table is added. |
| WAL | [sqlite.org/wal](https://www.sqlite.org/wal.html) | File databases only. Not the in-memory test connection. |
| Upstream hull | [k-dot-greyz/rust-app-core](https://github.com/k-dot-greyz/rust-app-core) | Fork point `6523b791`. Public template. |
| This repo | [k-dot-greyz/google-chat-but-in-rust](https://github.com/k-dot-greyz/google-chat-but-in-rust) | Public `main`. No pull request; there was no base branch. |

## Decisions held by this index

- Chat is the glass. The hull is the gate and the record. Phase 1 does not send and does not run n8n.
- Profile policy is compiled in. A watched directory cannot mint one.
- `x-profile` is a duty label, not a credential.
- Card ids and `blk_` ids stay. No UUIDv7 replacement.
- The JSON front door stays the hull parser. Not serde.
- The next cut is in-memory FR-1. rusqlite 0.42.0 waits until that is true.
- A later log is one insert-only `STRICT` transition table. Wall-clock time is not in the hash. The tip hash is the only future anchor, and it stays dry-docked.
- No blockchain, merkle tree, wallet, or empty config file ahead of a parser.

The reasoning for each line is in [docs/LOG.md](LOG.md), in order.
