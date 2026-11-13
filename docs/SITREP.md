---
id: SITREP
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
as_of: "2026-11-13T18:24:00+02:00"
commit: 17bc1be
---

# Sitrep

As of `17bc1be`. One row per module. The judgment is in [docs/META.md](META.md).

| Module | Lines | State | Sitrep |
|---|---|---|---|
| `src/lib.rs` | 25 | green | Crate root. Claims no outbound socket. `server` is native-only. `wasm` is the other port of the same dispatch. |
| `src/main.rs` | 43 | watch | Binds `HOST` or `127.0.0.1`, port `4471`. The banner always prints `127.0.0.1` even when `HOST` is wider. Watcher reloads modules every 400ms. |
| `src/embed.rs` | 1 | green | Build script output. Not a policy surface. |
| `src/json.rs` | 520 | green | Closed parser. Duplicate keys, trailing commas, and leading zeros fail. This is the JSON front door. |
| `src/schema.rs` | 686 | green | Card kinds, field lengths, routes. `QUERY`, sessions, and providers are in the contract. Remote media `src` is refused. |
| `src/load.rs` | 639 | green | Loads the deck. A symlink that leaves `modules/` is refused. A bad reload keeps the last good bundle. |
| `src/hull.rs` | 1433 | watch | Dispatch, pipes, rollback, sessions, and `QUERY` live here. Writes call `admit` first. This file is doing too many jobs. |
| `src/render.rs` | 689 | green | Server-rendered deck and pretext. The island only applies a view the server already computed. |
| `src/http.rs` | 438 | green | One request per connection. No chunked bodies. `QUERY` requires `Content-Length`. `201`, `401`, and `501` have reason phrases. |
| `src/server.rs` | 124 | green | Inbound only. 64 connections, then `503`. Log line is method, path, status, code. No query, no body, no session id. |
| `src/wasm.rs` | 129 | green | Same `dispatch` behind the hosted port. No TCP server on wasm. Session mint has no entropy there, so it returns `503` and does not invent an id. |
| `src/gate.rs` | 309 | watch | `admit` guards the four write routes. `identify` only names a profile. `may_file` is one installation and one repo. `may_io` is the channel check. Query-string secrets are `400`. An `Authorization` or `X-N8N-API-KEY` header is still ignored, not refused. |
| `src/session.rs` | 154 | green | Memory only. 32 rows, 15 minutes, reuse of a live pair, no eviction of a live row. Not encrypted. Restart drops the table. Reload keeps it. |
| `src/airlock.rs` | 645 | green | Default deny. Hull class beats the allowlist. `dry_dock` sets `sent` to false. No caller-supplied URL. |
| `src/sync.rs` | 158 | green | In-memory tape. Handshake and delta are deck edits, not the ledger. A stale cursor resets. |
| `src/radio.rs` | 382 | red | Factory, sha256, and `blk_` are real. `ingest`, `approve`, `file_issue`, and `llm` return `NotImplemented`. `LlmRequest.authorization` is still a token-shaped field. |
| `modules/profiles.json` | law | green | Seven profiles, compiled in. `shell` alone has channel `deck` and verbs `read` and `query`. No token field. |
| `modules/policy.json` | law | green | Egress allowlist. It cannot open a socket. |
| `tests/flow.rs` | 355 | green | Shell, rollback, airlock, night shift. |
| `tests/gate.rs` | 183 | green | Missing profile, bearer-is-not-a-profile, query secret, duplicate header, cross-profile. |
| `tests/session.rs` | 164 | green | Local mint, reuse, revoke, `QUERY` without a query string, no provider fallback, no profile fallback. |
| `tests/radio_factory.rs` | 92 | green | Three and three. `n8n` is absent. |
| `tests/radio_fr.rs` | 191 | red | Nine tests. Must keep failing on `NotImplemented`. |
| `tests/serve.rs` | 30 | green | Loopback health answers. |
| `.github/workflows` | — | green | `verify` and `codeql` are required on `main`. CodeRabbit is not required. It failed on pull request 1 because the pull request was already merged. |

No crate. No SQLite. No n8n node. No outbound client. FR-1, FR-2, and FR-3 are red.
