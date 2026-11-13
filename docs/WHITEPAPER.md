---
id: WHITEPAPER
status: accepted
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
source: "fork of k-dot-greyz/rust-app-core @ 6523b791"
---

# Whitepaper — a hull for a chat radio

## Problem

An operator wants to drop a link, a screenshot, or a drive pointer from a phone and have an agent do something small with it. The something is the dangerous part. Untrusted bytes will ask to be approved, merged, and mailed. A chat product is the wrong place to keep that state, and a workflow engine is the wrong place to start.

## Decision

Google Chat is the glass. The hull is the record and the gate. Three Workspace principals and three git installations do not share a capability. Phase 1 has no n8n, no outbound socket, and no crate.

What moved over from the slack-shaped work is the part that was never about Matrix: an offline contract, a store that does not echo secrets, and a smoke path that fails closed. The bus here is not a homeserver. It is a profile allowlist plus an airlock.

## Trust boundaries

```text
phone / chat glass
    |  pointer only (id, sha, verdict)
    v
profile gate  ---- compiled allowlist, not a watched file
    |
    +-- radio    ingest
    +-- operator approve by button
    +-- git-*    one installation, one repo
    +-- llm      approved slice only
    v
ledger  (hash, status, no render of the plate)
    |
    v
egress airlock  (https, hull class beats the allowlist, dry dock, sent: false)
```

The glass is allowed to forget the bytes. The ledger is not allowed to fetch a URL because a message contained one. The model is not allowed to see a row that is not `APPROVED`. A git installation is not allowed to notice the other two.

## What phase 1 is not

- Not a Chat bot, not a Gmail client, not an SSO product. Those are named seams. The verifier is not in the tree.
- Not n8n. n8n is phase 2, and only after the three red requirements are green on more than one git installation and more than one Workspace principal.
- Not a second card language. Kinds and pipe steps stay the closed hull set.
- Not a place to put a token in a query string, a deck, or an error body.

## Floor

Rust 1.100.0, edition 2024, zero crates. The ritual for moving that floor is `requirements/TOOLCHAIN.md`.
