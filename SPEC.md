---
id: SPEC
status: reference
index: docs/INDEX.md
owners: ["@k-dot-greyz"]
toolchain_floor: "1.100.0"
---

# Hull — pre-release reference

Rust floor **1.100.0** (stable, 2026-11-12). The workspace compiler must match `rust-toolchain.toml`. `Cargo.toml` has an empty `[dependencies]`. A crate is not allowed until a requirement names it, and the adopted version is the latest stable on that day, recorded as a new floor. This file is the contract the tree has to match. Cards, pipes, and policy are JSON under `modules/`. Profile allowlists are `modules/profiles.json`, compiled in, not watched. The page is server-rendered. `assets/island.js` only applies a view the server already computed.

## Contract

Closed kinds, matched in `src/schema.rs`: `hero`, `stat`, `list`, `form`, `note`, `gauge`, `media`.

Closed steps, matched in `src/load.rs`: `append-list`, `inc-stat`, `set-gauge`, `airlock`. A new pipe composes those. It does not bring its own code.

Probes are an enum in `src/airlock.rs`: `unlisted`, `metadata`, `plain-http`, `allowlisted`. Destinations are the `Probe::destination` table. Callers do not name a URL.

| probe | destination | expected |
| --- | --- | --- |
| unlisted | `https://example.com:443/hook` | not allowlisted |
| metadata | `http://169.254.169.254/latest/meta-data` | hull class |
| plain-http | `http://hooks.hull.invalid:80/plate` | scheme |
| allowlisted | `https://hooks.hull.invalid:443/plate` | permit, `sent: false` |

Airlock order for an egress push: parse the static destination, hull class of the literal host (no DNS), `https` when policy says https only, body size and content type, exact origin match, method, pipe id, then mint a `Permit`. Nothing above `airlock` can construct one. `dry_dock` consumes it and sets `sent: false`. The only TCP in the process is the inbound server.

Hull class beats the allowlist. Loopback, private, link-local, CGNAT, multicast, metadata names, and IPv4-mapped copies of those are denied even when policy lists them.

`build.rs` embeds `modules/` and `assets/` so the wasm build and the tests do not need a filesystem. The native server also reads `modules/` at boot and polls it (`src/main.rs`, 400 ms, `HULL_WATCH=0` stops it). `Hull::reload_dir` keeps the previous bundle on a bad file and sets `fault`. The same epoch clears the fault and leaves memory alone. A changed epoch swaps in the new bundle.

Symlinks are resolved and must stay under `modules/`. Asset paths are exact (`shell.css`, `island.js`, `mark.svg`). `..` is rejected in `src/http.rs`.

Bodies: headers 8 KiB, HTTP body 64 KiB, egress body whatever policy says (4096 in the sample). Lists hold 24. The delta tape holds 200. Card count is capped at 32. Push fields derived from `append-list` are checked at title 1..=48 and detail 0..=80. `append_item` still clips a list detail to 180 for lines that do not pass through that push check.

JSON writes need `X-Airlock-Intent: 1`. A form post with `Sec-Fetch-Site: cross-site` is refused. Form posts answer `303` to `/`. Redirects do not take a visitor URL.

Sync is `sysex-delta/1`. `GET /api/sync/handshake`, then `GET /api/sync/delta?cursor=`. A cursor ahead of the server, or a cursor the ring has already dropped, returns `reset: true` plus a snapshot view. Patches are leaf paths.

Inbound HTTP/1.1 on the standard library, one request per connection, `Content-Length` only, 5 second read and write timeouts, 503 when 64 connections are already taken.

| Method | Path |
| --- | --- |
| GET | `/`, `/assets/shell.css`, `/assets/island.js`, `/assets/mark.svg` |
| GET | `/api/health`, `/api/schema`, `/api/view`, `/api/cards`, `/api/cards/{id}` |
| POST | `/api/cards` |
| PATCH, DELETE | `/api/cards/{id}` |
| GET | `/api/pipes` |
| POST | `/api/pipes/{id}/push` |
| GET | `/api/airlock`, `/api/pipeline`, `/api/sync/handshake`, `/api/sync/delta` |

