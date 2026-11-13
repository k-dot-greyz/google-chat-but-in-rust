const seen = {
  epoch: document.body.dataset.epoch || "",
  cursor: Number(document.body.dataset.cursor || "0"),
};
let inflight = 0;
const CLIENT_FAULT = "Client fault held on the night shift. Copy a snapshot if you want it read.";
let deckNote = "";

function text(id, value) {
  const node = document.getElementById(id);
  if (node) node.textContent = value == null ? "" : String(value);
}

function apply(view) {
  if (!view || typeof view !== "object") return;
  if (view.epoch && view.epoch !== seen.epoch) {
    seen.epoch = view.epoch;
    seen.cursor = -1;
  }
  if (typeof view.cursor === "number" && view.cursor < seen.cursor) return;
  if (typeof view.cursor === "number") seen.cursor = view.cursor;

  latestView = view;
  let deckError = null;
  try {
    paintDeck(view);
    deckNote = "";
  } catch (error) {
    deckError = error;
  }
  try {
    fillPretext(view);
  } catch (error) {
    if (!deckError) {
      showClientFault();
      note("pretext", error && error.message ? error.message : "pretext apply failed");
    }
  }
  if (deckError) holdOnPretext(deckError);
}

function paintDeck(view) {
  const word = view.seal && view.seal.word ? view.seal.word : "SEALED";
  text("seal-word", word);
  text("seal-detail", view.seal && view.seal.detail);
  const stamp = document.getElementById("seal");
  if (stamp) {
    stamp.classList.remove("sealed", "denied", "permit");
    const tone = word === "DENIED" ? "denied" : word === "PERMIT" ? "permit" : "sealed";
    stamp.classList.add(tone);
  }
  text("covers-value", view.covers);
  text("verdicts-value", view.verdicts);
  text("gauge-caption", view.gauge && view.gauge.caption);
  text("cursor-readout", `cursor ${view.cursor} · epoch ${view.epoch}`);
  const cells = document.querySelectorAll("#gauge-cells span");
  const amount = view.gauge ? Number(view.gauge.value) : 0;
  const lit = Math.max(0, Math.min(10, Math.round((Number.isFinite(amount) ? amount : 0) / 10)));
  cells.forEach((cell, index) => {
    cell.classList.toggle("on", index < lit);
  });
  const gauge = document.getElementById("gauge-cells");
  if (gauge) {
    gauge.setAttribute("aria-label", `Hull integrity ${amount} out of 100`);
  }

  fillTickets("pass-list", view.pass, "The pass is clear. Send something worth plating.");
  fillTickets("verdict-list", view.log, "No verdicts yet. The dock is quiet.");
  fillTickets("trace-filed", view.trace, "Nothing filed. The bundle stays on this machine.");
  fillTape(view.tape);
  fillPipes(view.pipes);

  const fault = document.getElementById("fault");
  if (fault) {
    fault.hidden = !view.fault;
    fault.textContent = view.fault || "";
  }
  const notice = view.notice || "";
  const detail = view.seal && view.seal.detail ? view.seal.detail : "";
  if (inflight === 0) text("status", notice && notice !== detail ? notice : "");
}

function showClientFault() {
  const bar = document.getElementById("client-fault");
  if (!bar) return;
  bar.hidden = false;
  bar.textContent = CLIENT_FAULT;
}

function holdOnPretext(error) {
  document.body.dataset.surface = "pretext";
  try { sessionStorage.setItem("hull-surface", "pretext"); } catch { /* private mode */ }
  document.querySelectorAll("[data-surface-link]").forEach((item) => {
    if (item.dataset.surfaceLink === "pretext") item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  });
  showClientFault();
  const message = String(error && error.message ? error.message : "deck apply failed").slice(0, 240);
  if (deckNote === message) return;
  deckNote = message;
  try { note("ablation", message); } catch { /* the fault line is already up */ }
}

function fillTickets(id, items, empty) {
  const node = document.getElementById(id);
  if (!node) return;
  node.replaceChildren();
  if (!items || items.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = empty;
    node.append(li);
    return;
  }
  for (const item of items) {
    const li = document.createElement("li");
    const title = document.createElement("span");
    title.className = "dish";
    title.textContent = item && item.title ? item.title : "";
    const detail = document.createElement("span");
    detail.className = "detail";
    detail.textContent = item && item.detail ? item.detail : "";
    li.append(title, detail);
    node.append(li);
  }
}

