const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function renderDashboardRecentActivities",
  'data-dashboard-section="recent-activities"',
  "request(`${root}/dashboard/recent-activities?limit=15`)",
  "const recentActivities = asArray(recentSummary.activities)",
  "asArray(activitySummary?.activities).slice(0, 15)",
  'data-activity-audit="${escapeHtml(item.auditId)}"',
  'data-open-activity="${escapeHtml(item.auditId)}"',
  "item.typeLabel || item.activityType",
  "item.routeKey || \"ad-audit\"",
  "const openActivity = async (auditId) =>",
  "recentActivities.find",
  "await refreshContextFromBy(env, {",
  "by_id: item.byId",
  "customer_id: item.customerId",
  "year_label: item.fiscalYear",
  "env.navigate(item.routeKey || \"ad-audit\")",
  "dashboard/recent-activities?limit=15",
]) {
  assert(screens.includes(snippet), `dashboard recent activity UI must include ${snippet}`);
}

for (const label of ["최근활동", "감사 로그 전체", "이동"]) {
  assert(screens.includes(label), `dashboard recent activity UI must render ${label}`);
}

for (const snippet of [
  ".dashboard-activity-panel",
  ".dashboard-activity-list",
  ".dashboard-activity-item",
  ".activity-copy",
  ".activity-title-line",
]) {
  assert(css.includes(snippet), `dashboard recent activity CSS must include ${snippet}`);
}

console.log("frontend dashboard_recent_activity.spec.js passed");
