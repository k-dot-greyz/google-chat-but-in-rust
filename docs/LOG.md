---
id: LOG
status: reference
append_only: true
owners: ["@k-dot-greyz"]
index: docs/INDEX.md
clock: "Europe/Riga, +02:00"
---

# Intent and reasoning log

Append only. A new decision goes at the bottom with a timestamp. Do not edit a past entry except to fix a wrong URL or a typo, and say so in the new entry. Times before 17:05 were not stamped per turn. Their order is the order they happened.

## 2026-11-13, before 17:05+02

1. **Intent.** Use Google Chat instead of Slack as the dispatch glass. A dedicated agent mailbox on a phone would scope what an automation may do. The operator sends a link, a screenshot, or a Drive link. Ingest goes through the airlock. Grok bots should be pingable from that glass. Question left open: which jobs are reasonable, and whether to roll our own.
2. **Decision.** Roll a new tree, `google-chat-but-in-rust`, rather than extend a Slack-shaped bus. Keep what was never about the chat vendor: an offline contract, no secret echoed back, fail closed. Native edges are Workspace, git installations, and airlock ingest for later OpenAPI or model routes that require Google SSO. n8n is phase 2, and only after tests hold for more than one git installation and more than one Workspace principal. Start with three failing functional tests, happy paths in the asserts, and a factory. Three git installations, three Workspace principals.
3. **Decision.** The frontend is a fork of `k-dot-greyz/rust-app-core` (`6523b791`), not a second UI. Closed card kinds and pipe steps stay closed.
4. **Built.** Factory green. Gate was not in yet. FR-1, FR-2, and FR-3 red on `NotImplemented`. Deck rendered the contract. Preview smoked clean. Nine red tests, green hull and factory.
5. **Held.** A structured recap of that work was written into the session, not into the repo.

## 2026-11-13T17:05:00+02:00

**Intent.** Revise the hull for multi-profile scope. Lay the repo so requirements, flows, code owners, and agent instructions live in the tree. Use latest stable as the floor. Check official release notes after the toolchain is real.

**Reasoning.** Rust 1.100.0 had been announced the day before (12 Nov 2026). The sandbox was on 1.99.0. The floor moved to 1.100.0. The new language pieces (never type, allocator API, `DropGuard`) were read and not used. Profile policy was compiled in with `include_str!` because a watched `modules/` directory must not be able to mint a profile. Writes require exactly one `x-profile`. Secrets in the query string are refused. `Authorization` is not a profile. The listener default became loopback. The deck island sends `shell` because a form cannot set a header and the glass is the shell role. That name is not a secret.

**Built.** `src/gate.rs`, `modules/profiles.json`, `tests/gate.rs`, requirements, whitepaper, flows, plugin and integration lists, `scripts/verify.sh`. Commit `0634d25`. FR tests left red.

## 2026-11-13T17:19:00+02:00

**Intent.** Document a JSON card schema and a SQLite persistence layer: constructors, ranges, relational tables, many UUIDs, then SOW stubs. Then hold for a follow-up before any of that was written.

**Reasoning.** Writing that spec first would fork the schema that `schema.rs` and `tests/radio_fr.rs` already are. Held. No files.

## 2026-11-13T17:21:00+02:00

**Intent.** A review argued that the held plan was a spec trap: markdown ranges rot, UUIDv4 text keys hurt SQLite, stubs lock the wrong shape, and types should come first (`serde`, `schemars`, `sqlx`, snapshot tests).

**Reasoning.** The process criticism was accepted. The sample was not. `serde_json` accepts duplicate keys; the hull parser rejects them and escapes HTML. Card ids are stable slugs pipes join on. Plate ids are `blk_` plus the sha256 prefix. Replacing either with UUIDv7 breaks the deck and the airlock. `sqlx` is an async driver with a compile-time database, a poor fit for this synchronous binary. The one slice worth cutting later is the FR-1 ledger row, not a fictional task card.

**Built.** Nothing.

## 2026-11-13T17:22:00+02:00

**Intent.** Investigate rusqlite as the synchronous driver, still without adding it.

