---
id: AGENTS
status: accepted
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
codeowners: "CODEOWNERS"
---

# Agent session

This file is the frontmatter for work in this repo. It is not a prompt to widen the product.

## Before editing

1. Run `scripts/verify-toolchain.sh`. It must print `gucci` on rustc 1.100.0 or newer if the floor has moved.
2. Read the latest stable announcement at <https://blog.rust-lang.org/releases/latest> and the detailed notes. If stable is above `requirements/TOOLCHAIN.md`, bump the floor in that same session before any other change. Do not stay on the old compiler.
3. Read the requirement you are touching. Red requirements stay red until their asserts pass for the reason the assert names. Do not stub `Ok(...)` to turn `tests/radio_fr.rs` green.
4. Stay inside the closed card kinds and pipe steps. Profile changes are code changes: `modules/profiles.json` is compiled in.

## While editing

- No new crate without a requirement id, a crates.io stable floor, and the release note that justified it.
- No secret in a query string, a card, an error body, or a log line.
- No fetch of a URL that arrived inside a plate.
- No n8n.
- Writes go through `gate::admit`. A new route that mutates state and skips the gate is a defect.

## Owners

`CODEOWNERS` names `@k-dot-greyz` for the gate, the airlock, the requirements, and the docs. A change to those paths needs that owner even when the diff looks small.
