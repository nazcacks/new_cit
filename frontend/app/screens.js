import { request, downloadBinary, escapeHtml, money, statusClass, today, asArray } from "/app/api.js";
import { bindDataGridActions, renderDataGrid } from "/app/components/grid.js";
import { hasWorkContext, progressForStatus } from "/app/context.js";
import { fieldLabel, leafFocusText, localizeRouteMeta, routeKeyToLabelKey, statusLabel, t } from "/app/i18n.js";

const DASHBOARD_REFRESH_INTERVAL_MS = 30_000;
let dashboardRealtime = null;
let dashboardCacheVersion = 0;

const legacyRouteLabels = {
  dashboard: ["대시보드", "대시보드"],
  "ws-start": ["신고 작업", "0. 작업 시작"],
  "ws-info": ["신고 작업", "1. 세무정보 입력"],
  "ws-adj": ["신고 작업", "2. 세무조정"],
  "ws-form": ["신고 작업", "3. 서식 작성"],
  "ws-val": ["신고 작업", "4. 검증"],
  "ws-appr": ["신고 작업", "5. 결재"],
  "ws-print": ["신고 작업", "6. 출력"],
  "ws-file": ["신고 작업", "7. 전자신고"],
  "post-hist": ["사후 관리", "1. 신고 이력"],
  "post-amend": ["사후 관리", "2. 수정신고/경정청구"],
  "rp-alerts": ["분석/보고서", "1. 알림 센터"],
  "rp-compare": ["분석/보고서", "2. 사업연도 비교"],
  "rp-burden": ["분석/보고서", "3. 세부담 분석"],
  "rp-reserve": ["분석/보고서", "4. 유보 잔액 추이"],
  "ad-tenant": ["관리", "0. 테넌트 관리"],
  "ad-cust": ["관리", "A. 고객사 관리"],
  "ad-user-list": ["관리", "B. 사용자 관리"],
  "ad-role": ["관리", "C. 역할/권한 매트릭스"],
  "ad-menu-fn": ["관리", "D. 메뉴/기능 관리"],
  "ad-cacc": ["관리", "E. 담당 법인 권한"],
  "ad-law": ["관리", "F. 법령/세율 버전"],
  "ad-form": ["관리", "G. 서식 버전"],
  "ad-audit": ["관리", "H. 감사/로그"],
};

const legacyRoutes = Object.freeze(Object.fromEntries(Object.entries(legacyRouteLabels).map(([key, labels]) => {
  const [group, title] = labels;
  return [key, route(String(group), String(title), legacyLayout(key), key, key)];
})));

