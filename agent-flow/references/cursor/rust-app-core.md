# rust-app-core

Reference toolkit for the Hull shell. Read this when the task is a Rust web app core, a hosted shell, pipes, cards, or an egress airlock. The binary is `hull`. The repository name is `rust-app-core`.

Source of truth until a public GitHub remote exists: Origin repo `greyzxcursor/rust-web-scaffold`. Blueprint: `SPEC.md`. Human quickstart: `README.md`.

## Toolchain

Pinned in `rust-toolchain.toml`.

- Rust **1.100.0** floor, edition 2024. Read `requirements/TOOLCHAIN.md` before bumping.
- Components: `rustfmt`, `clippy`
- Target, only for the hosted port: `wasm32-unknown-unknown`
- `[dependencies]` in `Cargo.toml` stays empty

```bash
rustup toolchain install 1.100.0 --profile minimal --component rustfmt,clippy
rustup target add wasm32-unknown-unknown
cargo test
cargo clippy --all-targets -- -D warnings
cargo run
```

The pass listens on `http://127.0.0.1:4471` unless `PORT` or `HOST` is set. `HULL_MODULES` overrides `modules/`. `HULL_WATCH=0` stops the file poll.

## What you are allowed to extend

New behavior is a new file in the closed vocabulary, not a new crate and not an eval loop.

- Card kinds: `hero`, `stat`, `list`, `form`, `note`, `gauge`, `media`
- Pipe steps: `append-list`, `inc-stat`, `set-gauge`, `airlock`
- Cards: `modules/cards/*.json`. Pipes: `modules/pipes/*.json`. Policy: `modules/policy.json`
- A bad module file must not replace the last good bundle. The page shows the fault.

`media` sources are empty or a single file under `/assets/`. No scheme, no `..`, no remote URL.

## What you do not do

- Do not add a crate to get JSON, HTTP, or TLS.
- Do not accept a visitor-supplied egress URL. Probes are `unlisted`, `metadata`, `plain-http`, `allowlisted`. Destinations live in `src/airlock.rs`.
- Do not let an allowlist entry override hull class. Loopback, private, link-local, and metadata addresses stay denied.
- Do not open an outbound socket. `dry_dock` records the permit and sets `sent` to false.
- Do not ship a light theme or a white canvas. The color is `#0c0b0a`, written as a literal. `color-scheme` is `dark`. If the deck paint throws, the page drops to the pretext surface and stays dark.
- Do not send the debug bundle anywhere. Copy and download stay on the machine. Filing a line goes through `ingress.trace` only when the user asks, and that note is at most 80 characters.

## Two surfaces, one state

Deck is the panel. Pretext is the native text module (`?surface=pretext` or the header toggle). After a mutation, both have to show the same seal, cursor, pass, pipes, and tape. A full reload is not the fix if the in-page toggle is stale.

A failed `GET /api/view` keeps the last good view and retries at 1s, 2s, 4s, then 8s. A media error retries one local `/assets/` file, then stays on the designed plate.

## Hosted port

cursor.app runs web `Request` handlers, not the TCP server. `app/[[...path]]/route.js` loads `wasm/hull.wasm` and calls the same `dispatch`. Rebuild the wasm when Rust changes:

```bash
cargo build --release --target wasm32-unknown-unknown --lib
cp target/wasm32-unknown-unknown/release/hull.wasm wasm/hull.wasm
node scripts/wasm-smoke.mjs
```

## Checks before you call it done

- `cargo test` includes the night-canvas test: no `background: white`, no `#fff`, no `#ffffff`, no `color-scheme: light`.
- `cargo clippy --all-targets -- -D warnings` is clean.
- A plate through `ingress.pass` updates the list, the cover count, and the delta tape, including on the pretext surface without a reload.
- The metadata probe is denied. The allowlisted probe permits and does not send.