`POST /api/airlock/probe` is absent. Probes go through `egress.probe`.

Two ports, one dispatch:

- Local: `cargo run` binds `TcpListener` in `src/server.rs`. Default port **4471**.
- Hosted: `app/[[...path]]/route.js` loads `wasm/hull.wasm` through `lib/hull.mjs` and calls `hull_handle`. Next is the port. The crate is the product. One wasm instance per isolate; a cold start plates a fresh service.

## Sample pass

| id | kind | role |
| --- | --- | --- |
| card.hero | hero | the line the page is about |
| card.covers | stat | plates that landed |
| card.verdicts | stat | times the airlock was asked |
| card.pressure | gauge | integrity, ten cells, value stays 100 across probes |
| card.pass | list | the plates |
| card.log | list | verdicts |
| card.trace | list | lines filed on purpose |
| card.plate | form | ingress form, dish 48, note 80 |
| card.doctrine | note | the rules |
| card.bay | media | empty `src`, designed dark plate |

| id | direction | what it does |
| --- | --- | --- |
| ingress.pass | ingress | append a plate, then `internal.tally` |
| ingress.trace | ingress | append one short line onto `card.trace` |
| internal.tally | internal | increment covers. Not pushable from HTTP |
| egress.probe | egress | airlock, log the verdict, increment verdicts |

`then` points at internal pipes and must be a DAG.

## Surfaces

Night is the only canvas. `assets/shell.css` sets `color-scheme: dark`. The void `#0c0b0a` is written as a literal and again as `var(--void)` on the root, the document, media elements, and controls. The skip link is copper `#e0a07a` with ink `#1a120e` and a 44px target. There is no light theme.

Deck is the panel (`body[data-surface="deck"]`, the default). Pretext is the native text surface: `?surface=pretext`, the header toggle, and `body[data-surface="pretext"]`. The pretext block is `display: none` until that attribute is set. `fillPretext` rewrites `#pretext pre` with `textContent` on every `apply(view)`, from the same view as the deck.

`card.bay` with an empty `src` renders `.bay-fallback` and no player (`src/render.rs`). A source, when present, must be one file under `/assets/` (`schema::safe_media_src`). On a media `error`, the island shows that plate, retries a same-origin `/assets/` src once (`HTMLMediaElement.load`, or the same attribute for a frame), and then stays on the plate. It never writes a remote URL into `src`. A frame that has already failed stays on the plate, because a loaded error document can still paint white.

Client state stays on the machine. Copy and download build `hull.snapshot/1` (specs, epoch, cursor, seal, deck, tape, trace). Nothing is fetched. **File a line on the pass** is the only share, and it is one push through `ingress.trace`.

If `paintDeck` throws, `apply` still rewrites pretext, sets `data-surface="pretext"`, shows `#client-fault` with the night-shift line, and records one ablation trace per distinct message until a later paint succeeds. A failed `GET /api/view` does not call `apply`. The last good view stays. One poll trace is recorded for the streak. The next try waits 1s, then 2s, 4s, and 8s. A good view returns the cadence to 3s and clears the backoff.

## Contrast

### Territory actual

Verified against the tree:

- `src/schema.rs` closes the seven kinds. `safe_media_src` allows only an empty string or a single `/assets/` file. `modules/cards/card.bay.json` ships `src: ""` and the fallback "No reel loaded. The hatch does not turn white."
- `src/render.rs` paints deck and pretext from one hull, emits `#client-fault` hidden inside the trace panel (the aside, not the deck), and emits a bay plate with no media element when the src is empty. `GET /?surface=pretext` sets `data-surface="pretext"`.
- `assets/shell.css` is night only: literal `#0c0b0a`, then `var(--void)`, `color-scheme: dark` on the root, the document, and controls. The skip link is `#1a120e` on `#e0a07a` with `min-height: 44px`. No `#fff`, `#ffffff`, or `color-scheme: light`.
- `assets/island.js` `apply` updates the deck inside `paintDeck` and then `fillPretext` assigns `textContent`. A thrown deck paint switches the body to pretext, shows the existing client-fault sentence, and notes `ablation` once per distinct message. `runPoll` keeps the last view on a bad `/api/view`, notes `Holding the last good view. The pass did not answer.` once per streak, and backs off from 1s to a cap of 8s. `armBay` retries only a same-origin asset path and does not contain `http://` or `https://`.
- `src/hull.rs` `reload_dir` leaves cards, pipes, epoch, and in-memory plates in place when `from_dir` fails, and only clears `fault` when the reloaded epoch matches. `validate_body` rejects an append title over 48 and a detail over 80.
- `src/airlock.rs` `authorize` returns hull class before it consults the allow list. `dry_dock` sets `sent: false`.
- `src/server.rs` is `TcpListener` plus accept loop. `src/lib.rs` compiles it only off wasm. `src/wasm.rs` exports `hull_alloc`, `hull_handle`, `hull_free` and keeps one `Hull` in a thread local. `app/[[...path]]/route.js` forwards the request and does not implement cards or pipes.

### Goal reference model

The shell we are willing to ship:

- One dark canvas. Deck and pretext are two views of one state. Pretext is what remains when the deck is stripped off, including when the deck paint throws.
- A bay cannot turn white. Empty and failed sources show the designed plate. A retry may only repeat a same-origin `/assets/` file, once.
- Snapshots stay local. The only egress of a client note is the capped trace pipe, and the operator chooses it.
- The airlock is unchanged: enum probes, server destinations, hull class over the allowlist, dry dock, `sent: false`.

### Lessons learned

- A light paper theme looked finished and still failed the moment a control, skip link, or media element used a browser default. The night canvas writes the void twice and sets `color-scheme: dark` on the elements that otherwise paint their own background. The skip link sets both copper and `#1a120e`; it does not inherit a default link color.
- Pretext is `display: none` until `data-surface="pretext"`. A copy that is not rewritten on `apply` still shows the first server paint after the first mutation. `fillPretext` writes the live view with `textContent` on every apply, including the apply that just failed to paint the deck.
- cursor.app cannot run the `TcpListener`. `server` is not in the wasm build. `app/[[...path]]/route.js` loading `wasm/hull.wasm` is the hosted port, not a second product and not a second set of business rules.
- Filing a trace through the pipe is capped (title 48, detail 80) by `validate_body` before `append_item`. The full `hull.snapshot/1` bundle is a clipboard copy or a `Blob` download, not an egress.
- An allowlist cannot save a metadata or private address. Hull class is decided from the literal host before the allow entry is considered. Policy that names `169.254.169.254`, `127.0.0.1`, `[::1]`, `10.0.0.8`, `localhost`, or `metadata.google.internal` still loses.

### Done, this change, later

Done in the tree before this revision: night canvas and skip link, deck and pretext as two server renders, pretext text kept in step with `apply`, empty bay plate, local snapshot, `ingress.trace`, airlock and dry dock, wasm dispatch as the hosted port, last-good module reload.

This change: a deck paint that throws switches to pretext, shows `#client-fault`, and records an ablation trace; a failed view poll keeps the last good view, records one trace line, and retries on a capped backoff; a media error shows the plate, retries one same-origin asset, then stays there. The reload path is not loosened. `night_shift_never_falls_back_to_a_white_canvas` still rejects a white background, `#fff`, `#ffffff`, and `color-scheme: light`.

Later, and not in this tree: no light theme, no outbound socket, no visitor-supplied URL, no second business logic in JavaScript.