export const leafRoutes = Object.freeze({
  ...Object.fromEntries([
    ["dashboard:overview"],
    ["dashboard:duesoon"],
    ["dashboard:inbox"],
    ["dashboard:recent"],
    ["dashboard:kpi-tax"],
  ].map(([key]) => leafRoute(key, "plain", "dashboard"))),
  ...Object.fromEntries([
    ["ws/start:customer-pick"],
    ["ws/start:by-pick"],
    ["ws/start:snapshot"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-start"))),
  ...Object.fromEntries([
    ["ws/info:fs"],
    ["ws/info:mapping"],
    ["ws/info:assets"],
    ["ws/info:transactions"],
    ["ws/info:vehicle"],
    ["ws/info:consistency"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-info"))),
  ...Object.fromEntries([
    ["B1"],
    ["B2"],
    ["B3"],
    ["B4"],
    ["B5"],
    ["B6"],
    ["B7"],
    ["B8"],
    ["B9"],
    ["B10"],
    ["B11"],
    ["B12"],
    ["B13"],
    ["B14"],
    ["B15"],
    ["B16"],
    ["B17"],
  ].map(([code]) => leafRoute(`ws/adj:${code}`, "workspace", "ws-adj"))),
  ...Object.fromEntries([
    ["ws/form:form3"],
    ["ws/form:attachments"],
    ["ws/form:preview"],
    ["ws/form:linkage"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-form"))),
  ...Object.fromEntries([
    ["ws/val:run"],
    ["ws/val:issues"],
    ["ws/val:rules"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-val"))),
  ...Object.fromEntries([
    ["ws/appr:request"],
    ["ws/appr:inbox"],
    ["ws/appr:rejected"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-appr"))),
  ...Object.fromEntries([
    ["ws/print:preview"],
    ["ws/print:bulk"],
    ["ws/print:history"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-print"))),
  ...Object.fromEntries([
    ["ws/file:precheck"],
    ["ws/file:generate"],
    ["ws/file:submit"],
    ["ws/file:done"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-file"))),
  ...Object.fromEntries([
    ["post/hist:list", "post-hist"],
    ["post/amend:unlock", "post-amend"],
    ["post/amend:version", "post-amend"],
    ["post/amend:diff", "post-amend"],
    ["post/amend:resubmit", "post-amend"],
    ["post/correction", "post-amend"],
  ].map(([key, delegate]) => leafRoute(key, "plain", delegate))),
  ...Object.fromEntries([
    ["report:year-compare", "rp-compare"],
    ["report:tax-burden", "rp-burden"],
    ["report:reserve-trend", "rp-reserve"],
    ["report:loss-expiry", "rp-reserve"],
    ["report:industry-stats", "rp-burden"],
    ["report:custom", "rp-reserve"],
  ].map(([key, delegate]) => leafRoute(key, "plain", delegate))),
  ...Object.fromEntries([
    ["admin/tenant:list", "ad-tenant"],
    ["admin/cust:list", "ad-cust"],
    ["admin/cust:by-master", "ad-cust"],
    ["admin/cust:agent", "ad-cust"],
    ["admin/sec:users", "ad-user-list"],
    ["admin/sec:roles", "ad-role"],
    ["admin/sec:matrix", "ad-role"],
    ["admin/sec:menus", "ad-menu-fn"],
    ["admin/sec:functions", "ad-menu-fn"],
    ["admin/sec:mask", "ad-role"],
    ["admin/sec:scope", "ad-role"],
    ["admin/cacc:assign", "ad-cacc"],
    ["admin/cacc:groups", "ad-cacc"],
    ["admin/cacc:rules", "ad-cacc"],
    ["admin/cacc:delegate", "ad-cacc"],
    ["admin/cacc:override", "ad-cacc"],
    ["admin/law:master", "ad-law"],
    ["admin/law:rates", "ad-law"],
    ["admin/law:limits", "ad-law"],
    ["admin/law:credits", "ad-law"],
    ["admin/law:depr-lives", "ad-law"],
    ["admin/law:sme", "ad-law"],
    ["admin/law:loss-rule", "ad-law"],
    ["admin/law:snapshots", "ad-law"],
    ["admin/law:impact", "ad-law"],
    ["admin/law:history", "ad-law"],
    ["admin/form:master", "ad-form"],
    ["admin/form:versions", "ad-form"],
    ["admin/form:fields", "ad-form"],
    ["admin/form:validations", "ad-form"],
    ["admin/form:linkage-rule", "ad-form"],
    ["admin/form:migration", "ad-form"],
    ["admin/form:efile-map", "ad-form"],
    ["admin/form:by-set", "ad-form"],
    ["admin/form:impact", "ad-form"],
    ["admin/code:manage", "ad-menu-fn"],
    ["admin/audit:events", "ad-audit"],
    ["admin/audit:login", "ad-audit"],
    ["admin/audit:perm", "ad-audit"],
    ["admin/audit:settings", "ad-audit"],
  ].map(([key, delegate]) => leafRoute(key, "admin", delegate))),
});

const routes = Object.freeze({ ...legacyRoutes, ...leafRoutes });

const screenByDelegate = {
  dashboard: renderDashboard,
  "ws-start": renderWorkStart,
  "ws-info": renderWorkInfo,
  "ws-adj": renderAdjustments,
  "ws-form": renderForms,
  "ws-val": renderValidation,
  "ws-appr": renderApproval,
  "ws-print": renderPrint,
  "ws-file": renderEfiling,
  "post-hist": renderPostHistory,
  "post-amend": renderPostAmend,
  "rp-alerts": renderAlerts,
  "rp-compare": renderYearCompare,
  "rp-burden": renderTaxBurden,
  "rp-reserve": renderReserveTrend,
  "ad-tenant": renderAdminTenants,
  "ad-cust": renderAdminCustomers,
  "ad-user-list": renderAdminUsers,
  "ad-role": renderAdminRoles,
  "ad-menu-fn": renderAdminMenus,
  "ad-cacc": renderAdminCustomerAccess,
  "ad-law": renderAdminLaw,
  "ad-form": renderAdminForms,
  "ad-audit": renderAdminAudit,
};

export const workflowLeafRendererContract = Object.freeze({
  "ws/start:customer-pick": "renderWorkStartLeaf",
  "ws/start:by-pick": "renderWorkStartLeaf",
  "ws/start:snapshot": "renderWorkStartLeaf",
  "ws/info:fs": "renderWorkInfoLeaf",
  "ws/info:mapping": "renderWorkInfoLeaf",
  "ws/info:assets": "renderWorkInfoLeaf",
  "ws/info:transactions": "renderWorkInfoLeaf",
  "ws/info:vehicle": "renderWorkInfoLeaf",
  "ws/info:consistency": "renderWorkInfoLeaf",
  "ws/adj:B1": "renderAdjustmentLeaf",
  "ws/adj:B2": "renderAdjustmentLeaf",
  "ws/adj:B3": "renderAdjustmentLeaf",
  "ws/adj:B4": "renderAdjustmentLeaf",
  "ws/adj:B5": "renderAdjustmentLeaf",
  "ws/adj:B6": "renderAdjustmentLeaf",
  "ws/adj:B7": "renderAdjustmentLeaf",
  "ws/adj:B8": "renderAdjustmentLeaf",
  "ws/adj:B9": "renderAdjustmentLeaf",
  "ws/adj:B10": "renderAdjustmentLeaf",
  "ws/adj:B11": "renderAdjustmentLeaf",
  "ws/adj:B12": "renderAdjustmentLeaf",
  "ws/adj:B13": "renderAdjustmentLeaf",
  "ws/adj:B14": "renderAdjustmentLeaf",
  "ws/adj:B15": "renderAdjustmentLeaf",
  "ws/adj:B16": "renderAdjustmentLeaf",
  "ws/adj:B17": "renderAdjustmentLeaf",
  "ws/form:form3": "renderFormsLeaf",
  "ws/form:attachments": "renderFormsLeaf",
  "ws/form:preview": "renderFormsLeaf",
  "ws/form:linkage": "renderFormsLeaf",
  "ws/val:run": "renderValidationLeaf",
  "ws/val:issues": "renderValidationLeaf",
  "ws/val:rules": "renderValidationLeaf",
  "ws/appr:request": "renderApprovalLeaf",
  "ws/appr:inbox": "renderApprovalLeaf",
  "ws/appr:rejected": "renderApprovalLeaf",
  "ws/print:preview": "renderPrintLeaf",
  "ws/print:bulk": "renderPrintLeaf",
  "ws/print:history": "renderPrintLeaf",
  "ws/file:precheck": "renderEfilingLeaf",
  "ws/file:generate": "renderEfilingLeaf",
  "ws/file:submit": "renderEfilingLeaf",
  "ws/file:done": "renderEfilingLeaf",
  "post/hist:list": "renderPostHistoryLeaf",
  "post/amend:unlock": "renderPostAmendLeaf",
  "post/amend:version": "renderPostAmendLeaf",
  "post/amend:diff": "renderPostAmendLeaf",
  "post/amend:resubmit": "renderPostAmendLeaf",
  "post/correction": "renderPostAmendLeaf",
});

export const workflowStageContract = Object.freeze({
  workStart: {
    stage: "3.3",
    routes: ["ws/start:customer-pick", "ws/start:by-pick", "ws/start:snapshot"],
    renderer: "renderWorkStartLeaf",
    generic: false,
  },
  taxData: {
    stage: "3.5",
    routes: ["ws/info:fs", "ws/info:mapping", "ws/info:assets", "ws/info:transactions", "ws/info:vehicle", "ws/info:consistency"],
    renderer: "renderWorkInfoLeaf",
    generic: false,
  },
  adjustments: {
    stage: "3.6",
    routes: ["ws/adj:B1", "ws/adj:B2", "ws/adj:B3", "ws/adj:B4", "ws/adj:B5", "ws/adj:B6", "ws/adj:B7", "ws/adj:B8", "ws/adj:B9", "ws/adj:B10", "ws/adj:B11", "ws/adj:B12", "ws/adj:B13", "ws/adj:B14", "ws/adj:B15", "ws/adj:B16", "ws/adj:B17"],
    renderer: "renderAdjustmentLeaf",
    generic: false,
  },
  forms: {
    stage: "3.7",
    routes: ["ws/form:form3", "ws/form:attachments", "ws/form:preview", "ws/form:linkage"],
    renderer: "renderFormsLeaf",
    generic: false,
  },
  validation: {
    stage: "3.8",
    routes: ["ws/val:run", "ws/val:issues", "ws/val:rules"],
    renderer: "renderValidationLeaf",
    generic: false,
  },
  approval: {
    stage: "3.8",
    routes: ["ws/appr:request", "ws/appr:inbox", "ws/appr:rejected"],
    renderer: "renderApprovalLeaf",
    generic: false,
  },
  print: {
    stage: "3.9",
    routes: ["ws/print:preview", "ws/print:bulk", "ws/print:history"],
    renderer: "renderPrintLeaf",
    generic: false,
  },
  efiling: {
    stage: "3.10",
    routes: ["ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done"],
    renderer: "renderEfilingLeaf",
    generic: false,
  },
  postHistory: {
    stage: "3.11",
    routes: ["post/hist:list"],
    renderer: "renderPostHistoryLeaf",
    generic: false,
  },
  postAmend: {
    stage: "3.11",
    routes: ["post/amend:unlock", "post/amend:version", "post/amend:diff", "post/amend:resubmit", "post/correction"],
    renderer: "renderPostAmendLeaf",
    generic: false,
  },
});

function route(group, title, layout, delegate, key = "") {
  return {
    group,
    title,
    groupKey: groupKeyForDelegate(delegate),
    titleKey: routeKeyToLabelKey(key) || routeKeyToLabelKey(delegate) || title,
    layout,
    delegate,
    s1: false,
  };
}

function bindAdminRouteButtons(env) {
  document.querySelectorAll("[data-admin-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.adminRoute));
  });
}

function leafRoute(key, layout, delegate) {
  const titleKey = routeKeyToLabelKey(key);
  const groupKey = groupKeyForDelegate(delegate);
  return [key, {
    group: groupKey,
    title: titleKey || key,
    groupKey,
    titleKey: titleKey || key,
    layout,
    delegate,
    leafKey: key,
    s1: true,
  }];
}

function groupKeyForDelegate(delegate) {
  if (delegate === "dashboard") return "nav.dashboard";
  if (String(delegate).startsWith("ws-")) return "nav.workspace";
  if (String(delegate).startsWith("post-")) return "nav.post";
  if (String(delegate).startsWith("rp-")) return "nav.reports";
  if (String(delegate).startsWith("ad-")) return "nav.admin";
  return routeKeyToLabelKey(delegate) || String(delegate || "");
}

function legacyLayout(key) {
  if (key.startsWith("ws-")) return "workspace";
  if (key.startsWith("ad-")) return "admin";
  return "plain";
}

export const adjustmentTaxonomy = Object.freeze([
  { code: "B1", ko: "소득금액조정명세서", en: "Income adjustment statement", module: "income", api: "adjustments/income" },
  { code: "B2", ko: "기부금", en: "Donations", module: "transactions", api: "adjustments/transactions/B2" },
  { code: "B3", ko: "접대비", en: "Entertainment expense", module: "transactions", api: "adjustments/transactions/B3" },
  { code: "B4", ko: "감가상각비", en: "Depreciation expense", module: "assets", api: "adjustments/assets/B4" },
  { code: "B5", ko: "퇴직급여충당금/퇴직연금", en: "Retirement allowance reserve/pension", module: "assets", api: "adjustments/assets/B5" },
  { code: "B6", ko: "대손충당금 및 대손금", en: "Bad debt reserve and bad debts", module: "assets", api: "adjustments/assets/B6" },
  { code: "B7", ko: "외화자산·부채 평가", en: "Foreign currency asset/liability valuation", module: "evaluation", api: "adjustments/evaluation/B7" },
  { code: "B8", ko: "재고자산·유가증권 평가", en: "Inventory/securities valuation", module: "evaluation", api: "adjustments/evaluation/B8" },
  { code: "B9", ko: "지급이자 손금불산입", en: "Non-deductible interest expense", module: "transactions", api: "adjustments/transactions/B9" },
  { code: "B10", ko: "업무용승용차 관련비용", en: "Business vehicle expenses", module: "assets", api: "adjustments/assets/B10" },
  { code: "B11", ko: "이월결손금", en: "Loss carryforward", module: "evaluation", api: "adjustments/evaluation/B11" },
  { code: "B12", ko: "세액공제·감면", en: "Tax credits/reductions", module: "tax", api: "adjustments/tax/B12" },
  { code: "B13", ko: "최저한세", en: "Minimum tax", module: "tax", api: "adjustments/tax/B13" },
  { code: "B14", ko: "가산세", en: "Additional tax", module: "tax", api: "adjustments/tax/B14" },
  { code: "B15", ko: "자본금과 적립금", en: "Capital and reserves", module: "evaluation", api: "adjustments/evaluation/B15" },
  { code: "B16", ko: "외국법인 세무조정", en: "Foreign corporation adjustment", module: "special", api: "adjustments/special/B16" },
  { code: "B17", ko: "연결납세", en: "Consolidated tax", module: "special", api: "adjustments/special/B17" },
]);

const adjustmentModules = adjustmentTaxonomy.map(({ code, ko, module }) => [code, ko, module]);

const adjustmentGridColumns = [
  { key: "source_module", labelKey: "field.module" },
  { key: "item_code", labelKey: "field.code" },
  { key: "item_name", labelKey: "field.item" },
  { key: "direction", labelKey: "field.direction" },
  { key: "amount", labelKey: "field.amount", format: "money" },
  { key: "disposition", labelKey: "field.disposition" },
];

const leafViewState = new Map();
const adjustmentRunState = new Map();
const validationRunState = new Map();

export const leafScreenSpecs = Object.freeze({
  "dashboard:overview": leafSpec("GET", "/api/tenants/{tenant}/dashboard", "dashboard", "READ"),
  "dashboard:duesoon": leafSpec("GET", "/api/tenants/{tenant}/dashboard/filing-deadlines?withinDays=30", "dashboard", "READ"),
  "dashboard:inbox": leafSpec("GET", "/api/tenants/{tenant}/workflow/queue?assignee=me", "workflow", "READ"),
  "dashboard:recent": leafSpec("GET", "/api/tenants/{tenant}/dashboard/recent-activities?limit=15", "audit", "READ"),
  "dashboard:kpi-tax": leafSpec("GET", "/api/tenants/{tenant}/dashboard/kpi/tax-burden?years=5", "reports", "READ"),
  "ws/start:customer-pick": leafSpec("GET", "/api/tenants/{tenant}/customers", "customer", "READ"),
  "ws/start:by-pick": leafSpec("GET", "/api/tenants/{tenant}/business-years", "customer", "READ"),
  "ws/start:snapshot": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/snapshot", "customer", "READ", { requires: ["work-context"] }),
  "ws/info:fs": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/financial-statements", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:mapping": leafSpec("GET", "/api/tenants/{tenant}/customers/{customerId}/account-mappings", "tax-data", "UPDATE", { requires: ["work-context"] }),
  "ws/info:assets": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/assets", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:transactions": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/transactions", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:vehicle": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/vehicle-usage-logs", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:consistency": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/validation", "tax-data", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B1": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/income", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B2": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B2", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B3": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B3", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B4": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B4", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B5": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B5", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B6": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B6", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B7": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B7", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B8": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B8", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B9": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B9", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B10": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B10", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B11": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B11", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B12": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B12", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B13": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B13", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B14": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B14", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B15": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B15", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B16": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/special/B16", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B17": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/special/B17", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/form:form3": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "CREATE", { requires: ["work-context"] }),
  "ws/form:attachments": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/attachments", "forms", "CREATE", { requires: ["work-context"] }),
  "ws/form:preview": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "READ", { requires: ["work-context"] }),
  "ws/form:linkage": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/linkage-check", "forms", "READ", { requires: ["work-context"] }),
  "ws/val:run": leafSpec("POST", "/api/tenants/{tenant}/business-years/{byId}/validation/run", "validation", "CALCULATE", { requires: ["work-context"] }),
  "ws/val:issues": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/validation/issues", "validation", "READ", { requires: ["work-context"] }),
  "ws/val:rules": leafSpec("GET", "/api/tenants/{tenant}/validation/rules", "validation", "READ", { requires: ["work-context"] }),
  "ws/appr:request": leafSpec("POST", "/api/tenants/{tenant}/business-years/{byId}/workflow/request", "workflow", "APPROVE", { requires: ["work-context"] }),
  "ws/appr:inbox": leafSpec("GET", "/api/tenants/{tenant}/workflow/queue?assignee=me", "workflow", "READ", { requires: ["work-context"] }),
  "ws/appr:rejected": leafSpec("GET", "/api/tenants/{tenant}/workflow/events?status=REJECTED", "workflow", "READ", { requires: ["work-context"] }),
  "ws/print:preview": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "READ", { requires: ["work-context"] }),
  "ws/print:bulk": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/attachments", "forms", "PRINT", { requires: ["work-context"] }),
  "ws/print:history": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/print/history", "forms", "READ", { requires: ["work-context"] }),
  "ws/file:precheck": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/precheck", "efiling", "READ", { requires: ["work-context"] }),
  "ws/file:generate": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/format-spec", "efiling", "EFILE", { requires: ["work-context"] }),
  "ws/file:submit": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings", "efiling", "EFILE", { requires: ["work-context"] }),
  "ws/file:done": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/latest", "efiling", "READ", { requires: ["work-context"] }),
  "post/hist:list": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings", "efiling", "READ"),
  "post/amend:unlock": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "UPDATE", { requires: ["work-context"] }),
  "post/amend:version": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-version-mode", "post", "CREATE", { requires: ["work-context"] }),
  "post/amend:diff": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "READ", { requires: ["work-context"] }),
  "post/amend:resubmit": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "EFILE", { requires: ["work-context"] }),
  "post/correction": leafSpec("GET", "/api/tenants/{tenant}/correction-claims", "post", "CREATE"),
  "report:year-compare": leafSpec("GET", "/api/tenants/{tenant}/reports/year-comparison", "reports", "READ"),
  "report:tax-burden": leafSpec("GET", "/api/tenants/{tenant}/reports/tax-burden", "reports", "READ"),
  "report:reserve-trend": leafSpec("GET", "/api/tenants/{tenant}/reports/reserve-trend", "reports", "READ"),
  "report:loss-expiry": leafSpec("GET", "/api/tenants/{tenant}/reports/loss-expiry", "reports", "READ"),
  "report:industry-stats": leafSpec("GET", "/api/tenants/{tenant}/reports/industry-stats", "reports", "READ"),
  "report:custom": leafSpec("GET", "/api/tenants/{tenant}/reports/custom", "reports", "READ"),
  "admin/tenant:list": leafSpec("GET", "/api/tenants", "admin", "READ"),
  "admin/cust:list": leafSpec("GET", "/api/tenants/{tenant}/customers", "customer", "READ"),
  "admin/cust:by-master": leafSpec("GET", "/api/tenants/{tenant}/business-years?bare=true", "customer", "UPDATE"),
  "admin/cust:agent": leafSpec("GET", "/api/tenants/{tenant}/tax-agents", "customer", "UPDATE"),
  "admin/sec:users": leafSpec("GET", "/api/admin/tenants/{tenant}/users", "admin", "READ"),
  "admin/sec:roles": leafSpec("GET", "/api/admin/roles", "admin", "READ"),
  "admin/sec:matrix": leafSpec("GET", "/api/admin/role-permissions", "admin", "READ"),
  "admin/sec:menus": leafSpec("GET", "/api/admin/menus", "admin", "UPDATE"),
  "admin/sec:functions": leafSpec("GET", "/api/admin/functions", "admin", "UPDATE"),
  "admin/sec:mask": leafSpec("GET", "/api/admin/field-masking", "admin", "MASK_OFF"),
  "admin/sec:scope": leafSpec("GET", "/api/admin/data-scope", "admin", "UPDATE"),
  "admin/cacc:assign": leafSpec("GET", "/api/tenants/{tenant}/access-delegations", "permissions", "UPDATE"),
  "admin/cacc:groups": leafSpec("GET", "/api/admin/customer-groups", "permissions", "UPDATE"),
  "admin/cacc:rules": leafSpec("GET", "/api/admin/customer-rules", "permissions", "UPDATE"),
  "admin/cacc:delegate": leafSpec("GET", "/api/admin/access-delegations", "permissions", "DELEGATE"),
  "admin/cacc:override": leafSpec("GET", "/api/admin/customer-access/override", "permissions", "UPDATE"),
  "admin/law:master": leafSpec("GET", "/api/tax-laws", "law", "READ"),
  "admin/law:rates": leafSpec("GET", "/api/tax-rates", "law", "UPDATE"),
  "admin/law:limits": leafSpec("GET", "/api/tax-limits?category=LIMIT", "law", "UPDATE"),
  "admin/law:credits": leafSpec("GET", "/api/tax-limits?category=CREDIT", "law", "UPDATE"),
  "admin/law:depr-lives": leafSpec("GET", "/api/tax-limits?category=DEPRECIATION_LIFE", "law", "UPDATE"),
  "admin/law:sme": leafSpec("GET", "/api/tax-limits?category=SME_CRITERIA", "law", "UPDATE"),
  "admin/law:loss-rule": leafSpec("GET", "/api/tax-limits?category=LOSS_RULE", "law", "UPDATE"),
  "admin/law:snapshots": leafSpec("GET", "/api/law-versioning/summary", "law", "READ"),
  "admin/law:impact": leafSpec("GET", "/api/law-versioning/summary", "law", "CALCULATE"),
  "admin/law:history": leafSpec("GET", "/api/law-amendments", "law", "READ"),
  "admin/form:master": leafSpec("GET", "/api/form-versioning/forms", "forms", "READ"),
  "admin/form:versions": leafSpec("GET", "/api/form-versioning/versions", "forms", "READ"),
  "admin/form:fields": leafSpec("GET", "/api/form-versioning/versions/{formVersionId}/fields", "forms", "UPDATE"),
  "admin/form:validations": leafSpec("GET", "/api/form-versioning/versions/{formVersionId}/validations", "forms", "UPDATE"),
  "admin/form:linkage-rule": leafSpec("GET", "/api/form-versioning/relationships", "forms", "UPDATE"),
  "admin/form:migration": leafSpec("GET", "/api/form-versioning/versions", "forms", "CREATE"),
  "admin/form:efile-map": leafSpec("GET", "/api/form-versioning/efile-map", "forms", "UPDATE"),
  "admin/form:by-set": leafSpec("GET", "/api/form-versioning/by-set", "forms", "UPDATE"),
  "admin/form:impact": leafSpec("POST", "/api/form-versioning/impact", "forms", "CALCULATE"),
  "admin/code:manage": leafSpec("GET", "/api/tenants/{tenant}/codes?group=ALL", "admin", "UPDATE"),
  "admin/audit:events": leafSpec("GET", "/api/tenants/{tenant}/audit-logs", "audit", "READ"),
  "admin/audit:login": leafSpec("GET", "/api/login-history", "audit", "READ"),
  "admin/audit:perm": leafSpec("GET", "/api/permission-change-history", "audit", "READ"),
  "admin/audit:settings": leafSpec("GET", "/api/system-settings", "audit", "READ"),
});

export const screenByLeaf = Object.freeze({
  "dashboard:overview": (env) => renderDashboard(env),
  "dashboard:duesoon": (env) => renderDashboard(env),
  "dashboard:inbox": (env) => renderLeafScreen(env, "dashboard:inbox"),
  "dashboard:recent": (env) => renderLeafScreen(env, "dashboard:recent"),
  "dashboard:kpi-tax": (env) => renderLeafScreen(env, "dashboard:kpi-tax"),
  "ws/start:customer-pick": (env) => renderWorkStartLeaf(env),
  "ws/start:by-pick": (env) => renderWorkStartLeaf(env),
  "ws/start:snapshot": (env) => renderWorkStartLeaf(env),
  "ws/info:fs": (env) => renderWorkInfoLeaf(env),
  "ws/info:mapping": (env) => renderWorkInfoLeaf(env),
  "ws/info:assets": (env) => renderWorkInfoLeaf(env),
  "ws/info:transactions": (env) => renderWorkInfoLeaf(env),
  "ws/info:vehicle": (env) => renderWorkInfoLeaf(env),
  "ws/info:consistency": (env) => renderWorkInfoLeaf(env),
  "ws/adj:B1": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B2": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B3": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B4": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B5": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B6": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B7": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B8": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B9": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B10": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B11": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B12": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B13": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B14": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B15": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B16": (env) => renderAdjustmentLeaf(env),
  "ws/adj:B17": (env) => renderAdjustmentLeaf(env),
  "ws/form:form3": (env) => renderFormsLeaf(env),
  "ws/form:attachments": (env) => renderFormsLeaf(env),
  "ws/form:preview": (env) => renderFormsLeaf(env),
  "ws/form:linkage": (env) => renderFormsLeaf(env),
  "ws/val:run": (env) => renderValidationLeaf(env),
  "ws/val:issues": (env) => renderValidationLeaf(env),
  "ws/val:rules": (env) => renderValidationLeaf(env),
  "ws/appr:request": (env) => renderApprovalLeaf(env),
  "ws/appr:inbox": (env) => renderApprovalLeaf(env),
  "ws/appr:rejected": (env) => renderApprovalLeaf(env),
  "ws/print:preview": (env) => renderPrintLeaf(env),
  "ws/print:bulk": (env) => renderPrintLeaf(env),
  "ws/print:history": (env) => renderPrintLeaf(env),
  "ws/file:precheck": (env) => renderEfilingLeaf(env),
  "ws/file:generate": (env) => renderEfilingLeaf(env),
  "ws/file:submit": (env) => renderEfilingLeaf(env),
  "ws/file:done": (env) => renderEfilingLeaf(env),
  "post/hist:list": (env) => renderPostHistoryLeaf(env),
  "post/amend:unlock": (env) => renderPostAmendLeaf(env),
  "post/amend:version": (env) => renderPostAmendLeaf(env),
  "post/amend:diff": (env) => renderPostAmendLeaf(env),
  "post/amend:resubmit": (env) => renderPostAmendLeaf(env),
  "post/correction": (env) => renderPostAmendLeaf(env),
  "report:year-compare": (env) => renderLeafScreen(env, "report:year-compare"),
  "report:tax-burden": (env) => renderLeafScreen(env, "report:tax-burden"),
  "report:reserve-trend": (env) => renderLeafScreen(env, "report:reserve-trend"),
  "report:loss-expiry": (env) => renderLeafScreen(env, "report:loss-expiry"),
  "report:industry-stats": (env) => renderLeafScreen(env, "report:industry-stats"),
  "report:custom": (env) => renderLeafScreen(env, "report:custom"),
  "admin/tenant:list": (env) => renderAdminTenantLeaf(env),
  "admin/cust:list": (env) => renderAdminCustomers(env),
  "admin/cust:by-master": (env) => renderAdminCustomers(env),
  "admin/cust:agent": (env) => renderAdminCustomers(env),
  "admin/sec:users": (env) => renderAdminRoles(env),
  "admin/sec:roles": (env) => renderAdminRoles(env),
  "admin/sec:matrix": (env) => renderAdminRoles(env),
  "admin/sec:menus": (env) => renderAdminMenus(env),
  "admin/sec:functions": (env) => renderAdminMenus(env),
  "admin/sec:mask": (env) => renderAdminRoles(env),
  "admin/sec:scope": (env) => renderAdminRoles(env),
  "admin/cacc:assign": (env) => renderAdminCustomerAccess(env),
  "admin/cacc:groups": (env) => renderAdminCustomerAccess(env),
  "admin/cacc:rules": (env) => renderAdminCustomerAccess(env),
  "admin/cacc:delegate": (env) => renderAdminCustomerAccess(env),
  "admin/cacc:override": (env) => renderAdminCustomerAccess(env),
  "admin/law:master": (env) => renderAdminLaw(env),
  "admin/law:rates": (env) => renderAdminLaw(env),
  "admin/law:limits": (env) => renderAdminLaw(env),
  "admin/law:credits": (env) => renderAdminLaw(env),
  "admin/law:depr-lives": (env) => renderAdminLaw(env),
  "admin/law:sme": (env) => renderAdminLaw(env),
  "admin/law:loss-rule": (env) => renderAdminLaw(env),
  "admin/law:snapshots": (env) => renderAdminLaw(env),
  "admin/law:impact": (env) => renderAdminLaw(env),
  "admin/law:history": (env) => renderAdminLaw(env),
  "admin/form:master": (env) => renderAdminForms(env),
  "admin/form:versions": (env) => renderAdminForms(env),
  "admin/form:fields": (env) => renderAdminForms(env),
  "admin/form:validations": (env) => renderAdminForms(env),
  "admin/form:linkage-rule": (env) => renderAdminForms(env),
  "admin/form:migration": (env) => renderAdminForms(env),
  "admin/form:efile-map": (env) => renderAdminForms(env),
  "admin/form:by-set": (env) => renderAdminForms(env),
  "admin/form:impact": (env) => renderAdminForms(env),
  "admin/code:manage": (env) => renderLeafScreen(env, "admin/code:manage"),
  "admin/audit:events": (env) => renderAdminAudit(env),
  "admin/audit:login": (env) => renderAdminAudit(env),
  "admin/audit:perm": (env) => renderAdminAudit(env),
  "admin/audit:settings": (env) => renderAdminAudit(env),
});

async function renderWorkStartLeaf(env) {
  await renderWorkStart(env);
}

async function renderWorkInfoLeaf(env) {
  await renderWorkInfo(env);
}

async function renderAdjustmentLeaf(env) {
  await renderAdjustments(env);
}

async function renderFormsLeaf(env) {
  await renderForms(env);
}

async function renderValidationLeaf(env) {
  await renderValidation(env);
}

async function renderApprovalLeaf(env) {
  await renderApproval(env);
}

async function renderPrintLeaf(env) {
  await renderPrint(env);
}

async function renderEfilingLeaf(env) {
  await renderEfiling(env);
}

async function renderPostHistoryLeaf(env) {
  await renderPostHistory(env);
}

async function renderPostAmendLeaf(env) {
  await renderPostAmend(env);
}

function leafSpec(method, path, module, fn, options = {}) {
  return {
    primary: { method, path },
    action: { method: "POST", path: "/api/tenants/{tenant}/leaf-actions" },
    perm: { module, function: fn },
    requires: options.requires || [],
    featureFlag: options.featureFlag || null,
    typology: options.typology || null,
    columns: options.columns || null,
    rowKey: options.rowKey || null,
    update: options.update || null,
    form: options.form || null,
    title: options.title || null,
    description: options.description || null,
    kpis: options.kpis || null,
  };
}

async function renderAdminTenantLeaf(env) {
  const key = "admin/tenant:list";
  const spec = enrichLeafSpec(key, leafScreenSpecs[key], env.locale);
  const meta = { ...(env.routeMeta || routeMeta(key, env.locale)), leafKey: key };
  const roles = env.auth?.user?.roles || [];
  if (!roles.includes("SUPER_ADMIN") && !roles.includes("TENANT_ADMIN")) {
    env.outlet.innerHTML = renderEmptyState(key, {
      kind: "perm",
      title: t(env.locale, "context.requiredTitle"),
      message: `SUPER_ADMIN / TENANT_ADMIN ${t(env.locale, "common.permission")}`,
    }, meta, spec, "", "", env.locale);
    return;
  }
  await renderAdminTenants(env);
}

async function renderLeafScreen(env, key) {
  const spec = enrichLeafSpec(key, leafScreenSpecs[key], env.locale);
  const meta = { ...(env.routeMeta || routeMeta(key, env.locale)), leafKey: key };
  const gate = leafGate(env, key, spec);
  if (gate) {
    env.outlet.innerHTML = renderEmptyState(key, gate, meta, spec, "", "", env.locale);
    bindEmptyStateActions(env, gate);
    return;
  }

  const primaryApi = resolveApiPath(spec.primary.path, env);
  const actionApi = resolveApiPath(spec.action.path, env);
  let primaryPayload;
  try {
    primaryPayload = await request(primaryApi, apiOptions(spec.primary, key, primaryApi, env));
  } catch (error) {
    env.outlet.innerHTML = renderEmptyState(key, {
      kind: "error",
      title: t(env.locale, "validation.loadFailed"),
      message: error.message,
      action: "retry",
    }, meta, spec, primaryApi, actionApi, env.locale);
    bindEmptyStateActions(env, { action: "retry" });
    return;
  }

  const customPayload = await loadLeafRecords(env, key).catch(() => ({ rows: [] }));
  const primaryRows = normalizeLeafRows(primaryPayload, key, "api");
  const customRows = normalizeLeafRows(customPayload, key, "leaf_records");
  const rows = leafRowsForContext(key, env, [...customRows, ...primaryRows]);
  const state = {
    env,
    key,
    spec,
    meta,
    primaryApi,
    actionApi,
    rows,
    query: "",
    status: "ALL",
  };
  leafViewState.set(key, state);
  env.outlet.innerHTML = renderLeafTemplate(state);
  bindLeafTemplate(env, state);
}

async function loadLeafRecords(env, key) {
  return request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records?leaf_key=${encodeURIComponent(key)}`);
}

function leafRowsForContext(key, env, rows) {
  const customerId = env.context?.customerId;
  if (key !== "ws/start:by-pick" || !customerId) return rows;
  return rows.filter((row) => String(row.customer_id || "") === String(customerId));
}

const TYPOLOGY_RENDERERS = Object.freeze({
  grid: renderTypologyGrid,
  "grid-tree": renderTypologyGridTree,
  dashboard: renderTypologyDashboard,
  wizard: renderTypologyWizard,
  form: renderTypologyForm,
  chart: renderTypologyChart,
  detail: renderTypologyDetail,
});

const TYPOLOGY_GRID_TREE = new Set(["admin/sec:menus", "admin/form:fields", "admin/code:manage"]);
const TYPOLOGY_DASHBOARD = new Set(["dashboard:overview", "dashboard:duesoon", "dashboard:inbox", "dashboard:recent"]);
const TYPOLOGY_CHART = new Set(["dashboard:kpi-tax", "report:year-compare", "report:tax-burden", "report:reserve-trend", "report:loss-expiry", "report:industry-stats"]);
const TYPOLOGY_WIZARD = new Set(["ws/val:run", "ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done", "post/amend:resubmit", "admin/law:impact", "admin/form:migration", "admin/form:impact"]);
const TYPOLOGY_FORM = new Set(["ws/appr:request", "post/amend:unlock", "post/amend:version", "post/correction", "report:custom", "admin/cacc:delegate"]);
const TYPOLOGY_DETAIL = new Set(["ws/start:snapshot", "ws/info:fs", "ws/info:consistency", "ws/form:form3", "ws/form:preview", "ws/print:preview", "post/amend:diff", "admin/law:snapshots", "admin/form:by-set"]);
const LEAF_FORMATS = ["money", "bps", "date", "datetime", "biz", "corp", "tags", "status", "severity", "link", "boolean", "progress", "code", "email", "phone", "actions"];

function enrichLeafSpec(key, spec, locale = "ko") {
  const typology = spec.typology || leafTypology(key);
  return {
    ...spec,
    typology,
    rowKey: spec.rowKey || inferRowKey(key),
    update: spec.update || { method: "PATCH", path: "/api/tenants/{tenant}/leaf-records/{recordId}", fallback: "leaf-action" },
    description: spec.description || leafDescription(key, locale),
  };
}

function leafTypology(key) {
  if (TYPOLOGY_GRID_TREE.has(key)) return "grid-tree";
  if (TYPOLOGY_DASHBOARD.has(key)) return "dashboard";
  if (TYPOLOGY_CHART.has(key)) return "chart";
  if (TYPOLOGY_WIZARD.has(key)) return "wizard";
  if (TYPOLOGY_FORM.has(key)) return "form";
  if (TYPOLOGY_DETAIL.has(key)) return "detail";
  return "grid";
}

function inferRowKey(key) {
  if (key.includes("customer") || key === "admin/cust:list") return "customer_id";
  if (key.includes("by-pick") || key.includes("by-master") || key.includes("business-year")) return "by_id";
  if (key.includes("users")) return "login_id";
  if (key.includes("roles")) return "role_code";
  if (key.includes("menus")) return "menu_key";
  if (key.includes("law")) return "law_version_id";
  if (key.includes("form")) return "form_code";
  return "row_id";
}

function leafDescription(key, locale = "ko") {
  const typology = leafTypology(key);
  if (typology === "grid") return t(locale, "typology.grid.description");
  if (typology === "grid-tree") return t(locale, "typology.gridTree.description");
  if (typology === "dashboard") return t(locale, "typology.dashboard.description");
  if (typology === "wizard") return t(locale, "typology.wizard.description");
  if (typology === "form") return t(locale, "typology.form.description");
  if (typology === "chart") return t(locale, "typology.chart.description");
  return t(locale, "typology.detail.description");
}

function renderLeafTemplate(state) {
  const renderer = TYPOLOGY_RENDERERS[state.spec.typology] || renderTypologyGrid;
  return renderer(state);
}

function renderTypologyGrid(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="grid" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      ${renderLeafSummaryBlock(state, rows)}
      ${renderLeafTableBlock(state, rows, columns)}
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyGridTree(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology layout-tree-and-grid" data-typology="grid-tree" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <aside class="panel tree-panel">
        <div class="panel-head"><div><h2>${escapeHtml(t(locale, "common.category"))}</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
        ${renderLeafTree(state, rows)}
      </aside>
      <div class="grid-tree-main">
        ${renderLeafTableBlock(state, rows, columns)}
        ${renderLeafActionResult()}
      </div>
    </section>`;
}

function renderTypologyDashboard(state) {
  const rows = filterLeafRows(state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="dashboard" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="dashboard-grid">
        ${dashboardMetrics(state, rows).map(([label, value, tone]) => `
          <article class="metric dashboard-metric ${escapeHtml(tone || "")}">
            <span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong>
          </article>`).join("")}
      </section>
      <section class="dashboard-secondary">
        ${dashboardCards(state, rows).map((card) => `
          <article class="panel">
            <div class="panel-head"><div><h2>${escapeHtml(card.title)}</h2><p>${escapeHtml(card.caption)}</p></div></div>
            ${card.body}
          </article>`).join("")}
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyWizard(state) {
  const locale = state.env.locale;
  const steps = [
    t(locale, "typology.wizard.step.prepare"),
    t(locale, "typology.wizard.step.validate"),
    t(locale, "typology.wizard.step.execute"),
    t(locale, "typology.wizard.step.result"),
  ];
  const active = wizardActiveStep(state.key);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="wizard" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <ol class="wizard-stepper">
        ${steps.map((step, index) => `<li class="${index + 1 < active ? "done" : index + 1 === active ? "active" : ""}"><span>${index + 1}</span>${escapeHtml(step)}</li>`).join("")}
      </ol>
      <section class="panel wizard-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <button class="secondary-btn compact" type="button" data-step-edit data-row-id="${escapeHtml(firstRowId(state))}">${escapeHtml(t(locale, "typology.wizard.editStep"))}</button>
        </div>
        ${renderWizardBody(state, active)}
        <div class="wizard-nav">
          <button class="secondary-btn" type="button" data-wizard-prev ${active === 1 ? "disabled" : ""}>${escapeHtml(t(locale, "typology.wizard.previous"))}</button>
          <button class="primary-btn" type="button" data-wizard-next>${escapeHtml(active === steps.length ? t(locale, "typology.wizard.complete") : t(locale, "typology.wizard.next"))}</button>
        </div>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyForm(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = editableLeafColumns(state, row);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="form" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="grid two form-typology-body">
        <article class="panel">
          <div class="panel-head"><div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
          <form class="stack" data-leaf-form data-row-id="${escapeHtml(row.__rowId || "")}">
            ${columns.map((column) => renderEditField(column, row[column.key], locale)).join("")}
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.save"))}</button>
          </form>
        </article>
        <article class="panel form-preview">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.form.preview"))}</h2></div>
          ${renderObjectTable(row, leafColumns([row], state).slice(0, 6), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyChart(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="chart" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      ${renderLeafSummaryBlock(state, rows)}
      <section class="panel chart-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <div class="panel-head-actions">
            <select data-chart-range aria-label="${escapeHtml(t(locale, "typology.chart.range"))}"><option>3y</option><option selected>5y</option><option>10y</option></select>
            <button class="secondary-btn compact" type="button" data-chart-config-edit data-row-id="${escapeHtml(firstRowId(state))}">${escapeHtml(t(locale, "typology.chart.editConfig"))}</button>
          </div>
        </div>
        <div class="chart-area" data-chart-target>
          ${renderChartBars(rows, locale)}
        </div>
        ${renderLeafTableShell(state, rows.slice(0, 8), columns)}
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyDetail(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = leafColumns([row], state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="detail" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="panel detail-header">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "typology.detail.badge"))}</span>
            <h2>${escapeHtml(detailTitle(state, row))}</h2>
            <p>${escapeHtml(state.spec.description)}</p>
          </div>
          <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(row.__rowId || "")}">${escapeHtml(t(locale, "common.edit"))}</button>
        </div>
      </section>
      <section class="grid two detail-body">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.detail.basic"))}</h2></div>
          ${renderObjectTable(row, columns.slice(0, 8), state)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.detail.related"))}</h2></div>
          ${renderObjectTable(row, columns.slice(8, 16).length ? columns.slice(8, 16) : columns.slice(0, 4), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderLeafSummaryBlock(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const custom = rows.filter((row) => row.__source === "leaf_records").length;
  const locale = state.env.locale;
  return `
    <section class="panel leaf-summary" data-leaf-block="summary">
      <div class="panel-head">
        <div>
          <span class="badge info">${escapeHtml(state.spec.typology)}</span>
          <h2>${escapeHtml(state.meta.title || state.key)}</h2>
          <p>${escapeHtml(state.key)} · ${escapeHtml(state.spec.perm.module)}:${escapeHtml(state.spec.perm.function)}</p>
        </div>
      </div>
      ${metrics([
        [t(locale, "common.total"), money.format(rows.length)],
        [t(locale, "common.active"), money.format(active)],
        [t(locale, "common.custom"), money.format(custom)],
        [t(locale, "common.permission"), `${state.spec.perm.module}:${state.spec.perm.function}`],
      ])}
    </section>`;
}

function renderLeafTableBlock(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  return `
    <section class="panel leaf-table" data-leaf-block="table">
      <div class="panel-head">
        <div><h2>${escapeHtml(state.meta.title || t(locale, "common.list"))}</h2><p>${escapeHtml(t(locale, "leaf.count", { count: rows.length, description: state.spec.description }))}</p></div>
        <div class="panel-head-actions" data-leaf-block="toolbar">
          <div data-leaf-block="filters">
            ${renderLeafFilterControls(state)}
          </div>
          <button class="primary-btn compact" type="button" data-leaf-create="${escapeHtml(state.key)}">${escapeHtml(t(locale, "common.addPrefix"))}</button>
        </div>
      </div>
      ${renderLeafTableShell(state, rows, columns)}
    </section>`;
}

function renderLeafTableShell(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${columns.map((column) => `<th class="${escapeHtml(leafHeadClass(column))}">${escapeHtml(column.label)}</th>`).join("")}<th class="row-actions-th">${escapeHtml(t(locale, "common.actions"))}</th></tr></thead>
        <tbody data-leaf-table-body>${renderLeafTableRows(state, rows, columns)}</tbody>
      </table>
    </div>`;
}

function renderLeafFilterControls(state) {
  const locale = state.env.locale;
  return `
    <label class="inline-control">${escapeHtml(t(locale, "common.search"))} <input type="search" data-leaf-filter="q" value="${escapeHtml(state.query)}" placeholder="${escapeHtml(t(locale, "leaf.searchPlaceholder"))}" /></label>
    <label class="inline-control">${escapeHtml(t(locale, "field.status"))}
      <select data-leaf-filter="status">
        ${["ALL", "ACTIVE", "DRAFT", "IN_REVIEW", "APPROVED", "FILED", "SUSPENDED"].map((status) => `<option value="${status}" ${state.status === status ? "selected" : ""}>${escapeHtml(statusLabel(status, locale))}</option>`).join("")}
      </select>
    </label>
    <button class="secondary-btn compact" type="button" data-leaf-filter-reset>${escapeHtml(t(locale, "common.reset"))}</button>`;
}

function renderLeafTableRows(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  if (!rows.length) {
    return `<tr><td colspan="${columns.length + 1}"><div class="empty-state compact"><strong>${escapeHtml(t(locale, "typology.grid.emptyTitle"))}</strong><p class="empty">${escapeHtml(t(locale, "typology.grid.emptyDescription"))}</p></div></td></tr>`;
  }
  return rows.map((item) => `
    <tr data-leaf-row="${escapeHtml(item.__rowId)}">
      ${columns.map((column) => `<td class="${escapeHtml(leafCellClass(column))}" data-format="${escapeHtml(column.format)}">${formatLeafValue(item[column.key], column, item, state)}</td>`).join("")}
      <td class="row-actions" data-leaf-block="row-actions" data-format="actions">${renderLeafRowActions(state, item)}</td>
    </tr>`).join("");
}

function renderLeafRowActions(state, item) {
  const locale = state.env.locale;
  return `
    ${renderLeafPrimaryRowAction(state, item)}
    <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(item.__rowId)}" title="${escapeHtml(t(locale, "common.edit"))}">${escapeHtml(t(locale, "common.edit"))}</button>
    <button class="danger-btn compact" type="button" data-row-delete data-leaf-row-action="delete" data-row-id="${escapeHtml(item.__rowId)}" title="${escapeHtml(t(locale, "common.delete"))}">${escapeHtml(t(locale, "common.delete"))}</button>`;
}

function renderLeafPrimaryRowAction(state, item) {
  if (state.key === "ws/start:customer-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-customer" data-row-id="${escapeHtml(item.__rowId)}">${escapeHtml(t(state.env.locale, "route.ws.start.customerPick"))}</button>`;
  }
  if (state.key === "ws/start:by-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-by" data-row-id="${escapeHtml(item.__rowId)}">${escapeHtml(t(state.env.locale, "route.ws.start.byPick"))}</button>`;
  }
  return "";
}

function renderLeafActionResult() {
  return `<div class="leaf-action-result" aria-live="polite"></div>`;
}

function bindLeafTemplate(env, state) {
  state.env = env;
  if (env.outlet.__leafClickHandler) env.outlet.removeEventListener("click", env.outlet.__leafClickHandler);
  if (env.outlet.__leafInputHandler) env.outlet.removeEventListener("input", env.outlet.__leafInputHandler);
  if (env.outlet.__leafSubmitHandler) env.outlet.removeEventListener("submit", env.outlet.__leafSubmitHandler);
  env.outlet.__leafClickHandler = (event) => handleLeafClick(event, env, state);
  env.outlet.__leafInputHandler = (event) => handleLeafInput(event, env, state);
  env.outlet.__leafSubmitHandler = (event) => handleLeafSubmit(event, env, state);
  env.outlet.addEventListener("click", env.outlet.__leafClickHandler);
  env.outlet.addEventListener("input", env.outlet.__leafInputHandler);
  env.outlet.addEventListener("submit", env.outlet.__leafSubmitHandler);
}

async function handleLeafClick(event, env, state) {
  const reset = event.target.closest("[data-leaf-filter-reset]");
  if (reset) {
    state.query = "";
    state.status = "ALL";
    rerenderLeaf(env, state);
    return;
  }

  const create = event.target.closest("[data-leaf-create]");
  if (create) {
    await createLeafRow(env, state, create);
    return;
  }

  const close = event.target.closest("[data-edit-close]");
  if (close) {
    closeLeafModal(env);
    return;
  }

  const actionButton = event.target.closest("[data-leaf-row-action], [data-step-edit], [data-card-edit], [data-chart-config-edit]");
  if (!actionButton) return;
  const row = findLeafRow(state, actionButton.dataset.rowId) || state.rows[0] || newLeafRecordData(state);
  const action = actionButton.dataset.leafRowAction || (actionButton.dataset.stepEdit !== undefined ? "edit" : "edit");
  actionButton.disabled = true;
  try {
    if (action === "select-customer") {
      selectLeafCustomer(env, state, row);
      return;
    }
    if (action === "select-by") {
      await selectLeafBusinessYear(env, state, row);
      return;
    }
    if (action === "edit") {
      openEditModal(env, state, row);
      return;
    }
    if (action === "delete") {
      await deleteLeafRow(env, state, row);
      state.rows = state.rows.filter((item) => item.__rowId !== row.__rowId);
      setLeafActionMessage(t(env.locale, "modal.deleteSuccess"), false, env.locale);
      rerenderLeaf(env, state);
    }
  } catch (error) {
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    actionButton.disabled = false;
  }
}

function handleLeafInput(event, env, state) {
  if (!event.target.matches("[data-leaf-filter]")) return;
  state.query = env.outlet.querySelector('[data-leaf-filter="q"]')?.value || "";
  state.status = env.outlet.querySelector('[data-leaf-filter="status"]')?.value || "ALL";
  refreshLeafRows(env, state);
}

async function handleLeafSubmit(event, env, state) {
  const editForm = event.target.closest("[data-leaf-edit-form]");
  const leafForm = event.target.closest("[data-leaf-form]");
  if (!editForm && !leafForm) return;
  event.preventDefault();
  const form = editForm || leafForm;
  const row = findLeafRow(state, form.dataset.rowId) || state.rows[0] || normalizeLeafRow(newLeafRecordData(state), state.key, "leaf_records", 0);
  const message = form.querySelector("[data-edit-error]");
  const submit = form.querySelector('button[type="submit"]');
  if (submit) submit.disabled = true;
  if (message) message.textContent = "";
  try {
    const values = readLeafFormValues(form, row);
    const updated = await updateLeafRow(env, state, row, values);
    upsertLeafRow(state, updated);
    closeLeafModal(env);
    setLeafActionMessage(t(env.locale, "modal.saveSuccess"), false, env.locale);
    rerenderLeaf(env, state);
  } catch (error) {
    if (message) message.textContent = error.message;
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    if (submit) submit.disabled = false;
  }
}

async function createLeafRow(env, state, button) {
  button.disabled = true;
  try {
    const created = await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records`, {
      method: "POST",
      body: JSON.stringify({
        leaf_key: state.key,
        data: newLeafRecordData(state),
      }),
    });
    state.rows.unshift(normalizeLeafRow(created.row, state.key, "leaf_records", state.rows.length));
    setLeafActionMessage(t(env.locale, "leaf.addSuccess"), false, env.locale);
    rerenderLeaf(env, state);
  } catch (error) {
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    button.disabled = false;
  }
}

function openEditModal(env, state, row) {
  closeLeafModal(env);
  const columns = editableLeafColumns(state, row);
  const locale = env.locale;
  env.outlet.insertAdjacentHTML("beforeend", `
    <section class="leaf-modal-backdrop" data-leaf-modal>
      <form class="leaf-edit-modal" data-leaf-edit-form data-row-id="${escapeHtml(row.__rowId || "")}">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(locale, "modal.editTitle", { title: state.meta.title || state.key }))}</h2><p>${escapeHtml(row.__rowId || state.spec.rowKey || "-")}</p></div>
          <button class="secondary-btn compact" type="button" data-edit-close>${escapeHtml(t(locale, "common.cancel"))}</button>
        </div>
        <div class="form-grid">
          ${columns.map((column) => renderEditField(column, row[column.key], locale)).join("")}
        </div>
        <p class="edit-error" data-edit-error></p>
        <div class="button-row">
          <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.save"))}</button>
          <button class="secondary-btn" type="button" data-edit-close>${escapeHtml(t(locale, "common.cancel"))}</button>
        </div>
      </form>
    </section>`);
}

function closeLeafModal(env) {
  env.outlet.querySelector("[data-leaf-modal]")?.remove();
}

async function updateLeafRow(env, state, row, values = {}) {
  const updated = {
    ...row,
    ...values,
    status: values.status || row.status || nextLeafStatus(row.status),
    updated_at: today(),
  };
  if (row.__recordId) {
    const response = await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records/${encodeURIComponent(row.__recordId)}`, {
      method: "PATCH",
      body: JSON.stringify({ data: stripLeafInternalFields(updated) }),
    });
    return normalizeLeafRow(response.row, state.key, "leaf_records", 0);
  }
  await request(state.actionApi, {
    method: "POST",
    body: JSON.stringify({
      leaf_key: state.key,
      action: "update",
      row_id: row.__rowId,
      data: stripLeafInternalFields(updated),
    }),
  });
  return normalizeLeafRow(updated, state.key, row.__source || "api", 0);
}

async function deleteLeafRow(env, state, row) {
  if (row.__recordId) {
    await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records/${encodeURIComponent(row.__recordId)}`, { method: "DELETE" });
    return;
  }
  await request(state.actionApi, {
    method: "POST",
    body: JSON.stringify({
      leaf_key: state.key,
      action: "delete",
      row_id: row.__rowId,
    }),
  });
}

function refreshLeafRows(env, state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const tbody = env.outlet.querySelector("[data-leaf-table-body]");
  if (tbody) tbody.innerHTML = renderLeafTableRows(state, rows, columns);
  const tableHead = env.outlet.querySelector('[data-leaf-block="table"] .panel-head p');
  if (tableHead) tableHead.textContent = t(env.locale, "leaf.count", { count: rows.length, description: state.spec.description });
}

function rerenderLeaf(env, state) {
  env.outlet.innerHTML = renderLeafTemplate(state);
  bindLeafTemplate(env, state);
}

function selectLeafCustomer(env, state, row) {
  const customerId = row.customer_id || row.id;
  if (!customerId) {
    throw new Error(t(env.locale, "context.missingCustomer"));
  }
  env.setContext({
    customerId,
    customerName: row.customer_name || row.name || row.customer_code || env.context.customerName,
  });
  setLeafActionMessage(t(env.locale, "context.customerSelected"), false, env.locale);
  env.navigate("ws/start:by-pick", { customerId });
}

async function selectLeafBusinessYear(env, state, row) {
  const byId = row.by_id || row.business_year_id || row.id;
  if (!byId || !row.customer_id) {
    throw new Error(t(env.locale, "context.missingBusinessYear"));
  }
  const by = { ...row, by_id: byId };
  const customer = await customerForBusinessYear(env, by);
  await refreshContextFromBy(env, by, customer);
  setLeafActionMessage(t(env.locale, "context.businessYearSelected"), false, env.locale);
  env.navigate("ws/info:fs", { byId: by.by_id, customerId: by.customer_id });
}

async function customerForBusinessYear(env, by) {
  if (String(env.context?.customerId || "") === String(by.customer_id || "")) {
    return {
      customer_id: by.customer_id,
      customer_name: env.context.customerName,
    };
  }
  const customers = await request(`${routeRoot(env)}/customers`).catch(() => []);
  return asArray(customers).find((item) => String(item.customer_id) === String(by.customer_id)) || {
    customer_id: by.customer_id,
    customer_name: by.customer_name || String(by.customer_id),
  };
}

function normalizeLeafRows(payload, key, source) {
  return extractLeafRows(payload).map((row, index) => normalizeLeafRow(row, key, source, index));
}

function normalizeLeafRow(row, key, source, index) {
  const object = row && typeof row === "object" && !Array.isArray(row) ? { ...row } : { value: row };
  const recordId = object.record_id || null;
  const rowId = recordId ? `record-${recordId}` : String(object.row_id || object.id || object[`${key.split(":")[0].split("/").pop()}_id`] || object.customer_id || object.by_id || object.login_id || object.menu_key || `api-${index + 1}`);
  return {
    ...object,
    __recordId: recordId,
    __rowId: rowId,
    __source: source,
  };
}

function extractLeafRows(payload) {
  if (Array.isArray(payload)) return payload;
  if (!payload || typeof payload !== "object") return payload === null || payload === undefined ? [] : [{ value: payload }];
  if (Array.isArray(payload.rows)) return payload.rows;
  const preferred = ["items", "customers", "business_years", "users", "roles", "permissions", "events", "issues", "rules", "attachments", "forms", "versions", "fields", "relationships", "rates", "limits", "logs", "histories", "reports", "notifications", "data"];
  for (const key of preferred) {
    if (Array.isArray(payload[key])) return payload[key];
  }
  const firstArray = Object.values(payload).find((value) => Array.isArray(value));
  if (firstArray) return firstArray;
  return [payload];
}

function leafColumns(rows, state = null) {
  const locale = state?.env?.locale || "ko";
  if (state?.spec?.columns?.length) {
    return state.spec.columns.map((column) => ({ ...column, label: column.labelKey ? t(locale, column.labelKey) : column.labels?.[locale] || column.label || fieldLabel(column.key, locale), format: column.format || inferColumnFormat(column.key) }));
  }
  const keys = [];
  rows.forEach((row) => {
    Object.keys(row || {}).forEach((key) => {
      if (!key.startsWith("__") && !["_source", "metadata", "snapshot_data", "payload"].includes(key) && !keys.includes(key)) keys.push(key);
    });
  });
  const selected = prioritizeLeafKeys(keys).slice(0, 7);
  return (selected.length ? selected : ["row_id", "title", "status"]).map((key) => ({
    key,
    label: leafColumnLabel(key, locale),
    format: inferColumnFormat(key),
  }));
}

function prioritizeLeafKeys(keys) {
  const preferred = ["row_id", "record_id", "tenant_code", "customer_code", "customer_name", "login_id", "role_code", "menu_key", "title", "name", "status", "severity", "year_label", "amount", "tax_due", "progress", "biz_reg_no", "corp_reg_no", "email", "phone", "created_at"];
  return [...preferred.filter((key) => keys.includes(key)), ...keys.filter((key) => !preferred.includes(key))];
}

function leafColumnLabel(key, locale = "ko") {
  return fieldLabel(key, locale);
}

function inferColumnFormat(key) {
  const normalized = key.toLowerCase();
  if (normalized === "actions") return "actions";
  if (normalized.includes("email")) return "email";
  if (normalized.includes("phone") || normalized.includes("mobile")) return "phone";
  if (normalized.includes("biz_reg")) return "biz";
  if (normalized.includes("corp_reg")) return "corp";
  if (normalized.includes("code") || normalized.endsWith("_id") || normalized === "id" || normalized.includes("key")) return "code";
  if (normalized.includes("bps") || normalized.includes("rate")) return "bps";
  if (normalized.includes("amount") || normalized.includes("tax") || normalized.includes("income") || normalized.includes("revenue") || normalized.includes("balance") || normalized.includes("refund")) return "money";
  if (normalized.includes("created_at") || normalized.includes("updated_at") || normalized.includes("acted_at") || normalized.includes("timestamp")) return "datetime";
  if (normalized.endsWith("_date") || normalized.includes("valid_from") || normalized.includes("valid_to") || normalized.includes("contract_")) return "date";
  if (normalized.includes("scopes") || normalized.includes("roles") || normalized.includes("tags")) return "tags";
  if (normalized === "status" || normalized === "state") return "status";
  if (normalized === "severity") return "severity";
  if (normalized.includes("url") || normalized.includes("link")) return "link";
  if (normalized.startsWith("is_") || normalized.includes("locked") || normalized.includes("active") || normalized.includes("valid") || normalized.includes("balanced")) return "boolean";
  if (normalized.includes("progress") || normalized.includes("percent")) return "progress";
  return "text";
}

function filterLeafRows(state) {
  const query = state.query.trim().toLowerCase();
  return state.rows.filter((row) => {
    const status = String(row.status || row.state || "").toUpperCase();
    const matchesStatus = state.status === "ALL" || status === state.status;
    const matchesQuery = !query || Object.values(row).some((value) => String(value ?? "").toLowerCase().includes(query));
    return matchesStatus && matchesQuery;
  });
}

function formatLeafValue(value, column = {}, row = {}, state = null) {
  if (value === null || value === undefined || value === "") return "-";
  const format = column.format || inferColumnFormat(column.key || "");
  if (format === "money") return `<span class="num">${escapeHtml(money.format(Number(value) || 0))}</span>`;
  if (format === "bps") return `${((Number(value) || 0) / 100).toFixed(2)}%`;
  if (format === "date") return escapeHtml(formatDate(value));
  if (format === "datetime") return escapeHtml(formatDateTime(value));
  if (format === "biz") return `<span class="code-cell">${escapeHtml(formatBizNo(value))}</span>`;
  if (format === "corp") return `<span class="code-cell">${escapeHtml(formatCorpNo(value))}</span>`;
  if (format === "tags") return renderTags(value);
  if (format === "status") return pill(value, state?.env?.locale || "ko");
  if (format === "severity") return `<span class="badge ${escapeHtml(severityClass(value))}">${escapeHtml(statusLabel(value, state?.env?.locale || "ko"))}</span>`;
  if (format === "link") return renderLeafLink(value, row, state);
  if (format === "boolean") return `<span class="boolean-mark ${value ? "yes" : "no"}">${value ? "Y" : "N"}</span>`;
  if (format === "progress") return renderProgress(value);
  if (format === "code") return `<span class="code-cell">${escapeHtml(value)}</span>`;
  if (format === "email") return escapeHtml(maskEmail(value));
  if (format === "phone") return escapeHtml(maskPhone(value));
  if (Array.isArray(value)) return renderTags(value);
  if (typeof value === "object") return escapeHtml(compactObjectLabel(value));
  return escapeHtml(value);
}

function compactObjectLabel(value) {
  const keys = Object.keys(value || {});
  if (!keys.length) return "{}";
  return keys.slice(0, 3).map((key) => `${key}:${value[key]}`).join(" · ");
}

function newLeafRecordData(state) {
  const locale = state.env?.locale || "ko";
  return {
    title: t(locale, "leaf.newItem", { title: state.meta.title || state.key }),
    status: "DRAFT",
    leaf_key: state.key,
    created_at: today(),
    owner: "UI",
  };
}

function stripLeafInternalFields(row) {
  return Object.fromEntries(Object.entries(row).filter(([key]) => !key.startsWith("__")));
}

function nextLeafStatus(status) {
  const value = String(status || "DRAFT").toUpperCase();
  if (value === "DRAFT") return "ACTIVE";
  if (value === "ACTIVE") return "IN_REVIEW";
  if (value === "IN_REVIEW") return "APPROVED";
  return "DRAFT";
}

function setLeafActionMessage(message, error = false, locale = "ko") {
  const result = document.querySelector(".leaf-action-result");
  if (result) result.innerHTML = `<strong>${escapeHtml(error ? t(locale, "leaf.actionFailed") : t(locale, "leaf.actionDone"))}</strong><p class="empty">${escapeHtml(message)}</p>`;
}

function leafHeadClass(column) {
  return ["money", "bps", "progress"].includes(column.format) ? "align-right" : "";
}

function leafCellClass(column) {
  return ["money", "bps", "progress"].includes(column.format) ? "align-right" : "";
}

function editableLeafColumns(state, row = {}) {
  const blocked = new Set(["record_id", "row_id", "tenant_code", "leaf_key", "_source", state.spec.rowKey]);
  const columns = leafColumns([row], state).filter((column) => {
    const value = row[column.key];
    return !blocked.has(column.key)
      && column.format !== "actions"
      && value !== undefined
      && (value === null || typeof value !== "object" || Array.isArray(value));
  });
  if (columns.length) return columns.slice(0, 8);
  return [
    { key: "title", label: fieldLabel("title", state.env?.locale || "ko"), format: "text" },
    { key: "status", label: fieldLabel("status", state.env?.locale || "ko"), format: "status" },
  ];
}

function renderEditField(column, value, locale = "ko") {
  const inputType = editInputType(column.format);
  if (column.format === "boolean") {
    return `<label class="checkbox-field"><span>${escapeHtml(column.label)}</span><input name="${escapeHtml(column.key)}" type="checkbox" ${value ? "checked" : ""} /></label>`;
  }
  if (column.format === "tags") {
    return `<label>${escapeHtml(column.label)}<input name="${escapeHtml(column.key)}" value="${escapeHtml(asArray(value).join(", "))}" placeholder="${escapeHtml(t(locale, "validation.commaSeparated"))}" /></label>`;
  }
  if (String(value || "").length > 80) {
    return `<label>${escapeHtml(column.label)}<textarea name="${escapeHtml(column.key)}">${escapeHtml(value || "")}</textarea></label>`;
  }
  return `<label>${escapeHtml(column.label)}<input name="${escapeHtml(column.key)}" type="${inputType}" value="${escapeHtml(value ?? "")}" /></label>`;
}

function editInputType(format) {
  if (format === "date") return "date";
  if (format === "datetime") return "datetime-local";
  if (format === "money" || format === "bps" || format === "progress") return "number";
  if (format === "email") return "email";
  if (format === "phone") return "tel";
  return "text";
}

function readLeafFormValues(form, row) {
  const values = {};
  form.querySelectorAll("[name]").forEach((control) => {
    const key = control.name;
    const current = row[key];
    if (control.type === "checkbox") {
      values[key] = control.checked;
    } else if (Array.isArray(current)) {
      values[key] = control.value.split(",").map((item) => item.trim()).filter(Boolean);
    } else if (typeof current === "number") {
      values[key] = Number(control.value || 0);
    } else {
      values[key] = control.value;
    }
  });
  return values;
}

function findLeafRow(state, rowId) {
  if (!rowId) return null;
  return state.rows.find((row) => String(row.__rowId) === String(rowId)) || null;
}

function upsertLeafRow(state, row) {
  const index = state.rows.findIndex((item) => item.__rowId === row.__rowId);
  if (index >= 0) {
    state.rows[index] = row;
  } else {
    state.rows.unshift(row);
  }
}

function firstRowId(state) {
  return state.rows[0]?.__rowId || "";
}

function renderLeafTree(state, rows) {
  const groups = new Map();
  const locale = state.env.locale;
  rows.forEach((row) => {
    const raw = row.parent_key || row.menu_key || row.group_code || row.category || row.status || t(locale, "status.all");
    const label = String(raw).split(/[/:.]/)[0] || t(locale, "status.all");
    groups.set(label, (groups.get(label) || 0) + 1);
  });
  if (!groups.size) return `<p class="empty">${escapeHtml(t(locale, "leaf.emptyCategories"))}</p>`;
  return `<ul class="leaf-tree">${[...groups.entries()].map(([label, count]) => `<li><button type="button" class="secondary-btn compact" data-tree-node="${escapeHtml(label)}">${escapeHtml(label)} <span>${money.format(count)}</span></button></li>`).join("")}</ul>`;
}

function dashboardMetrics(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const warnings = rows.filter((row) => ["WARN", "ERROR"].includes(String(row.severity || "").toUpperCase())).length;
  const locale = state.env.locale;
  return [
    [t(locale, "common.total"), money.format(rows.length), "info"],
    [t(locale, "status.active"), money.format(active), "ok"],
    [t(locale, "typology.dashboard.waiting"), money.format(rows.filter((row) => String(row.status || "").includes("PENDING")).length), "warn"],
    [t(locale, "typology.dashboard.warning"), money.format(warnings), "warn"],
    [t(locale, "common.custom"), money.format(rows.filter((row) => row.__source === "leaf_records").length), "info"],
  ];
}

function dashboardCards(state, rows) {
  const sample = rows.slice(0, 5);
  const locale = state.env.locale;
  const list = sample.length
    ? `<ul class="compact-list">${sample.map((row) => `<li><strong>${escapeHtml(detailTitle(state, row))}</strong><span>${escapeHtml(row.status ? statusLabel(row.status, locale) : row.severity || row.created_at || "-")}</span></li>`).join("")}</ul>`
    : `<p class="empty">${escapeHtml(t(locale, "leaf.emptyItems"))}</p>`;
  return [
    { title: t(locale, "typology.dashboard.overview"), caption: state.key, body: list },
    { title: t(locale, "typology.dashboard.recent"), caption: t(locale, "leaf.count", { count: sample.length, description: "" }).trim(), body: list },
    { title: t(locale, "typology.dashboard.guide"), caption: state.spec.typology, body: `<p class="empty">${escapeHtml(state.spec.description)}</p>` },
  ];
}

function wizardActiveStep(key) {
  if (key.endsWith(":generate")) return 2;
  if (key.endsWith(":submit") || key.endsWith(":resubmit")) return 3;
  if (key.endsWith(":done")) return 4;
  return 1;
}

function renderWizardBody(state, active) {
  const rows = state.rows.slice(0, 4);
  const locale = state.env.locale;
  return `
    <div class="wizard-body" data-wizard-step="${active}">
      ${metrics([
        [t(locale, "typology.wizard.next"), `${active}/4`],
        [t(locale, "common.item"), money.format(state.rows.length)],
        [t(locale, "field.status"), statusLabel(state.rows[0]?.status || "READY", locale)],
        [t(locale, "common.category"), state.spec.typology],
      ])}
      <div class="wizard-checklist">
        ${(rows.length ? rows : [newLeafRecordData(state)]).map((row, index) => `
          <article class="card">
            <span class="badge ${index + 1 <= active ? "ok" : "info"}">${index + 1}</span>
            <strong>${escapeHtml(detailTitle(state, row))}</strong>
            <p>${escapeHtml(row.status || row.state || state.spec.description)}</p>
          </article>`).join("")}
      </div>
    </div>`;
}

function renderChartBars(rows, locale = "ko") {
  const points = rows.slice(0, 8).map((row) => ({ label: chartLabel(row), value: chartValue(row) }));
  const max = Math.max(...points.map((point) => point.value), 1);
  if (!points.length) return `<p class="empty">${escapeHtml(t(locale, "leaf.noChartData"))}</p>`;
  return `<div class="chart-bars">${points.map((point) => `
    <div class="chart-bar-row">
      <span>${escapeHtml(point.label)}</span>
      <div class="chart-bar-track"><i style="width:${Math.max(4, Math.round(point.value / max * 100))}%"></i></div>
      <strong>${escapeHtml(money.format(point.value))}</strong>
    </div>`).join("")}</div>`;
}

function chartLabel(row) {
  return String(row.customer_name || row.report_name || row.year_label || row.item_name || row.title || row.row_id || row.__rowId || "-");
}

function chartValue(row) {
  const entry = Object.entries(row).find(([key, value]) => typeof value === "number" && !key.endsWith("_id"));
  return Math.max(0, Number(entry?.[1] || 0));
}

function detailTitle(state, row) {
  return row.customer_name || row.report_name || row.form_name || row.title || row.name || row.menu_key || row.login_id || row.role_code || row.__rowId || state.meta.title || state.key;
}

function renderObjectTable(object, columns, state) {
  const locale = state.env.locale;
  if (!columns.length) return `<p class="empty">${escapeHtml(t(locale, "leaf.emptyFields"))}</p>`;
  return table([t(locale, "common.item"), t(locale, "common.value")], columns.map((column) => row([
    escapeHtml(column.label),
    formatLeafValue(object[column.key], column, object, state),
  ])));
}

function formatDate(value) {
  const text = String(value || "");
  return text.includes("T") ? text.slice(0, 10) : text.slice(0, 10) || "-";
}

function formatDateTime(value) {
  const text = String(value || "");
  if (!text) return "-";
  return text.replace("T", " ").slice(0, 16);
}

function formatBizNo(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length !== 10) return String(value || "-");
  return `${digits.slice(0, 3)}-${digits.slice(3, 5)}-${digits.slice(5)}`;
}

function formatCorpNo(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length !== 13) return String(value || "-");
  return `${digits.slice(0, 6)}-${digits.slice(6)}`;
}

function renderTags(value) {
  const tags = Array.isArray(value) ? value : String(value || "").split(",").map((item) => item.trim()).filter(Boolean);
  if (!tags.length) return "-";
  return `<span class="tag-list">${tags.map((tag) => `<span class="tag-chip">${escapeHtml(typeof tag === "object" ? compactObjectLabel(tag) : tag)}</span>`).join("")}</span>`;
}

function severityClass(value) {
  const severity = String(value || "").toUpperCase();
  if (severity === "ERROR" || severity === "CRITICAL") return "danger";
  if (severity === "WARN" || severity === "WARNING") return "warn";
  return "info";
}

function renderLeafLink(value, row, state) {
  const href = String(value || "").startsWith("http") || String(value || "").startsWith("#") ? String(value) : keyToHash(String(value || state?.key || "dashboard:overview"));
  return `<a class="leaf-link" href="${escapeHtml(href)}">${escapeHtml(row.title || row.name || value)}</a>`;
}

function renderProgress(value) {
  const progress = Math.max(0, Math.min(100, Number(value) || 0));
  return `<div class="bar-track progress-cell"><span style="width:${progress}%"></span></div><span class="progress-label">${progress}%</span>`;
}

function maskEmail(value) {
  const text = String(value || "");
  const [name, domain] = text.split("@");
  if (!domain) return text;
  return `${name.slice(0, 2)}***@${domain}`;
}

function maskPhone(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length < 7) return String(value || "");
  return `${digits.slice(0, 3)}-****-${digits.slice(-4)}`;
}

function leafGate(env, key, spec) {
  const locale = env.locale || "ko";
  if (spec.requires.includes("work-context") && !hasWorkContext(env.context)) {
    return {
      kind: "ctx",
      title: t(locale, "context.requiredTitle"),
      message: t(locale, "context.requiredMessage"),
      action: "work-start",
    };
  }
  if (!canAccessLeaf(env, spec.perm)) {
    return {
      kind: "perm",
      title: t(locale, "context.requiredTitle"),
      message: `${spec.perm.module}:${spec.perm.function} ${t(locale, "common.permission")}`,
    };
  }
  const flag = spec.featureFlag || env.routeMeta?.feature_flag || null;
  if (flag && !isFeatureEnabled(env, flag)) {
    return {
      kind: "flag",
      title: t(locale, "menu.unavailable"),
      message: `${flag} ${t(locale, "menu.unavailable")}`,
    };
  }
  return null;
}

function canAccessLeaf(env, perm) {
  const permissions = env.auth?.permissions;
  if (!Array.isArray(permissions)) return true;
  return permissions.some((item) => {
    if (item === "*") return true;
    if (typeof item === "string") return item === `${perm.module}:${perm.function}` || item === `${perm.module}:*`;
    return item.module === perm.module && (item.function === perm.function || item.function === "*");
  });
}

function isFeatureEnabled(env, flag) {
  const flags = env.auth?.featureFlags || env.auth?.feature_flags || {};
  if (Array.isArray(flags)) return flags.includes(flag);
  return flags[flag] !== false;
}

function renderEmptyState(key, gate, meta, spec, primaryApi = "", actionApi = "", locale = "ko") {
  return `
    <section class="panel empty-state" data-leaf-key="${escapeHtml(key)}" data-empty-kind="${escapeHtml(gate.kind)}" data-primary-api="${escapeHtml(primaryApi)}" data-action-api="${escapeHtml(actionApi)}">
      <div class="panel-head">
        <div>
          <span class="badge warn">${escapeHtml(t(locale, "grid.emptyTitle"))}</span>
          <h2>${escapeHtml(meta.title || key)}</h2>
          <p>${escapeHtml(key)} / ${escapeHtml(spec.perm.module)}:${escapeHtml(spec.perm.function)}</p>
        </div>
      </div>
      <p class="empty">${escapeHtml(gate.message)}</p>
      ${gate.action === "work-start" ? `<button id="goStart" class="primary-btn" type="button">${escapeHtml(t(locale, "context.pickCustomerYear"))}</button>` : ""}
      ${gate.action === "retry" ? `<button id="retryLeaf" class="primary-btn" type="button">${escapeHtml(t(locale, "common.retry"))}</button>` : ""}
    </section>`;
}

function bindEmptyStateActions(env, gate) {
  if (gate.action === "work-start") {
    document.getElementById("goStart")?.addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  }
  if (gate.action === "retry") {
    document.getElementById("retryLeaf")?.addEventListener("click", () => env.navigate(env.routeKey || env.leafKey || env.key));
  }
}

function apiOptions(api, key, primaryApi, env) {
  if (api.method === "GET") return {};
  return {
    method: api.method,
    body: JSON.stringify({
      leaf_key: key,
      primary_api: primaryApi,
      tenant_code: tenantCode(env),
      business_year_id: env.context?.byId || 1,
      by_id: env.context?.byId || 1,
      customer_id: env.context?.customerId || 1,
      form_code: "FORM3",
      to_version_id: 1,
      law_version_id: 1,
      include_locked: false,
      actor: env.auth?.user?.login_id || "ui",
    }),
  };
}

function resolveApiPath(template, env) {
  const replacements = {
    tenant: tenantCode(env),
    byId: env.context?.byId || 1,
    customerId: env.context?.customerId || 1,
    formVersionId: env.context?.formVersionId || 1,
    efilingId: env.context?.efilingId || 1,
  };
  return template.replace(/\{(\w+)\}/g, (_, key) => encodeURIComponent(replacements[key] ?? ""));
}

function cssEscape(value) {
  if (window.CSS?.escape) return CSS.escape(value);
  return String(value).replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

export function routeMeta(key, locale = "ko") {
  const meta = routes[key] || routes.dashboard;
  return localizeRouteMeta({ group: meta.group, groupKey: meta.groupKey, title: meta.title, titleKey: meta.titleKey, layout: meta.layout, delegate: meta.delegate, s1: meta.s1 }, locale);
}

export async function refreshHealth(badge, text, locale = "ko") {
  try {
    await request("/health");
    badge.className = "health-badge ok";
    text.textContent = t(locale, "health.ok");
  } catch {
    badge.className = "health-badge error";
    text.textContent = t(locale, "health.error");
  }
}

export async function renderScreen(env) {
  const meta = routes[env.key] || routes.dashboard;
  const displayMeta = localizeRouteMeta({ ...meta, ...(env.routeMeta || {}) }, env.locale);
  const showFlowChrome = shouldShowFlowChrome(env.key, meta);
  if (showFlowChrome) {
    renderLawBanner(env.lawBanner, env.context, env.locale);
  } else {
    hideLawBanner(env.lawBanner);
  }
  const screen = screenByLeaf[env.key] || screenByDelegate[meta.delegate] || renderDashboard;
  if (screen !== renderDashboard) {
    stopDashboardRealtime();
  }
  const leafEnv = {
    ...env,
    key: meta.delegate,
    routeKey: env.key,
    routeMeta: displayMeta,
    leafKey: meta.s1 ? env.key : null,
    leafSuffix: meta.s1 ? leafSuffix(env.key) : null,
    leafTitle: displayMeta.title,
  };
  await screen(leafEnv);
  if (meta.s1) {
    prependLeafFocus(env.outlet, env.key, displayMeta, env.locale);
  }
  if (showFlowChrome) {
    await appendNextStepCard(env.outlet, leafEnv);
  }
}

function shouldShowFlowChrome(key, meta) {
  return meta.layout === "workspace"
    || key.startsWith("post/amend:")
    || key.startsWith("admin/law:")
    || key.startsWith("admin/form:");
}

function hideLawBanner(container) {
  container.classList.add("hidden");
  container.innerHTML = "";
}

function prependLeafFocus(outlet, key, meta, locale = "ko") {
  const section = document.createElement("section");
  section.className = "leaf-focus leaf-watermark";
  section.dataset.leafKey = key;
  section.dataset.leafDelegate = meta.delegate;
  const siblings = siblingLeafRoutes(meta.delegate, key);
  section.innerHTML = `
    <div>
      <div class="leaf-watermark-head">
        <span class="badge info">${escapeHtml(t(locale, "leaf.badge"))}</span>
        <strong>${escapeHtml(meta.title)}</strong>
        <span class="leaf-key">${escapeHtml(key)}</span>
      </div>
      <p>${leafFocusText(locale, key, meta.delegate)}</p>
      <div class="leaf-subnav" aria-label="${escapeHtml(t(locale, "leaf.siblingNavigation"))}">
        ${siblings.map(([siblingKey, siblingMeta]) => `
          <a class="${siblingKey === key ? "active" : ""}" href="${escapeHtml(keyToHash(siblingKey))}" data-leaf-nav="${escapeHtml(siblingKey)}">
            ${escapeHtml(localizeRouteMeta(siblingMeta, locale).title)}
          </a>`).join("")}
      </div>
    </div>
  `;
  outlet.prepend(section);
}

function siblingLeafRoutes(delegate, activeKey) {
  const activePrefix = activeKey.includes(":") ? activeKey.split(":")[0] : activeKey;
  return Object.entries(leafRoutes).filter(([key, meta]) => {
    const prefix = key.includes(":") ? key.split(":")[0] : key;
    return meta.delegate === delegate && prefix === activePrefix;
  });
}

function leafSuffix(key) {
  if (key.includes(":")) return key.split(":").slice(1).join(":");
  return key.split("/").pop();
}

function keyToHash(key) {
  if (key.includes(":")) {
    const [scope, suffix] = key.split(":");
    if (scope === "dashboard") return `#/dashboard/${suffix}`;
    if (scope.startsWith("ws/")) return `#/workspace/${scope}/${suffix}`;
    if (scope.startsWith("post/")) return `#/post/${scope.slice("post/".length)}/${suffix}`;
    if (scope === "report") return `#/report/${suffix}`;
    if (scope.startsWith("admin/")) return `#/admin/${scope.slice("admin/".length)}/${suffix}`;
  }
  if (key === "post/correction") return "#/post/correction";
  return `#/${key}`;
}

function tenantCode(env) {
  return env.auth?.user?.tenant_code || "demo";
}

function routeRoot(env) {
  return `/api/tenants/${tenantCode(env)}`;
}

function workRoot(env) {
  return `${routeRoot(env)}/business-years/${env.context.byId}`;
}

function requireWorkContext(env) {
  if (hasWorkContext(env.context)) return true;
  env.outlet.innerHTML = `
    <section class="panel empty-state work-context-empty" data-leaf-key="${escapeHtml(env.routeKey || env.leafKey || env.key)}">
      <div class="panel-head"><h2>${escapeHtml(t(env.locale, "context.requiredTitle"))}</h2></div>
      <p class="empty">${escapeHtml(t(env.locale, "context.requiredMessage"))}</p>
      <button id="goStart" class="primary-btn" type="button">${escapeHtml(t(env.locale, "context.pickCustomerYear"))}</button>
    </section>`;
  document.getElementById("goStart").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  return false;
}

function renderLawBanner(container, context, locale = "ko") {
  if (!hasWorkContext(context)) {
    container.classList.remove("hidden");
    container.classList.add("empty");
    container.innerHTML = `
      <div>
        <span>${escapeHtml(t(locale, "common.workflow"))}</span>
        <strong>${escapeHtml(t(locale, "context.select"))}</strong>
      </div>
      <button class="secondary-btn compact" type="button" data-flow-start>${escapeHtml(t(locale, "common.startWork"))}</button>`;
    container.querySelector("[data-flow-start]")?.addEventListener("click", () => {
      window.location.hash = "#/workspace/ws/start/customer-pick";
    });
    return;
  }
  const snapshot = context.snapshot || {};
  const data = snapshot.snapshot_data || {};
  container.classList.remove("hidden");
  container.classList.remove("empty");
  container.innerHTML = `
    <div><span>${escapeHtml(t(locale, "field.customerName"))}</span><strong>${escapeHtml(context.customerName || "-")}</strong></div>
    <div><span>${escapeHtml(t(locale, "field.yearLabel"))}</span><strong>${escapeHtml(context.fy || "-")}</strong></div>
    <div><span>${escapeHtml(t(locale, "nav.admin.law"))}</span><strong>${escapeHtml(lawLabel(data.law_version?.version_code || snapshot.law_version_id || "-"))}</strong></div>
    <div><span>${escapeHtml(t(locale, "nav.admin.forms"))}</span><strong>${escapeHtml(lawLabel(data.form?.version_no || data.form_version || snapshot.form_version_id || "-"))}</strong></div>
  `;
}

async function appendNextStepCard(outlet, env) {
  const key = env.routeKey || env.leafKey || env.key;
  if (!hasWorkContext(env.context)) {
    outlet.insertAdjacentHTML("beforeend", `
      <section class="flow-next-card" data-flow-card="${escapeHtml(key)}">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(env.locale, "common.nextStep"))}</h2><p class="empty">${escapeHtml(t(env.locale, "context.select"))}</p></div>
          <button class="primary-btn" type="button" data-next-leaf="ws/start:customer-pick">${escapeHtml(t(env.locale, "common.startWork"))}</button>
        </div>
      </section>`);
    bindNextStepNavigation(outlet, env);
    return;
  }
  let progress;
  try {
    progress = await request(`${workRoot(env)}/progress`);
  } catch {
    progress = { status: env.context.status || "DRAFT", next_leaf: "ws/info:fs", progress: env.context.progress || 0, recommendations: [] };
  }
  const next = progress.recommendations?.[0] || { leaf_key: progress.next_leaf || "ws/info:fs", label: t(env.locale, "common.nextStep"), enabled: true };
  outlet.insertAdjacentHTML("beforeend", `
    <section class="flow-next-card" data-flow-card="${escapeHtml(key)}" data-progress-api="${escapeHtml(`${workRoot(env)}/progress`)}">
      <div class="panel-head">
        <div>
          <span class="badge ok">${escapeHtml(t(env.locale, "common.workflow"))}</span>
          <h2>${escapeHtml(t(env.locale, "common.nextStep"))}</h2>
          <p>${escapeHtml(statusLabel(progress.status || env.context.status || "DRAFT", env.locale))} / ${escapeHtml(t(env.locale, "field.progress"))} ${escapeHtml(progress.progress ?? env.context.progress ?? 0)}%</p>
        </div>
        <button class="primary-btn" type="button" data-next-leaf="${escapeHtml(next.leaf_key)}" ${next.enabled === false ? "disabled" : ""}>${escapeHtml(next.label || t(env.locale, "common.nextStep"))}</button>
      </div>
    </section>`);
  bindNextStepNavigation(outlet, env);
}

function bindNextStepNavigation(outlet, env) {
  outlet.querySelectorAll("[data-next-leaf]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.nextLeaf));
  });
}

function lawLabel(value) {
  if (!value || value === "-") return "-";
  return String(value).replaceAll("_", " ");
}

function metrics(items) {
  return `<div class="grid four">${items.map(([label, value]) => `<article class="metric"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></article>`).join("")}</div>`;
}

function table(headers, rows, empty = t(currentDocumentLocale(), "grid.empty")) {
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${headers.map((head) => `<th>${escapeHtml(head)}</th>`).join("")}</tr></thead>
        <tbody>${rows.length ? rows.join("") : `<tr><td colspan="${headers.length}">${escapeHtml(empty)}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function currentDocumentLocale() {
  return typeof document === "undefined" ? "ko" : document.documentElement.lang;
}

function row(cells) {
  return `<tr>${cells.map((cell) => `<td>${cell}</td>`).join("")}</tr>`;
}

function pill(status, locale = "ko") {
  return `<span class="status-pill ${statusClass(status)}">${escapeHtml(statusLabel(status, locale))}</span>`;
}

function renderSnapshotSummary(snapshot, locale = currentDocumentLocale()) {
  const data = snapshot?.snapshot_data || {};
  const law = data.law || data.law_version || {};
  const form = data.form || {};
  return table([t(locale, "common.item"), t(locale, "common.value")], [
    row(["Snapshot ID", escapeHtml(snapshot?.snapshot_id || "-")]),
    row([escapeHtml(t(locale, "nav.admin.law")), escapeHtml(law.version_code || snapshot?.law_version_id || "-")]),
    row([escapeHtml(t(locale, "nav.admin.forms")), escapeHtml(form.version_no || form.form_version || snapshot?.form_version_ids || "-")]),
    row([escapeHtml(t(locale, "status.locked")), snapshot?.locked ? "Y" : "N"]),
  ]);
}

function renderTaxDataValidationSummary(validation) {
  return table(["항목", "값"], [
    row(["차변 합계", money.format(validation.debit_total || 0)]),
    row(["대변 합계", money.format(validation.credit_total || 0)]),
    row(["차대 일치", validation.balanced ? "Y" : "N"]),
    row(["미매핑 계정", money.format(validation.unresolved_mapping_count || 0)]),
    row(["배치 오류", money.format(validation.batch_error_count || 0)]),
  ]);
}

function renderValidationOverview(taxData, efile) {
  return `
    ${metrics([
      ["차대 일치", taxData.balanced ? "Y" : "N"],
      ["미매핑", money.format(taxData.unresolved_mapping_count || 0)],
      ["배치 오류", money.format(taxData.batch_error_count || 0)],
      ["전자신고", efile?.valid ? "가능" : "확인 필요"],
    ])}
    ${table(["검증 항목", "현재 값"], [
      row(["재무제표 라인", money.format(taxData.fs_line_count || 0)]),
      row(["자산", money.format(taxData.asset_count || 0)]),
      row(["업무용 차량", money.format(taxData.business_vehicle_count || 0)]),
      row(["거래", money.format(taxData.transaction_count || 0)]),
    ])}`;
}

async function refreshContextFromBy(env, by, customer) {
  const snapshot = await request(`${routeRoot(env)}/business-years/${by.by_id}/snapshot`);
  env.setContext({
    customerId: by.customer_id,
    customerName: customer?.customer_name || env.context.customerName,
    byId: by.by_id,
    fy: String(by.year_label),
    period: `${by.start_date} ~ ${by.end_date}`,
    status: by.status,
    progress: progressForStatus(by.status),
    snapshot,
    lockMode: by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN"),
  });
}

function formatDday(daysRemaining) {
  const days = Number(daysRemaining ?? 0);
  if (days === 0) return "D-Day";
  if (days < 0) return `D+${Math.abs(days)}`;
  return `D-${days}`;
}

function deadlineUrgencyClass(urgencyLevel) {
  const normalized = String(urgencyLevel || "NOTICE").toLowerCase();
  return `deadline-${normalized}`;
}

function formatNotificationTime(value) {
  if (!value) return "-";
  return String(value).replace("T", " ").slice(0, 16);
}

function notificationSeverityClass(severity) {
  if (severity === "ERROR") return "danger";
  if (severity === "WARN") return "warn";
  if (severity === "OK") return "ok";
  return "info";
}

function renderDashboardDeadlineTable(deadlines, locale = "ko") {
  const rows = asArray(deadlines).map((item) => {
    const statusText = item.status === "DRAFT"
      ? `${item.statusLabel || statusLabel(item.status, locale)} (${item.progressPct || 0}%)`
      : item.statusLabel || statusLabel(item.status, locale);
    return `
      <tr class="deadline-row ${escapeHtml(deadlineUrgencyClass(item.urgencyLevel))}" data-deadline-by="${escapeHtml(item.businessYearId)}" tabindex="0">
        <td><span class="badge ${item.urgencyLevel === "CRITICAL" ? "danger" : item.urgencyLevel === "WARNING" ? "warn" : "info"}">${escapeHtml(formatDday(item.daysRemaining))}</span></td>
        <td>${escapeHtml(item.customerName)}</td>
        <td>${escapeHtml(item.fiscalYear)}</td>
        <td>${escapeHtml(item.filingDueDate)}</td>
        <td>${pill(item.status, locale)} <span class="muted">${escapeHtml(statusText)}</span></td>
      </tr>`;
  });
  return `
    <div class="table-wrap dashboard-deadlines" data-dashboard-section="filing-deadlines">
      <table>
        <thead><tr><th>긴급도</th><th>고객사</th><th>사업연도</th><th>마감일</th><th>상태</th></tr></thead>
        <tbody>${rows.length ? rows.join("") : `<tr><td colspan="5">${escapeHtml(t(locale, "grid.empty"))}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function dashboardStatusRoute(status) {
  return {
    DRAFT: "ws/start:customer-pick",
    IN_REVIEW_VALIDATION: "ws/val:run",
    IN_REVIEW_APPROVAL: "ws/appr:inbox",
    APPROVED: "ws/print:preview",
    FILED: "post/hist:list",
  }[status] || "dashboard:overview";
}

function renderDashboardWorkStatusCards(summary, locale = "ko") {
  const statuses = asArray(summary.workStatus);
  return `
    <section class="dashboard-status-grid" data-dashboard-section="work-status" aria-label="업무현황">
      ${statuses.map((item) => `
        <button class="dashboard-status-card" type="button" data-work-status="${escapeHtml(item.status)}" style="--status-color: ${escapeHtml(item.color || "#3B82F6")}">
          <span class="status-accent" aria-hidden="true"></span>
          <span class="status-title">${escapeHtml(item.label || statusLabel(item.status, locale))}</span>
          <strong>${money.format(item.yearCount || 0)}</strong>
          <span class="status-meta">고객사 ${money.format(item.customerCount || 0)}개</span>
          ${Number(item.urgentCount || 0) > 0 ? `<span class="status-urgent">즉시 처리 필요 ${money.format(item.urgentCount)}건</span>` : `<span class="status-quiet">마감 안정</span>`}
        </button>`).join("")}
    </section>`;
}

function renderDashboardNotificationPanel(notificationSummary, queue, locale = "ko", showApprovals = true) {
  const notifications = asArray(notificationSummary?.notifications).slice(0, 10);
  const unreadCount = Number(notificationSummary?.unreadCount || 0);
  const approvalRows = asArray(queue);
  return `
    <article class="panel dashboard-notification-panel" data-dashboard-section="notifications">
      <div class="panel-head">
        <div>
          <h2>알림 / 결재 대기함</h2>
          <p class="empty">마감 알림과 결재 대기 업무를 한 곳에서 처리합니다.</p>
        </div>
        <button id="dashAlerts" class="secondary-btn compact" type="button">알림 센터</button>
      </div>
      <div class="dashboard-tabs" role="tablist" aria-label="대시보드 알림 탭">
        <button class="tab active" type="button" role="tab" aria-selected="true" data-dashboard-tab="notifications">
          알림 <span class="dashboard-unread-badge" data-notification-unread-badge>${money.format(unreadCount)}</span>
        </button>
        ${showApprovals ? `<button class="tab" type="button" role="tab" aria-selected="false" data-dashboard-tab="approvals">
          결재 대기 <span class="dashboard-unread-badge quiet">${money.format(approvalRows.length)}</span>
        </button>` : ""}
      </div>
      <div class="dashboard-tab-panel" data-dashboard-tab-panel="notifications">
        ${renderDashboardNotificationList(notifications, locale)}
      </div>
      ${showApprovals ? `<div class="dashboard-tab-panel hidden" data-dashboard-tab-panel="approvals">
        <div class="panel-head compact-head"><h3>내 결재함</h3><button id="dashApproval" class="secondary-btn compact" type="button">열기</button></div>
        ${renderDashboardApprovalQueue(approvalRows, locale)}
      </div>` : ""}
    </article>`;
}

function renderDashboardApprovalQueue(queue, locale = "ko") {
  if (!queue.length) {
    return `<p class="empty dashboard-empty">내 결재 대기 항목이 없습니다.</p>`;
  }
  return `
    <ul class="dashboard-approval-list">
      ${queue.map((item) => `
        <li class="dashboard-approval-item" data-approval-by="${escapeHtml(item.by_id)}" tabindex="0">
          <div class="approval-target">
            <span class="badge warn">결재 대기</span>
            <strong>${escapeHtml(item.customer_name)} · ${escapeHtml(item.year_label)}</strong>
            <span class="muted">사업연도 ${escapeHtml(item.start_date || "-")} ~ ${escapeHtml(item.end_date || "-")} · ${escapeHtml(statusLabel(item.status, locale))}</span>
          </div>
          <div class="approval-meta">
            <span>요청자 <strong>${escapeHtml(item.requester_login_id || "-")}</strong></span>
            <span>대기일 <strong>${money.format(item.pending_days || 0)}일</strong></span>
          </div>
          <div class="approval-inline-actions">
            <button class="primary-btn compact" type="button" data-approve-approval="${escapeHtml(item.by_id)}">승인</button>
            <button class="danger-btn compact" type="button" data-reject-approval="${escapeHtml(item.by_id)}">반려</button>
            <button class="secondary-btn compact" type="button" data-open-approval="${escapeHtml(item.by_id)}">상세</button>
          </div>
        </li>`).join("")}
    </ul>`;
}

function renderDashboardNotificationList(notifications, locale = "ko") {
  if (!notifications.length) {
    return `<p class="empty dashboard-empty">최근 알림이 없습니다.</p>`;
  }
  return `
    <ul class="dashboard-notification-list">
      ${notifications.map((item) => {
        const unread = item.status === "UNREAD";
        const bucket = item.dueBucket ? `<span class="badge info">${escapeHtml(item.dueBucket)}</span>` : "";
        return `
          <li class="dashboard-notification-item ${unread ? "unread" : "read"}" data-notification-id="${escapeHtml(item.notificationId)}">
            <span class="notification-dot" aria-hidden="true"></span>
            <div class="notification-copy">
              <div class="notification-title-line">
                <span class="badge ${notificationSeverityClass(item.severity)}">${escapeHtml(item.severity)}</span>
                ${bucket}
                <strong>${escapeHtml(item.title)}</strong>
              </div>
              <p>${escapeHtml(item.message)}</p>
              <span class="muted">${escapeHtml(item.customerName || "공통")} · ${escapeHtml(formatNotificationTime(item.createdAt))} · ${escapeHtml(item.notificationType || "GENERAL")}</span>
            </div>
            <div class="notification-actions">
              <button class="secondary-btn compact" type="button" data-open-notification="${escapeHtml(item.notificationId)}">이동</button>
              <button class="secondary-btn compact" type="button" data-read-notification="${escapeHtml(item.notificationId)}" ${unread ? "" : "disabled"}>읽음</button>
            </div>
          </li>`;
      }).join("")}
    </ul>`;
}

function renderDashboardRecentActivities(activitySummary, locale = "ko") {
  const activities = asArray(activitySummary?.activities).slice(0, 15);
  return `
    <article class="panel dashboard-activity-panel" data-dashboard-section="recent-activities">
      <div class="panel-head">
        <div>
          <h2>최근활동</h2>
          <p class="empty">감사 로그를 업무 피드로 요약해 최근 변경된 화면으로 바로 이동합니다.</p>
        </div>
        <button id="dashAudit" class="secondary-btn compact" type="button">감사 로그 전체</button>
      </div>
      ${activities.length ? `
        <ul class="dashboard-activity-list">
          ${activities.map((item) => {
            const target = [item.customerName || "공통", item.fiscalYear || ""].filter(Boolean).join(" / ");
            return `
              <li class="dashboard-activity-item" data-activity-audit="${escapeHtml(item.auditId)}" tabindex="0">
                <time>${escapeHtml(formatNotificationTime(item.occurredAt))}</time>
                <div class="activity-copy">
                  <div class="activity-title-line">
                    <span class="badge info">${escapeHtml(item.typeLabel || item.activityType || "업무 변경")}</span>
                    <strong>${escapeHtml(item.description || item.activityType || item.action)}</strong>
                  </div>
                  <p>${escapeHtml(target || item.tableName || "-")}</p>
                  <span class="muted">${escapeHtml(item.actorName || item.actorLoginId || "system")} · ${escapeHtml(item.routeKey || "ad-audit")}</span>
                </div>
                <button class="secondary-btn compact" type="button" data-open-activity="${escapeHtml(item.auditId)}">이동</button>
              </li>`;
          }).join("")}
        </ul>` : `<p class="empty dashboard-empty">최근활동이 없습니다.</p>`}
    </article>`;
}

function kpiDonutGradient(industries) {
  const colors = ["#0ea5e9", "#22c55e", "#f59e0b", "#ef4444", "#64748b"];
  let start = 0;
  const segments = asArray(industries).slice(0, 5).map((item, index) => {
    const pct = Math.max(0, Number(item.percentageBps || 0) / 100);
    const end = Math.min(100, start + pct);
    const segment = `${colors[index % colors.length]} ${start}% ${end}%`;
    start = end;
    return segment;
  });
  if (start < 100) segments.push(`#e2e8f0 ${start}% 100%`);
  return `conic-gradient(${segments.join(", ")})`;
}

function renderKpiIndustryDistribution(industrySummary) {
  const industries = asArray(industrySummary?.industries);
  return `
    <section class="kpi-subpanel" data-dashboard-section="kpi-industry-distribution">
      <div class="kpi-subpanel-head">
        <h3>업종별 법인 분포</h3>
        <span>${money.format(industrySummary?.totalCustomers || 0)}개 법인</span>
      </div>
      ${industries.length ? `
        <div class="kpi-donut-layout">
          <div class="kpi-donut" style="background:${escapeHtml(kpiDonutGradient(industries))}" aria-label="업종별 법인 분포"></div>
          <ul class="kpi-distribution-list">
            ${industries.slice(0, 5).map((item) => `
              <li class="kpi-distribution-row" data-kpi-industry="${escapeHtml(item.industryCode)}">
                <span>${escapeHtml(item.industryName || item.industryCode)}</span>
                <strong>${Number(item.percentagePct || 0).toFixed(1)}%</strong>
                <em>${money.format(item.customerCount || 0)}개</em>
              </li>`).join("")}
          </ul>
        </div>` : `<p class="empty dashboard-empty">업종별 법인 데이터가 없습니다.</p>`}
    </section>`;
}

function renderKpiLossExpiry(lossSummary) {
  const buckets = asArray(lossSummary?.buckets);
  const maxAmount = Math.max(1, ...buckets.map((item) => Number(item.totalAmount || 0)));
  return `
    <section class="kpi-subpanel" data-dashboard-section="kpi-loss-expiry">
      <div class="kpi-subpanel-head">
        <h3>이월결손금 만료 예측</h3>
        <span>${money.format(lossSummary?.totalCustomerCount || 0)}개 법인</span>
      </div>
      ${buckets.length ? `
        <div class="kpi-loss-table">
          ${buckets.map((item) => `
            <div class="kpi-loss-row" data-kpi-loss-year="${escapeHtml(item.expiresYear)}">
              <span>${escapeHtml(item.expiresYear)}년</span>
              <div class="bar-track"><span style="width:${Math.max(4, Math.round(Number(item.totalAmount || 0) / maxAmount * 100))}%"></span></div>
              <strong>${money.format(item.totalAmount || 0)}</strong>
              <em>${money.format(item.customerCount || 0)}개 / ${money.format(item.lossCount || 0)}건</em>
            </div>`).join("")}
        </div>
        <p class="kpi-caption">향후 ${escapeHtml(lossSummary?.years || 3)}개년 만료 예정 잔액 ${money.format(lossSummary?.totalAmount || 0)}</p>`
        : `<p class="empty dashboard-empty">만료 예정 이월결손금이 없습니다.</p>`}
    </section>`;
}

function renderDashboardTaxBurdenKpi(kpiSummary, industrySummary, lossSummary, locale = "ko") {
  const trend = asArray(kpiSummary?.trend).slice(-5);
  const maxRate = Math.max(1, ...trend.map((item) => Number(item.effectiveTaxRateBps || 0)));
  const latest = trend[trend.length - 1];
  const averagePct = Number(kpiSummary?.averageEffectiveTaxRatePct || 0);
  return `
    <article class="panel dashboard-kpi-panel" data-dashboard-section="kpi-tax-burden">
      <div class="panel-head">
        <div>
          <h2>핵심지표</h2>
          <p class="empty">최근 ${escapeHtml(kpiSummary?.years || 5)}개년 당기 세부담 추이</p>
        </div>
        <button id="dashKpiTax" class="secondary-btn compact" type="button">세부담 분석</button>
      </div>
      <div class="kpi-summary-strip">
        <span>평균 실효세율 <strong>${averagePct.toFixed(2)}%</strong></span>
        <span>총 부담세액 <strong>${money.format(kpiSummary?.totalTaxDue || 0)}</strong></span>
      </div>
      ${trend.length ? `
        <div class="dashboard-kpi-chart" aria-label="당기 세부담 추이">
          ${trend.map((item) => {
            const rate = Number(item.effectiveTaxRateBps || 0);
            return `
              <div class="kpi-trend-row" data-kpi-year="${escapeHtml(item.fiscalYear)}">
                <span>${escapeHtml(item.fiscalYear)}</span>
                <div class="bar-track"><span style="width:${Math.max(4, Math.round(rate / maxRate * 100))}%"></span></div>
                <strong>${(rate / 100).toFixed(2)}%</strong>
              </div>`;
          }).join("")}
        </div>
        <p class="kpi-caption">최신 ${escapeHtml(latest?.fiscalYear || "-")}년: 과세표준 ${money.format(latest?.taxableIncome || 0)}, 부담세액 ${money.format(latest?.totalTaxDue || 0)}, 담당 법인 ${money.format(latest?.customerCount || 0)}개</p>`
        : `<p class="empty dashboard-empty">세부담 추이 데이터가 없습니다.</p>`}
      <section class="dashboard-kpi-secondary">
        ${renderKpiIndustryDistribution(industrySummary)}
        ${renderKpiLossExpiry(lossSummary)}
      </section>
    </article>`;
}

function dashboardRoles(auth) {
  return asArray(auth?.user?.roles).map((role) => String(role).toUpperCase());
}

function canViewDashboardApprovals(auth) {
  return dashboardRoles(auth).includes("TAX_REVIEWER");
}

function canViewDashboardKpi(auth) {
  return dashboardRoles(auth).some((role) => ["SUPER_ADMIN", "TENANT_ADMIN", "SYSTEM_ADMIN", "TAX_EXPERT", "TAX_REVIEWER"].includes(role));
}

function invalidateDashboardCache(reason = "manual") {
  dashboardCacheVersion += 1;
  return { version: dashboardCacheVersion, reason };
}

function stopDashboardRealtime() {
  if (!dashboardRealtime) return;
  clearInterval(dashboardRealtime.pollTimer);
  dashboardRealtime = null;
}

function startDashboardRealtime(env, root) {
  stopDashboardRealtime();
  const realtime = { root, pollTimer: null, refreshing: false };
  realtime.pollTimer = setInterval(async () => {
    if (!dashboardRealtime || dashboardRealtime.root !== root || realtime.refreshing) return;
    realtime.refreshing = true;
    try {
      invalidateDashboardCache("poll");
      await renderDashboard(env);
    } finally {
      if (dashboardRealtime === realtime) {
        realtime.refreshing = false;
      }
    }
  }, DASHBOARD_REFRESH_INTERVAL_MS);
  dashboardRealtime = realtime;
}

async function renderDashboard(env) {
  const root = routeRoot(env);
  const showApprovals = canViewDashboardApprovals(env.auth);
  const showKpi = canViewDashboardKpi(env.auth);
  const [
    summary,
    deadlines,
    notificationSummary,
    queue,
    recentSummary,
    kpiTaxBurden,
    kpiIndustryDistribution,
    kpiLossExpiry,
  ] = await Promise.all([
    request(`${root}/dashboard`),
    request(`${root}/dashboard/filing-deadlines?withinDays=30`),
    request(`${root}/dashboard/notifications?limit=10`),
    showApprovals ? request(`${root}/workflow/queue?assignee=me`) : Promise.resolve([]),
    request(`${root}/dashboard/recent-activities?limit=15`),
    showKpi ? request(`${root}/dashboard/kpi/tax-burden?years=5`) : Promise.resolve({ trend: [] }),
    showKpi ? request(`${root}/dashboard/kpi/industry-distribution`) : Promise.resolve({ industries: [] }),
    showKpi ? request(`${root}/dashboard/kpi/loss-expiry?years=3`) : Promise.resolve({ buckets: [] }),
  ]);
  const deadlineRows = asArray(deadlines.deadlines || summary.filingDeadlines?.deadlines).slice(0, 10);
  const dashboardNotifications = asArray(notificationSummary.notifications);
  const approvalQueue = asArray(queue);
  const recentActivities = asArray(recentSummary.activities);
  env.outlet.innerHTML = `
    <section class="dashboard-home" data-dashboard="overview" data-dashboard-cache-version="${dashboardCacheVersion}">
      <section class="panel dashboard-hero" data-dashboard-section="start">
        <div>
          <span class="badge info">Dashboard</span>
          <h2>신고 업무 현황</h2>
          <p class="empty">작성, 검증, 결재, 승인, 신고 완료 상태를 확인하고 다음 작업으로 바로 이동합니다.</p>
        </div>
        <button id="dashStartWork" class="primary-btn" type="button">신고 작업 시작</button>
      </section>
      ${Number(summary.rejectedCount || 0) > 0 ? `<section class="dashboard-rejected-banner" data-dashboard-section="rejected">반려 ${money.format(summary.rejectedCount)}건 - 재작성 필요</section>` : ""}
      ${renderDashboardWorkStatusCards(summary, env.locale)}
      <section class="dashboard-main-grid">
        <article class="panel dashboard-deadline-panel">
          <div class="panel-head"><h2>신고마감 임박</h2><button id="dashDueSoonAll" class="secondary-btn compact" type="button">전체 보기</button></div>
          ${renderDashboardDeadlineTable(deadlineRows, env.locale)}
        </article>
        ${renderDashboardNotificationPanel(notificationSummary, queue, env.locale, showApprovals)}
      </section>
      <section class="dashboard-lower-grid">
        ${renderDashboardRecentActivities(recentSummary, env.locale)}
        ${showKpi ? renderDashboardTaxBurdenKpi(kpiTaxBurden, kpiIndustryDistribution, kpiLossExpiry, env.locale) : ""}
      </section>
    </section>`;
  document.getElementById("dashStartWork").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  document.querySelectorAll("[data-work-status]").forEach((card) => {
    card.addEventListener("click", () => env.navigate(dashboardStatusRoute(card.dataset.workStatus)));
  });
  document.getElementById("dashDueSoonAll").addEventListener("click", () => env.navigate("dashboard:duesoon"));
  document.querySelectorAll("[data-dashboard-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll("[data-dashboard-tab]").forEach((tab) => {
        const active = tab === button;
        tab.classList.toggle("active", active);
        tab.setAttribute("aria-selected", active ? "true" : "false");
      });
      document.querySelectorAll("[data-dashboard-tab-panel]").forEach((panel) => {
        panel.classList.toggle("hidden", panel.dataset.dashboardTabPanel !== button.dataset.dashboardTab);
      });
    });
  });
  document.querySelectorAll("[data-read-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/notifications/${button.dataset.readNotification}`, {
        method: "PATCH",
        body: JSON.stringify({ status: "READ" }),
      });
      invalidateDashboardCache("notification-read");
      await renderDashboard(env);
    });
  });
  document.querySelectorAll("[data-open-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      const item = dashboardNotifications.find((candidate) => String(candidate.notificationId) === String(button.dataset.openNotification));
      if (!item) return;
      if (item.byId) {
        await refreshContextFromBy(env, {
          by_id: item.byId,
          customer_id: item.customerId,
          year_label: item.fiscalYear,
          start_date: item.startDate,
          end_date: item.filingDueDate,
          status: item.businessYearStatus || "DRAFT",
        }, { customer_name: item.customerName });
      }
      env.navigate(item.routeKey || "rp-alerts");
    });
  });
  const openApproval = async (byId) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    await refreshContextFromBy(env, {
      by_id: item.by_id,
      customer_id: item.customer_id,
      year_label: item.year_label,
      start_date: item.start_date,
      end_date: item.end_date,
      status: item.status,
    }, { customer_name: item.customer_name });
    env.navigate(item.route_key || "ws/appr:inbox");
  };
  const runApprovalAction = async (byId, status) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    const approved = status === "APPROVED";
    await request(`${root}/business-years/${encodeURIComponent(item.by_id)}/status`, {
      method: "POST",
      body: JSON.stringify({
        status,
        actor: env.auth?.user?.login_id || "dashboard",
        approver: env.auth?.user?.login_id || item.approver_login_id || "dashboard",
        comment: approved ? "dashboard inline approval" : "dashboard inline rejection",
      }),
    });
    invalidateDashboardCache("approval-action");
    await renderDashboard(env);
  };
  document.querySelectorAll("[data-open-approval]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openApproval(button.dataset.openApproval);
    });
  });
  document.querySelectorAll("[data-approve-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.approveApproval, "APPROVED");
    });
  });
  document.querySelectorAll("[data-reject-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.rejectApproval, "DRAFT");
    });
  });
  document.querySelectorAll("[data-approval-by]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openApproval(rowElement.dataset.approvalBy));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openApproval(rowElement.dataset.approvalBy);
      }
    });
  });
  document.querySelectorAll("[data-deadline-by]").forEach((rowElement) => {
    const openDeadline = async () => {
      const item = deadlineRows.find((candidate) => String(candidate.businessYearId) === rowElement.dataset.deadlineBy);
      if (!item) return;
      await refreshContextFromBy(env, {
        by_id: item.businessYearId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.filingDueDate,
        status: item.status,
      }, { customer_name: item.customerName });
      env.navigate(item.routeKey || "ws/start:snapshot");
    };
    rowElement.addEventListener("click", openDeadline);
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openDeadline();
      }
    });
  });
  const openActivity = async (auditId) => {
    const item = recentActivities.find((candidate) => String(candidate.auditId) === String(auditId));
    if (!item) return;
    if (item.byId) {
      await refreshContextFromBy(env, {
        by_id: item.byId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.endDate,
        status: item.businessYearStatus || "DRAFT",
      }, { customer_name: item.customerName });
    }
    env.navigate(item.routeKey || "ad-audit");
  };
  document.querySelectorAll("[data-activity-audit]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openActivity(rowElement.dataset.activityAudit));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openActivity(rowElement.dataset.activityAudit);
      }
    });
  });
  document.querySelectorAll("[data-open-activity]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openActivity(button.dataset.openActivity);
    });
  });
  document.getElementById("dashKpiTax")?.addEventListener("click", () => env.navigate("report:tax-burden"));
  document.getElementById("dashApproval")?.addEventListener("click", () => env.navigate("ws-appr"));
  document.getElementById("dashAlerts")?.addEventListener("click", () => env.navigate("rp-alerts"));
  document.getElementById("dashAudit")?.addEventListener("click", () => env.navigate("ad-audit"));
  startDashboardRealtime(env, root);
}