**Reasoning.** 0.42.0 (11 Nov 2026) bundles SQLite 3.54.1, which fixes a 32-bit heap-buffer overflow from 3.54.0 (16 Oct 2026). `Connection` is `Send` and not `Sync`, which matches one hull behind the server mutex. `query_row` hides extra rows; `query_one` does not. Default features pull `hashlink`. The useful set is `bundled`, `limits`, `extra_check`, native target only, because the bundled C sources do not build for wasm. SQLCipher and the `codec` feature stay off.

**Built.** Nothing. Floor if adopted: 0.42.0.

## 2026-11-13T17:28:00+02:00

**Intent.** Make JSON config for program, auth, and integrations, and make the design ledger-capable, with no blockchain in the early iterations.

**Reasoning.** A hash-chained insert-only transition log is the ledger. A chain, a wallet, and a merkle tree are a second product. Three config classes must not be one file: profiles stay compiled law, program settings may later move and must not grant a verb, integration names stay compiled and cannot set `send: true` or carry a secret. Domain events (ingest, approve, file, complete) are the log. Deck edits are not. Wall-clock time is stored and is not hashed, so a replay has a stable tip. Genesis is 32 zero bytes. The only future egress is the tip hash, still dry-docked.

**Built.** Nothing. Empty `program.json` and `integrations.json` were deliberately not created.

## 2026-11-13T17:30:00+02:00

**Intent.** Compile the task flow, the test prose, and the architecture into one bible, then say what should change.

**Reasoning.** The bible is a map. It points at tests instead of copying card ranges. Open changes, in order: stop carrying `tok-*` on the FR-3 request while it is still red; derive the factory from the compiled profile table; do not add rusqlite in the same change as in-memory FR-1; do not add config files before a parser.

**Built.** `docs/BIBLE.md`. README and `AGENTS.md` point at it.

## 2026-11-13T17:33:00+02:00

**Intent.** Integrate the worktree and ship it upstream as a new repo, as a pull-request payload a GitHub bot could apply.

**Reasoning.** The bible commit had to land first or the shipped tree would be missing its map. A new repository has no base branch, so a pull request could not be the ship. Both commits were pushed to `main`.

**Built.** Commit `13619ce`. Public repo [k-dot-greyz/google-chat-but-in-rust](https://github.com/k-dot-greyz/google-chat-but-in-rust). Fixture `tok-*` strings are in the public tests and are not credentials.

## 2026-11-13T17:36:00+02:00

**Intent.** Give the docs and artifacts permanent reference status. Enforce that through an index and the agent instructions. Keep this thread, the official docs, and the reasoning, with timestamps, as a backup of intent.

**Reasoning.** A bible nobody is required to open does not enforce anything. The index is the catalog and the session entry. The log is append-only so a later session can see why serde, UUIDv7 cards, n8n, and a blockchain were refused. Red requirements stay red. Reference status freezes settled decisions, not unfinished tests.

**Built.** This file and `docs/INDEX.md`. Agent instructions, code owners, and the catalog point here.

## 2026-11-13T17:40:00+02:00

**Intent.** Put CodeQL or an equivalent on pull requests, with a testing methodology and metrics for the bible workflow, type safety, and auth.

**Reasoning.** CodeQL's current language list includes Rust, editions 2021 and 2024, and the `none` build mode is supported for Rust, so the scan does not need a second cargo build. That covers generic Rust defects. It does not know the profile allowlist, so it is not the auth check. Auth stays the gate tests plus a static count: seven profiles, and `admit_request` on create, patch, delete, and push. Type safety for this tree is `cargo clippy -- -D warnings` on edition 2024, not a generated JSON Schema. The red FR suite is a metric of its own: the check fails if those tests pass or if they fail for any reason other than `NotImplemented`. A required second reviewer was not turned on, because the only code owner cannot approve their own pull request.

**Built.** `scripts/ci-checks.sh`, `scripts/verify.sh` now runs those checks plus fmt and clippy, `docs/CI.md`, `.github/workflows/verify.yml`, `.github/workflows/codeql.yml`. Clippy's needless-borrow hits in `src/gate.rs` were fixed so the deny-warnings gate is honest.

