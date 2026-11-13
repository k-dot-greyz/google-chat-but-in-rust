---
id: FR-1
status: red
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
tests: ["tests/radio_fr.rs"]
profiles: ["radio", "operator"]
verbs: ["ingest", "approve"]
toolchain_floor: "1.100.0"
depends_on: ["FR-GATE"]
---

# FR-1 — ingest, then a button

`radio::ingest` and `radio::approve` return `NotImplemented`. The asserts name the row they want.

Happy path:

1. The radio profile hands over a PNG plate. Magic bytes match, size stays under 512000, sha256 is `3f8c44dbd2b8da1c89ba2942b4ad10e2d535a69bb4ef579818d749604c7efb82`, block id is `blk_3f8c44dbd2b8da1c`.
2. The ledger row is `INGESTED`, `CanvasExport`, `fetched: false`, `rendered: false`. The chat card carries the id, the sha, and the verdict. It does not carry the bytes.
3. A link plate stores the URL string. Nothing is fetched.
4. Prose that says APPROVE is ingested and stays unapproved. Approving via prose is `ProseCannotApprove`.
5. The operator profile approves that block id via a button. The row becomes `APPROVED`. The radio profile pressing the same button is `Forbidden`.