async function renderWorkStart(env) {
  const root = routeRoot(env);
  const [customers, years] = await Promise.all([
    request(`${root}/customers`),
    request(`${root}/business-years`),
  ]);
  const locale = env.locale;
  const currentYear = new Date().getFullYear();
  const customerOptions = customers
    .map((customer) => `<option value="${escapeHtml(customer.customer_id)}">${escapeHtml(customer.customer_name)} (${escapeHtml(customer.customer_code)})</option>`)
    .join("");
  const yearsByCustomer = new Map();
  years.forEach((year) => {
    const list = yearsByCustomer.get(year.customer_id) || [];
    list.push(year);
    yearsByCustomer.set(year.customer_id, list);
  });
  const recentYears = [...years]
    .sort((a, b) => String(b.updated_at || "").localeCompare(String(a.updated_at || "")))
    .slice(0, 5);
  env.outlet.innerHTML = `
    <section class="leaf-workbench work-start-workbench" data-stage="work-start">
      <section class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "workStart.title"))}</span>
            <h2>${escapeHtml(t(locale, "route.ws.start.customerPick"))}</h2>
            <p>${escapeHtml(t(locale, "context.pickCustomerYear"))}</p>
          </div>
        </div>
        ${metrics([
          [t(locale, "field.customerName"), money.format(customers.length)],
          [t(locale, "field.yearLabel"), money.format(years.length)],
          [t(locale, "workStart.recent"), money.format(recentYears.length)],
          [t(locale, "workStart.snapshotPreview"), env.context?.snapshot ? t(locale, "status.ready") : t(locale, "status.pending")],
        ])}
      </section>
      <section class="grid two">
      <article class="panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(locale, "workStart.recent"))}</h2><p>${escapeHtml(t(locale, "workStart.selectWork"))}</p></div>
          <label class="inline-control">${escapeHtml(t(locale, "workStart.customerSearch"))} <input id="workStartSearch" type="search" placeholder="${escapeHtml(t(locale, "field.customerName"))}" /></label>
        </div>
        ${table([t(locale, "field.customerName"), t(locale, "field.yearLabel"), t(locale, "field.status"), t(locale, "field.progress"), ""], recentYears.map((by) => {
          const customer = customers.find((item) => item.customer_id === by.customer_id);
          return row([
            escapeHtml(customer?.customer_name || by.customer_id),
            escapeHtml(by.year_label),
            pill(by.status, locale),
            `<div class="bar-track"><span style="width:${progressForStatus(by.status)}%"></span></div>`,
            `<button class="primary-btn compact" type="button" data-select-by="${escapeHtml(by.by_id)}">${escapeHtml(t(locale, "common.continue"))}</button>`,
          ]);
        }))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.newBusinessYear"))}</h2></div>
        <form id="businessYearForm" class="stack">
          <label>${escapeHtml(t(locale, "field.customerName"))} <select id="byCustomer">${customerOptions}</select></label>
          <label>${escapeHtml(t(locale, "field.yearLabel"))} <input id="byYear" type="number" value="${currentYear}" /></label>
          <div class="form-grid">
            <label>${escapeHtml(t(locale, "workStart.startDate"))} <input id="byStart" type="date" value="${currentYear}-01-01" /></label>
            <label>${escapeHtml(t(locale, "workStart.endDate"))} <input id="byEnd" type="date" value="${currentYear}-12-31" /></label>
          </div>
          <label class="inline-control"><input id="byCarryForward" type="checkbox" checked /> ${escapeHtml(t(locale, "workStart.carryForward"))}</label>
          <label>${escapeHtml(t(locale, "workStart.carryForwardSource"))} <select id="byCarryForwardSource"></select></label>
          <p class="empty">${escapeHtml(t(locale, "workStart.carryForwardHelp"))}</p>
          <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.create"))}</button>
        </form>
      </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.newCustomer"))}</h2></div>
          <form id="customerForm" class="stack">
            <label>${escapeHtml(t(locale, "field.customerCode"))} <input id="newCustomerCode" value="cust${Date.now().toString(36).slice(-4)}" /></label>
            <label>${escapeHtml(t(locale, "field.customerName"))} <input id="newCustomerName" value="${escapeHtml(t(locale, "workStart.newCustomer"))}" /></label>
            <label>${escapeHtml(t(locale, "field.bizRegNo"))} <input id="newCustomerBiz" value="1234567890" /></label>
            <label>SME <input id="newCustomerSme" type="checkbox" checked /></label>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.create"))}</button>
          </form>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.snapshotPreview"))}</h2></div>
          <div id="snapshotPreview" class="stack">${env.context?.snapshot ? renderSnapshotSummary(env.context.snapshot, locale) : `<p class="empty">${escapeHtml(t(locale, "context.select"))}</p>`}</div>
        </article>
      </section>
    </section>`;

  document.getElementById("workStartSearch")?.addEventListener("input", (event) => {
    const query = event.target.value.trim().toLowerCase();
    document.querySelectorAll("[data-select-by]").forEach((button) => {
      const tr = button.closest("tr");
      if (!tr) return;
      tr.style.display = !query || tr.textContent.toLowerCase().includes(query) ? "" : "none";
    });
  });

  document.querySelectorAll("[data-select-by]").forEach((button) => {
    button.addEventListener("click", async () => {
      const by = years.find((item) => String(item.by_id) === button.dataset.selectBy);
      const customer = customers.find((item) => item.customer_id === by.customer_id);
      await refreshContextFromBy(env, by, customer);
      document.getElementById("snapshotPreview").innerHTML = renderSnapshotSummary(env.context.snapshot || {}, locale);
      env.navigate("ws-info", { byId: by.by_id, customerId: by.customer_id });
    });
  });

  const byCustomerSelect = document.getElementById("byCustomer");
  const byYearInput = document.getElementById("byYear");
  const byCarryForward = document.getElementById("byCarryForward");
  const byCarryForwardSource = document.getElementById("byCarryForwardSource");
  const byStartInput = document.getElementById("byStart");
  const byEndInput = document.getElementById("byEnd");

  function syncCarryForwardOptions() {
    const customerId = Number(byCustomerSelect.value);
    const candidates = [...(yearsByCustomer.get(customerId) || [])]
      .sort((a, b) => b.year_label - a.year_label || b.by_id - a.by_id);
    byCarryForwardSource.innerHTML = candidates.length
      ? candidates.map((item) => `<option value="${escapeHtml(item.by_id)}">${escapeHtml(item.year_label)} (${escapeHtml(item.start_date)} ~ ${escapeHtml(item.end_date)})</option>`).join("")
      : `<option value="">${escapeHtml(t(locale, "context.select"))}</option>`;
    byCarryForwardSource.disabled = !byCarryForward.checked || !candidates.length;
  }

  function syncBusinessYearDates() {
    const nextYear = Number(byYearInput.value || currentYear);
    byStartInput.value = `${nextYear}-01-01`;
    byEndInput.value = `${nextYear}-12-31`;
  }

  byCustomerSelect.addEventListener("change", syncCarryForwardOptions);
  byYearInput.addEventListener("change", syncBusinessYearDates);
  byCarryForward.addEventListener("change", () => {
    syncCarryForwardOptions();
    byCarryForwardSource.disabled = !byCarryForward.checked || !byCarryForwardSource.options.length;
  });
  syncCarryForwardOptions();

  document.getElementById("customerForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/customers`, {
      method: "POST",
      body: JSON.stringify({
        customer_code: document.getElementById("newCustomerCode").value.trim(),
        customer_name: document.getElementById("newCustomerName").value.trim(),
        biz_reg_no: document.getElementById("newCustomerBiz").value.trim(),
        is_sme: document.getElementById("newCustomerSme").checked,
        work_scopes: ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"],
      }),
    });
    await renderWorkStart(env);
  });

  document.getElementById("businessYearForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${root}/business-years`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: Number(document.getElementById("byCustomer").value),
        year_label: Number(document.getElementById("byYear").value),
        start_date: document.getElementById("byStart").value,
        end_date: document.getElementById("byEnd").value,
        carry_forward_from_by_id: document.getElementById("byCarryForward").checked && document.getElementById("byCarryForwardSource").value
          ? Number(document.getElementById("byCarryForwardSource").value)
          : null,
      }),
    });
    const customer = customers.find((item) => item.customer_id === by.customer_id);
    await refreshContextFromBy(env, by, customer);
    document.getElementById("snapshotPreview").innerHTML = renderSnapshotSummary(env.context.snapshot || {}, locale);
    env.navigate("ws-info", { byId: by.by_id, customerId: by.customer_id });
  });
}

