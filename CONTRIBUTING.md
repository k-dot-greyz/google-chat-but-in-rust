---
id: CONTRIBUTING
status: accepted
owners: ["@k-dot-greyz"]
toolchain_floor: "1.100.0"
---

# Contributing

1. `scripts/verify.sh` before a change is offered. Green tests stay green. `tests/radio_fr.rs` stays red until the requirement it names is actually implemented.
2. Requirements live in `requirements/`. A behavior change updates the requirement in the same change as the test.
3. Dependencies: latest stable, never below the floor in `requirements/TOOLCHAIN.md`. Empty `[dependencies]` is the current state and the preferred state.
4. Profiles and egress policy are security reviews. See `CODEOWNERS`.
5. Do not add a card kind, a pipe step, an outbound socket, or an n8n node.

The deck is the hull. New operator-facing copy is a JSON card, not a second frontend.
