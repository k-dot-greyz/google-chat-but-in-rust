import { readFileSync } from "node:fs";
import { join } from "node:path";

const wasmFile = join(process.cwd(), "wasm", "hull.wasm");

let instancePromise;

function load() {
  instancePromise ??= (async () => {
    const bytes = readFileSync(wasmFile);
    const { instance } = await WebAssembly.instantiate(bytes, {});
    return instance.exports;
  })();
  return instancePromise;
}

function reset() {
  instancePromise = null;
}

export async function dispatch(request) {
  const url = new URL(request.url);
  const body = await request.text();
  if (body.length > 65536) {
    return json(413, "too_large", "That body does not fit through the hatch.");
  }
  const headers = {};
  request.headers.forEach((value, key) => {
    headers[key] = value;
  });
  const payload = JSON.stringify({
    method: request.method,
    path: url.pathname,
    query: url.search.startsWith("?") ? url.search.slice(1) : "",
    headers,
    body,
  });
  if (payload.length > 196608) {
    return json(413, "too_large", "That request does not fit through the hatch.");
  }
  try {
    const exports = await load();
    const encoded = new TextEncoder().encode(payload);
    const ptr = exports.hull_alloc(encoded.length);
    if (!ptr) return json(500, "alloc", "The hull could not take the request.");
    new Uint8Array(exports.memory.buffer, ptr, encoded.length).set(encoded);
    const out = exports.hull_handle(ptr, encoded.length);
    if (!out) return json(500, "alloc", "The hull could not answer.");
    const memory = new Uint8Array(exports.memory.buffer);
    const view = new DataView(exports.memory.buffer);
    const len = view.getUint32(out, true);
    const bytes = memory.slice(out + 4, out + 4 + len);
    exports.hull_free(out, len + 4);
    const decoded = JSON.parse(new TextDecoder().decode(bytes));
    const responseHeaders = new Headers();
    for (const pair of decoded.headers ?? []) {
      if (Array.isArray(pair) && pair.length === 2) responseHeaders.append(pair[0], pair[1]);
    }
    return new Response(decoded.body ?? "", { status: decoded.status ?? 500, headers: responseHeaders });
  } catch {
    reset();
    return json(500, "trap", "The hull trapdoor opened. The next request starts clean.");
  }
}

function json(status, code, detail) {
  return new Response(JSON.stringify({ ok: false, error: { code, detail } }), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}