function fillTape(items) {
  const node = document.getElementById("tape-rows");
  if (!node) return;
  node.replaceChildren();
  if (!items || items.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "No patches yet. The handshake is the whole bird.";
    node.append(li);
    return;
  }
  for (const item of items) {
    const li = document.createElement("li");
    const path = document.createElement("span");
    path.className = "path";
    path.textContent = item && item.path ? item.path : "";
    const preview = document.createElement("span");
    preview.textContent = item && item.preview ? item.preview : "";
    li.append(path, preview);
    node.append(li);
  }
}

function fillPretext(view) {
  const node = document.querySelector("#pretext pre");
  if (!node || !view) return;
  const seal = view.seal || {};
  const word = seal.word || "SEALED";
  const detail = seal.detail || "";
  const cursor = typeof view.cursor === "number" ? view.cursor : 0;
  const lines = [
    "HULL / NIGHT / PRETEXT",
    "native text surface. the deck can leave. this stays.",
    "",
    `seal ${word}  ${detail}`,
    `epoch ${view.epoch || ""}`,
    `cursor ${cursor}`,
    "",
    "CARDS",
  ];
  const deck = Array.isArray(view.deck) ? view.deck : [];
  for (const card of deck) {
    const id = card && card.id ? card.id : "";
    const kind = card && card.kind ? card.kind : "";
    const title = card && card.title ? card.title : "";
    lines.push(`${id} ${kind}  ${title}`);
  }
  lines.push("", "PIPES");
  const pipes = Array.isArray(view.pipes) ? view.pipes : [];
  for (const pipe of pipes) {
    const direction = pipe && pipe.direction ? pipe.direction : "";
    const id = pipe && pipe.id ? pipe.id : "";
    const summary = pipe && pipe.summary ? pipe.summary : "";
    lines.push(`${direction} ${id}  ${summary}`);
  }
  lines.push("", "PASS");
  const pass = Array.isArray(view.pass) ? view.pass : [];
  if (pass.length === 0) {
    lines.push("(clear)");
  } else {
    for (const item of pass) {
      const title = item && item.title ? item.title : "";
      const note = item && item.detail ? item.detail : "";
      lines.push(note ? `- ${title} / ${note}` : `- ${title}`);
    }
  }
  lines.push("", "TAPE");
  const tape = Array.isArray(view.tape) ? view.tape : [];
  if (tape.length === 0) {
    lines.push("(handshake only)");
  } else {
    for (const patch of tape) {
      const at = patch && typeof patch.cursor === "number" ? patch.cursor : 0;
      const path = patch && patch.path ? patch.path : "";
      const preview = patch && patch.preview ? patch.preview : "";
      lines.push(`${at} ${path}  ${preview}`);
    }
  }
  node.textContent = `${lines.join("\n")}\n`;
}

function fillPipes(pipes) {
  const node = document.getElementById("pipe-list");
  if (!node || !pipes) return;
  node.replaceChildren();
  for (const pipe of pipes) {
    const row = document.createElement("div");
    row.className = "pipe";
    const dir = document.createElement("span");
    dir.className = `dir ${pipe.direction || ""}`;
    dir.textContent = pipe.direction || "";
    const body = document.createElement("span");
    const id = document.createElement("span");
    id.className = "pipe-id";
    id.textContent = pipe.id || "";
    const steps = document.createElement("span");
    steps.className = "pipe-steps";
    steps.textContent = pipe.summary || "";
    body.append(id, steps);
    row.append(dir, body);
    node.append(row);
  }
}

async function send(url, payload, button) {
  const status = document.getElementById("status");
  inflight += 1;
  if (button) button.disabled = true;
  if (status) status.textContent = "Sending…";
  let next = null;
  let failed = "";
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json",
        "x-airlock-intent": "1",
        "x-profile": "shell",
      },
      body: JSON.stringify(payload),
    });
    const body = await response.json().catch(() => null);
    if (!response.ok) {
      failed = body && body.error && body.error.detail ? body.error.detail : "The hull refused that.";
    } else if (body && body.view) {
      next = body.view;
    }
  } catch {
    failed = "The hull missed a beat. The plate is still on the counter.";
  } finally {
    inflight = Math.max(0, inflight - 1);
    if (button) button.disabled = false;
  }
  if (next) apply(next);
  if (failed && status) status.textContent = failed;
  return Boolean(next);
}

const plate = document.getElementById("plate-form");
if (plate) {
  plate.addEventListener("submit", async (event) => {
    event.preventDefault();
    const data = new FormData(plate);
    const ok = await send(plate.action, {
      dish: String(data.get("dish") || ""),
      note: String(data.get("note") || ""),
    }, event.submitter);
    if (ok) plate.reset();
  });
}

