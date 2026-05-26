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
  'class="dashboard-status-card"',
  'data-work-status="${escapeHtml(item.status)}"',
  'id="dashStartWork"',
  "신고 작업 시작",
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
  ".dashboard-main-grid",
  ".dashboard-deadline-panel",
  ".dashboard-rejected-banner",
]) {
  assert(css.includes(snippet), `dashboard first screen CSS must include ${snippet}`);
}

console.log("frontend dashboard_layout.spec.js passed");
