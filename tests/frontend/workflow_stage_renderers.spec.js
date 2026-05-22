const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

function objectBlock(name) {
  const match = screens.match(new RegExp(`export const ${name} = Object\\.freeze\\(\\{([\\s\\S]*?)\\n\\}\\);`));
  assert(match, `${name} block is missing`);
  return match[1];
}

function screenRegistrations() {
  return Object.fromEntries(
    [...objectBlock("screenByLeaf").matchAll(/^\s*"([^"]+)": \(env\) => (.+)$/gm)]
      .map((match) => [match[1], match[2]])
  );
}

function contractRegistrations() {
  return Object.fromEntries(
    [...objectBlock("workflowLeafRendererContract").matchAll(/^\s*"([^"]+)": "([^"]+)"/gm)]
      .map((match) => [match[1], match[2]])
  );
}

const expectedGroups = {
  renderWorkStartLeaf: ["ws/start:customer-pick", "ws/start:by-pick", "ws/start:snapshot"],
  renderWorkInfoLeaf: ["ws/info:fs", "ws/info:mapping", "ws/info:assets", "ws/info:transactions", "ws/info:vehicle", "ws/info:consistency"],
  renderAdjustmentLeaf: Array.from({ length: 17 }, (_, index) => `ws/adj:B${index + 1}`),
  renderFormsLeaf: ["ws/form:form3", "ws/form:attachments", "ws/form:preview", "ws/form:linkage"],
  renderValidationLeaf: ["ws/val:run", "ws/val:issues", "ws/val:rules"],
  renderApprovalLeaf: ["ws/appr:request", "ws/appr:inbox", "ws/appr:rejected"],
  renderPrintLeaf: ["ws/print:preview", "ws/print:bulk", "ws/print:history"],
  renderEfilingLeaf: ["ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done"],
  renderPostHistoryLeaf: ["post/hist:list"],
  renderPostAmendLeaf: ["post/amend:unlock", "post/amend:version", "post/amend:diff", "post/amend:resubmit", "post/correction"],
};

const delegateByRenderer = {
  renderWorkStartLeaf: "renderWorkStart",
  renderWorkInfoLeaf: "renderWorkInfo",
  renderAdjustmentLeaf: "renderAdjustments",
  renderFormsLeaf: "renderForms",
  renderValidationLeaf: "renderValidation",
  renderApprovalLeaf: "renderApproval",
  renderPrintLeaf: "renderPrint",
  renderEfilingLeaf: "renderEfiling",
  renderPostHistoryLeaf: "renderPostHistory",
  renderPostAmendLeaf: "renderPostAmend",
};

const screensByLeaf = screenRegistrations();
const contract = contractRegistrations();
const stageContract = objectBlock("workflowStageContract");

for (const [renderer, keys] of Object.entries(expectedGroups)) {
  assert(screens.includes(`async function ${renderer}(env)`), `${renderer} wrapper must exist`);
  assert(screens.includes(`await ${delegateByRenderer[renderer]}(env);`), `${renderer} must call ${delegateByRenderer[renderer]}`);
  assert(stageContract.includes(`renderer: "${renderer}"`), `workflowStageContract must reference ${renderer}`);
  for (const key of keys) {
    assert.strictEqual(contract[key], renderer, `${key} contract must use ${renderer}`);
    assert(stageContract.includes(`"${key}"`), `workflowStageContract must include ${key}`);
    assert(screensByLeaf[key], `${key} must be registered in screenByLeaf`);
    assert(screensByLeaf[key].includes(`${renderer}(env)`), `${key} must dispatch to ${renderer}`);
    assert(!screensByLeaf[key].includes("renderLeafScreen"), `${key} must not dispatch to generic renderLeafScreen`);
  }
}

assert(!/generic:\s*true/.test(stageContract), "core workflow stage contract must not mark stages generic");

for (const key of Object.keys(contract)) {
  assert(
    Object.values(expectedGroups).some((keys) => keys.includes(key)),
    `${key} contract entry must belong to a known workflow stage`
  );
}

console.log("frontend workflow_stage_renderers.spec.js passed");
