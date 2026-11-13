---
id: PLUGINS
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
---

# Plugins

A plugin is a named sink with a profile and a requirement. Phase 1 loads none of them as running code. The deck lists the names so the boundary stays visible.

| Plugin | Phase | Profile | Runs? | Requirement |
| --- | --- | --- | --- | --- |
| ledger | 1 | airlock (in process) | the row shape is specified, the writer is red | FR-1 |
| chat-card | 1 | radio | pointer render only, via the hull deck | FR-1 |
| github-issue | 1 | git-zenos, git-hull, git-devmaster | grant check is green, filing is red | FR-2, FR-GATE |
| llm-route | 1 | llm | call shape is red, socket stays shut | FR-3 |
| rss-mirror | later | none | no | not opened |
| drive-inbox | later | radio, read of one folder | no | not opened |
| n8n | 2 | none | no | forbidden until FR-1, FR-2, and FR-3 are green |

Adding a plugin means a new requirement file with `status: red`, a profile verb, and a test that fails on `NotImplemented` or on a scope denial. It does not mean a new card kind or a new pipe step.

n8n is not imported, not configured, and not a sink in `radio::phase1`.