const probes = document.getElementById("probe-form");
if (probes) {
  probes.addEventListener("submit", (event) => {
    event.preventDefault();
    const button = event.submitter;
    if (!button) return;
    send(probes.action, { probe: button.value }, button);
  });
}

const POLL_OK_MS = 3000;
const POLL_BACKOFF_MS = 1000;
const POLL_CAP_MS = 8000;
let backoff = POLL_BACKOFF_MS;
let pollNoted = false;
let pollTimer = 0;
let pollBusy = false;

function schedulePoll(wait) {
  window.clearTimeout(pollTimer);
  pollTimer = window.setTimeout(runPoll, wait);
}

async function runPoll() {
  if (pollBusy) return;
  if (document.hidden || inflight > 0) {
    schedulePoll(pollNoted ? backoff : POLL_OK_MS);
    return;
  }
  pollBusy = true;
  let wait = POLL_OK_MS;
  try {
    const response = await fetch("/api/view", { headers: { accept: "application/json" } });
    if (!response.ok) throw new Error("view status");
    const body = await response.json();
    if (!body || typeof body !== "object" || Array.isArray(body)) throw new Error("view shape");
    apply(body);
    backoff = POLL_BACKOFF_MS;
    pollNoted = false;
  } catch {
    if (!pollNoted) {
      note("poll", "Holding the last good view. The pass did not answer.");
      pollNoted = true;
    }
    wait = backoff;
    backoff = Math.min(POLL_CAP_MS, backoff * 2);
  } finally {
    pollBusy = false;
    schedulePoll(wait);
  }
}

const trace = [];
const errors = [];
let latestView = readEmbedded();

function readEmbedded() {
  const node = document.getElementById("hull-view");
  if (!node) return null;
  try {
    return JSON.parse(node.textContent || "");
  } catch {
    return null;
  }
}

function note(level, message) {
  trace.push({
    t: new Date().toISOString(),
    level,
    message: String(message).slice(0, 240),
  });
  while (trace.length > 40) trace.shift();
  renderLocalTrace();
}

function renderLocalTrace() {
  const node = document.getElementById("trace-list");
  if (!node) return;
  node.replaceChildren();
  const rows = errors.concat(trace).slice(-8);
  if (rows.length === 0) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "No client faults.";
    node.append(li);
    return;
  }
  for (const row of rows) {
    const li = document.createElement("li");
    const title = document.createElement("span");
    title.className = "dish";
    title.textContent = row.level || row.message || "trace";
    const detail = document.createElement("span");
    detail.className = "detail";
    detail.textContent = row.message || "";
    li.append(title, detail);
    node.append(li);
  }
}

function clientSpecs() {
  const motion = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  return {
    user_agent: navigator.userAgent,
    language: navigator.language || "",
    languages: navigator.languages ? Array.from(navigator.languages) : [],
    viewport: { w: window.innerWidth, h: window.innerHeight, dpr: window.devicePixelRatio || 1 },
    color_scheme: "dark",
    reduced_motion: Boolean(motion),
    hardware_concurrency: navigator.hardwareConcurrency || 0,
    device_memory: navigator.deviceMemory || null,
    platform: navigator.platform || "",
  };
}

function snapshotBundle() {
  const view = latestView || {};
  return {
    bundle: "hull.snapshot/1",
    purpose: "debug",
    captured_at: new Date().toISOString(),
    surface: document.body.dataset.surface || "deck",
    client: clientSpecs(),
    cornerstone: {
      epoch: view.epoch || document.body.dataset.epoch || "",
      cursor: typeof view.cursor === "number" ? view.cursor : Number(document.body.dataset.cursor || 0),
      seal: view.seal || { word: "SEALED", detail: "" },
      notice: view.notice || "",
      fault: view.fault || null,
      surface: document.body.dataset.surface || "deck",
    },
    manifest: {
      protocol: "sysex-delta/1",
      kinds: ["hero", "stat", "list", "form", "note", "gauge", "media"],
    },
    deck: view.deck || [],
    pass: view.pass || [],
    log: view.log || [],
    trace_filed: view.trace || [],
    tape: view.tape || [],
    pipes: view.pipes || [],
    trace,
    errors,
  };
}

function traceStatus(message) {
  const node = document.getElementById("trace-status");
  if (node) node.textContent = message;
}

async function copySnapshot() {
  const text = JSON.stringify(snapshotBundle(), null, 2);
  try {
    await navigator.clipboard.writeText(text);
    traceStatus("Snapshot copied. It leaves only if you paste it.");
  } catch {
    const dump = document.getElementById("snapshot-dump");
    if (dump) {
      dump.hidden = false;
      dump.textContent = text;
    }
    traceStatus("Clipboard refused. The bundle is on the page, still local.");
  }
}

