---
id: META
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
as_of: "2026-11-13T18:24:00+02:00"
commit: 17bc1be
---

# Meta review

The hull is a loopback record with a dry dock, not a chat client and not an automation runner. That shape is intact at `17bc1be`. The green surface is the deck, the write gate, the airlock, and a memory session bound to one channel. The red surface is still the radio: ingest, approve, file, and the model route do not run. Nothing in the tree opens an outbound socket.

## What is holding

The JSON parser, the card schema, and the profile table are the law, and they agree with the tests. Writes go through `gate::admit` before the body is trusted. `QUERY` keeps the filter in the body. A remote provider does not fall back to `local`. A session whose `x-profile` disagrees returns `401` and no cards. The factory test fails if `n8n` appears as a sink. `scripts/verify.sh` fails the run when `tests/radio_fr.rs` exits 0, and it fails the run when the output does not contain `NotImplemented`. It does not count the nine failures. Eight passes plus one `NotImplemented` still satisfy it, and so does an unrelated failure beside that string. CodeQL on Rust with `build-mode: none` is required and has passed.

Secrets that the code knows how to refuse stay out of the query string, the card, the error body, and the log line. The session id is random, short-lived, and memory-only. It is still a capability on the wire. That is acceptable on loopback. It is not acceptable on a public bind.

## What is not holding

Four findings, in the order they should be closed. None of them are fixed in this review.

1. **The banner lies.** `main.rs` binds whatever `HOST` says, then always prints `http://127.0.0.1`. A non-loopback bind looks local. Default `HOST` is loopback, so the default is safe and the report is not.
2. **Credential headers are ignored, not refused.** `authorization`, `token`, and `session` in the query string are `400`. `Authorization` and `X-N8N-API-KEY` as headers are skipped. A valid `x-profile` is still admitted, so `POST /api/sessions` for `shell` succeeds with that header present. `401` happens when the profile is missing or invalid, not because the header was seen. The value is still read into the request. That is the plaintext-token hole, and it is wider than a failed login.
3. **There is no per-caller rate limit.** The server stops at 64 connections and the session table stops at 32. Neither answers `429` or `Retry-After`. A live session can query as fast as the connection cap allows.
4. **`src/hull.rs` is 1433 lines.** Dispatch, pipes, rollback, sessions, and channel query are one type. The behavior is tested. The next change to it will be hard to review. Split only when a requirement forces a seam, not before.

`radio::LlmRequest.authorization` is still a token-shaped field. FR-3 must change that shape before anyone implements the function. Implementing it as written would put a token back into the contract.

## What stays out

n8n's public API, webhook secret, and `N8N_ENCRYPTION_KEY` are not this process's problem yet. Encrypting a pointer the receiver must read is not confidentiality. Encrypting a key and sending the key beside it is the stupid case. The hull should keep refusing to carry either. TLS in front of the listener is the transport. A homemade cipher is not on the list.

Phase 2 does not start by adding a node. It starts when FR-1, FR-2, and FR-3 are green for the reason their tests name.

## Verdict

Ship-ready as a closed phase-1 hull. Not ready as a dispatcher, a status-report source, or an n8n caller. The next code change that is worth more than the others is finding 2: refuse credential headers with `400` and do not log the value. Finding 1 is a one-line honesty fix. Finding 3 waits for a real caller. Finding 4 waits for a seam that a requirement needs.
