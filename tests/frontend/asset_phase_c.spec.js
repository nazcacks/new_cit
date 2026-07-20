const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

function bodyOf(name, prefix = "function") {
  const start = screens.indexOf(`${prefix} ${name}`);
  assert(start >= 0, `${name} must exist`);
  const nextAsync = screens.indexOf("\nasync function ", start + 1);
  const nextPlain = screens.indexOf("\nfunction ", start + 1);
  const candidates = [nextAsync, nextPlain].filter((index) => index > start);
  const next = candidates.length ? Math.min(...candidates) : undefined;
  return screens.slice(start, next);
}

const loader = bodyOf("loadAssetPhaseCData(root)", "async function");
const assetScreen = bodyOf("renderWorkInfoAssets(env)", "async function");
const helpers = [
  "renderAssetBsReconcile(result, locale)",
  "bindAssetPhaseCActions(env, root)",
].map((name) => bodyOf(name)).join("\n");
const combined = `${loader}\n${assetScreen}\n${helpers}`;

for (const endpoint of [
  'request(`${root}/tax-data/assets/depr-preview`)',
  'request(`${root}/tax-data/assets/bs-reconcile`)',
  'request(`${root}/tax-data/assets/carry-forward`',
]) {
  assert(combined.includes(endpoint), `asset Phase C UI must call ${endpoint}`);
}

for (const snippet of [
  'data-asset-action="carry-forward"',
  'data-asset-action="b4"',
  'data-asset-depr-preview',
  'data-asset-bs-reconcile',
  'data-asset-phasec-error',
  'data-asset-depr-empty',
  'data-asset-reconcile-empty',
  'renderAssetBsReconcile(assetPhaseC.bsReconcile, locale)',
  'bindAssetPhaseCActions(env, data.root)',
  'env.navigate("ws/adj:B4")',
]) {
  assert(combined.includes(snippet), `asset Phase C UI must include ${snippet}`);
}

for (const field of [
  "tax_depr_limit",
  "depr_excess",
  "depr_shortfall",
  "acct_depr_current",
  "tax_depr_rate_bps",
]) {
  assert(combined.includes(field), `asset Phase C UI must render ${field}`);
}

console.log("frontend asset_phase_c.spec.js passed");
