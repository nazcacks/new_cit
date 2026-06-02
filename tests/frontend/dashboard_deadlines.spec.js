const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  '"/api/tenants/{tenant}/dashboard/filing-deadlines?withinDays=30"',
  '"dashboard:duesoon": (env) => renderDashboardDueSoon(env)',
  "function renderDashboardDeadlineTable",
  "formatDday(item.daysRemaining)",
  "deadlineUrgencyClass(item.urgencyLevel)",
  'data-dashboard-section="filing-deadlines"',
  'data-deadline-by="${escapeHtml(item.businessYearId)}"',
  'request(`${root}/dashboard/filing-deadlines?withinDays=30`)',
  'await refreshContextFromBy(env, {',
  'env.navigate(item.routeKey || "ws/start:snapshot")',
  'rowElement.addEventListener("keydown"',
]) {
  assert(screens.includes(snippet), `dashboard deadline UI must include ${snippet}`);
}

for (const snippet of [
  ".dashboard-deadlines .deadline-critical",
  ".dashboard-deadlines .deadline-warning",
  ".dashboard-deadlines .deadline-notice",
  ".dashboard-deadlines .deadline-row",
]) {
  assert(css.includes(snippet), `dashboard deadline CSS must include ${snippet}`);
}

console.log("frontend dashboard_deadlines.spec.js passed");
