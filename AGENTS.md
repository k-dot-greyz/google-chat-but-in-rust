---
id: AGENTS
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
codeowners: "CODEOWNERS"
---

# Agent session

Open [docs/INDEX.md](docs/INDEX.md) before editing. It is the catalog. [docs/LOG.md](docs/LOG.md) is append-only. This file is not a prompt to widen the product.

If the index, the bible, and a test disagree, the test wins. Then fix the index and append a log line. Do not delete or rewrite a log entry.

## Before editing

1. Read [docs/INDEX.md](docs/INDEX.md). Obey the status column. A `reference` row is not a scratchpad. A `red` row stays red until its assert passes for the reason it names.
2. Run `scripts/verify-toolchain.sh`. It must print `gucci` on rustc 1.100.0 or newer if the floor has moved.
3. Read [the latest Rust release](https://blog.rust-lang.org/releases/latest) and the detailed notes. If stable is above `requirements/TOOLCHAIN.md`, bump the floor in that same change and append [docs/LOG.md](docs/LOG.md). Do not stay on the old compiler.
4. Read the requirement you are touching. Do not stub `Ok(...)` to turn `tests/radio_fr.rs` green.
5. Stay inside the closed card kinds and pipe steps. Profile changes are code changes: `modules/profiles.json` is compiled in.

## While editing

- A decision that changes the index gets a timestamped entry at the bottom of [docs/LOG.md](docs/LOG.md) in the same change.
- No new crate without a requirement id, a crates.io stable floor, and the release note that justified it.
- No secret in a query string, a card, an error body, or a log line.
- No fetch of a URL that arrived inside a plate.
- No n8n. No blockchain. No empty config file ahead of a parser.
- Writes go through `gate::admit`. A new route that mutates state and skips the gate is a defect.

## Owners

`CODEOWNERS` names `@k-dot-greyz` for the index, the log, the gate, the airlock, the requirements, and the docs. A change to those paths needs that owner even when the diff looks small.