function downloadSnapshot() {
  const blob = new Blob([JSON.stringify(snapshotBundle(), null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "hull-snapshot.json";
  link.click();
  URL.revokeObjectURL(url);
  traceStatus("Bundle downloaded. Nothing was sent.");
}

document.getElementById("copy-snapshot")?.addEventListener("click", () => {
  copySnapshot();
});
document.getElementById("download-snapshot")?.addEventListener("click", downloadSnapshot);
document.getElementById("file-trace")?.addEventListener("click", async (event) => {
  const view = latestView || {};
  const seal = view.seal && view.seal.word ? view.seal.word : "SEALED";
  const detail = `cursor ${view.cursor ?? 0} ${seal}`.slice(0, 80);
  const ok = await send("/api/pipes/ingress.trace/push", { title: "client", detail }, event.currentTarget);
  traceStatus(ok ? "Filed on the pass." : "The pass refused the line.");
});

document.querySelectorAll("[data-surface-link]").forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    const surface = link.dataset.surfaceLink === "pretext" ? "pretext" : "deck";
    document.body.dataset.surface = surface;
    try { sessionStorage.setItem("hull-surface", surface); } catch { /* private mode */ }
    document.querySelectorAll("[data-surface-link]").forEach((item) => {
      if (item.dataset.surfaceLink === surface) item.setAttribute("aria-current", "page");
      else item.removeAttribute("aria-current");
    });
    note("surface", surface);
  });
});

try {
  const params = new URLSearchParams(location.search);
  if (!params.has("surface")) {
    const stored = sessionStorage.getItem("hull-surface");
    if (stored === "pretext" || stored === "deck") {
      document.body.dataset.surface = stored;
    }
  }
} catch { /* private mode */ }

window.addEventListener("error", (event) => {
  const message = String(event.message || "client error").slice(0, 240);
  errors.push({ t: new Date().toISOString(), level: "error", message });
  while (errors.length > 20) errors.shift();
  showClientFault();
  renderLocalTrace();
});

window.addEventListener("unhandledrejection", (event) => {
  const message = String(event.reason && event.reason.message ? event.reason.message : event.reason || "rejection").slice(0, 240);
  errors.push({ t: new Date().toISOString(), level: "rejection", message });
  renderLocalTrace();
});

function sameOriginAsset(src) {
  if (typeof src !== "string" || src.length < 9 || src.length > 88) return false;
  if (!src.startsWith("/assets/")) return false;
  const name = src.slice("/assets/".length);
  if (!name || name.includes("/") || name.includes("\\") || name.includes(":") || name.includes("?") || name.includes("#") || name.includes("..")) {
    return false;
  }
  for (const ch of name) {
    const code = ch.charCodeAt(0);
    const digit = code >= 48 && code <= 57;
    const upper = code >= 65 && code <= 90;
    const lower = code >= 97 && code <= 122;
    const mark = ch === "." || ch === "_" || ch === "-";
    if (!digit && !upper && !lower && !mark) return false;
  }
  return true;
}

function armBay(bay) {
  const media = bay.querySelector("video, audio, iframe");
  const fallback = bay.querySelector(".bay-fallback");
  if (!media || !fallback) return;
  const original = media.getAttribute("src") || "";
  let phase = "live";
  const plate = () => {
    media.hidden = true;
    fallback.hidden = false;
  };
  media.addEventListener("error", () => {
    plate();
    if (phase === "down") return;
    if (phase === "live" && sameOriginAsset(original)) {
      phase = "retry";
      note("media", "bay retry");
      if (typeof media.load === "function") {
        try { media.load(); } catch { /* iframe */ }
      } else {
        media.setAttribute("src", original);
      }
      return;
    }
    phase = "down";
    media.removeAttribute("src");
    note("media", "bay fallback");
  });
  const reveal = () => {
    if (phase !== "retry" || media.tagName === "IFRAME") return;
    const current = media.getAttribute("src") || "";
    if (!sameOriginAsset(current)) return;
    media.hidden = false;
    fallback.hidden = true;
    phase = "live";
  };
  media.addEventListener("loadeddata", reveal);
  if (media.error) media.dispatchEvent(new Event("error"));
}

document.querySelectorAll("[data-bay]").forEach(armBay);

note("boot", document.body.dataset.surface || "deck");
schedulePoll(POLL_OK_MS);
