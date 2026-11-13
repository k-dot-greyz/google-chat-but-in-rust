import { dispatch } from "../lib/hull.mjs";

const health = await dispatch(new Request("http://hull.local/api/health"));
const healthText = await health.text();
if (health.status !== 200 || !healthText.includes("\"service\":\"hull\"")) {
  console.error("health failed", health.status, healthText);
  process.exit(1);
}

const page = await dispatch(new Request("http://hull.local/"));
const html = await page.text();
if (page.status !== 200 || !html.includes("The pass is open")) {
  console.error("page failed", page.status, html.slice(0, 200));
  process.exit(1);
}

const island = await dispatch(new Request("http://hull.local/assets/island.js"));
const source = await island.text();
if (
  island.status !== 200 ||
  !source.includes("Holding the last good view. The pass did not answer.") ||
  !source.includes('dataset.surface = "pretext"') ||
  !source.includes("sameOriginAsset") ||
  source.includes("https://") ||
  source.includes("http://")
) {
  console.error("island fallback missing", island.status, source.slice(0, 160));
  process.exit(1);
}

const plated = await dispatch(new Request("http://hull.local/api/pipes/ingress.pass/push", {
  method: "POST",
  headers: { "content-type": "application/json", "x-airlock-intent": "1" },
  body: JSON.stringify({ dish: "omelette", note: "baveuse" }),
}));
const platedText = await plated.text();
if (plated.status !== 200 || !platedText.includes("omelette")) {
  console.error("plate failed", plated.status, platedText);
  process.exit(1);
}

const probed = await dispatch(new Request("http://hull.local/api/pipes/egress.probe/push", {
  method: "POST",
  headers: { "content-type": "application/json", "x-airlock-intent": "1" },
  body: JSON.stringify({ probe: "metadata" }),
}));
const probedText = await probed.text();
if (!probedText.includes("\"sent\":false") || probedText.includes("\"sent\":true")) {
  console.error("probe failed", probed.status, probedText);
  process.exit(1);
}

console.log("wasm smoke ok", healthText);
