# Hull

## Story

As a developer bootstrapping a hosted app, I want a shell that already has cards, pipes, a schema, and a shut airlock, so I can see the shape of the system before I add a single feature of my own.

## MVP flow

1. Open the shell and read the pass.
2. Send a plate through `ingress.pass`.
3. Watch the list, the cover count, and the delta tape change.
4. Run the four airlock probes and read four different verdicts.
5. Fetch `/api/schema` and `/api/sync/handshake`.

## Done when

- `cargo test` for the hull, the factory, and the gate is green on Rust 1.100.0 with zero crates. `tests/radio_fr.rs` stays red until FR-1, FR-2, and FR-3 are implemented.
- The page works with and without the island script.
- A metadata destination is denied even if policy allows it.
- The dry dock never sets `sent` to true.
- The same wasm module answers the published host.
