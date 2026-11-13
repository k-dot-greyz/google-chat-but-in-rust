## Requirement

- [ ] Requirement id:
- [ ] Status still matches [docs/INDEX.md](../docs/INDEX.md) and the tests (red stays red)
- [ ] If this records a decision, [docs/LOG.md](../docs/LOG.md) gained one append-only entry and no past entry was rewritten

## Floor

- [ ] `scripts/verify.sh` is the check this pull request must pass
- [ ] Auth changes still go through `gate::admit` and `tests/gate.rs`
- [ ] Latest Rust release notes were read. Floor bumped in this PR if stable moved.
- [ ] No new crate, or the crate's floor and the release note are named here

## Scope

- [ ] No new card kind or pipe step
- [ ] Writes still pass `gate::admit`
- [ ] No secret in a query, a card, or an error
- [ ] n8n was not added
