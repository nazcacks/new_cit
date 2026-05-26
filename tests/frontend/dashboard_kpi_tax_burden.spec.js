const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

for (const snippet of [
  "function renderDashboardTaxBurdenKpi",
  'data-dashboard-section="kpi-tax-burden"',
  'request(`${root}/dashboard/kpi/tax-burden?years=5`)',
  '"/api/tenants/{tenant}/dashboard/kpi/tax-burden?years=5"',
  "const trend = asArray(kpiSummary?.trend).slice(-5)",
  "averageEffectiveTaxRatePct",
  "totalTaxDue",
  "effectiveTaxRateBps",
  'data-kpi-year="${escapeHtml(item.fiscalYear)}"',
  'id="dashKpiTax"',
  'env.navigate("report:tax-burden")',
  "dashboard-lower-grid",
]) {
  assert(screens.includes(snippet), `dashboard KPI tax burden UI must include ${snippet}`);
}

for (const label of ["핵심지표", "당기 세부담 추이", "평균 실효세율", "총 부담세액", "세부담 분석"]) {
  assert(screens.includes(label), `dashboard KPI tax burden UI must render ${label}`);
}

for (const snippet of [
  ".dashboard-lower-grid",
  ".dashboard-kpi-panel",
  ".kpi-summary-strip",
  ".dashboard-kpi-chart",
  ".kpi-trend-row",
  ".kpi-caption",
]) {
  assert(css.includes(snippet), `dashboard KPI tax burden CSS must include ${snippet}`);
}

console.log("frontend dashboard_kpi_tax_burden.spec.js passed");
