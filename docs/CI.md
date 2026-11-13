---
id: CI
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
toolchain_floor: "1.100.0"
---

# Review checks

`scripts/verify.sh` is the methodology. The pull request workflows run that script and CodeQL. A review is not a second reading of the bible. It is these checks, plus the diff.

## What has to pass

| Check | Where | Pass means |
| --- | --- | --- |
| Toolchain | `scripts/verify-toolchain.sh` | `rustc` is the floor in `requirements/TOOLCHAIN.md`. |
| Reference index | `scripts/ci-checks.sh` | Every catalog file exists, has `status` and `index: docs/INDEX.md`, and every relative link in the index resolves. The log stays `append_only`. |
| Empty crate list | `scripts/ci-checks.sh` | `[dependencies]` has no crate. `serde`, `sqlx`, `rusqlite`, `nutype`, and `schemars` are refused by name. |
| Auth | `scripts/ci-checks.sh` and `tests/gate.rs` | Seven profile ids. The four write routes call `admit_request`. The bearer, query-secret, missing-profile, duplicate-header, and cross-profile tests are still in the file, and they pass. |
| Session | `tests/session.rs` | Local mint, reuse, revoke, QUERY without a query string, no provider fallback, no profile fallback. |
| Format | `cargo fmt --check` | The tree matches rustfmt for edition 2024. |
| Types | `cargo clippy --all-targets -- -D warnings` | No warning. This is the type-safety gate. It is not a generated JSON Schema. |
| Green suite | `cargo test`, FR names skipped | Hull, gate, factory, and serve stay green. |
| Red suite | `tests/radio_fr.rs` | The run fails, and the failure text contains `NotImplemented`. A green FR suite fails the check. |
| CodeQL | `.github/workflows/codeql.yml` | Rust analysis, `build-mode: none`, query suite `security-extended`. Official language support includes edition 2024. |

## Metrics the script prints

`deps=0`, `profiles=7`, `admit_sites` at least 4, `auth_tests=6`, `fmt=clean`, `clippy=denied-warnings`, `fr_failed`, `fr_reason=NotImplemented`.

## What this does not do

CodeQL does not understand the profile allowlist. Clippy does not know that prose cannot approve. Those stay in `tests/gate.rs` and `tests/radio_fr.rs`. Requiring a second human approval is not turned on: one code owner cannot approve their own pull request, and that would only block `main`. The required signal is the `verify` and `codeql` checks.

Official notes used for this setup: [CodeQL language support](https://codeql.github.com/docs/codeql-overview/supported-languages-and-frameworks/) (Rust, editions 2021 and 2024) and [compiled-language build modes](https://docs.github.com/en/code-security/code-scanning/creating-an-advanced-setup-for-code-scanning/codeql-code-scanning-for-compiled-languages) (`none` is supported for Rust).