async function renderWorkInfo(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const locale = env.locale;
  const [validation, fs, assets, transactions, vehicleLogs, batches, mappings, issues] = await Promise.all([
    request(`${root}/tax-data/validation`),
    request(`${root}/tax-data/financial-statements`),
    request(`${root}/tax-data/assets`),
    request(`${root}/tax-data/transactions`),
    request(`${root}/vehicle-usage-logs`),
    request(`${root}/tax-data/import-batches`),
    request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`).catch(() => []),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const vehicleAssetOptions = assets
    .filter((asset) => asset.is_business_vehicle)
    .map((asset) => `<option value="${escapeHtml(asset.asset_id)}">${escapeHtml(asset.asset_name)} (${escapeHtml(asset.asset_code)})</option>`)
    .join("");
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-workbench="tax-data">
      <section class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "taxData.title"))}</span>
            <h2>${escapeHtml(t(locale, "route.ws.info.fs"))}</h2>
            <p>${escapeHtml(env.context.customerName || "-")} / ${escapeHtml(env.context.fy || "-")}</p>
          </div>
          <div class="button-row">
            <button class="secondary-btn compact" type="button" data-tax-template="financial-statements">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.fsTab") }))}</button>
            <button class="secondary-btn compact" type="button" data-tax-template="assets">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.assetTab") }))}</button>
            <button class="secondary-btn compact" type="button" data-tax-template="transactions">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.transactionTab") }))}</button>
          </div>
        </div>
      ${metrics([
          [t(locale, "taxData.fsTab"), money.format(validation.fs_line_count || 0)],
          [t(locale, "taxData.assetTab"), money.format(validation.asset_count || 0)],
          [t(locale, "taxData.transactionTab"), money.format(validation.transaction_count || 0)],
          [t(locale, "taxData.consistency"), validation.balanced ? t(locale, "status.ok") : t(locale, "status.warn")],
      ])}
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.upload"))}</h2></div>
          <form id="importForm" class="stack">
            <label>${escapeHtml(t(locale, "common.category"))}
              <select id="importType">
                <option value="financial-statements">${escapeHtml(t(locale, "taxData.fsTab"))}</option>
                <option value="assets">${escapeHtml(t(locale, "taxData.assetTab"))}</option>
                <option value="transactions">${escapeHtml(t(locale, "taxData.transactionTab"))}</option>
              </select>
            </label>
            <label>CSV/Excel <input id="importFile" type="file" /></label>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.upload"))}</button>
          </form>
          <div id="importResult" class="empty" aria-live="polite"></div>
          <h3>${escapeHtml(t(locale, "taxData.importHistory"))}</h3>
          ${table([t(locale, "common.category"), "File", t(locale, "common.total"), t(locale, "status.error"), ""], batches.map((batch) => row([
            escapeHtml(batch.data_type),
            escapeHtml(batch.source_file_name || "-"),
            money.format(batch.row_count),
            money.format(batch.error_count),
            `<button class="secondary-btn compact" type="button" data-import-errors="${escapeHtml(batch.batch_id)}">${escapeHtml(t(locale, "common.errorDetail"))}</button>`,
          ])))}
          <div id="importErrors" class="stack"></div>
        </article>
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(t(locale, "taxData.consistency"))}</h2><p>${escapeHtml(t(locale, "taxData.sourceJump"))}</p></div>
            <button id="taxDataValidate" class="secondary-btn compact" type="button">${escapeHtml(t(locale, "common.run"))}</button>
          </div>
          ${renderTaxDataValidationSummary(validation)}
          ${table([t(locale, "field.severity"), t(locale, "field.title"), t(locale, "common.jump")], issues.slice(0, 8).map((issue) => row([
            escapeHtml(statusLabel(issue.severity || "WARN", locale)),
            escapeHtml(issue.message || issue.rule_code || "-"),
            `<button class="secondary-btn compact" type="button" data-source-jump="${escapeHtml(sourceTabForIssue(issue))}">${escapeHtml(t(locale, "common.jump"))}</button>`,
          ])))}
        </article>
      </section>
      <section class="panel">
        <div class="tabs" role="tablist">
          <button class="active" type="button" data-tax-tab-button="fs">${escapeHtml(t(locale, "taxData.fsTab"))}</button>
          <button type="button" data-tax-tab-button="assets">${escapeHtml(t(locale, "taxData.assetTab"))}</button>
          <button type="button" data-tax-tab-button="transactions">${escapeHtml(t(locale, "taxData.transactionTab"))}</button>
          <button type="button" data-tax-tab-button="vehicle">${escapeHtml(t(locale, "taxData.vehicleTab"))}</button>
        </div>
        <div data-tax-tab="fs">
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "field.value")], fs.slice(0, 20).map((item) => row([escapeHtml(item.account_code), escapeHtml(item.account_name), money.format(item.amount)])))}
        </div>
        <div class="hidden" data-tax-tab="assets">
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount")], assets.slice(0, 20).map((item) => row([escapeHtml(item.asset_code), escapeHtml(item.asset_name), escapeHtml(item.asset_category), money.format(item.acquisition_cost)])))}
        </div>
        <div class="hidden" data-tax-tab="transactions">
          ${table(["Date", t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount")], transactions.slice(0, 20).map((item) => row([escapeHtml(item.tx_date), escapeHtml(item.partner_name), escapeHtml(item.category), money.format(item.amount)])))}
        </div>
        <div class="hidden" data-tax-tab="vehicle">
          ${table(["Asset ID", "Month", "Total km", "Business km", "%"], vehicleLogs.map((item) => row([escapeHtml(item.asset_id), escapeHtml(item.usage_month), escapeHtml(item.total_distance_km), escapeHtml(item.business_distance_km), `${(item.business_use_bps / 100).toFixed(1)}%`])))}
        </div>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.mapping"))}</h2></div>
          <form id="mappingForm" class="stack">
            <div class="form-grid">
              <label>${escapeHtml(t(locale, "field.code"))} <input id="mapSourceCode" value="${escapeHtml(fs[0]?.account_code || "")}" /></label>
              <label>${escapeHtml(t(locale, "field.name"))} <input id="mapSourceName" value="${escapeHtml(fs[0]?.account_name || "")}" /></label>
              <label>Standard code <input id="mapStandardCode" value="${escapeHtml(fs[0]?.standard_account_code || "")}" /></label>
              <label>Standard name <input id="mapStandardName" value="${escapeHtml(fs[0]?.standard_account_name || "")}" /></label>
            </div>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "taxData.mappingRule"))}</button>
          </form>
          ${table([t(locale, "field.code"), t(locale, "field.name"), "Standard"], mappings.slice(0, 8).map((item) => row([escapeHtml(item.source_account_code), escapeHtml(item.source_account_name), escapeHtml(item.standard_account_name)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.vehicleEditor"))}</h2></div>
          <form id="vehicleLogForm" class="stack">
            <label>${escapeHtml(t(locale, "taxData.vehicleTab"))} <select id="vehicleAsset">${vehicleAssetOptions}</select></label>
            <div class="form-grid">
              <label>Month <input id="vehicleMonth" type="date" value="${today().slice(0, 7)}-01" /></label>
              <label>Total km <input id="vehicleTotalKm" type="number" value="1000" /></label>
              <label>Business km <input id="vehicleBusinessKm" type="number" value="700" /></label>
            </div>
            <button class="primary-btn" type="submit" ${vehicleAssetOptions ? "" : "disabled"}>${escapeHtml(t(locale, "taxData.addVehicleLog"))}</button>
          </form>
        </article>
      </section>
      <div class="button-row">
        <button class="primary-btn" type="button" id="taxDataComplete">${escapeHtml(t(locale, "taxData.completeInput"))}</button>
      </div>
    </section>`;

  document.querySelectorAll("[data-tax-template]").forEach((button) => {
    button.addEventListener("click", () => {
      downloadBinary(`${routeRoot(env)}/tax-data/templates/${button.dataset.taxTemplate}`, `tax-data-${button.dataset.taxTemplate}-template.csv`);
    });
  });
  document.querySelectorAll("[data-tax-tab-button]").forEach((button) => {
    button.addEventListener("click", () => activateTaxDataTab(button.dataset.taxTabButton));
  });
  document.querySelectorAll("[data-source-jump]").forEach((button) => {
    button.addEventListener("click", () => activateTaxDataTab(button.dataset.sourceJump));
  });
  document.querySelectorAll("[data-import-errors]").forEach((button) => {
    button.addEventListener("click", async () => {
      const errors = await request(`${root}/tax-data/import-batches/${encodeURIComponent(button.dataset.importErrors)}/errors`);
      document.getElementById("importErrors").innerHTML = `
        <h3>${escapeHtml(t(locale, "taxData.issueDrilldown"))}</h3>
        ${table(["Row", t(locale, "field.status"), t(locale, "field.name"), t(locale, "field.value")], errors.map((error) => row([
          escapeHtml(error.row_no),
          escapeHtml(statusLabel(error.severity, locale)),
          escapeHtml(error.field_name || "-"),
          escapeHtml(error.message),
        ])))}`;
    });
  });
  document.getElementById("taxDataValidate").addEventListener("click", async () => {
    const result = await request(`${root}/tax-data/validation`, { method: "POST", body: "{}" });
    await renderWorkInfo(env);
    console.info(result);
  });
  document.getElementById("importForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const file = document.getElementById("importFile").files[0];
    if (!file) return;
    const form = new FormData();
    form.append("file", file);
    await request(`${root}/tax-data/${document.getElementById("importType").value}/import`, {
      method: "POST",
      body: form,
    });
    document.getElementById("importResult").textContent = t(locale, "taxData.importResult");
    await renderWorkInfo(env);
  });
  document.getElementById("mappingForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`, {
      method: "POST",
      body: JSON.stringify({
        statement_type: "FS",
        source_account_code: document.getElementById("mapSourceCode").value.trim(),
        source_account_name: document.getElementById("mapSourceName").value.trim(),
        standard_account_code: document.getElementById("mapStandardCode").value.trim(),
        standard_account_name: document.getElementById("mapStandardName").value.trim(),
      }),
    });
    await renderWorkInfo(env);
  });
  document.getElementById("vehicleLogForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/vehicle-usage-logs`, {
      method: "POST",
      body: JSON.stringify({
        asset_id: Number(document.getElementById("vehicleAsset").value),
        usage_month: document.getElementById("vehicleMonth").value,
        total_distance_km: Number(document.getElementById("vehicleTotalKm").value || 0),
        business_distance_km: Number(document.getElementById("vehicleBusinessKm").value || 0),
      }),
    });
    await renderWorkInfo(env);
  });
  document.getElementById("taxDataComplete").addEventListener("click", () => env.navigate("ws/adj:B1"));
}

