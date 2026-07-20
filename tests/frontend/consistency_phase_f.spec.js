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

const consistency = bodyOf("renderWorkInfoConsistency(env)", "async function");
const gateHelpers = [
  "phaseFStdFsIssueRows(result)",
  "isPhaseFIssue(issue)",
  "phaseFIssues(result)",
  "phaseFErrorCount(result)",
  "renderPhaseFGateResult(result, locale)",
  "runPhaseFValidationGate(env, root, { navigateOnPass = false } = {})",
  "ensurePhaseFAdjustmentGate(env, moduleCode)",
  "renderPhaseFAdjustmentGate(env, moduleCode, result)",
  "bindPhaseFAdjustmentGateActions(env, moduleCode)",
].map((name) => bodyOf(name, name.startsWith("run") || name.startsWith("ensure") ? "async function" : "function")).join("\n");
const adjustmentLeaf = bodyOf("renderAdjustmentModuleLeaf(env, moduleCode)", "async function");
const validationCounts = bodyOf("validationCounts(issues)");
const combined = `${consistency}\n${gateHelpers}\n${adjustmentLeaf}\n${validationCounts}`;

for (const snippet of [
  'request(`${root}/validation/run`',
  'data-phase-f-gate',
  'data-phase-f-enter-adjustment',
  'data-adjustment-gated="phase-f"',
  'ensurePhaseFAdjustmentGate(env, moduleCode)',
  'renderPhaseFAdjustmentGate(env, moduleCode, result)',
  'env.navigate("ws/adj:B1")',
  'env.navigate("ws/info:consistency")',
  'issue.status || "OPEN"',
  'issue.severity === "ERROR"',
  'area === "tax-data" || area === "std-fs"',
  "phaseFErrorCount(result) === 0",
]) {
  assert(combined.includes(snippet), `Phase F UI gate must include ${snippet}`);
}

for (const ruleCode of [
  "CHK_STDBS_BALANCE",
  "CHK_STDBS_VS_FS",
  "CHK_STDIS_VS_FS",
  "CHK_STDFS_UNMAPPED",
  "CHK_STDFS_CONFIRMED",
]) {
  assert(gateHelpers.includes(ruleCode), `Phase F gate must surface ${ruleCode}`);
}

console.log("frontend consistency_phase_f.spec.js passed");
