---
id: FR-2
status: red
owners: ["@k-dot-greyz"]
phase: 1
tests: ["tests/radio_fr.rs"]
profiles: ["git-zenos", "git-hull"]
verbs: ["file-issue"]
toolchain_floor: "1.100.0"
depends_on: ["FR-GATE", "FR-1"]
---

# FR-2 — one installation, one repo

`radio::file_issue` returns `NotImplemented`. The gate already knows the grants.

Happy path:

1. An approved slice (`blk_3f8c44dbd2b8da1c`) is filed by `git-zenos` as installation `inst-zenos` on `k-dot-greyz/zenOS`.
2. The issue title is `airlock blk_3f8c44dbd2b8da1c`. The body is a pointer. It does not contain the raw bytes. The only scope used is `issues:write`.
3. `git-hull` filing the same repo is `Isolation`. The operator profile cannot file. No profile has `workflow` or `admin`.
