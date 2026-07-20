const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function renderDashboardWorkStatusCards",
  "function dashboardStatusRoute",
  "summary.workStatus",
  'data-dashboard-section="work-status"',
  'class="dashboard-status-grid"',
  'data-work-status="${escapeHtml(item.status)}"',
  'data-work-status="REJECTED"',
  "DASHBOARD_STATUS_ICON",
  "function dashboardEmphasisStatuses",
  "function dashboardDeadlineBucket",
  'dash.deadline.bucket.dday',
  'dash.hero.greeting',
  'id="dashStartWork"',
  'data-dashboard-section="start"',
  'document.querySelectorAll("[data-work-status]")',
  'env.navigate(dashboardStatusRoute(card.dataset.workStatus))',
  'env.navigate("ws/start:customer-pick")',
  'data-dashboard-section="filing-deadlines"',
]) {
  assert(screens.includes(snippet), `dashboard first screen must include ${snippet}`);
}

for (const status of [
  "DRAFT",
  "IN_REVIEW_VALIDATION",
  "IN_REVIEW_APPROVAL",
  "APPROVED",
  "FILED",
]) {
  assert(screens.includes(status), `dashboard status cards must route ${status}`);
}

for (const snippet of [
  ".dashboard-home",
  ".dashboard-hero",
  ".dashboard-status-grid",
  ".dashboard-status-card",
  ".dashboard-status-card.rejected",
  ".dashboard-status-card.emphasized",
  ".dashboard-status-card .status-icon",
  ".dashboard-main-grid",
  ".dashboard-deadline-panel",
  ".deadline-bucket-row",
  ".dashboard-lower-grid--single",
]) {
  assert(css.includes(snippet), `dashboard first screen CSS must include ${snippet}`);
}

console.log("frontend dashboard_layout.spec.js passed");