function activateTaxDataTab(tab) {
  document.querySelectorAll("[data-tax-tab]").forEach((panel) => {
    panel.classList.toggle("hidden", panel.dataset.taxTab !== tab);
  });
  document.querySelectorAll("[data-tax-tab-button]").forEach((button) => {
    button.classList.toggle("active", button.dataset.taxTabButton === tab);
  });
}

function sourceTabForIssue(issue) {
  const source = String(issue?.source || issue?.source_module || issue?.field_path || issue?.rule_code || "").toLowerCase();
  if (source.includes("asset")) return "assets";
  if (source.includes("vehicle")) return "vehicle";
  if (source.includes("transaction") || source.includes("tx")) return "transactions";
  return "fs";
}

async function renderAdjustmentsLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [adjustments, reserves, b1Items, b4Items, b15Items, history] = await Promise.all([
    request(`${root}/adjustments`),
    request(`${root}/reserves`),
    request(`${root}/adjustments/income`).catch(() => []),
    request(`${root}/adjustments/assets/B4`).catch(() => []),
    request(`${root}/adjustments/evaluation/B15`).catch(() => []),
    request(`${root}/adjustments/history`).catch(() => []),
  ]);
  const itemGrids = {
    B1: { rows: b1Items },
    B4: { rows: b4Items },
    B15: { rows: b15Items },
  };
  const evidenceItem = [...b1Items, ...b4Items, ...b15Items][0];
  const evidenceAttachments = evidenceItem
    ? await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`).catch(() => [])
    : [];
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([
        ["조정 건수", adjustments.length],
        ["유보 건수", reserves.length],
        ["가산", money.format(adjustments.filter((item) => item.direction === "ADD").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
        ["차감", money.format(adjustments.filter((item) => item.direction === "DEDUCT").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
      ])}
      <article class="panel">
        <div class="panel-head"><h2>17개 세무조정 모듈</h2></div>
        <div class="grid four">
          ${adjustmentModules.map(([code, label, family]) => `
            <article class="card">
              <h3>${escapeHtml(code)} ${escapeHtml(label)}</h3>
              <p class="eyebrow">${escapeHtml(family)}</p>
              <button class="primary-btn compact" type="button" data-run-adjustment="${code}">실행</button>
            </article>`).join("")}
        </div>
      </article>
      <section class="grid three">
        ${renderDataGrid({ id: "B1", title: `B1 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B1"), rows: b1Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
        ${renderDataGrid({ id: "B4", title: `B4 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B4"), rows: b4Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
        ${renderDataGrid({ id: "B15", title: `B15 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B15"), rows: b15Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>조정 결과</h2></div>
          ${table(["코드", "방향", "금액", "상태"], adjustments.map((item) => row([
            escapeHtml(item.adj_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.status),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>유보 잔액</h2></div>
          ${table(["코드", "방향", "금액", "모듈"], reserves.map((item) => row([
            escapeHtml(item.reserve_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.source_module),
          ])))}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Adjustment History</h2></div>
        ${table(["Module", "Action", "Item", "Changed"], history.slice(0, 20).map((item) => row([
          escapeHtml(item.source_module),
          escapeHtml(item.action),
          escapeHtml(item.new_data?.item_code || item.old_data?.item_code || "-"),
          escapeHtml(item.changed_at),
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Evidence Attachments</h2></div>
        ${table(["File", "Type", "Storage URL", "Uploaded"], evidenceAttachments.map((item) => row([
          escapeHtml(item.file_name),
          escapeHtml(item.content_type),
          escapeHtml(item.storage_url || "-"),
          escapeHtml(item.created_at),
        ])))}
        <form id="adjustmentEvidenceForm" class="stack">
          <label>File name <input id="evidenceFileName" value="evidence.pdf" /></label>
          <label>Storage URL <input id="evidenceStorageUrl" value="evidence/${Date.now()}.pdf" /></label>
          <button class="primary-btn" type="submit" ${evidenceItem ? "" : "disabled"}>Attach evidence</button>
        </form>
      </article>
    </section>`;

  bindDataGridActions({
    grids: itemGrids,
    onRun: async (moduleCode) => {
      await runAdjustment(root, moduleCode);
      await renderAdjustments(env);
    },
    onImport: async (moduleCode, payload) => {
      await request(`${root}/${adjustmentModulePath(moduleCode)}`, {
        method: "POST",
        body: JSON.stringify(normalizeAdjustmentImportPayload(moduleCode, payload)),
      });
      await renderAdjustments(env);
    },
  });
  document.querySelectorAll("[data-run-adjustment]").forEach((button) => {
    button.addEventListener("click", async () => {
      await runAdjustment(root, button.dataset.runAdjustment);
      await renderAdjustments(env);
    });
  });
  document.getElementById("adjustmentEvidenceForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!evidenceItem) return;
    await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`, {
      method: "POST",
      body: JSON.stringify({
        file_name: document.getElementById("evidenceFileName").value,
        content_type: "application/pdf",
        storage_url: document.getElementById("evidenceStorageUrl").value,
        memo: "Uploaded from adjustment grid",
        uploaded_by: env.auth.user.login_id,
        adjustment_item_id: evidenceItem.adjustment_item_id,
      }),
    });
    await renderAdjustments(env);
  });
}

async function runAdjustmentLegacy(root, moduleCode) {
  if (moduleCode === "B1") {
    return request(`${root}/adjustments/income`, {
      method: "POST",
      body: JSON.stringify({
        accounting_income: 500000000,
        items: [
          { section: "ADD", item_code: "B1_SAMPLE_ADD", item_name: "Sample addback", amount: 12000000 },
          { section: "DEDUCT", item_code: "B1_SAMPLE_DEDUCT", item_name: "Sample deduction", amount: 3000000 },
        ],
      }),
    });
  }
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode);
  const path = adjustmentModulePath(moduleCode);
  return request(`${root}/${path}`, { method: "POST", body: JSON.stringify(sampleAdjustmentBody(code, family)) });
}

function adjustmentModulePath(moduleCode) {
  if (moduleCode === "B1") return "adjustments/income";
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode) || [];
  const path = {
    assets: `adjustments/assets/${code}`,
    transactions: `adjustments/transactions/${code}`,
    evaluation: `adjustments/evaluation/${code}`,
    tax: `adjustments/tax/${code}`,
    special: `adjustments/special/${code}`,
  }[family];
  if (!path) throw new Error(`Unsupported adjustment module: ${moduleCode}`);
  return path;
}

function normalizeAdjustmentImportPayload(moduleCode, payload) {
  if (moduleCode === "B1" && Array.isArray(payload)) {
    return {
      accounting_income: null,
      items: payload.map((item) => ({
        section: item.section || item.direction || "ADD",
        item_code: item.item_code || "B1_IMPORT",
        item_name: item.item_name || "Imported item",
        amount: Number(item.amount || 0),
      })),
    };
  }
  return payload;
}

function sampleAdjustmentBody(code, family) {
  if (family === "assets") {
    return code === "B10"
      ? { business_use_bps: 7200 }
      : { book_reserve: 90000000, estimated_liability: 65000000, external_fund: 10000000, receivable_balance: 500000000, rate_bps: 100 };
  }
  if (family === "transactions") {
    return {
      accounting_income: 500000000,
      taxable_income_before_donation: 480000000,
      gross_revenue: 3000000000,
      revenue_breakdowns: [{ revenue_category: "domestic", amount: 3000000000 }],
      weighted_average_loan_balance: 120000000,
      weighted_average_interest_rate_bps: 460,
    };
  }
  if (family === "evaluation") {
    if (code === "B11") return { taxable_income_before_loss: 200000000, loss_carryforwards: [{ origin_year: 2025, original_amount: 100000000, remaining_amount: 100000000, expires_year: 2035 }] };
    if (code === "B15") return { capital_changes: [{ change_date: today(), change_type: "PAID_IN_CAPITAL", amount: 50000000, description: "capital increase" }] };
    return { positions: [{ item_code: "FX01", item_name: "USD receivable", book_amount: 120000000, tax_amount: 100000000 }] };
  }
  if (family === "tax") {
    if (code === "B12") return { tax_base: 500000000, calculated_tax: 70000000, credits: [{ credit_type: "RND", base_amount: 100000000, rate_bps: 2500 }] };
    if (code === "B13") return { tax_base: 500000000, regular_tax_after_credits: 30000000, minimum_tax_rate_bps: 1000 };
    return { penalties: [{ penalty_type: "UNDER_REPORTED", tax_base: 100000000, rate_bps: 1000, days_late: 1, reduction_bps: 5000 }] };
  }
  if (code === "B16") {
    return { foreign_incomes: [{ income_type: "INTEREST", gross_amount: 100000000, attributable_expense: 20000000, pe_allocation_bps: 10000, withholding_tax: 5000000 }] };
  }
  return { consolidated_entities: [{ entity_code: "PARENT", entity_name: "Parent", ownership_bps: 10000, taxable_income: 100000000 }], eliminations: [] };
}

function renderAdjustmentModuleNavigator(selectedCode, locale) {
  return adjustmentModules.map(([code, label, family]) => `
    <article class="card ${code === selectedCode ? "active" : ""}" data-adjustment-card="${escapeHtml(code)}">
      <h3>${escapeHtml(code)} ${escapeHtml(label)}</h3>
      <p class="eyebrow">${escapeHtml(family)}</p>
      <div class="button-row">
        <button class="secondary-btn compact" type="button" data-adjustment-route="ws/adj:${escapeHtml(code)}">${escapeHtml(t(locale, "common.open"))}</button>
        <button class="primary-btn compact" type="button" data-run-adjustment="${escapeHtml(code)}">${escapeHtml(t(locale, "common.run"))}</button>
      </div>
    </article>`).join("");
}

function renderAdjustmentModuleHighlights(spec, context, locale) {
  const vehicleCount = context.vehicleLogs.length;
  const businessAssetCount = context.assets.filter((item) => item.is_business_vehicle).length;
  const rows = [
    ["Workflow status", statusLabel(context.workStatus || "DRAFT", locale)],
    ["Lock mode", context.lockMode || "OPEN"],
    ["Progress", `${context.progress ?? 0}%`],
    ...({
    B1: [["Accounting base", "FORM3 / income bridge"], ["Item workflow", "Addback, deduction, reserve"], ["Current items", money.format(context.currentRows.length)]],
    B2: [["Donation rows", money.format(context.transactions.filter((item) => item.category === "DONATION").length)], ["Carryforward", "Special/general donation tracking"], ["Limit", "Taxable income based"]],
    B3: [["Entertainment rows", money.format(context.transactions.filter((item) => item.category === "ENTERTAINMENT").length)], ["Limit", "Revenue based cap"], ["No-card check", "Receipt / card control"]],
    B4: [["Asset rows", money.format(context.assets.length)], ["Auto calc", "Useful life and tax law"], ["Reserve", "Depreciation gap tracking"]],
    B5: [["Reserve basis", "Book reserve vs estimated liability"], ["External fund", "Pension funding offset"], ["Result", "Reserve disposition"]],
    B6: [["Receivable base", "Bad debt cap by rate"], ["Rate input", "bps-based limit"], ["Output", "Reserve / write-off handling"]],
    B7: [["Position input", "FX monetary positions"], ["Comparison", "Book vs tax valuation"], ["Output", "Gain/loss adjustment"]],
    B8: [["Position input", "Inventory / securities valuation"], ["Comparison", "Book vs tax valuation"], ["Output", "Valuation reserve impact"]],
    B9: [["Interest rows", money.format(context.transactions.filter((item) => item.category === "INTEREST").length)], ["Loan average", "Weighted-average debt and rate"], ["Output", "Disallowed interest categories"]],
    B10: [["Vehicle assets", money.format(businessAssetCount)], ["Usage logs", money.format(vehicleCount)], ["Limit", "Business-use based addback"]],
    B11: [["Carryforward years", "Origin-year remaining balance"], ["Limit", "Deduction cap vs taxable income"], ["Output", "Usage and expiry trace"]],
    B12: [["Credit set", "Credit / reduction catalog"], ["Limit", "Calculated tax cap"], ["Output", "Credit impact on final tax"]],
    B13: [["Tax base", "Minimum tax comparison"], ["Input", "Regular tax after credits"], ["Output", "Additional minimum tax"]],
    B14: [["Penalty set", "Penalty type / delay / reduction"], ["Formula", "Base x rate x timing"], ["Output", "Penalty reflected in payable tax"]],
    B15: [["Capital changes", "Paid-in capital / earnings / reserve"], ["Linkage", "Capital and reserve schedule"], ["Output", "Reserve total and items"]],
    B16: [["Foreign income", "Income / expense / withholding"], ["Allocation", "PE allocation"], ["Output", "Domestic taxable base and tax"]],
    B17: [["Entity set", "Consolidated entity taxable income"], ["Elimination", "Intercompany elimination"], ["Output", "Consolidated tax base"]],
  }[spec.code] || []),
  ];
  return table(["Focus", "Detail"], rows.map(([left, right]) => row([escapeHtml(left), escapeHtml(right)])), t(locale, "grid.empty"));
}

function renderAdjustmentModuleForm(spec, context, locale) {
  const vehicleAsset = context.assets.find((item) => item.is_business_vehicle);
  const transaction = context.transactions[0];
  const forms = {
    B1: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Accounting income <input id="adjB1AccountingIncome" type="number" value="500000000" /></label><label>Addback amount <input id="adjB1AddbackAmount" type="number" value="12000000" /></label><label>Deduction amount <input id="adjB1DeductionAmount" type="number" value="3000000" /></label></form>`,
    B2: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Taxable income before donation <input id="adjB2TaxableIncome" type="number" value="480000000" /></label><label>Donation rows in source <input type="text" value="${escapeHtml(String(context.transactions.filter((item) => item.category === "DONATION").length))}" readonly /></label></form>`,
    B3: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Gross revenue <input id="adjB3GrossRevenue" type="number" value="3000000000" /></label><label>Revenue category <input id="adjB3RevenueCategory" value="domestic" /></label></form>`,
    B4: `<div class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><p class="empty">Uses imported assets and depreciation life data to calculate book-tax gaps automatically.</p></div>`,
    B5: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Book reserve <input id="adjB5BookReserve" type="number" value="90000000" /></label><label>Estimated liability <input id="adjB5EstimatedLiability" type="number" value="65000000" /></label><label>External fund <input id="adjB5ExternalFund" type="number" value="10000000" /></label></form>`,
    B6: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Book reserve <input id="adjB6BookReserve" type="number" value="5000000" /></label><label>Receivable balance <input id="adjB6ReceivableBalance" type="number" value="100000000" /></label><label>Rate (bps) <input id="adjB6RateBps" type="number" value="100" /></label></form>`,
    B7: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Position code <input id="adjB7PositionCode" value="FX01" /></label><label>Position name <input id="adjB7PositionName" value="USD receivable" /></label><label>Book amount <input id="adjB7BookAmount" type="number" value="120000000" /></label><label>Tax amount <input id="adjB7TaxAmount" type="number" value="100000000" /></label></form>`,
    B8: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Position code <input id="adjB8PositionCode" value="INV01" /></label><label>Position name <input id="adjB8PositionName" value="Inventory reserve" /></label><label>Book amount <input id="adjB8BookAmount" type="number" value="90000000" /></label><label>Tax amount <input id="adjB8TaxAmount" type="number" value="70000000" /></label></form>`,
    B9: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Weighted average loan balance <input id="adjB9LoanBalance" type="number" value="120000000" /></label><label>Weighted average interest rate (bps) <input id="adjB9InterestRateBps" type="number" value="460" /></label><label>Source transaction <input type="text" value="${escapeHtml(transaction?.partner_name || "Interest source rows")}" readonly /></label></form>`,
    B10: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Business use (bps) <input id="adjB10BusinessUseBps" type="number" value="7200" /></label><label>Vehicle asset <input type="text" value="${escapeHtml(vehicleAsset?.asset_name || "No vehicle asset")}" readonly /></label></form>`,
    B11: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Taxable income before loss <input id="adjB11TaxableIncome" type="number" value="200000000" /></label><label>Origin year <input id="adjB11OriginYear" type="number" value="2025" /></label><label>Original amount <input id="adjB11OriginalAmount" type="number" value="100000000" /></label><label>Remaining amount <input id="adjB11RemainingAmount" type="number" value="100000000" /></label><label>Expiry year <input id="adjB11ExpiryYear" type="number" value="2035" /></label></form>`,
    B12: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Tax base <input id="adjB12TaxBase" type="number" value="500000000" /></label><label>Calculated tax <input id="adjB12CalculatedTax" type="number" value="70000000" /></label><label>Credit type <input id="adjB12CreditType" value="RND" /></label><label>Credit base <input id="adjB12BaseAmount" type="number" value="100000000" /></label><label>Rate (bps) <input id="adjB12RateBps" type="number" value="2500" /></label></form>`,
    B13: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Tax base <input id="adjB13TaxBase" type="number" value="500000000" /></label><label>Regular tax after credits <input id="adjB13RegularTax" type="number" value="30000000" /></label><label>Minimum tax rate (bps) <input id="adjB13RateBps" type="number" value="1000" /></label></form>`,
    B14: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Penalty type <input id="adjB14PenaltyType" value="UNDER_REPORTED" /></label><label>Tax base <input id="adjB14TaxBase" type="number" value="100000000" /></label><label>Rate (bps) <input id="adjB14RateBps" type="number" value="1000" /></label><label>Days late <input id="adjB14DaysLate" type="number" value="1" /></label><label>Reduction (bps) <input id="adjB14ReductionBps" type="number" value="5000" /></label></form>`,
    B15: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Change type <input id="adjB15ChangeType" value="PAID_IN_CAPITAL" /></label><label>Change amount <input id="adjB15Amount" type="number" value="50000000" /></label><label>Description <input id="adjB15Description" value="capital increase" /></label></form>`,
    B16: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Income type <input id="adjB16IncomeType" value="INTEREST" /></label><label>Gross amount <input id="adjB16GrossAmount" type="number" value="100000000" /></label><label>Attributable expense <input id="adjB16Expense" type="number" value="20000000" /></label><label>PE allocation (bps) <input id="adjB16PeBps" type="number" value="10000" /></label><label>Withholding tax <input id="adjB16WithholdingTax" type="number" value="5000000" /></label></form>`,
    B17: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>Entity code <input id="adjB17EntityCode" value="PARENT" /></label><label>Entity name <input id="adjB17EntityName" value="Parent" /></label><label>Ownership (bps) <input id="adjB17OwnershipBps" type="number" value="10000" /></label><label>Taxable income <input id="adjB17TaxableIncome" type="number" value="100000000" /></label></form>`,
  };
  return `<div class="stack">${forms[spec.code] || ""}<div class="button-row"><button class="primary-btn" type="button" data-run-adjustment="${escapeHtml(spec.code)}">${escapeHtml(t(locale, "common.run"))}</button><button class="secondary-btn" type="button" data-adjustment-route="ws/form:linkage">Form linkage</button><button class="secondary-btn" type="button" data-adjustment-route="ws/val:issues">${escapeHtml(t(locale, "common.jump"))} validation</button></div></div>`;
}

function renderAdjustmentRunSummary(spec, lastRun, locale) {
  if (!lastRun) return `<p class="empty">Run the ${escapeHtml(spec.code)} module to populate calculation summary, law banner, and downstream tax impact.</p>`;
  return `${metrics([["Items", money.format(lastRun.items?.length || 0)], ["Addbacks", money.format(lastRun.addbacks || 0)], ["Deductions", money.format(lastRun.deductions || 0)], ["Snapshot", money.format(lastRun.snapshot_id || 0)]])}${table(["Field", "Value"], summarizeAdjustmentRunRows(lastRun).map(([label, value]) => row([escapeHtml(label), escapeHtml(value)])), t(locale, "grid.empty"))}`;
}

function summarizeAdjustmentRunRows(lastRun) {
  const rows = [["Module", lastRun.module_code || "-"], ["Law version", lastRun.law_banner?.law?.version_code || "-"], ["Reserves created", money.format(lastRun.reserves_created?.length || 0)]];
  if (typeof lastRun.calculated_tax === "number") rows.push(["Calculated tax", money.format(lastRun.calculated_tax)]);
  if (typeof lastRun.determined_tax === "number") rows.push(["Determined tax", money.format(lastRun.determined_tax)]);
  if (typeof lastRun.taxable_income === "number") rows.push(["Taxable income", money.format(lastRun.taxable_income)]);
  if (Array.isArray(lastRun.donation_carryforwards)) rows.push(["Donation carryforward", money.format(lastRun.donation_carryforwards.length)]);
  if (lastRun.details) rows.push(["Detail sections", money.format(Object.keys(lastRun.details).length)]);
  return rows;
}

function renderAdjustmentDataContext(context, locale) {
  return table(["Source", "Count"], [
    row(["Current module rows", money.format(context.currentRows.length)]),
    row(["Asset rows", money.format(context.assets.length)]),
    row(["Transaction rows", money.format(context.transactions.length)]),
    row(["Vehicle logs", money.format(context.vehicleLogs.length)]),
    row(["Work status", statusLabel(context.workStatus || "DRAFT", locale)]),
    row(["Lock mode", context.lockMode || "OPEN"]),
  ], t(locale, "grid.empty"));
}

function collectAdjustmentPayload(moduleCode) {
  switch (moduleCode) {
    case "B1":
      return { accounting_income: numberValue("adjB1AccountingIncome"), items: [{ section: "ADD", item_code: "B1_SAMPLE_ADD", item_name: "Sample addback", amount: numberValue("adjB1AddbackAmount") }, { section: "DEDUCT", item_code: "B1_SAMPLE_DEDUCT", item_name: "Sample deduction", amount: numberValue("adjB1DeductionAmount") }] };
    case "B2":
      return { taxable_income_before_donation: numberValue("adjB2TaxableIncome") };
    case "B3":
      return { gross_revenue: numberValue("adjB3GrossRevenue"), revenue_breakdowns: [{ revenue_category: textValue("adjB3RevenueCategory") || "domestic", amount: numberValue("adjB3GrossRevenue") }] };
    case "B4":
      return {};
    case "B5":
      return { book_reserve: numberValue("adjB5BookReserve"), estimated_liability: numberValue("adjB5EstimatedLiability"), external_fund: numberValue("adjB5ExternalFund") };
    case "B6":
      return { book_reserve: numberValue("adjB6BookReserve"), receivable_balance: numberValue("adjB6ReceivableBalance"), rate_bps: numberValue("adjB6RateBps") };
    case "B7":
      return { positions: [{ item_code: textValue("adjB7PositionCode"), item_name: textValue("adjB7PositionName"), book_amount: numberValue("adjB7BookAmount"), tax_amount: numberValue("adjB7TaxAmount"), monetary: true }] };
    case "B8":
      return { positions: [{ item_code: textValue("adjB8PositionCode"), item_name: textValue("adjB8PositionName"), book_amount: numberValue("adjB8BookAmount"), tax_amount: numberValue("adjB8TaxAmount"), monetary: false }] };
    case "B9":
      return { weighted_average_loan_balance: numberValue("adjB9LoanBalance"), weighted_average_interest_rate_bps: numberValue("adjB9InterestRateBps") };
    case "B10":
      return { business_use_bps: numberValue("adjB10BusinessUseBps") };
    case "B11":
      return { taxable_income_before_loss: numberValue("adjB11TaxableIncome"), loss_carryforwards: [{ origin_year: numberValue("adjB11OriginYear"), original_amount: numberValue("adjB11OriginalAmount"), remaining_amount: numberValue("adjB11RemainingAmount"), expires_year: numberValue("adjB11ExpiryYear") }] };
    case "B12":
      return { tax_base: numberValue("adjB12TaxBase"), calculated_tax: numberValue("adjB12CalculatedTax"), credits: [{ credit_type: textValue("adjB12CreditType"), base_amount: numberValue("adjB12BaseAmount"), rate_bps: numberValue("adjB12RateBps") }] };
    case "B13":
      return { tax_base: numberValue("adjB13TaxBase"), regular_tax_after_credits: numberValue("adjB13RegularTax"), minimum_tax_rate_bps: numberValue("adjB13RateBps") };
    case "B14":
      return { penalties: [{ penalty_type: textValue("adjB14PenaltyType"), tax_base: numberValue("adjB14TaxBase"), rate_bps: numberValue("adjB14RateBps"), days_late: numberValue("adjB14DaysLate"), reduction_bps: numberValue("adjB14ReductionBps") }] };
    case "B15":
      return { capital_changes: [{ change_date: today(), change_type: textValue("adjB15ChangeType"), amount: numberValue("adjB15Amount"), description: textValue("adjB15Description") }] };
    case "B16":
      return { foreign_incomes: [{ income_type: textValue("adjB16IncomeType"), gross_amount: numberValue("adjB16GrossAmount"), attributable_expense: numberValue("adjB16Expense"), pe_allocation_bps: numberValue("adjB16PeBps"), withholding_tax: numberValue("adjB16WithholdingTax") }] };
    case "B17":
      return { consolidated_entities: [{ entity_code: textValue("adjB17EntityCode"), entity_name: textValue("adjB17EntityName"), ownership_bps: numberValue("adjB17OwnershipBps"), taxable_income: numberValue("adjB17TaxableIncome") }], eliminations: [] };
    default:
      return sampleAdjustmentBody(moduleCode, adjustmentTaxonomy.find((item) => item.code === moduleCode)?.module);
  }
}

function numberValue(id) {
  return Number(document.getElementById(id)?.value || 0);
}

function textValue(id) {
  return document.getElementById(id)?.value?.trim() || "";
}

async function renderAdjustments(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const selectedCode = env.leafSuffix || "B1";
  const selectedModule = adjustmentTaxonomy.find((item) => item.code === selectedCode) || adjustmentTaxonomy[0];
  const [adjustments, reserves, history, currentRows, assets, transactions, vehicleLogs] = await Promise.all([
    request(`${root}/adjustments`),
    request(`${root}/reserves`),
    request(`${root}/adjustments/history`).catch(() => []),
    request(`${root}/${adjustmentModulePath(selectedCode)}`).catch(() => []),
    request(`${root}/tax-data/assets`).catch(() => []),
    request(`${root}/tax-data/transactions`).catch(() => []),
    request(`${root}/vehicle-usage-logs`).catch(() => []),
  ]);
  const itemGrids = {
    [selectedCode]: { rows: currentRows },
  };
  const shellContext = {
    assets,
    transactions,
    vehicleLogs,
    currentRows,
    workStatus: env.context?.status || "DRAFT",
    lockMode: env.context?.lockMode || "OPEN",
    progress: env.context?.progress ?? 0,
  };
  const evidenceItem = currentRows[0];
  const evidenceAttachments = evidenceItem
    ? await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`).catch(() => [])
    : [];
  const lastRun = adjustmentRunState.get(selectedCode) || null;
  env.outlet.innerHTML = `
    <section class="leaf-workbench adjustment-workbench" data-stage="adjustment" data-module-code="${escapeHtml(selectedCode)}">
      ${metrics([
        ["Adjustments", adjustments.length],
        ["Reserves", reserves.length],
        ["Addbacks", money.format(adjustments.filter((item) => item.direction === "ADD").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
        ["Deductions", money.format(adjustments.filter((item) => item.direction === "DEDUCT").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Adjustment workbench</span>
            <h2>${escapeHtml(selectedModule.code)} ${escapeHtml(selectedModule.ko)}</h2>
            <p>${escapeHtml(selectedModule.en)} / ${escapeHtml(selectedModule.module)}</p>
          </div>
          <div>
            <p>${escapeHtml(statusLabel(shellContext.workStatus, env.locale))} / ${escapeHtml(shellContext.lockMode)}</p>
            <p>${escapeHtml(t(env.locale, "field.progress"))} ${escapeHtml(shellContext.progress)}%</p>
          </div>
        </div>
        <div class="grid four adjustment-module-grid">
          ${renderAdjustmentModuleNavigator(selectedCode, env.locale)}
        </div>
      </article>
      <section class="grid two adjustment-shell">
        <article class="panel">
          <div class="panel-head">
            <div><h2>Module shell</h2><p>${escapeHtml(selectedModule.code)} / ${escapeHtml(t(env.locale, `route.ws.adj.${selectedModule.code}`))}</p></div>
            <div class="button-row">
              <button class="primary-btn compact" type="button" data-run-adjustment="${escapeHtml(selectedCode)}">Run module</button>
              <button class="secondary-btn compact" type="button" data-adjustment-route="ws/form:form3">FORM3</button>
              <button class="secondary-btn compact" type="button" data-adjustment-route="ws/form:linkage">Linkage</button>
            </div>
          </div>
          ${renderAdjustmentModuleHighlights(selectedModule, shellContext, env.locale)}
          ${renderAdjustmentModuleForm(selectedModule, shellContext, env.locale)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Run summary</h2><p>Last calculation and downstream impact</p></div>
          ${renderAdjustmentRunSummary(selectedModule, lastRun, env.locale)}
          ${renderAdjustmentDataContext(shellContext, env.locale)}
        </article>
      </section>
      <section class="grid two">
        ${renderDataGrid({ id: selectedCode, title: `${selectedCode} ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, `route.ws.adj.${selectedCode}`), rows: currentRows, columns: adjustmentGridColumns, importable: true, runLabelKey: "common.run", locale: env.locale })}
        <article class="panel">
          <div class="panel-head"><h2>Adjustment results</h2></div>
          ${table(["Code", "Direction", "Amount", "Status"], adjustments.map((item) => row([
            escapeHtml(item.adj_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.status),
          ])))}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Reserve summary</h2></div>
          ${table(["Code", "Direction", "Amount", "Module"], reserves.map((item) => row([
            escapeHtml(item.reserve_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.source_module),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Adjustment History</h2></div>
          ${table(["Module", "Action", "Item", "Changed"], history.slice(0, 20).map((item) => row([
            escapeHtml(item.source_module),
            escapeHtml(item.action),
            escapeHtml(item.new_data?.item_code || item.old_data?.item_code || "-"),
            escapeHtml(item.changed_at),
          ])))}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Evidence Attachments</h2></div>
        ${table(["File", "Type", "Storage URL", "Uploaded"], evidenceAttachments.map((item) => row([
          escapeHtml(item.file_name),
          escapeHtml(item.content_type),
          escapeHtml(item.storage_url || "-"),
          escapeHtml(item.created_at),
        ])))}
        <form id="adjustmentEvidenceForm" class="stack">
          <label>File name <input id="evidenceFileName" value="evidence.pdf" /></label>
          <label>Storage URL <input id="evidenceStorageUrl" value="evidence/${Date.now()}.pdf" /></label>
          <button class="primary-btn" type="submit" ${evidenceItem ? "" : "disabled"}>Attach evidence</button>
        </form>
      </article>
    </section>`;

  bindDataGridActions({
    grids: itemGrids,
    onRun: async (moduleCode) => {
      const result = await runAdjustment(root, moduleCode, collectAdjustmentPayload(moduleCode));
      adjustmentRunState.set(moduleCode, result);
      await renderAdjustments(env);
    },
    onImport: async (moduleCode, payload) => {
      await request(`${root}/${adjustmentModulePath(moduleCode)}`, {
        method: "POST",
        body: JSON.stringify(normalizeAdjustmentImportPayload(moduleCode, payload)),
      });
      await renderAdjustments(env);
    },
  });
  document.querySelectorAll("[data-run-adjustment]").forEach((button) => {
    button.addEventListener("click", async () => {
      const moduleCode = button.dataset.runAdjustment;
      const result = await runAdjustment(root, moduleCode, collectAdjustmentPayload(moduleCode));
      adjustmentRunState.set(moduleCode, result);
      await renderAdjustments(env);
    });
  });
  document.querySelectorAll("[data-adjustment-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.adjustmentRoute));
  });
  document.getElementById("adjustmentEvidenceForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!evidenceItem) return;
    await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`, {
      method: "POST",
      body: JSON.stringify({
        file_name: document.getElementById("evidenceFileName").value,
        content_type: "application/pdf",
        storage_url: document.getElementById("evidenceStorageUrl").value,
        memo: "Uploaded from adjustment grid",
        uploaded_by: env.auth.user.login_id,
        adjustment_item_id: evidenceItem.adjustment_item_id,
      }),
    });
    await renderAdjustments(env);
  });
}

async function runAdjustment(root, moduleCode, payload = null) {
  if (moduleCode === "B1") {
    return request(`${root}/adjustments/income`, {
      method: "POST",
      body: JSON.stringify(payload || sampleAdjustmentBody("B1", "income")),
    });
  }
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode);
  const path = adjustmentModulePath(moduleCode);
  return request(`${root}/${path}`, { method: "POST", body: JSON.stringify(payload || sampleAdjustmentBody(code, family)) });
}

async function renderFormsLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [attachments, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/FORM3/preview`).catch(() => null),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>별지/부속서류</h2><div class="button-row">
          ${["FORM3", "FORM15", "FORM22", "FORM32", "FORM50"].map((code) => `<button class="primary-btn compact" data-generate-form="${code}" type="button">${code}</button>`).join("")}
          <button id="downloadForms" class="secondary-btn compact" type="button">ZIP</button>
        </div></div>
        ${table(["서식", "상태", "검증", "금액"], attachments.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.status),
          money.format(item.validation_count),
          money.format(item.total_amount),
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>별지 3호 미리보기</h2><button id="downloadForm3" class="secondary-btn compact" type="button">PDF</button></div>
        ${preview ? table(["필드", "값", "원천"], preview.fields.map((field) => row([
          escapeHtml(field.label),
          escapeHtml(field.value),
          escapeHtml(field.source),
        ]))) : "<p class=\"empty\">FORM3 생성 전입니다.</p>"}
      </article>
    </section>`;
  document.querySelectorAll("[data-generate-form]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/forms/${button.dataset.generateForm}`, { method: "POST", body: "{}" });
      await renderForms(env);
    });
  });
  document.getElementById("downloadForms").addEventListener("click", () => downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip"));
  document.getElementById("downloadForm3").addEventListener("click", () => downloadBinary(`${root}/forms/FORM3/pdf`, "FORM3.pdf"));
}

async function renderValidationLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [rules, taxData, efile] = await Promise.all([
    request(`${routeRoot(env)}/validation/rules`),
    request(`${root}/tax-data/validation`),
    request(`${root}/efilings/precheck`).catch(() => null),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["규칙", rules.length], ["차변/대변", taxData.balanced ? "일치" : "불일치"], ["전자신고", efile?.valid ? "가능" : "확인 필요"], ["오류", "-"]])}
      <article class="panel">
        <div class="panel-head"><h2>통합 검증</h2><button id="runValidation" class="primary-btn" type="button">실행</button></div>
        <div id="validationResult">${renderValidationOverview(taxData, efile)}</div>
      </article>
    </section>`;
  document.getElementById("runValidation").addEventListener("click", async () => {
    const result = await request(`${root}/validation/run`, { method: "POST", body: "{}" });
    document.getElementById("validationResult").innerHTML = renderValidationResult(root, result);
    bindDismissButtons(root);
  });
}

function renderValidationResultLegacy(root, result) {
  return `
    ${metrics([["실행 규칙", result.executed_rules], ["오류", result.error_count], ["경고", result.warn_count], ["정보", result.info_count]])}
    ${table(["등급", "규칙", "메시지", ""], result.issues.map((issue) => row([
      `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
      escapeHtml(issue.rule_code),
      escapeHtml(issue.message),
      `<button class="secondary-btn compact" data-dismiss-issue="${issue.issue_id}" type="button">무시</button>`,
    ])), "검증 이슈가 없습니다.")}
    <p class="empty">검증 결과: ${result.pass ? "통과" : "확인 필요"}</p>`;
}

function bindDismissButtons(root) {
  document.querySelectorAll("[data-dismiss-issue]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/validation/issues/${button.dataset.dismissIssue}/dismiss`, {
        method: "POST",
        body: JSON.stringify({ reason: "user dismissed from validation screen" }),
      });
      button.closest("tr").remove();
    });
  });
}

async function renderApprovalLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [queue, workflow] = await Promise.all([
    request(`${routeRoot(env)}/workflow/queue?assignee=me`),
    request(`${root}/workflow`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>결재 대기함</h2></div>
        ${table(["고객사", "사업연도", "대기일"], queue.map((item) => row([escapeHtml(item.customer_name), escapeHtml(item.year_label), `${item.pending_days}일`])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>결재 처리</h2></div>
        <form id="workflowForm" class="stack">
          <label>의견 <textarea id="workflowComment">검토 완료</textarea></label>
          <label>Approvers <input id="workflowApprovers" value="${escapeHtml(env.auth.user.login_id)}" /></label>
          <div class="button-row">
            <button class="secondary-btn" type="button" data-status="IN_REVIEW">결재 요청</button>
            <button class="primary-btn" type="button" data-status="APPROVED">승인</button>
            <button class="danger-btn" type="button" data-status="DRAFT">반려</button>
          </div>
        </form>
        ${table(["작업", "상태", "사용자", "의견"], workflow.events.map((event) => row([escapeHtml(event.action), escapeHtml(event.to_status), escapeHtml(event.actor), escapeHtml(event.comment || "-")])))} 
      </article>
    </section>`;
  document.querySelectorAll("[data-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const updated = await request(`${root}/status`, {
        method: "POST",
        body: JSON.stringify({ status: button.dataset.status, actor: env.auth.user.login_id, approver: env.auth.user.login_id, approvers: document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean), comment: document.getElementById("workflowComment").value }),
      });
      env.setContext({ status: updated.status, progress: progressForStatus(updated.status), lockMode: updated.locked_at ? "LOCKED" : "OPEN" });
      await renderApproval(env);
    });
  });
}

async function renderPrintLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [attachments, printHistory] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/print-history`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>출력</h2><button id="printBundle" class="primary-btn" type="button">일괄 ZIP</button></div>
        ${table(["서식", "상태", "PDF"], attachments.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.status),
          `<button class="secondary-btn compact" data-download-form="${escapeHtml(item.form_code)}" type="button">다운로드</button>`,
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Print History</h2></div>
        ${table(["Form", "Watermark", "Printed by", "Printed at"], printHistory.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.watermark),
          escapeHtml(item.printed_by),
          escapeHtml(item.printed_at),
        ])))}
      </article>
    </section>`;
  document.getElementById("printBundle").addEventListener("click", () => downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip"));
  document.querySelectorAll("[data-download-form]").forEach((button) => {
    button.addEventListener("click", () => downloadBinary(`${root}/forms/${button.dataset.downloadForm}/pdf`, `${button.dataset.downloadForm}.pdf`));
  });
}

async function renderEfilingLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [spec, precheck, history] = await Promise.all([
    request(`${root}/efilings/format-spec`),
    request(`${root}/efilings/precheck`),
    request(`${root}/efilings`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["레코드", precheck.record_count], ["검증", precheck.valid ? "통과" : "확인"], ["체크섬", precheck.checksum_preview], ["파일", history.length]])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>전자신고 생성</h2><button id="createEfile" class="primary-btn" type="button">생성</button></div>
          ${env.auth.user.use_2fa ? `<label>OTP <input id="efileOtp" inputmode="numeric" autocomplete="one-time-code" placeholder="2FA code" /></label>` : ""}
          ${table(["코드", "등급", "메시지"], asArray(precheck.issues).map((issue) => row([escapeHtml(issue.validation_code), escapeHtml(issue.severity), escapeHtml(issue.message)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>생성 이력</h2></div>
          ${table(["ID", "상태", "파일"], history.map((item) => row([
            escapeHtml(item.efiling_id),
            escapeHtml(item.status),
            `<button class="secondary-btn compact" data-download-efile="${item.efiling_id}" type="button">다운로드</button>`,
          ])))}
        </article>
      </section>
      <article class="panel">${table(["레코드", "필드", "길이", "원천"], spec.slice(0, 30).map((field) => row([escapeHtml(field.record_type), escapeHtml(field.field_name), escapeHtml(field.byte_length), escapeHtml(field.source_path || "-")])))}</article>
    </section>`;
  document.getElementById("createEfile").addEventListener("click", async () => {
    await request(`${root}/efilings`, { method: "POST", body: JSON.stringify({ max_attempts: 3, otp: document.getElementById("efileOtp")?.value || null }) });
    await renderEfiling(env);
  });
  document.querySelectorAll("[data-download-efile]").forEach((button) => {
    button.addEventListener("click", () => downloadBinary(`${routeRoot(env)}/efilings/${button.dataset.downloadEfile}/file`, `efiling-${button.dataset.downloadEfile}.txt`));
  });
}

async function renderPostHistoryLegacy(env) {
  const root = routeRoot(env);
  const years = await request(`${root}/business-years`);
  const efilings = hasWorkContext(env.context) ? await request(`${workRoot(env)}/efilings`) : [];
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>사업연도</h2></div>
        ${table(["ID", "사업연도", "상태", "잠금"], years.map((by) => row([escapeHtml(by.by_id), escapeHtml(by.year_label), pill(by.status), escapeHtml(by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN"))])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>전자신고 이력</h2></div>
        ${table(["접수 ID", "상태", "레코드", "체크섬"], efilings.map((item) => row([escapeHtml(item.efiling_id), escapeHtml(item.status), escapeHtml(item.total_records), escapeHtml(item.checksum)])))}
      </article>
    </section>`;
}

async function renderPostAmendLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const preview = await request(`${root}/amendment-preview`);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>차이 미리보기</h2></div>
        ${table(["영역", "필드", "현재"], asArray(preview.differences).map((item) => row([escapeHtml(item.area), escapeHtml(item.field), escapeHtml(JSON.stringify(item.current_value))])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>잠금 해제</h2></div>
        <form id="unlockForm" class="stack">
          <label>버전 기준
            <select id="unlockMode"><option value="FILED_VERSION">신고시점 버전</option><option value="CURRENT">최신 버전</option></select>
          </label>
          <label>사유 <textarea id="unlockReason">수정신고 착수</textarea></label>
          <button class="primary-btn" type="submit">해제</button>
        </form>
      </article>
    </section>`;
  document.getElementById("unlockForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${root}/unlock`, {
      method: "POST",
      body: JSON.stringify({ reason: document.getElementById("unlockReason").value, version_mode: document.getElementById("unlockMode").value, actor: env.auth.user.login_id }),
    });
    env.setContext({ byId: by.by_id, fy: String(by.year_label || env.context.fy || ""), period: `${by.start_date || ""} ~ ${by.end_date || ""}`, status: by.status, progress: progressForStatus(by.status), lockMode: by.lock_mode || "AMENDMENT_UNLOCK" });
    await renderPostAmend(env);
  });
}

function statusIn(status, allowed) {
  return allowed.includes(String(status || "").toUpperCase());
}

function renderStageRouteButtons(activeLeaf, routes, locale) {
  return routes.map((routeKey) => `
    <button class="${routeKey === activeLeaf ? "primary-btn" : "secondary-btn"} compact" type="button" data-stage-route="${escapeHtml(routeKey)}">
      ${escapeHtml(t(locale, routeKeyToLabelKey(routeKey)))}
    </button>`).join("");
}

function formatWorkbenchValue(value) {
  if (value == null) return "-";
  if (typeof value === "number") return money.format(value);
  if (typeof value === "boolean") return value ? "Y" : "N";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function parseManualFieldValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return "";
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw === "null") return null;
  try {
    if ((raw.startsWith("{") && raw.endsWith("}")) || (raw.startsWith("[") && raw.endsWith("]"))) {
      return JSON.parse(raw);
    }
  } catch {}
  return raw;
}

function formSourceLeaf(field) {
  const ref = String(field?.source_ref || field?.source || field?.field_path || "").toLowerCase();
  if (ref.includes("form")) return "ws/form:preview";
  if (ref.includes("adjust") || ref.includes("reserve")) return "ws/adj:B1";
  if (ref.includes("asset")) return "ws/info:assets";
  if (ref.includes("vehicle")) return "ws/info:vehicle";
  if (ref.includes("transaction") || ref.includes("revenue") || ref.includes("expense")) return "ws/info:transactions";
  return "ws/info:fs";
}

function validationIssueLeaf(issue) {
  const target = String(issue?.target_path || issue?.area || issue?.message || "").toLowerCase();
  if (/b1[0-7]|adjust|reserve/.test(target)) return "ws/adj:B1";
  if (target.includes("form")) return "ws/form:preview";
  if (target.includes("vehicle")) return "ws/info:vehicle";
  if (target.includes("asset")) return "ws/info:assets";
  if (target.includes("transaction")) return "ws/info:transactions";
  return "ws/info:fs";
}

function efilingIssueLeaf(issue) {
  const target = String(issue?.field_path || issue?.message || "").toLowerCase();
  if (target.includes("biz") || target.includes("corp")) return "ws/info:fs";
  if (target.includes("tax") || target.includes("form3")) return "ws/form:preview";
  return "ws/val:issues";
}

function validationCounts(issues) {
  return {
    errors: issues.filter((issue) => issue.severity === "ERROR" && issue.status !== "DISMISSED").length,
    warns: issues.filter((issue) => issue.severity === "WARN" && issue.status !== "DISMISSED").length,
    infos: issues.filter((issue) => issue.severity === "INFO" && issue.status !== "DISMISSED").length,
  };
}

function renderValidationResult(result) {
  return `
    ${metrics([["Executed rules", result.executed_rules], ["Errors", result.error_count], ["Warnings", result.warn_count], ["Infos", result.info_count]])}
    ${table(["Severity", "Rule", "Message"], asArray(result.issues).map((issue) => row([
      `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
      escapeHtml(issue.rule_code),
      escapeHtml(issue.message),
    ])), "No validation issues.")}
    <p class="empty">Validation status: ${result.pass ? "PASS" : "ACTION REQUIRED"}</p>`;
}

async function renderForms(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const selectedFormCode = env.context?.selectedFormCode || "FORM3";
  const [attachments, linkage, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/linkage-check`).catch(() => ({ balanced: false, differences: [] })),
    request(`${root}/forms/${selectedFormCode}/preview`).catch(() => null),
  ]);
  const selectedAttachment = attachments.find((item) => item.form_code === selectedFormCode) || attachments[0] || null;
  const editableFields = asArray(preview?.fields).filter((field) => field.editable).slice(0, 6);
  const stageRoutes = ["ws/form:form3", "ws/form:attachments", "ws/form:preview", "ws/form:linkage"];
  const canPrint = statusIn(env.context?.status, ["APPROVED", "FILED", "AMENDED"]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench forms-workbench" data-stage="forms" data-form-code="${escapeHtml(selectedFormCode)}">
      ${metrics([
        ["Forms", attachments.length],
        ["Generated", attachments.filter((item) => item.generated).length],
        ["Linkage", linkage?.balanced ? "BALANCED" : "CHECK"],
        ["Preview validations", money.format(asArray(preview?.validations).length)],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Form review workbench</span>
            <h2>${escapeHtml(selectedFormCode)} review</h2>
            <p>${escapeHtml(statusLabel(env.context?.status || "DRAFT", env.locale))} / ${escapeHtml(env.context?.lockMode || "OPEN")}</p>
          </div>
          <div class="button-row">
            ${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}
          </div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head">
            <div><h2>Form catalog</h2><p>Generate, review, and print by form.</p></div>
            <div class="button-row">
              <select id="selectedFormCode">${attachments.map((item) => `<option value="${item.form_code}" ${item.form_code === selectedFormCode ? "selected" : ""}>${escapeHtml(item.form_code)}</option>`).join("")}</select>
              <button class="primary-btn compact" type="button" data-generate-form="${escapeHtml(selectedFormCode)}">Generate</button>
              <button class="secondary-btn compact" type="button" data-form-pdf="${escapeHtml(selectedFormCode)}" ${canPrint ? "" : "disabled"}>PDF</button>
            </div>
          </div>
          ${table(["Form", "Status", "Validations", "Amount", "Updated"], attachments.map((item) => row([
            `<button class="link-btn" type="button" data-select-form="${escapeHtml(item.form_code)}">${escapeHtml(item.form_code)}</button>`,
            escapeHtml(item.status),
            money.format(item.validation_count),
            money.format(item.total_amount),
            escapeHtml(item.updated_at || "-"),
          ])), "No forms generated yet.")}
          <p class="empty">${canPrint ? "Approved or filed work can be printed." : "PDF output is gated until approval is complete."}</p>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Preview and source</h2><p>${escapeHtml(selectedAttachment?.form_name || selectedFormCode)}</p></div>
          ${preview ? table(["Field", "Value", "Source", ""], asArray(preview.fields).slice(0, 12).map((field) => row([
            escapeHtml(field.label),
            escapeHtml(formatWorkbenchValue(field.value)),
            escapeHtml(field.source_ref || field.source || "-"),
            `<button class="secondary-btn compact" type="button" data-form-source-jump="${escapeHtml(formSourceLeaf(field))}">Jump</button>`,
          ])), "No preview fields.") : '<p class="empty">Generate the selected form to review preview fields.</p>'}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Validation and linkage</h2><p>Preview validation results and form-to-form deltas.</p></div>
          ${table(["Severity", "Rule", "Message"], asArray(preview?.validations).map((issue) => row([
            `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
            escapeHtml(issue.rule_code),
            escapeHtml(issue.message),
          ])), "No form validation issues.")}
          ${table(["Source", "Target", "Delta"], asArray(linkage?.differences).map((item) => row([
            escapeHtml(item.source),
            escapeHtml(item.target),
            escapeHtml(formatWorkbenchValue(item.delta)),
          ])), "No linkage differences.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Manual overrides</h2><p>Editable preview fields write back as manual overrides.</p></div>
          ${editableFields.length ? `
            <form id="formOverrideForm" class="stack" data-form-override="${escapeHtml(selectedFormCode)}">
              ${editableFields.map((field) => `<label>${escapeHtml(field.label)} <input data-form-edit-field="${escapeHtml(field.field_path)}" value="${escapeHtml(formatWorkbenchValue(field.value))}" /></label>`).join("")}
              <label>Reason <textarea id="formOverrideReason">Manual review adjustment</textarea></label>
              <button class="primary-btn" type="submit">Save overrides</button>
            </form>` : '<p class="empty">No editable fields in the current preview.</p>'}
          ${preview ? table(["Change", "By", "Reason", "At"], asArray(preview.history).slice(0, 10).map((item) => row([
            escapeHtml(item.change_type),
            escapeHtml(item.changed_by),
            escapeHtml(item.reason || "-"),
            escapeHtml(item.changed_at),
          ])), "No form edit history.") : ""}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.getElementById("selectedFormCode")?.addEventListener("change", (event) => {
    env.setContext({ selectedFormCode: event.target.value });
    renderForms(env);
  });
  document.querySelectorAll("[data-select-form]").forEach((button) => button.addEventListener("click", () => {
    env.setContext({ selectedFormCode: button.dataset.selectForm });
    renderForms(env);
  }));
  document.querySelectorAll("[data-form-source-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.formSourceJump)));
  document.querySelectorAll("[data-generate-form]").forEach((button) => button.addEventListener("click", async () => {
    await request(`${root}/forms/${button.dataset.generateForm}`, { method: "POST", body: "{}" });
    await renderForms(env);
  }));
  document.querySelectorAll("[data-form-pdf]").forEach((button) => button.addEventListener("click", () => {
    if (!button.disabled) downloadBinary(`${root}/forms/${button.dataset.formPdf}/pdf`, `${button.dataset.formPdf}.pdf`);
  }));
  document.getElementById("formOverrideForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const fields = Object.fromEntries([...document.querySelectorAll("[data-form-edit-field]")].map((input) => [input.dataset.formEditField, parseManualFieldValue(input.value)]));
    await request(`${root}/forms/${selectedFormCode}`, {
      method: "PUT",
      body: JSON.stringify({ fields, reason: document.getElementById("formOverrideReason").value, changed_by: env.auth.user.login_id }),
    });
    await renderForms(env);
  });
}

async function renderValidation(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const [rules, taxData, efile, issues] = await Promise.all([
    request(`${routeRoot(env)}/validation/rules`),
    request(`${root}/tax-data/validation`),
    request(`${root}/efilings/precheck`).catch(() => null),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const byId = env.context?.byId || env.context?.businessYearId || "default";
  const lastResult = validationRunState.get(byId) || null;
  const counts = validationCounts(asArray(issues));
  const approvalBlocked = counts.errors > 0;
  const stageRoutes = ["ws/val:run", "ws/val:issues", "ws/val:rules"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench validation-workbench" data-stage="validation">
      ${metrics([
        ["Rules", rules.length],
        ["Open errors", counts.errors],
        ["Open warnings", counts.warns],
        ["E-file precheck", efile?.valid ? "READY" : "CHECK"],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge ${approvalBlocked ? "warn" : "ok"}">${approvalBlocked ? "Validation blocked" : "Ready for approval"}</span>
            <h2>Validation triage</h2>
            <p>${approvalBlocked ? "Approval request is blocked until open errors are cleared." : "No validation error is currently blocking approval."}</p>
          </div>
          <div class="button-row">
            ${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}
            <button id="runValidation" class="primary-btn compact" type="button">Run validation</button>
            <button id="jumpApproval" class="secondary-btn compact" type="button" ${approvalBlocked ? "disabled" : ""}>Request approval</button>
          </div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Run result</h2><p>Tax data consistency and filing readiness.</p></div>
          <div id="validationResult">${lastResult ? renderValidationResult(lastResult) : renderValidationOverview(taxData, efile)}</div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Issue triage</h2><p>Dismiss non-blocking issues or jump to the source screen.</p></div>
          ${table(["Severity", "Rule", "Message", "Status", ""], asArray(issues).map((issue) => row([
            `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
            escapeHtml(issue.rule_code || "-"),
            escapeHtml(issue.message),
            escapeHtml(issue.status || "OPEN"),
            `<div class="button-row"><button class="secondary-btn compact" type="button" data-validation-jump="${escapeHtml(validationIssueLeaf(issue))}">Jump</button><button class="secondary-btn compact" type="button" data-dismiss-issue="${escapeHtml(issue.issue_id)}">Dismiss</button></div>`,
          ])), "No validation issues.")}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Rule catalog</h2><p>Sample of active validation rules.</p></div>
          ${table(["Rule", "Severity", "Area", "Active"], rules.slice(0, 20).map((rule) => row([
            escapeHtml(rule.rule_code),
            escapeHtml(rule.severity),
            escapeHtml(rule.area),
            rule.active ? "Y" : "N",
          ])), "No rules loaded.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Gate summary</h2><p>Approval and filing depend on these checks.</p></div>
          ${table(["Check", "State"], [
            row(["Financial data balanced", taxData.balanced ? "OK" : "CHECK"]),
            row(["Asset rows", money.format(taxData.asset_count || 0)]),
            row(["Vehicle logs", money.format(taxData.business_vehicle_count || 0)]),
            row(["Transaction rows", money.format(taxData.transaction_count || 0)]),
            row(["E-file precheck", efile?.valid ? "READY" : "CHECK"]),
          ])}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.getElementById("runValidation")?.addEventListener("click", async () => {
    const result = await request(`${root}/validation/run`, { method: "POST", body: "{}" });
    validationRunState.set(byId, result);
    await renderValidation(env);
  });
  document.getElementById("jumpApproval")?.addEventListener("click", () => env.navigate("ws/appr:request"));
  document.querySelectorAll("[data-validation-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.validationJump)));
  document.querySelectorAll("[data-dismiss-issue]").forEach((button) => button.addEventListener("click", async () => {
    await request(`${root}/validation/issues/${button.dataset.dismissIssue}/dismiss`, {
      method: "POST",
      body: JSON.stringify({ reason: "dismissed from validation workbench", dismissed_by: env.auth.user.login_id }),
    });
    await renderValidation(env);
  }));
}

async function renderApproval(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const [queue, workflow, issues] = await Promise.all([
    request(`${routeRoot(env)}/workflow/queue?assignee=me`),
    request(`${root}/workflow`),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const counts = validationCounts(asArray(issues));
  const approvalBlocked = counts.errors > 0;
  const stageRoutes = ["ws/appr:request", "ws/appr:inbox", "ws/appr:rejected"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench approval-workbench" data-stage="approval">
      ${metrics([
        ["Queue", queue.length],
        ["Approval lines", asArray(workflow.approval_lines).length],
        ["Open errors", counts.errors],
        ["Current status", statusLabel(env.context?.status || workflow.business_year?.status || "DRAFT", env.locale)],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge ${approvalBlocked ? "warn" : "ok"}">${approvalBlocked ? "Validation blocked" : "Ready for approval"}</span>
            <h2>Approval workflow</h2>
            <p>${approvalBlocked ? "Validation errors must be resolved before requesting approval." : "Approval request and decision actions are available."}</p>
          </div>
          <div class="button-row">${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Approval request</h2><p>Request review, approve, or return to draft.</p></div>
          <form id="workflowForm" class="stack">
            <label>Comment <textarea id="workflowComment">Validation reviewed and ready for approval.</textarea></label>
            <label>Approvers <input id="workflowApprovers" value="${escapeHtml(asArray(workflow.approval_lines).map((line) => line.approver_login_id).join(",") || env.auth.user.login_id)}" /></label>
            <div class="button-row">
              <button class="secondary-btn" type="button" id="requestWorkflow" ${approvalBlocked ? "disabled" : ""}>Request review</button>
              <button class="primary-btn" type="button" data-status="APPROVED" ${approvalBlocked ? "disabled" : ""}>Approve</button>
              <button class="danger-btn" type="button" data-status="DRAFT">Return to draft</button>
            </div>
          </form>
          ${table(["Approver", "Status", "Acted at", "Comment"], asArray(workflow.approval_lines).map((line) => row([
            escapeHtml(line.approver_login_id),
            escapeHtml(line.status),
            escapeHtml(line.acted_at || "-"),
            escapeHtml(line.comment || "-"),
          ])), "No approval lines.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Queue and timeline</h2><p>My queue and business year workflow events.</p></div>
          ${table(["Customer", "Year", "Status", "Pending days"], queue.map((item) => row([
            escapeHtml(item.customer_name),
            escapeHtml(item.year_label),
            escapeHtml(item.status),
            money.format(item.pending_days),
          ])), "No items in queue.")}
          ${table(["Action", "From", "To", "Actor", "Comment"], asArray(workflow.events).map((event) => row([
            escapeHtml(event.action),
            escapeHtml(event.from_status || "-"),
            escapeHtml(event.to_status),
            escapeHtml(event.actor),
            escapeHtml(event.comment || "-"),
          ])), "No workflow events.")}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.getElementById("requestWorkflow")?.addEventListener("click", async () => {
    await request(`${root}/workflow/request`, {
      method: "POST",
      body: JSON.stringify({
        approvers: document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean),
        comment: document.getElementById("workflowComment").value,
        requested_by: env.auth.user.login_id,
      }),
    });
    const updated = await request(`${root}/status`, {
      method: "POST",
      body: JSON.stringify({ status: "IN_REVIEW", actor: env.auth.user.login_id, approver: env.auth.user.login_id, approvers: document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean), comment: document.getElementById("workflowComment").value }),
    });
    env.setContext({ status: updated.status, progress: progressForStatus(updated.status), lockMode: updated.locked_at ? "LOCKED" : "OPEN" });
    await renderApproval(env);
  });
  document.querySelectorAll("[data-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const updated = await request(`${root}/status`, {
        method: "POST",
        body: JSON.stringify({ status: button.dataset.status, actor: env.auth.user.login_id, approver: env.auth.user.login_id, approvers: document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean), comment: document.getElementById("workflowComment").value }),
      });
      env.setContext({ status: updated.status, progress: progressForStatus(updated.status), lockMode: updated.locked_at ? "LOCKED" : "OPEN" });
      await renderApproval(env);
    });
  });
}

async function renderPrint(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const selectedFormCode = env.context?.selectedPrintFormCode || env.context?.selectedFormCode || "FORM3";
  const [attachments, printHistory, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/print-history`).catch(() => []),
    request(`${root}/forms/${selectedFormCode}/preview`).catch(() => null),
  ]);
  const printable = statusIn(env.context?.status, ["APPROVED", "FILED", "AMENDED"]);
  const watermark = statusIn(env.context?.status, ["FILED"]) ? "FILED" : statusIn(env.context?.status, ["APPROVED"]) ? "APPROVED" : statusIn(env.context?.status, ["AMENDED"]) ? "AMENDED" : "DRAFT";
  const stageRoutes = ["ws/print:preview", "ws/print:bulk", "ws/print:history"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench print-workbench" data-stage="print" data-print-form="${escapeHtml(selectedFormCode)}">
      ${metrics([
        ["Printable forms", attachments.filter((item) => item.generated).length],
        ["Watermark", watermark],
        ["Print history", printHistory.length],
        ["Workflow status", statusLabel(env.context?.status || "DRAFT", env.locale)],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge ${printable ? "ok" : "warn"}">${printable ? "PDF ready" : "Approval required"}</span>
            <h2>PDF output</h2>
            <p>${printable ? "Individual PDF and bundle output are enabled." : "PDF output is disabled until approval is complete."}</p>
          </div>
          <div class="button-row">
            ${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}
            <button id="printBundle" class="primary-btn compact" type="button" ${printable ? "" : "disabled"}>Print bundle</button>
          </div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Printable forms</h2><p>Select a form and download PDF output.</p></div>
          <label>Form
            <select id="selectedPrintFormCode">${attachments.map((item) => `<option value="${item.form_code}" ${item.form_code === selectedFormCode ? "selected" : ""}>${escapeHtml(item.form_code)}</option>`).join("")}</select>
          </label>
          ${table(["Form", "Status", "Validations", ""], attachments.map((item) => row([
            escapeHtml(item.form_code),
            escapeHtml(item.status),
            money.format(item.validation_count),
            `<button class="secondary-btn compact" data-download-form="${escapeHtml(item.form_code)}" type="button" ${printable ? "" : "disabled"}>PDF</button>`,
          ])), "No form attachments.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Preview snapshot</h2><p>Watermark: ${escapeHtml(watermark)}</p></div>
          ${preview ? table(["Field", "Value", "Source"], asArray(preview.fields).slice(0, 10).map((field) => row([
            escapeHtml(field.label),
            escapeHtml(formatWorkbenchValue(field.value)),
            escapeHtml(field.source_ref || field.source || "-"),
          ])), "No preview fields.") : '<p class="empty">No preview is available for the selected form.</p>'}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Print history</h2><p>Audit trail of downloaded output files.</p></div>
        ${table(["Form", "Watermark", "Printed by", "File", "Printed at"], asArray(printHistory).map((item) => row([
          escapeHtml(item.form_code || "-"),
          escapeHtml(item.watermark || "-"),
          escapeHtml(item.printed_by || "-"),
          escapeHtml(item.file_name || "-"),
          escapeHtml(item.created_at || item.printed_at || "-"),
        ])), "No print history.")}
      </article>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.getElementById("selectedPrintFormCode")?.addEventListener("change", (event) => {
    env.setContext({ selectedPrintFormCode: event.target.value });
    renderPrint(env);
  });
  document.getElementById("printBundle")?.addEventListener("click", () => {
    if (!printable) return;
    downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip");
  });
  document.querySelectorAll("[data-download-form]").forEach((button) => button.addEventListener("click", () => {
    if (!printable) return;
    downloadBinary(`${root}/forms/${button.dataset.downloadForm}/pdf`, `${button.dataset.downloadForm}.pdf`);
  }));
}

async function renderEfiling(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const [spec, precheck, history, latest] = await Promise.all([
    request(`${root}/efilings/format-spec`),
    request(`${root}/efilings/precheck`),
    request(`${root}/efilings`),
    request(`${root}/efilings/latest`).catch(() => null),
  ]);
  const latestHistory = history[0] || latest || null;
  const filedLocked = statusIn(env.context?.status, ["FILED"]);
  const efileEnabled = statusIn(env.context?.status, ["APPROVED", "AMENDED"]);
  const stageRoutes = ["ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench efiling-workbench" data-stage="efiling">
      ${metrics([
        ["Record count", precheck.record_count],
        ["Precheck", precheck.valid ? "READY" : "CHECK"],
        ["Checksum", precheck.checksum_preview],
        ["Files", history.length],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge ${efileEnabled ? "ok" : "warn"}">${efileEnabled ? "Filing open" : filedLocked ? "Filed locked" : "Approval required"}</span>
            <h2>E-filing wizard</h2>
            <p>${efileEnabled ? "Generate the text file, submit it, and lock the business year as filed." : filedLocked ? "The filed business year is locked for e-filing." : "E-filing remains blocked until approval is complete."}</p>
          </div>
          <div class="button-row">${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Precheck</h2><p>Issues must be resolved before generation.</p></div>
          ${env.auth.user.use_2fa ? `<label>OTP <input id="efileOtp" inputmode="numeric" autocomplete="one-time-code" placeholder="2FA code" /></label>` : ""}
          ${table(["Code", "Severity", "Message", ""], asArray(precheck.issues).map((issue) => row([
            escapeHtml(issue.validation_code),
            escapeHtml(issue.severity),
            escapeHtml(issue.message),
            `<button class="secondary-btn compact" type="button" data-efile-jump="${escapeHtml(efilingIssueLeaf(issue))}">Jump</button>`,
          ])), "No precheck issues.")}
          <div class="button-row">
            <button id="createEfile" class="primary-btn" type="button" ${(efileEnabled && precheck.valid) ? "" : "disabled"}>Generate file</button>
            <button id="goPrint" class="secondary-btn" type="button">Open print stage</button>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Submission</h2><p>Latest generated file and receipt status.</p></div>
          ${latestHistory ? table(["Field", "Value"], [
            row(["E-filing id", escapeHtml(latestHistory.efiling_id)]),
            row(["Status", escapeHtml(latestHistory.status)]),
            row(["Records", escapeHtml(latestHistory.total_records || precheck.record_count)]),
            row(["Checksum", escapeHtml(latestHistory.checksum || precheck.checksum_preview)]),
            row(["Submitted at", escapeHtml(latestHistory.submitted_at || "-")]),
          ]) : '<p class="empty">Generate a filing file first.</p>'}
          <div class="button-row">
            <button id="submitEfile" class="primary-btn" type="button" ${(efileEnabled && latestHistory) ? "" : "disabled"}>Submit and lock</button>
            <button id="downloadLatestEfile" class="secondary-btn" type="button" ${latestHistory ? "" : "disabled"}>Download latest</button>
          </div>
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Filing history</h2><p>Generated files and status timeline.</p></div>
          ${table(["ID", "Status", "Records", "Checksum", ""], history.map((item) => row([
            escapeHtml(item.efiling_id),
            escapeHtml(item.status),
            escapeHtml(item.total_records),
            escapeHtml(item.checksum),
            `<button class="secondary-btn compact" data-download-efile="${item.efiling_id}" type="button">Download</button>`,
          ])), "No e-filing history.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Format spec</h2><p>Representative excerpt of the e-file layout.</p></div>
          ${table(["Record", "Field", "Length", "Source"], spec.slice(0, 20).map((field) => row([
            escapeHtml(field.record_type),
            escapeHtml(field.field_name),
            escapeHtml(field.byte_length),
            escapeHtml(field.source_path || "-"),
          ])), "No format spec rows.")}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.querySelectorAll("[data-efile-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.efileJump)));
  document.getElementById("goPrint")?.addEventListener("click", () => env.navigate("ws/print:preview"));
  document.getElementById("createEfile")?.addEventListener("click", async () => {
    await request(`${root}/efilings`, { method: "POST", body: JSON.stringify({ max_attempts: 3, otp: document.getElementById("efileOtp")?.value || null }) });
    await renderEfiling(env);
  });
  document.getElementById("submitEfile")?.addEventListener("click", async () => {
    if (!latestHistory) return;
    await request(`${root}/efilings/${latestHistory.efiling_id}/submit`, {
      method: "POST",
      body: JSON.stringify({ otp: document.getElementById("efileOtp")?.value || null, actor: env.auth.user.login_id }),
    });
    const by = await request(`${root}/status`, {
      method: "POST",
      body: JSON.stringify({ status: "FILED", actor: env.auth.user.login_id, approver: env.auth.user.login_id, comment: "e-filing submitted" }),
    });
    env.setContext({ status: by.status, progress: progressForStatus(by.status), lockMode: by.locked_at ? "LOCKED" : "OPEN" });
    await renderEfiling(env);
  });
  document.getElementById("downloadLatestEfile")?.addEventListener("click", () => {
    if (!latestHistory) return;
    downloadBinary(`${routeRoot(env)}/efilings/${latestHistory.efiling_id}/file`, `efiling-${latestHistory.efiling_id}.txt`);
  });
  document.querySelectorAll("[data-download-efile]").forEach((button) => button.addEventListener("click", () => {
    downloadBinary(`${routeRoot(env)}/efilings/${button.dataset.downloadEfile}/file`, `efiling-${button.dataset.downloadEfile}.txt`);
  }));
}

async function renderPostHistory(env) {
  const root = routeRoot(env);
  const workRootPath = hasWorkContext(env.context) ? workRoot(env) : null;
  const [years, efilings, notifications] = await Promise.all([
    request(`${root}/business-years`),
    workRootPath ? request(`${workRootPath}/efilings`).catch(() => []) : Promise.resolve([]),
    request(`${root}/notifications`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-history-workbench" data-stage="post-history">
      ${metrics([
        ["Business years", years.length],
        ["Filed years", years.filter((by) => by.status === "FILED").length],
        ["E-filing history", efilings.length],
        ["Notifications", notifications.length],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Business year timeline</h2><p>Track filed, amended, and open years.</p></div>
          ${table(["ID", "Year", "Status", "Lock", ""], years.map((by) => row([
            escapeHtml(by.by_id),
            escapeHtml(by.year_label),
            pill(by.status, env.locale),
            escapeHtml(by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN")),
            `<button class="secondary-btn compact" type="button" data-open-amend="${escapeHtml(by.by_id)}">Amend</button>`,
          ])), "No business years.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Filed artifacts</h2><p>Latest e-filing output for the active work context.</p></div>
          ${table(["Receipt id", "Status", "Records", "Checksum"], efilings.map((item) => row([
            escapeHtml(item.efiling_id),
            escapeHtml(item.status),
            escapeHtml(item.total_records),
            escapeHtml(item.checksum),
          ])), "No e-filing history in the current work context.")}
          ${table(["Severity", "Title", "Status"], notifications.slice(0, 8).map((item) => row([
            escapeHtml(item.severity),
            escapeHtml(item.title),
            escapeHtml(item.status),
          ])), "No post-filing notifications.")}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-open-amend]").forEach((button) => button.addEventListener("click", () => env.navigate("post/amend:unlock")));
}

async function renderPostAmend(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const [preview, versionMode] = await Promise.all([
    request(`${root}/amendment-preview`),
    request(`${root}/amendment-version-mode`).catch(() => ({ mode: "AMENDMENT", versions: [] })),
  ]);
  const stageRoutes = ["post/amend:unlock", "post/amend:version", "post/amend:diff", "post/amend:resubmit", "post/correction"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend">
      ${metrics([
        ["Current status", escapeHtml(preview.current_status)],
        ["Locked", preview.locked ? "Y" : "N"],
        ["Differences", asArray(preview.differences).length],
        ["Version mode", escapeHtml(versionMode.mode || "AMENDMENT")],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge ${preview.locked ? "warn" : "ok"}">${preview.locked ? "Locked return" : "Unlocked for amendment"}</span>
            <h2>Post-filing amendment</h2>
            <p>Unlock, compare, choose a version basis, and resubmit the amended return.</p>
          </div>
          <div class="button-row">${renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Difference report</h2><p>Current state compared with the filed baseline.</p></div>
          ${table(["Area", "Field", "Original", "Current", "Description"], asArray(preview.differences).map((item) => row([
            escapeHtml(item.area),
            escapeHtml(item.field),
            escapeHtml(formatWorkbenchValue(item.original_value)),
            escapeHtml(formatWorkbenchValue(item.current_value)),
            escapeHtml(item.description || "-"),
          ])), "No amendment differences.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Unlock and version basis</h2><p>Choose the amendment baseline before reopening the filed year.</p></div>
          <form id="unlockForm" class="stack">
            <label>Version mode
              <select id="unlockMode">
                <option value="FILED_VERSION">Filed version</option>
                <option value="CURRENT">Current latest</option>
              </select>
            </label>
            <label>Reason <textarea id="unlockReason">Amendment filing kickoff</textarea></label>
            <button class="primary-btn" type="submit" ${preview.locked ? "" : "disabled"}>Unlock for amendment</button>
          </form>
          ${table(["Version", "Label"], asArray(versionMode.versions).map((item) => row([
            escapeHtml(item.version),
            escapeHtml(item.label),
          ])), "No version metadata.")}
          <div class="button-row">
            <button id="resubmitAmendment" class="secondary-btn" type="button">Resubmit amendment</button>
            <button id="goValidationFromAmend" class="secondary-btn" type="button">Open validation</button>
          </div>
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.getElementById("unlockForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${root}/unlock`, {
      method: "POST",
      body: JSON.stringify({ reason: document.getElementById("unlockReason").value, version_mode: document.getElementById("unlockMode").value, actor: env.auth.user.login_id }),
    });
    env.setContext({ byId: by.by_id, fy: String(by.year_label || env.context.fy || ""), period: `${by.start_date || ""} ~ ${by.end_date || ""}`, status: by.status, progress: progressForStatus(by.status), lockMode: by.lock_mode || "AMENDMENT_UNLOCK" });
    await renderPostAmend(env);
  });
  document.getElementById("resubmitAmendment")?.addEventListener("click", async () => {
    await request(`${root}/resubmit`, {
      method: "POST",
      body: JSON.stringify({ actor: env.auth.user.login_id, reason: "amendment resubmission", version_mode: document.getElementById("unlockMode")?.value || "CURRENT" }),
    });
    await renderPostAmend(env);
  });
  document.getElementById("goValidationFromAmend")?.addEventListener("click", () => env.navigate("ws/val:run"));
}

async function renderAlerts(env) {
  const root = routeRoot(env);
  const notifications = await request(`${root}/notifications`);
  env.outlet.innerHTML = `
    <section class="panel">
      <div class="panel-head"><h2>알림 센터</h2></div>
      ${table(["등급", "제목", "상태", ""], notifications.map((item) => row([
        `<span class="badge ${item.severity === "WARN" ? "warn" : "info"}">${escapeHtml(item.severity)}</span>`,
        escapeHtml(item.title),
        escapeHtml(item.status),
        `<button class="secondary-btn compact" data-read-notification="${item.notification_id}" type="button">읽음</button>`,
      ])))}
    </section>`;
  document.querySelectorAll("[data-read-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/notifications/${button.dataset.readNotification}`, { method: "PATCH", body: JSON.stringify({ status: "READ" }) });
      await renderAlerts(env);
    });
  });
}

async function renderYearCompare(env) {
  const rows = await request(`${routeRoot(env)}/reports/year-comparison`);
  const max = Math.max(1, ...rows.map((item) => Math.abs(Number(item.total_adjustment_amount || 0))));
  env.outlet.innerHTML = `
    <section class="panel">
      <div class="panel-head"><h2>사업연도 비교</h2></div>
      <div class="mini-chart">
        ${rows.map((item) => chartRow(`${item.customer_id} / ${item.year_label}`, item.total_adjustment_amount, max)).join("")}
      </div>
      ${table(["고객사", "사업연도", "상태", "조정합계", "유보"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), escapeHtml(item.status), money.format(item.total_adjustment_amount), money.format(item.reserve_count)])))}
    </section>`;
}

async function renderTaxBurden(env) {
  const [rows, industry] = await Promise.all([
    request(`${routeRoot(env)}/reports/tax-burden`),
    request(`${routeRoot(env)}/reports/industry-statistics`).catch(() => []),
  ]);
  const max = Math.max(1, ...rows.map((item) => Number(item.total_tax_due || 0)));
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>세부담 분석</h2></div>
        <div class="mini-chart">${rows.map((item) => chartRow(`${item.customer_id} / ${item.year_label}`, item.total_tax_due, max)).join("")}</div>
        ${table(["고객사", "사업연도", "과세표준", "총부담세액", "실효세율"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), money.format(item.taxable_income), money.format(item.total_tax_due), `${(item.effective_tax_rate_bps / 100).toFixed(2)}%`])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Industry Statistics</h2></div>
        ${table(["Industry", "SME", "Customers", "Avg tax"], industry.map((item) => row([
          escapeHtml(item.industry_code),
          item.is_sme ? "Y" : "N",
          money.format(item.customer_count),
          money.format(item.average_tax_due),
        ])))}
      </article>
    </section>`;
}

async function renderReserveTrend(env) {
  const root = routeRoot(env);
  const [rows, lossExpiry, userReports] = await Promise.all([
    request(`${root}/reports/reserve-trend`),
    request(`${root}/reports/loss-expiry`).catch(() => []),
    request(`${root}/reports/user-defined`).catch(() => []),
  ]);
  const max = Math.max(1, ...rows.map((item) => Number(item.amount || 0)));
  env.outlet.innerHTML = `
    <section class="grid">
      <article class="panel">
        <div class="panel-head"><h2>유보 잔액 추이</h2></div>
        <div class="mini-chart">${rows.map((item) => chartRow(`${item.reserve_code} / ${item.year_label}`, item.amount, max)).join("")}</div>
        ${table(["고객사", "사업연도", "유보코드", "구분", "금액"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), escapeHtml(item.reserve_code), escapeHtml(item.direction), money.format(item.amount)])))}
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Loss Expiry</h2></div>
          ${table(["Customer", "Origin", "Expires", "Remaining"], lossExpiry.map((item) => row([
            escapeHtml(item.customer_name),
            escapeHtml(item.origin_year),
            escapeHtml(item.expires_year),
            money.format(item.remaining_amount),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>User Reports</h2><button id="createLossReport" class="primary-btn compact" type="button">Loss report</button></div>
          ${table(["Name", "Source", "Updated"], userReports.map((item) => row([
            escapeHtml(item.report_name),
            escapeHtml(item.source),
            escapeHtml(item.updated_at),
          ])))}
        </article>
      </section>
    </section>`;
  document.getElementById("createLossReport").addEventListener("click", async () => {
    await request(`${root}/reports/user-defined`, {
      method: "POST",
      body: JSON.stringify({ report_name: `Loss expiry ${today()}`, source: "LOSS_EXPIRY", columns: ["customer_name", "origin_year", "expires_year", "remaining_amount"], filters: {} }),
    });
    await renderReserveTrend(env);
  });
}

function chartRow(label, value, max) {
  const width = Math.min(100, Math.round((Math.abs(Number(value || 0)) / max) * 100));
  return `<div class="chart-row"><strong>${escapeHtml(label)}</strong><div class="bar-track"><span style="width:${width}%"></span></div><span>${money.format(value || 0)}</span></div>`;
}

async function renderAdminTenants(env) {
  const tenants = await request("/api/tenants");
  const canManage = env.auth?.user?.roles?.includes("SUPER_ADMIN");
  const locale = env.locale;
  const planCounts = tenants.reduce((acc, item) => {
    acc[item.plan || "STANDARD"] = (acc[item.plan || "STANDARD"] || 0) + 1;
    return acc;
  }, {});
  env.outlet.innerHTML = `
    <section class="leaf-workbench leaf-typology" data-typology="grid" data-leaf-key="admin/tenant:list">
      <section class="panel leaf-summary" data-leaf-block="summary">
      <div class="panel-head">
        <div><span class="badge info">${escapeHtml(t(locale, "leaf.workbench"))}</span><h2>${escapeHtml(t(locale, "route.admin.tenant.list"))}</h2><p>admin/tenant:list / admin:READ</p></div>
      </div>
      ${metrics([
        [t(locale, "field.tenantName"), tenants.length],
        [statusLabel("ACTIVE", locale), tenants.filter((item) => item.status === "ACTIVE").length],
        [statusLabel("SUSPENDED", locale), tenants.filter((item) => item.status === "SUSPENDED").length],
        ["ENTERPRISE", planCounts.ENTERPRISE || 0],
      ])}
      </section>
      <article class="panel leaf-table" data-leaf-block="table">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(locale, "route.admin.tenant.list"))}</h2><p>${escapeHtml(t(locale, "leaf.count", { count: tenants.length, description: t(locale, "typology.grid.description") }))}</p></div>
          <div class="panel-head-actions" data-leaf-block="toolbar">
          <div data-leaf-block="filters">
            <label>${escapeHtml(t(locale, "common.search"))} <input type="search" data-tenant-filter="q" placeholder="${escapeHtml(t(locale, "field.tenantCode"))}/${escapeHtml(t(locale, "field.tenantName"))}" /></label>
            <label>${escapeHtml(t(locale, "field.status"))} <select data-tenant-filter="status"><option value="ALL">${escapeHtml(statusLabel("ALL", locale))}</option><option value="ACTIVE">${escapeHtml(statusLabel("ACTIVE", locale))}</option><option value="SUSPENDED">${escapeHtml(statusLabel("SUSPENDED", locale))}</option><option value="CLOSED">${escapeHtml(statusLabel("CLOSED", locale))}</option></select></label>
            <button class="secondary-btn compact" type="button" data-tenant-filter-reset>${escapeHtml(t(locale, "common.reset"))}</button>
          </div>
            <button class="primary-btn compact" type="submit" form="tenantForm" ${canManage ? "" : "disabled"}>${escapeHtml(t(locale, "common.addPrefix"))}</button>
          </div>
        </div>
        ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "field.status"), t(locale, "field.plan"), t(locale, "field.maxUsers"), t(locale, "common.actions")], tenants.map((item) => row([
          escapeHtml(item.tenant_code),
          escapeHtml(item.tenant_name),
          canManage ? `<select data-tenant-status="${escapeHtml(item.tenant_code)}"><option value="ACTIVE" ${item.status === "ACTIVE" ? "selected" : ""}>${escapeHtml(statusLabel("ACTIVE", locale))}</option><option value="SUSPENDED" ${item.status === "SUSPENDED" ? "selected" : ""}>${escapeHtml(statusLabel("SUSPENDED", locale))}</option><option value="CLOSED" ${item.status === "CLOSED" ? "selected" : ""}>${escapeHtml(statusLabel("CLOSED", locale))}</option></select>` : pill(item.status, locale),
          canManage ? `<select data-tenant-plan="${escapeHtml(item.tenant_code)}"><option ${item.plan === "FREE" ? "selected" : ""}>FREE</option><option ${item.plan === "STANDARD" ? "selected" : ""}>STANDARD</option><option ${item.plan === "PRO" ? "selected" : ""}>PRO</option><option ${item.plan === "ENTERPRISE" ? "selected" : ""}>ENTERPRISE</option></select>` : escapeHtml(item.plan || "STANDARD"),
          escapeHtml(item.max_users),
          canManage ? `<button class="secondary-btn compact" type="button" data-save-tenant="${escapeHtml(item.tenant_code)}">${escapeHtml(t(locale, "common.save"))}</button>` : "",
        ])))}
      </article>
      <article class="panel tenant-create-panel">
        <div class="panel-head"><h2>${escapeHtml(t(locale, "common.add"))} ${escapeHtml(t(locale, "field.tenantName"))}</h2><span class="badge info">${escapeHtml(t(locale, "common.addPrefix"))}</span></div>
        <form id="tenantForm" class="stack">
          <label>${escapeHtml(t(locale, "field.code"))} <input id="tenantCodeInput" value="tenant${Date.now().toString(36).slice(-4)}" /></label>
          <label>${escapeHtml(t(locale, "field.name"))} <input id="tenantNameInput" value="${escapeHtml(t(locale, "common.add"))} ${escapeHtml(t(locale, "field.tenantName"))}" /></label>
          <label>${escapeHtml(t(locale, "field.bizRegNo"))} <input id="tenantBizInput" value="1234567890" /></label>
          <label>${escapeHtml(t(locale, "field.plan"))} <select id="tenantPlanInput"><option>STANDARD</option><option>PRO</option><option>ENTERPRISE</option><option>FREE</option></select></label>
          <label>${escapeHtml(t(locale, "field.allowedIps"))} <input id="tenantAllowedIpsInput" placeholder="203.0.113.10/32" /></label>
          <label>${escapeHtml(t(locale, "field.contractStart"))} <input id="tenantStartInput" type="date" value="${today()}" /></label>
        </form>
      </article>
    </section>`;
  const applyTenantFilters = () => {
    const query = document.querySelector('[data-tenant-filter="q"]')?.value.toLowerCase() || "";
    const status = document.querySelector('[data-tenant-filter="status"]')?.value || "ALL";
    document.querySelectorAll('[data-leaf-block="table"] tbody tr').forEach((tr) => {
      const text = tr.textContent.toLowerCase();
      const rowStatus = tr.querySelector("[data-tenant-status]")?.value || tr.children[2]?.textContent.trim() || "";
      tr.style.display = (!query || text.includes(query)) && (status === "ALL" || rowStatus === status) ? "" : "none";
    });
  };
  document.querySelectorAll("[data-tenant-filter]").forEach((control) => control.addEventListener("input", applyTenantFilters));
  document.querySelector("[data-tenant-filter-reset]")?.addEventListener("click", () => {
    const q = document.querySelector('[data-tenant-filter="q"]');
    const status = document.querySelector('[data-tenant-filter="status"]');
    if (q) q.value = "";
    if (status) status.value = "ALL";
    applyTenantFilters();
  });
  document.querySelectorAll("[data-save-tenant]").forEach((button) => {
    button.addEventListener("click", async () => {
      const code = button.dataset.saveTenant;
      const status = document.querySelector(`[data-tenant-status="${CSS.escape(code)}"]`)?.value;
      const plan = document.querySelector(`[data-tenant-plan="${CSS.escape(code)}"]`)?.value;
      await request(`/api/tenants/${code}/status`, { method: "PATCH", body: JSON.stringify({ status }) });
      await request(`/api/tenants/${code}/plan`, { method: "PATCH", body: JSON.stringify({ plan }) });
      await renderAdminTenants(env);
    });
  });
  document.getElementById("tenantForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!canManage) return;
    await request("/api/tenants", {
      method: "POST",
      body: JSON.stringify({ tenant_code: document.getElementById("tenantCodeInput").value, tenant_name: document.getElementById("tenantNameInput").value, biz_reg_no: document.getElementById("tenantBizInput").value, contract_start: document.getElementById("tenantStartInput").value, contract_end: null, allowed_ips: document.getElementById("tenantAllowedIpsInput").value || null, max_users: 10, plan: document.getElementById("tenantPlanInput").value }),
    });
    await renderAdminTenants(env);
  });
}

async function renderAdminCustomers(env) {
  const root = routeRoot(env);
  const customers = await request(`${root}/customers`);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">${table(["코드", "고객사", "사업자번호", "범위"], customers.map((item) => row([escapeHtml(item.customer_code), escapeHtml(item.customer_name), escapeHtml(item.biz_reg_no), escapeHtml(asArray(item.work_scopes).join(", "))])))}</article>
      <article class="panel">
        <div class="panel-head"><h2>고객사 등록</h2></div>
        <form id="customerForm" class="stack">
          <label>코드 <input id="custCode" value="C${Date.now().toString(36).slice(-4).toUpperCase()}" /></label>
          <label>이름 <input id="custName" value="신규 고객사" /></label>
          <label>사업자번호 <input id="custBiz" value="2208112345" /></label>
          <button class="primary-btn" type="submit">등록</button>
        </form>
      </article>
    </section>`;
  document.getElementById("customerForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/customers`, {
      method: "POST",
      body: JSON.stringify({ customer_code: document.getElementById("custCode").value, customer_name: document.getElementById("custName").value, biz_reg_no: document.getElementById("custBiz").value, corp_reg_no: null, industry_code: "62010", is_sme: true, work_scopes: ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"] }),
    });
    await renderAdminCustomers(env);
  });
}

async function renderAdminUsers(env) {
  const root = routeRoot(env);
  const [users, customers] = await Promise.all([request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users`), request(`${root}/customers`)]);
  const firstCustomer = customers[0];
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">${table(["ID", "이름", "상태", "2FA", "역할", ""], users.map((item) => row([
        escapeHtml(item.login_id),
        escapeHtml(item.user_name),
        escapeHtml(item.status),
        item.use_2fa ? "Y" : "N",
        escapeHtml(asArray(item.roles).join(", ")),
        `<button class="secondary-btn compact" data-unlock-user="${escapeHtml(item.login_id)}" type="button">Unlock</button>`,
      ])))}</article>
      <article class="panel">
        <div class="panel-head"><h2>사용자 등록</h2></div>
        <form id="userForm" class="stack">
          <label>ID <input id="userLogin" value="u${Date.now().toString(36).slice(-4)}" /></label>
          <label>이름 <input id="userName" value="세무 담당자" /></label>
          <label>비밀번호 <input id="userPassword" value="ChangeMe123!" /></label>
          <label><input id="userUse2fa" type="checkbox" /> Use 2FA</label>
          <label>TOTP Secret <input id="userTotpSecret" placeholder="Base32 or raw secret" /></label>
          <button class="primary-btn" type="submit" ${firstCustomer ? "" : "disabled"}>등록</button>
        </form>
      </article>
    </section>`;
  document.getElementById("userForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users`, {
      method: "POST",
      body: JSON.stringify({ login_id: document.getElementById("userLogin").value, password: document.getElementById("userPassword").value, user_name: document.getElementById("userName").value, use_2fa: document.getElementById("userUse2fa").checked, totp_secret: document.getElementById("userTotpSecret").value || null, roles: ["TAX_EXPERT"], customer_access: [{ customer_id: firstCustomer.customer_id, access_level: "OWNER", is_primary: true, work_scopes: ["INFO", "ADJUST", "FORM"] }] }),
    });
    await renderAdminUsers(env);
  });
  document.querySelectorAll("[data-unlock-user]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users/${button.dataset.unlockUser}/status`, {
        method: "POST",
        body: JSON.stringify({ status: "ACTIVE", locked: false, reason: "admin unlock" }),
      });
      await renderAdminUsers(env);
    });
  });
}

