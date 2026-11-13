---
id: FR-GATE
status: green
owners: ["@k-dot-greyz"]
phase: 1
tests: ["tests/gate.rs", "tests/flow.rs"]
profiles: ["shell", "operator", "radio", "llm", "git-zenos", "git-hull", "git-devmaster"]
toolchain_floor: "1.100.0"
---

# FR-GATE — profile scope on every write

A write names exactly one profile. The profile is an allowlist compiled into the binary from `modules/profiles.json`. The watched module directory cannot mint a profile.

| Profile | May | May not |
| --- | --- | --- |
| `shell` | Mutate cards. Push `ingress.pass`, `ingress.trace`, `egress.probe`. | Approve, ingest, call the model, file an issue. |
| `operator` | `approve` | Everything else, including git. |
| `radio` | `ingest` | Approve, mutate cards, push street pipes, call the model. |
| `llm` | `complete` | Ingest, approve, file. |
| `git-zenos` | `file-issue` on `inst-zenos` / `k-dot-greyz/zenOS` | Any other repo or installation. |
| `git-hull` | `file-issue` on `inst-hull` / `k-dot-greyz/rust-app-core` | zenOS, dev-master. |
| `git-devmaster` | `file-issue` on `inst-devmaster` / `k-dot-greyz/dev-master` | The other two repos. |

Rules:

- Missing, duplicate, empty, or unknown `x-profile` is 401. The body does not say which.
- `Authorization` is ignored until an SSO verifier exists. A bearer token is not a profile name.
- `profile`, `token`, `authorization`, `access_token`, `id_token`, `code`, or `secret` in the query string is 400, even if the header is valid.
- A profile outside its allowlist is 403 `scope`. An `internal.*` pipe is 403 even if a future edit lists it.
- The listener defaults to loopback. Binding a wider address before the SSO verifier replaces `x-profile` is a misconfiguration, not a deployment.

`x-profile` separates duties inside one process. It is not authentication of a Google user. The deck island sends `x-profile: shell` on its own writes. That name marks the glass as the shell role. It is not a secret, and it does not grant approve, ingest, or file.
