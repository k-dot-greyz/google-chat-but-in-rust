---
id: FR-3
status: red
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
tests: ["tests/radio_fr.rs"]
profiles: ["llm"]
verbs: ["complete"]
toolchain_floor: "1.100.0"
depends_on: ["FR-GATE", "FR-1"]
---

# FR-3 — the model sees an approved slice

`radio::llm` returns `NotImplemented`. There is no outbound socket in this crate. A future call still has to pass the hull egress airlock, and the dry dock keeps `sent: false` until a later requirement says otherwise.

Happy path, host `gsuite.test`:

| Session | Block | Status | Model called | Outbound |
| --- | --- | --- | --- | --- |
| none | approved | 401 | no | nothing |
| radio principal, or host `other.test` | approved | 403 | no | nothing |
| llm principal | ingested | 409 | no | nothing |
| llm principal | approved | 200 | yes | the sha only |

The 200 body carries `3f8c44dbd2b8da1c89ba2942b4ad10e2d535a69bb4ef579818d749604c7efb82` and `outbound_has_raw_bytes: false`.
