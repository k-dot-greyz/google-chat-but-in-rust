---
id: FR-SESSION
status: green
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
---

# FR-SESSION — local session and QUERY

A session is an in-memory row: one profile, one channel, fifteen minutes. It is not a token, it is not written to disk, and it is not a fallback for a missing `x-profile` on the old write routes.

`POST /api/sessions` names `channel` and may name `provider`. The only provider is `local`. Any other provider is `501` and does not mint a local session instead. The profile must already hold that channel and the `query` verb. A second mint for the same profile and channel returns the live row.

`QUERY /api/channels/{channel}` carries the filter in the JSON body (`select` of `id`, `title`, `kind`). A query string is `400`. The session header is `x-session`. A disagreeing `x-profile` is `401` and the cards are not returned. An unknown, expired, or revoked session is the same `401`. `GET` on that path is `405` with `Allow: QUERY`.

`may_io` is the channel check for read, edit, write, and query. Edit and write still mean `mutate-cards`. The existing deck reads stay open. Restart drops the table. A module reload keeps it. The log line is method, path, status, and code. It does not include the session id, the query string, or the body. `retry` is false on these responses.
