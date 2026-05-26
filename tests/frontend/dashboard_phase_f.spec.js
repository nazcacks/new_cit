const fs = require("fs");
const assert = require("assert");

const api = fs.readFileSync("frontend/app/api.js", "utf8");
const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "const DASHBOARD_REFRESH_INTERVAL_MS = 30_000",
  "let dashboardRealtime = null",
  "let dashboardCacheVersion = 0",
  "function startDashboardRealtime",
  "function stopDashboardRealtime",
  "setInterval(async () =>",
  'invalidateDashboardCache("poll")',
  'invalidateDashboardCache("notification-read")',
  'invalidateDashboardCache("approval-action")',
  'data-dashboard-cache-version="${dashboardCacheVersion}"',
]) {
  assert(screens.includes(snippet), `dashboard Phase F realtime/cache code must include ${snippet}`);
}

for (const snippet of [
  "function canViewDashboardApprovals",
  'dashboardRoles(auth).includes("TAX_REVIEWER")',
  "function canViewDashboardKpi",
  'showApprovals ? request(`${root}/workflow/queue?assignee=me`) : Promise.resolve([])',
  'showKpi ? request(`${root}/dashboard/kpi/tax-burden?years=5`) : Promise.resolve({ trend: [] })',
  'showKpi ? renderDashboardTaxBurdenKpi',
  'renderDashboardNotificationPanel(notificationSummary, queue, env.locale, showApprovals)',
]) {
  assert(screens.includes(snippet), `dashboard Phase F role display code must include ${snippet}`);
}

for (const snippet of [
  "function shouldBypassHttpCache",
  'path.includes("/dashboard")',
  'path.includes("/workflow/queue")',
  'path.includes("/notifications")',
  '"Cache-Control": "no-cache"',
  'fetchOptions.cache = "no-store"',
]) {
  assert(api.includes(snippet), `dashboard request cache policy must include ${snippet}`);
}

for (const snippet of [
  "@media (max-width: 1060px)",
  ".dashboard-main-grid",
  ".dashboard-lower-grid",
  ".dashboard-status-grid",
  "@media (max-width: 680px)",
  ".dashboard-notification-item",
  ".dashboard-approval-item",
]) {
  assert(css.includes(snippet), `dashboard responsive CSS must include ${snippet}`);
}

console.log("frontend dashboard_phase_f.spec.js passed");
