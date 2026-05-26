const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function kpiDonutGradient",
  "function renderKpiIndustryDistribution",
  "function renderKpiLossExpiry",
  'request(`${root}/dashboard/kpi/industry-distribution`)',
  'request(`${root}/dashboard/kpi/loss-expiry?years=3`)',
  'data-dashboard-section="kpi-industry-distribution"',
  'data-dashboard-section="kpi-loss-expiry"',
  'data-kpi-industry="${escapeHtml(item.industryCode)}"',
  'data-kpi-loss-year="${escapeHtml(item.expiresYear)}"',
  "renderDashboardTaxBurdenKpi(kpiTaxBurden, kpiIndustryDistribution, kpiLossExpiry",
  "kpiDonutGradient(industries)",
  "lossSummary?.totalAmount",
]) {
  assert(screens.includes(snippet), `dashboard KPI distribution/loss UI must include ${snippet}`);
}

for (const label of ["업종별 법인 분포", "이월결손금 만료 예측", "만료 예정 잔액"]) {
  assert(screens.includes(label), `dashboard KPI distribution/loss UI must render ${label}`);
}

for (const snippet of [
  ".dashboard-kpi-secondary",
  ".kpi-subpanel",
  ".kpi-donut-layout",
  ".kpi-donut",
  ".kpi-distribution-list",
  ".kpi-distribution-row",
  ".kpi-loss-table",
  ".kpi-loss-row",
]) {
  assert(css.includes(snippet), `dashboard KPI distribution/loss CSS must include ${snippet}`);
}

console.log("frontend dashboard_kpi_distribution_loss.spec.js passed");
