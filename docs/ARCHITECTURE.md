---
id: ARCHITECTURE
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
---

# Architecture

The binary is still `hull`. The radio is a module, not a second product.

```text
google-chat-but-in-rust/
  AGENTS.md                 session ritual for agents
  CODEOWNERS
  CONTRIBUTING.md
  SPEC.md                   hull contract (cards, pipes, airlock, routes)
  Cargo.toml                empty [dependencies], rust-version floor
  rust-toolchain.toml       exact stable
  modules/
    cards/                  deck JSON, closed kinds
    pipes/                  closed steps
    policy.json             egress allowlist
    profiles.json           compiled-in write allowlist
  src/
    airlock.rs              egress. hull class beats the allowlist. dry dock.
    gate.rs                 profile scope for writes. identify, may_io.
    session.rs              memory sessions. one profile, one channel.
    radio.rs                phase-1 contract. factory is real. FR behavior is not.
    hull.rs                 dispatch
    http.rs server.rs       inbound only. default bind is loopback.
  requirements/             the reqs. status green or red matches the tests.
  docs/INDEX.md              enforced catalog. session entry
  docs/CI.md                 review checks and the metrics they print
  .github/workflows/        verify and CodeQL
  docs/LOG.md                append-only intent and reasoning
  docs/BIBLE.md              map. tests win if it drifts
  docs/                     whitepaper, this file, flows, plugins, integrations
  tests/
    flow.rs                 shell still works, as the shell profile
    gate.rs                 scope
    radio_factory.rs        the mock world and the deck
    radio_fr.rs             red on purpose
  scripts/
    verify-toolchain.sh
    verify.sh
```

## What is law

| Law | Where |
| --- | --- |
| Card kinds and pipe steps are closed | `SPEC.md`, `src/schema.rs`, `src/load.rs` |
| Egress is default-deny and does not send | `src/airlock.rs` |
| A write names one profile | `src/gate.rs`, `modules/profiles.json` |
| Profile policy is not hot-reloaded | `include_str!` of `modules/profiles.json` |
| Ingest does not fetch, prose does not approve | `requirements/FR-1.md` |
| Git grants do not cross | `requirements/FR-2.md`, `gate::may_file` |
| The model sees a sha, not a plate | `requirements/FR-3.md` |
| Dependencies have a floor and a reason | `requirements/TOOLCHAIN.md` |

## What is data

Cards, pipes, and the egress policy reload from `modules/` when the process is watching. A bad file keeps the last good bundle. Profiles do not join that watch. Widening a profile is a rebuild, reviewed like code.

## Phase 2

n8n, live Google SSO, and a real egress send are out of this tree. Each one needs its own requirement, its own tests, and a crate floor if a crate shows up. None of them widen phase-1 profiles in place.
