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

const transactionCombined = [
  bodyOf("loadTransactionPhaseDData(root)", "async function"),
  bodyOf("renderWorkInfoTransactions(env)", "async function"),
  bodyOf("renderTransactionPhaseDTabs(transactions, locale)"),
  bodyOf("renderTransactionIsReconcile(result, locale)"),
  bodyOf("bindTransactionPhaseDActions(env)"),
].join("\n");

for (const snippet of [
  'request(`${root}/tax-data/transactions/is-reconcile`)',
  "TRANSACTION_PHASE_D_TABS",
  'data-transaction-four-tabs',
  'data-transaction-tab-button',
  'data-transaction-tab',
  'data-transaction-action',
  'data-transaction-is-reconcile',
  'data-transaction-phase-error',
  'env.navigate(`ws/adj:${button.dataset.transactionAction}`)',
  "issue.rule_code",
]) {
  assert(transactionCombined.includes(snippet), `Phase D transaction UI must include ${snippet}`);
}
assert(screens.includes('key: "RECEIVABLE"'), "Phase D transaction tabs must include receivable/B6 tab");

const vehicleCombined = [
  bodyOf("loadVehiclePhaseEData(root)", "async function"),
  bodyOf("renderWorkInfoVehicleUsage(env)", "async function"),
  bodyOf("renderVehicleB10Reconcile(result, locale)"),
  bodyOf("bindVehiclePhaseEActions(env)"),
].join("\n");

for (const snippet of [
  'request(`${root}/vehicle-usage-logs/b10-reconcile`)',
  'data-vehicle-b10-reconcile',
  'data-vehicle-usage-ratio',
  'data-vehicle-action="b10"',
  'data-vehicle-phase-error',
  'env.navigate("ws/adj:B10")',
  "business_use_bps",
  "business_use_source",
  "expected_addback",
  "b10_item_amount",
]) {
  assert(vehicleCombined.includes(snippet), `Phase E vehicle UI must include ${snippet}`);
}

console.log("frontend transaction_vehicle_phase_d_e.spec.js passed");
