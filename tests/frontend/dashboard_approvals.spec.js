const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function renderDashboardApprovalQueue",
  'data-dashboard-tab="approvals"',
  'request(`${root}/workflow/queue?assignee=me`)',
  'data-approval-by="${escapeHtml(item.by_id)}"',
  'data-open-approval="${escapeHtml(item.by_id)}"',
  'data-approve-approval="${escapeHtml(item.by_id)}"',
  'data-reject-approval="${escapeHtml(item.by_id)}"',
  "item.requester_login_id",
  "item.pending_days",
  "item.route_key",
  "const approvalQueue = asArray(queue)",
  "const openApproval = async (byId) =>",
  "const runApprovalAction = async (byId, status) =>",
  'request(`${root}/business-years/${encodeURIComponent(item.by_id)}/status`',
  'status === "APPROVED"',
  'runApprovalAction(button.dataset.approveApproval, "APPROVED")',
  'runApprovalAction(button.dataset.rejectApproval, "DRAFT")',
  "await renderDashboard(env)",
  "await refreshContextFromBy(env, {",
  'env.navigate(item.route_key || "ws/appr:inbox")',
]) {
  assert(screens.includes(snippet), `dashboard approval tab must include ${snippet}`);
}

for (const label of ["내 결재함", "요청자", "대기일", "승인", "반려", "상세"]) {
  assert(screens.includes(label), `dashboard approval tab must render ${label}`);
}

for (const snippet of [
  ".dashboard-approval-list",
  ".dashboard-approval-item",
  ".approval-target",
  ".approval-meta",
  ".approval-inline-actions",
]) {
  assert(css.includes(snippet), `dashboard approval CSS must include ${snippet}`);
}

console.log("frontend dashboard_approvals.spec.js passed");
