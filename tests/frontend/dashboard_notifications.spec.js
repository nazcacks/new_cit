const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function renderDashboardNotificationPanel",
  "function renderDashboardNotificationList",
  "request(`${root}/dashboard/notifications?limit=10`)",
  'data-dashboard-section="notifications"',
  'data-dashboard-tab="notifications"',
  'data-dashboard-tab="approvals"',
  'data-notification-unread-badge',
  'data-read-notification="${escapeHtml(item.notificationId)}"',
  'data-open-notification="${escapeHtml(item.notificationId)}"',
  'await request(`${root}/notifications/${button.dataset.readNotification}`',
  'body: JSON.stringify({ status: "READ" })',
  "dashboardNotifications.find",
  "await refreshContextFromBy(env, {",
  'env.navigate(item.routeKey || "dashboard:inbox")',
]) {
  assert(screens.includes(snippet), `dashboard notification UI must include ${snippet}`);
}

for (const snippet of [
  ".dashboard-notification-panel",
  ".dashboard-tabs",
  ".dashboard-unread-badge",
  ".dashboard-notification-list",
  ".dashboard-notification-item.unread",
  ".notification-actions",
]) {
  assert(css.includes(snippet), `dashboard notification CSS must include ${snippet}`);
}

console.log("frontend dashboard_notifications.spec.js passed");