function renderAdminRouteButtons(activeLeaf, routes, locale) {
  return routes.map((key) => {
    const meta = routeMeta(key, locale);
    return `<button class="${key === activeLeaf ? "primary-btn" : "secondary-btn"} compact" type="button" data-admin-route="${escapeHtml(key)}">${escapeHtml(meta.title)}</button>`;
  }).join("");
}

async function renderAdminRolesWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const stageRoutes = ["admin/sec:users", "admin/sec:roles", "admin/sec:matrix", "admin/sec:mask", "admin/sec:scope"];
  const [roles, permissions, functionCodes, roleMenuFunctions, maskingPolicies, dataScopes, loginHistory, systemSettings] = await Promise.all([
    request("/api/admin/roles"),
    request("/api/admin/role-permissions"),
    request("/api/admin/function-codes").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
    request("/api/admin/field-masking").catch(() => []),
    request("/api/admin/data-scope").catch(() => []),
    request("/api/login-history").catch(() => []),
    request("/api/system-settings").catch(() => []),
  ]);
  const selectedRole = roles.find((item) => item.role_code === "TAX_EXPERT") || roles[0] || null;
  const rolePermissions = permissions.filter((item) => item.role_code === selectedRole?.role_code);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security">
      ${metrics([
        ["Roles", roles.length],
        ["Permissions", permissions.length],
        ["Mask rules", maskingPolicies.length],
        ["Session policy", systemSettings.find((item) => item.setting_key === "session_timeout_minutes")?.setting_value || "-"],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Security and permission controls</h2>
            <p>Manage role matrix, masking, data scope, and authentication policy inputs.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Role catalog</h2><p>System roles and assigned permission volume.</p></div>
          ${table(["Role", "Name", "System", "Permissions"], roles.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.role_name),
            item.system_role ? "Y" : "N",
            money.format(permissions.filter((permission) => permission.role_code === item.role_code).length),
          ])), "No roles found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Permission matrix</h2><button id="saveExpertPerm" class="primary-btn compact" type="button">${escapeHtml(selectedRole?.role_code || "Role")} baseline</button></div>
          ${table(["Role", "Module", "Function", "Effect"], rolePermissions.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.module_code),
            escapeHtml(item.function_code),
            escapeHtml(item.effect),
          ])), "No permissions for the selected role.")}
          <p class="empty">Role-menu bindings: ${escapeHtml(String(roleMenuFunctions.filter((item) => item.role_code === selectedRole?.role_code).length))}</p>
        </article>
      </section>
      <section class="grid three">
        <article class="panel">
          <div class="panel-head"><h2>Function codes</h2><p>Action catalog used across modules and menus.</p></div>
          ${table(["Code", "Name", "Sort"], functionCodes.map((item) => row([
            escapeHtml(item.function_code),
            escapeHtml(item.function_name),
            escapeHtml(item.sort_order),
          ])), "No function codes found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Masking policies</h2><p>Field-level privacy defaults.</p></div>
          ${table(["Field", "Policy", "Role"], maskingPolicies.map((item) => row([
            escapeHtml(item.field),
            escapeHtml(item.policy),
            escapeHtml(item.role),
          ])), "No masking policy configured.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Data scopes</h2><p>Tenant and customer visibility rules.</p></div>
          ${table(["Scope", "Rule"], dataScopes.map((item) => row([
            escapeHtml(item.scope),
            escapeHtml(item.rule),
          ])), "No data scope rules configured.")}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Security operations</h2><p>Controls reflected in login and e-filing security steps.</p></div>
          ${table(["Setting", "Value"], systemSettings.map((item) => row([
            escapeHtml(item.setting_key),
            escapeHtml(item.setting_value),
          ])), "No system settings loaded.")}
          <form id="securityPolicyForm" class="stack">
            <label>Session timeout minutes <input id="sessionTimeoutInput" value="${escapeHtml(systemSettings.find((item) => item.setting_key === "session_timeout_minutes")?.setting_value || "60")}" /></label>
            <label>Masking default role <input id="maskRoleInput" value="${escapeHtml(maskingPolicies[0]?.role || "staff")}" /></label>
            <button class="secondary-btn" type="submit">Save security payload</button>
          </form>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Login activity</h2><p>Recent authentication events and IP review.</p></div>
          ${table(["Login", "Success", "IP"], loginHistory.map((item) => row([
            escapeHtml(item.login_id),
            item.success ? "Y" : "N",
            escapeHtml(item.ip_address || "-"),
          ])), "No login activity.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  document.getElementById("saveExpertPerm")?.addEventListener("click", async () => {
    await request("/api/admin/roles/TAX_EXPERT/permissions", {
      method: "PUT",
      body: JSON.stringify({ permissions: [
        { module_code: "tax-data", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "adjustment", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "efiling", function_code: "EFILE", effect: "ALLOW" },
      ] }),
    });
    await renderAdminRolesWorkbench(env);
  });
  document.getElementById("securityPolicyForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/field-masking", {
      method: "PUT",
      body: JSON.stringify([{ field: "biz_reg_no", policy: "partial", role: document.getElementById("maskRoleInput").value || "staff" }]),
    });
    await request("/api/admin/data-scope", {
      method: "PUT",
      body: JSON.stringify([{ scope: "tenant", rule: `session_timeout=${document.getElementById("sessionTimeoutInput").value || "60"}` }]),
    });
    await renderAdminRolesWorkbench(env);
  });
}

async function renderAdminMenusWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const locale = env.locale || currentDocumentLocale();
  const stageRoutes = ["admin/sec:menus", "admin/sec:functions"];
  const [menus, menuFunctions, roleMenuFunctions] = await Promise.all([
    request("/api/admin/menus"),
    request("/api/admin/menu-functions").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="menus">
      ${metrics([
        ["Menu nodes", menus.length],
        ["Menu functions", menuFunctions.length],
        ["Role bindings", roleMenuFunctions.length],
        ["Enabled", menus.filter((item) => item.enabled).length],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Menu and function governance</h2>
            <p>Control route exposure, feature flags, and action bindings.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Menu registry</h2><p>Permission gate and feature flag by menu leaf.</p></div>
          ${table(["Menu", "Parent", "Label", "Permission", "Feature flag", "Enabled", ""], menus.map((item) => row([
            escapeHtml(item.menu_key),
            escapeHtml(item.parent_key || "-"),
            escapeHtml(item.label),
            escapeHtml([item.required_perm_module, item.required_perm_function].filter(Boolean).join(":") || "-"),
            `<input value="${escapeHtml(item.feature_flag || "")}" data-menu-flag="${escapeHtml(item.menu_key)}" />`,
            item.enabled ? "Y" : "N",
            `<button class="secondary-btn compact" data-save-menu="${escapeHtml(item.menu_key)}" type="button">${escapeHtml(t(locale, "common.save"))}</button>`,
          ])), "No admin menus.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Menu functions</h2><p>Action-level controls attached to menu nodes.</p></div>
          ${table(["Menu", "Function", "Label", "Enabled"], menuFunctions.map((item) => row([
            escapeHtml(item.menu_key),
            escapeHtml(item.function_code),
            escapeHtml(item.function_name || item.label || "-"),
            item.enabled ? "Y" : "N",
          ])), "No menu functions.")}
          <p class="empty">Role-specific bindings: ${escapeHtml(String(roleMenuFunctions.length))}</p>
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Role to menu function matrix</h2><p>Visibility and action overrides by role.</p></div>
        ${table(["Role", "Menu", "Function", "Effect"], roleMenuFunctions.map((item) => row([
          escapeHtml(item.role_code),
          escapeHtml(item.menu_key),
          escapeHtml(item.function_code),
          escapeHtml(item.effect),
        ])), "No role-menu function grants.")}
      </article>
    </section>`;
  bindAdminRouteButtons(env);
  document.querySelectorAll("[data-save-menu]").forEach((button) => {
    button.addEventListener("click", async () => {
      const input = document.querySelector(`[data-menu-flag="${CSS.escape(button.dataset.saveMenu)}"]`);
      await request(`/api/admin/menus/${button.dataset.saveMenu}`, {
        method: "PUT",
        body: JSON.stringify({ feature_flag: input.value || null, enabled: true }),
      });
      await renderAdminMenusWorkbench(env);
    });
  });
}

async function renderAdminCustomerAccessWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const root = routeRoot(env);
  const stageRoutes = ["admin/cacc:assign", "admin/cacc:groups", "admin/cacc:rules", "admin/cacc:delegate", "admin/cacc:override"];
  const [users, customers, delegations, customerGroups, customerRules, adminDelegations, overrides] = await Promise.all([
    request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users`),
    request(`${root}/customers`),
    request(`${root}/access-delegations`).catch(() => []),
    request("/api/admin/customer-groups").catch(() => []),
    request("/api/admin/customer-rules").catch(() => []),
    request("/api/admin/access-delegations").catch(() => []),
    request("/api/admin/customer-access/override").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access">
      ${metrics([
        ["Users", users.length],
        ["Customers", customers.length],
        ["Delegations", delegations.length + adminDelegations.length],
        ["Overrides", overrides.length],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Customer access and assignment</h2>
            <p>Assignment rules, delegation, exception handling, and approval routing inputs.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">${table(["User", "Customer", "Access", "Scopes"], users.flatMap((user) => asArray(user.customer_access).map((access) => {
          const customer = customers.find((item) => item.customer_id === access.customer_id);
          return row([escapeHtml(user.login_id), escapeHtml(customer?.customer_name || access.customer_id), escapeHtml(access.access_level), escapeHtml(asArray(access.work_scopes).join(", "))]);
        })), "No customer assignments.")}</article>
        <article class="panel">${table(["Customer", "Scopes"], customers.map((item) => row([
          escapeHtml(item.customer_name || item.customer_id),
          escapeHtml(asArray(item.work_scopes).join(", ") || "-"),
        ])), "No customers available.")}</article>
      </section>
      <section class="grid three">
        <article class="panel">
          <div class="panel-head"><h2>Delegations</h2><p>Tenant delegation and handoff windows.</p></div>
          ${table(["Grantor", "Delegatee", "Customer", "Scope", "Period"], delegations.map((item) => row([
            escapeHtml(item.grantor_login_id),
            escapeHtml(item.delegatee_login_id),
            escapeHtml(item.customer_id),
            escapeHtml(item.work_scope),
            `${escapeHtml(item.valid_from || "-")} ~ ${escapeHtml(item.valid_to || "-")}`,
          ])), "No live delegations.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Access groups and rules</h2><p>Default assignment logic before manual overrides.</p></div>
          ${table(["Group", "Members"], customerGroups.map((item) => row([
            escapeHtml(item.group_name),
            escapeHtml(item.member_count),
          ])), "No customer groups.")}
          ${table(["Condition", "Access"], customerRules.map((item) => row([
            escapeHtml(item.condition),
            escapeHtml(item.access_level),
          ])), "No customer rules.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Access overrides</h2><p>Per-customer exception and admin delegation records.</p></div>
          ${table(["Customer", "Access", "Reason"], overrides.map((item) => row([
            escapeHtml(item.customer_code || "-"),
            escapeHtml(item.access_level || "-"),
            escapeHtml(item.reason || "-"),
          ])), "No access overrides.")}
          ${table(["Grantor", "Delegatee", "Status"], adminDelegations.map((item) => row([
            escapeHtml(item.grantor || "-"),
            escapeHtml(item.delegatee || "-"),
            escapeHtml(item.status || "-"),
          ])), "No admin delegation rules.")}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Create delegation</h2></div>
          <form id="delegationForm" class="stack">
            <label>Grantor <input id="delegationGrantor" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Delegatee <input id="delegationDelegatee" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Customer <select id="delegationCustomer">${customers.map((item) => `<option value="${item.customer_id}">${escapeHtml(item.customer_name)}</option>`).join("")}</select></label>
            <label>Scope <select id="delegationScope"><option>INFO</option><option>ADJUST</option><option>FORM</option><option>VALIDATE</option><option>APPROVE</option><option>PRINT</option><option>EFILE</option><option>POST</option></select></label>
            <label>Valid to <input id="delegationValidTo" type="date" value="${today()}" /></label>
            <button class="primary-btn" type="submit" ${customers.length ? "" : "disabled"}>Create</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  document.getElementById("delegationForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/access-delegations`, {
      method: "POST",
      body: JSON.stringify({
        grantor_login_id: document.getElementById("delegationGrantor").value,
        delegatee_login_id: document.getElementById("delegationDelegatee").value,
        customer_id: Number(document.getElementById("delegationCustomer").value),
        work_scope: document.getElementById("delegationScope").value,
        valid_from: today(),
        valid_to: document.getElementById("delegationValidTo").value || null,
        reason: "admin delegation",
      }),
    });
    await renderAdminCustomerAccessWorkbench(env);
  });
}

async function renderAdminLawWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const stageRoutes = ["admin/law:master", "admin/law:rates", "admin/law:limits", "admin/law:impact", "admin/law:history"];
  const [laws, summary, histories] = await Promise.all([
    request("/api/tax-laws"),
    request("/api/law-versioning/summary"),
    request("/api/law-amendments").catch(() => []),
  ]);
  const activeLaw = laws[0];
  const [rates, limits, impact] = activeLaw ? await Promise.all([
    request(`/api/tax-rates?law_version_id=${activeLaw.law_version_id}`),
    request(`/api/tax-limits?law_version_id=${activeLaw.law_version_id}`),
    request("/api/law-versioning/impact", { method: "POST", body: JSON.stringify({ law_version_id: activeLaw.law_version_id, sample_size: 5 }) }).catch(() => ({ impacted_business_years: 0, impacted_forms: 0, impacted_rules: 0 })),
  ]) : [[], [], { impacted_business_years: 0, impacted_forms: 0, impacted_rules: 0 }];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law">
      ${metrics([["Laws", summary.laws || laws.length], ["Rates", summary.rates || rates.length], ["Limits", summary.limits || limits.length], ["Active version", activeLaw?.version_code || "-"]])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Law and rate version control</h2>
            <p>Law versions, rate tables, impact preview, and amendment history.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Law versions</h2><button id="createLaw" class="primary-btn compact" type="button">Create version</button></div>
          ${table(["ID", "Version", "Status", "Effective"], laws.map((item) => row([
            escapeHtml(item.law_version_id),
            escapeHtml(item.version_code),
            escapeHtml(item.status),
            `${item.effective_from} ~ ${item.effective_to || ""}`,
          ])), "No law versions.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Rates and limits</h2><p>Representative slices of the active version.</p></div>
          ${table(["Item", "Range", "Value"], rates.slice(0, 10).map((item) => row([
            escapeHtml(item.item_code),
            `${money.format(item.taxable_from)} ~ ${item.taxable_to ? money.format(item.taxable_to) : ""}`,
            `${(item.rate_bps / 100).toFixed(2)}%`,
          ])).concat(limits.slice(0, 10).map((item) => row([
            escapeHtml(item.item_code),
            escapeHtml(item.category || "LIMIT"),
            money.format(item.amount),
          ]))), "No rate or limit rows.")}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Impact preview</h2><p>Downstream effect before version activation.</p></div>
          ${table(["Metric", "Value"], [
            row(["Business years", escapeHtml(impact.impacted_business_years ?? 0)]),
            row(["Forms", escapeHtml(impact.impacted_forms ?? 0)]),
            row(["Rules", escapeHtml(impact.impacted_rules ?? 0)]),
          ])}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Amendment history</h2><p>Tracked legal changes and notes.</p></div>
          ${table(["Law", "Summary", "Effective"], histories.map((item) => row([
            escapeHtml(item.version_code || item.law_version_id || "-"),
            escapeHtml(item.summary || item.amendment_summary || "-"),
            escapeHtml(item.effective_from || item.created_at || "-"),
          ])), "No law amendment history.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  document.getElementById("createLaw")?.addEventListener("click", async () => {
    const suffix = Date.now().toString(36).slice(-4).toUpperCase();
    await request("/api/tax-laws", { method: "POST", body: JSON.stringify({ version_code: `CIT-${new Date().getFullYear()}-${suffix}`, law_name: "Corporate income tax amendment", effective_from: `${new Date().getFullYear()}-01-01`, effective_to: null, metadata: { source: "admin-ui" } }) });
    await renderAdminLawWorkbench(env);
  });
}

async function renderAdminFormsWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const stageRoutes = ["admin/form:master", "admin/form:versions", "admin/form:fields", "admin/form:validations", "admin/form:linkage-rule", "admin/form:migration", "admin/form:impact"];
  const [forms, versions, relationships, cycleCheck, efileMap, bySet, impact] = await Promise.all([
    request("/api/form-versioning/forms"),
    request("/api/form-versioning/versions"),
    request("/api/form-versioning/relationships"),
    request("/api/form-versioning/cycle-check").catch(() => ({ valid: false })),
    request("/api/form-versioning/efile-map").catch(() => []),
    request("/api/form-versioning/by-set").catch(() => []),
    request("/api/form-versioning/impact").catch(() => ({ impacted_business_years: 0, impacted_forms: 0 })),
  ]);
  const selectedVersion = versions[0];
  const [fields, validations] = selectedVersion ? await Promise.all([
    request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/fields`).catch(() => []),
    request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/validations`).catch(() => []),
  ]) : [[], []];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="forms-admin">
      ${metrics([
        ["Forms", forms.length],
        ["Versions", versions.length],
        ["Relationships", relationships.length],
        ["Cycle check", cycleCheck.valid ? "OK" : "CHECK"],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Form version administration</h2>
            <p>Master forms, version metadata, field definitions, validations, and migration readiness.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid three">
        <article class="panel"><div class="panel-head"><h2>Forms</h2></div>${table(["Code", "Name", "Active"], forms.map((item) => row([escapeHtml(item.form_code), escapeHtml(item.form_name), item.active ? "Y" : "N"])), "No forms.")}</article>
        <article class="panel"><div class="panel-head"><h2>Versions</h2></div>${table(["ID", "Form", "Version", "Status"], versions.map((item) => row([escapeHtml(item.form_version_id), escapeHtml(item.form_code), escapeHtml(item.version_no), escapeHtml(item.status)])), "No versions.")}</article>
        <article class="panel"><div class="panel-head"><h2>Relationships</h2><span class="badge ${cycleCheck.valid ? "ok" : "error"}">${cycleCheck.valid ? "ACYCLIC" : "CYCLE"}</span></div>${table(["Source", "Target", "Rule"], relationships.map((item) => row([`${escapeHtml(item.source_form)}.${escapeHtml(item.source_field)}`, `${escapeHtml(item.target_form)}.${escapeHtml(item.target_field)}`, escapeHtml(JSON.stringify(item.rule_json))])), "No relationships.")}</article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Selected version fields</h2><p>${escapeHtml(selectedVersion ? `${selectedVersion.form_code} v${selectedVersion.version_no}` : "No selected version")}</p></div>
          ${table(["Field", "Label"], fields.map((item) => row([
            escapeHtml(item.field_path),
            escapeHtml(item.label),
          ])), "No fields for the selected version.")}
          ${table(["Rule", "Severity"], validations.map((item) => row([
            escapeHtml(item.rule_code),
            escapeHtml(item.severity),
          ])), "No validations for the selected version.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>E-filing map and by-set</h2><p>Business-year mapping and outbound file alignment.</p></div>
          ${table(["Form", "Target", "Field"], efileMap.map((item) => row([
            escapeHtml(item.form_code || item.source_form || "-"),
            escapeHtml(item.record_type || item.target || "-"),
            escapeHtml(item.field_name || item.target_field || "-"),
          ])), "No e-file map rows.")}
          ${table(["Business year", "Form set"], bySet.map((item) => row([
            escapeHtml(item.by_id || "-"),
            escapeHtml(item.form_set_code || item.set_code || "-"),
          ])), "No business-year form sets.")}
          ${table(["Impact", "Value"], [
            row(["Business years", escapeHtml(impact.impacted_business_years ?? 0)]),
            row(["Forms", escapeHtml(impact.impacted_forms ?? 0)]),
          ])}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminAuditWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const stageRoutes = ["admin/audit:events", "admin/audit:login", "admin/audit:perm", "admin/audit:settings"];
  const [logs, verify, loginHistory, permissionHistory, systemSettings] = await Promise.all([
    request(`${routeRoot(env)}/audit-logs`),
    request(`${routeRoot(env)}/audit-logs/verify`).catch(() => ({ valid: false })),
    request("/api/login-history").catch(() => []),
    request("/api/permission-change-history").catch(() => []),
    request("/api/system-settings").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="audit">
      ${metrics([
        ["Audit logs", logs.length],
        ["Login events", loginHistory.length],
        ["Permission events", permissionHistory.length],
        ["Chain", verify.valid ? "HASH OK" : "CHECK"],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Admin settings workbench</span>
            <h2>Audit and change review</h2>
            <p>Trace login, permission, and system-setting changes after policy updates.</p>
          </div>
          <div class="button-row">${renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Audit events</h2><span class="badge ${verify.valid ? "ok" : "error"}">${verify.valid ? "HASH OK" : "HASH CHECK"}</span></div>
          ${table(["ID", "Table", "Action", "Actor", "Hash"], logs.map((item) => row([
            escapeHtml(item.audit_id),
            escapeHtml(item.table_name),
            escapeHtml(item.action),
            escapeHtml(item.changed_by),
            escapeHtml(item.hash_current || "-"),
          ])), "No audit logs.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Login and permission history</h2></div>
          ${table(["Login", "Success", "IP"], loginHistory.map((item) => row([
            escapeHtml(item.login_id),
            item.success ? "Y" : "N",
            escapeHtml(item.ip_address || "-"),
          ])), "No login history.")}
          ${table(["Role", "Function", "Changed by"], permissionHistory.map((item) => row([
            escapeHtml(item.role_code || "-"),
            escapeHtml(item.function || item.function_code || "-"),
            escapeHtml(item.changed_by || "-"),
          ])), "No permission change history.")}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>System settings snapshot</h2><p>Current global configuration visible to audit reviewers.</p></div>
        ${table(["Key", "Value"], systemSettings.map((item) => row([
          escapeHtml(item.setting_key),
          escapeHtml(item.setting_value),
        ])), "No system settings rows.")}
      </article>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminRoles(env) {
  return renderAdminRolesWorkbench(env);
  const [roles, permissions, functionCodes, roleMenuFunctions] = await Promise.all([
    request("/api/admin/roles"),
    request("/api/admin/role-permissions"),
    request("/api/admin/function-codes").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      <section class="grid two">
      <article class="panel">${table(["역할", "이름", "시스템"], roles.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.role_name), item.system_role ? "Y" : "N"])))}</article>
      <article class="panel">
        <div class="panel-head"><h2>권한 매트릭스</h2><button id="saveExpertPerm" class="primary-btn compact" type="button">TAX_EXPERT 저장</button></div>
        ${table(["역할", "모듈", "기능", "효과"], permissions.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.module_code), escapeHtml(item.function_code), escapeHtml(item.effect)])))}
      </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Function Codes</h2></div>
          ${table(["Code", "Name", "Sort"], functionCodes.map((item) => row([escapeHtml(item.function_code), escapeHtml(item.function_name), escapeHtml(item.sort_order)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Role Menu Functions</h2></div>
          ${table(["Role", "Menu", "Function", "Effect"], roleMenuFunctions.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.menu_key), escapeHtml(item.function_code), escapeHtml(item.effect)])))}
        </article>
      </section>
    </section>`;
  document.getElementById("saveExpertPerm").addEventListener("click", async () => {
    await request("/api/admin/roles/TAX_EXPERT/permissions", {
      method: "PUT",
      body: JSON.stringify({ permissions: [
        { module_code: "tax-data", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "adjustment", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "efiling", function_code: "EFILE", effect: "ALLOW" },
      ] }),
    });
    await renderAdminRoles(env);
  });
}

async function renderAdminMenus(env) {
  return renderAdminMenusWorkbench(env);
  const locale = env.locale || currentDocumentLocale();
  const [menus, menuFunctions] = await Promise.all([
    request("/api/admin/menus"),
    request("/api/admin/menu-functions").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>메뉴/기능 관리</h2></div>
        ${table(["키", "상위", "라벨", "권한", "플래그", "사용", ""], menus.map((item) => row([
          escapeHtml(item.menu_key),
          escapeHtml(item.parent_key || "-"),
          escapeHtml(item.label),
          escapeHtml([item.required_perm_module, item.required_perm_function].filter(Boolean).join(":") || "-"),
          `<input value="${escapeHtml(item.feature_flag || "")}" data-menu-flag="${escapeHtml(item.menu_key)}" />`,
          item.enabled ? "Y" : "N",
          `<button class="secondary-btn compact" data-save-menu="${escapeHtml(item.menu_key)}" type="button">${escapeHtml(t(locale, "common.save"))}</button>`,
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Menu Functions</h2></div>
        ${table(["Menu", "Function", "Label", "Enabled"], menuFunctions.map((item) => row([
          escapeHtml(item.menu_key),
          escapeHtml(item.function_code),
          escapeHtml(item.function_name || item.label || "-"),
          item.enabled ? "Y" : "N",
        ])))}
      </article>
    </section>`;
  document.querySelectorAll("[data-save-menu]").forEach((button) => {
    button.addEventListener("click", async () => {
      const input = document.querySelector(`[data-menu-flag="${CSS.escape(button.dataset.saveMenu)}"]`);
      await request(`/api/admin/menus/${button.dataset.saveMenu}`, {
        method: "PUT",
        body: JSON.stringify({ feature_flag: input.value || null, enabled: true }),
      });
      await renderAdminMenus(env);
    });
  });
}

async function renderAdminCustomerAccess(env) {
  return renderAdminCustomerAccessWorkbench(env);
  const root = routeRoot(env);
  const [users, customers, delegations] = await Promise.all([
    request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users`),
    request(`${root}/customers`),
    request(`${root}/access-delegations`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      <section class="grid two">
      <article class="panel">${table(["사용자", "고객사", "권한", "업무범위"], users.flatMap((user) => asArray(user.customer_access).map((access) => {
        const customer = customers.find((item) => item.customer_id === access.customer_id);
        return row([escapeHtml(user.login_id), escapeHtml(customer?.customer_name || access.customer_id), escapeHtml(access.access_level), escapeHtml(asArray(access.work_scopes).join(", "))]);
      })))}</article>
      <article class="panel">${table(["고객사", "업무범위"], customers.map((item) => row([
        escapeHtml(item.customer_name || item.customer_id),
        escapeHtml(asArray(item.work_scopes).join(", ") || "-"),
      ])))}</article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Delegations</h2></div>
          ${table(["Grantor", "Delegatee", "Customer", "Scope", "Period"], delegations.map((item) => row([
            escapeHtml(item.grantor_login_id),
            escapeHtml(item.delegatee_login_id),
            escapeHtml(item.customer_id),
            escapeHtml(item.work_scope),
            `${escapeHtml(item.valid_from || "-")} ~ ${escapeHtml(item.valid_to || "-")}`,
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create Delegation</h2></div>
          <form id="delegationForm" class="stack">
            <label>Grantor <input id="delegationGrantor" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Delegatee <input id="delegationDelegatee" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Customer <select id="delegationCustomer">${customers.map((item) => `<option value="${item.customer_id}">${escapeHtml(item.customer_name)}</option>`).join("")}</select></label>
            <label>Scope <select id="delegationScope"><option>INFO</option><option>ADJUST</option><option>FORM</option><option>VALIDATE</option><option>APPROVE</option><option>PRINT</option><option>EFILE</option><option>POST</option></select></label>
            <label>Valid to <input id="delegationValidTo" type="date" value="${today()}" /></label>
            <button class="primary-btn" type="submit" ${customers.length ? "" : "disabled"}>Create</button>
          </form>
        </article>
      </section>
    </section>`;
  document.getElementById("delegationForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/access-delegations`, {
      method: "POST",
      body: JSON.stringify({
        grantor_login_id: document.getElementById("delegationGrantor").value,
        delegatee_login_id: document.getElementById("delegationDelegatee").value,
        customer_id: Number(document.getElementById("delegationCustomer").value),
        work_scope: document.getElementById("delegationScope").value,
        valid_from: today(),
        valid_to: document.getElementById("delegationValidTo").value || null,
        reason: "admin delegation",
      }),
    });
    await renderAdminCustomerAccess(env);
  });
}

async function renderAdminLaw(env) {
  return renderAdminLawWorkbench(env);
  const [laws, summary] = await Promise.all([
    request("/api/tax-laws"),
    request("/api/law-versioning/summary"),
  ]);
  const activeLaw = laws[0];
  const rates = activeLaw ? await request(`/api/tax-rates?law_version_id=${activeLaw.law_version_id}`) : [];
  const limits = activeLaw ? await request(`/api/tax-limits?law_version_id=${activeLaw.law_version_id}`) : [];
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["법령", summary.laws || laws.length], ["세율", summary.rates || rates.length], ["한도", summary.limits || limits.length], ["활성 버전", activeLaw?.version_code || "-"]])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>법령 버전</h2><button id="createLaw" class="primary-btn compact" type="button">등록</button></div>
          ${table(["ID", "버전", "상태", "기간"], laws.map((item) => row([escapeHtml(item.law_version_id), escapeHtml(item.version_code), escapeHtml(item.status), `${item.effective_from} ~ ${item.effective_to || ""}`])))}
        </article>
        <article class="panel">${table(["항목", "구간", "율/금액"], rates.slice(0, 10).map((item) => row([escapeHtml(item.item_code), `${money.format(item.taxable_from)} ~ ${item.taxable_to ? money.format(item.taxable_to) : ""}`, `${(item.rate_bps / 100).toFixed(2)}%`])).concat(limits.slice(0, 10).map((item) => row([escapeHtml(item.item_code), "한도", money.format(item.amount)]))))}</article>
      </section>
    </section>`;
  document.getElementById("createLaw").addEventListener("click", async () => {
    const suffix = Date.now().toString(36).slice(-4).toUpperCase();
    await request("/api/tax-laws", { method: "POST", body: JSON.stringify({ version_code: `CIT-${new Date().getFullYear()}-${suffix}`, law_name: "법인세법 개정", effective_from: `${new Date().getFullYear()}-01-01`, effective_to: null, metadata: { source: "admin-ui" } }) });
    await renderAdminLaw(env);
  });
}

async function renderAdminForms(env) {
  return renderAdminFormsWorkbench(env);
  const [forms, versions, relationships, cycleCheck] = await Promise.all([
    request("/api/form-versioning/forms"),
    request("/api/form-versioning/versions"),
    request("/api/form-versioning/relationships"),
    request("/api/form-versioning/cycle-check").catch(() => ({ valid: false })),
  ]);
  env.outlet.innerHTML = `
    <section class="grid three">
      <article class="panel"><div class="panel-head"><h2>서식</h2></div>${table(["코드", "이름", "활성"], forms.map((item) => row([escapeHtml(item.form_code), escapeHtml(item.form_name), item.active ? "Y" : "N"])))}</article>
      <article class="panel"><div class="panel-head"><h2>버전</h2></div>${table(["ID", "서식", "버전", "상태"], versions.map((item) => row([escapeHtml(item.form_version_id), escapeHtml(item.form_code), escapeHtml(item.version_no), escapeHtml(item.status)])))}</article>
      <article class="panel"><div class="panel-head"><h2>연동</h2><span class="badge ${cycleCheck.valid ? "ok" : "error"}">${cycleCheck.valid ? "ACYCLIC" : "CYCLE"}</span></div>${table(["원천", "대상", "규칙"], relationships.map((item) => row([`${escapeHtml(item.source_form)}.${escapeHtml(item.source_field)}`, `${escapeHtml(item.target_form)}.${escapeHtml(item.target_field)}`, escapeHtml(JSON.stringify(item.rule_json))])))}</article>
    </section>`;
}

async function renderAdminAudit(env) {
  return renderAdminAuditWorkbench(env);
  const [logs, verify] = await Promise.all([
    request(`${routeRoot(env)}/audit-logs`),
    request(`${routeRoot(env)}/audit-logs/verify`).catch(() => ({ valid: false })),
  ]);
  env.outlet.innerHTML = `
    <section class="panel">
      <div class="panel-head"><h2>감사/로그</h2><span class="badge ${verify.valid ? "ok" : "error"}">${verify.valid ? "HASH OK" : "HASH CHECK"}</span></div>
      ${table(["ID", "테이블", "작업", "사용자", "해시"], logs.map((item) => row([escapeHtml(item.audit_id), escapeHtml(item.table_name), escapeHtml(item.action), escapeHtml(item.changed_by), escapeHtml(item.hash_current || "-")])))}
    </section>`;
}
