import { request, downloadBinary, escapeHtml, money, statusClass, today, asArray } from "/app/api.js";
import { bindDataGridActions, renderDataGrid } from "/app/components/grid.js";
import { hasWorkContext, progressForStatus } from "/app/context.js";
import { leafFocusText, t } from "/app/i18n.js";

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
  return [key, route(String(group), String(title), legacyLayout(key), key)];
})));

export const leafRoutes = Object.freeze({
  ...Object.fromEntries([
    ["dashboard:overview", "Overview"],
    ["dashboard:duesoon", "Due soon"],
    ["dashboard:inbox", "Inbox"],
    ["dashboard:recent", "Recent activity"],
    ["dashboard:kpi-tax", "KPI"],
  ].map(([key, title]) => leafRoute(key, "Dashboard", title, "plain", "dashboard"))),
  ...Object.fromEntries([
    ["ws/start:customer-pick", "Customer selection"],
    ["ws/start:by-pick", "Business year selection/new"],
    ["ws/start:snapshot", "Law snapshot"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-start"))),
  ...Object.fromEntries([
    ["ws/info:fs", "Financial statements import"],
    ["ws/info:mapping", "Account mapping"],
    ["ws/info:assets", "Asset register"],
    ["ws/info:transactions", "Transactions"],
    ["ws/info:vehicle", "Vehicle usage"],
    ["ws/info:consistency", "Consistency check"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-info"))),
  ...Object.fromEntries([
    ["B1", "B1 Income add/deduct"],
    ["B2", "B2 Donations"],
    ["B3", "B3 Entertainment expense"],
    ["B4", "B4 Depreciation"],
    ["B5", "B5 Deemed interest"],
    ["B6", "B6 Retirement allowance reserve"],
    ["B7", "B7 Bad debt reserve"],
    ["B8", "B8 Currency valuation"],
    ["B9", "B9 Inventory/securities valuation"],
    ["B10", "B10 Business transfer difference"],
    ["B11", "B11 Loss carryforward"],
    ["B12", "B12 Tax credits"],
    ["B13", "B13 Minimum tax"],
    ["B14", "B14 Additional tax"],
    ["B15", "B15 Capital/equity"],
    ["B16", "B16 Foreign corporation"],
    ["B17", "B17 Consolidated tax"],
  ].map(([code, title]) => leafRoute(`ws/adj:${code}`, "Tax workspace", title, "workspace", "ws-adj"))),
  ...Object.fromEntries([
    ["ws/form:form3", "Form 3 main statement"],
    ["ws/form:attachments", "Attachments"],
    ["ws/form:preview", "Preview"],
    ["ws/form:linkage", "Form linkage"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-form"))),
  ...Object.fromEntries([
    ["ws/val:run", "Run validation"],
    ["ws/val:issues", "Validation issues"],
    ["ws/val:rules", "Validation rules"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-val"))),
  ...Object.fromEntries([
    ["ws/appr:request", "Request approval"],
    ["ws/appr:inbox", "Approval inbox"],
    ["ws/appr:rejected", "Rejected items"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-appr"))),
  ...Object.fromEntries([
    ["ws/print:preview", "Print preview"],
    ["ws/print:bulk", "Bulk print"],
    ["ws/print:history", "Print history"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-print"))),
  ...Object.fromEntries([
    ["ws/file:precheck", "E-file precheck"],
    ["ws/file:generate", "Generate e-file"],
    ["ws/file:submit", "Submit e-file"],
    ["ws/file:done", "Submission result"],
  ].map(([key, title]) => leafRoute(key, "Tax workspace", title, "workspace", "ws-file"))),
  ...Object.fromEntries([
    ["post/hist:list", "Filing history list", "post-hist"],
    ["post/amend:unlock", "Unlock for amendment", "post-amend"],
    ["post/amend:version", "Amendment version", "post-amend"],
    ["post/amend:diff", "Amendment diff", "post-amend"],
    ["post/amend:resubmit", "Resubmit amendment", "post-amend"],
    ["post/correction", "Correction request", "post-amend"],
  ].map(([key, title, delegate]) => leafRoute(key, "Post filing", title, "plain", delegate))),
  ...Object.fromEntries([
    ["report:year-compare", "Year comparison", "rp-compare"],
    ["report:tax-burden", "Tax burden", "rp-burden"],
    ["report:reserve-trend", "Reserve trend", "rp-reserve"],
    ["report:loss-expiry", "Loss expiry", "rp-reserve"],
    ["report:industry-stats", "Industry stats", "rp-burden"],
    ["report:custom", "Custom report", "rp-reserve"],
  ].map(([key, title, delegate]) => leafRoute(key, "Analytics/reports", title, "plain", delegate))),
  ...Object.fromEntries([
    ["admin/tenant:list", "Tenant management", "ad-tenant"],
    ["admin/cust:list", "Customer list", "ad-cust"],
    ["admin/cust:by-master", "Business year master", "ad-cust"],
    ["admin/cust:agent", "Tax agent", "ad-cust"],
    ["admin/sec:users", "Users", "ad-user-list"],
    ["admin/sec:roles", "Roles", "ad-role"],
    ["admin/sec:matrix", "Permission matrix", "ad-role"],
    ["admin/sec:menus", "Menus", "ad-menu-fn"],
    ["admin/sec:functions", "Functions", "ad-menu-fn"],
    ["admin/sec:mask", "Masking policies", "ad-role"],
    ["admin/sec:scope", "Data scopes", "ad-role"],
    ["admin/cacc:assign", "Customer assignment", "ad-cacc"],
    ["admin/cacc:groups", "Access groups", "ad-cacc"],
    ["admin/cacc:rules", "Access rules", "ad-cacc"],
    ["admin/cacc:delegate", "Delegation", "ad-cacc"],
    ["admin/cacc:override", "Access override", "ad-cacc"],
    ["admin/law:master", "Law version master", "ad-law"],
    ["admin/law:rates", "Tax rates", "ad-law"],
    ["admin/law:limits", "Limits", "ad-law"],
    ["admin/law:credits", "Credits", "ad-law"],
    ["admin/law:depr-lives", "Depreciation lives", "ad-law"],
    ["admin/law:sme", "SME rules", "ad-law"],
    ["admin/law:loss-rule", "Loss carryforward rules", "ad-law"],
    ["admin/law:snapshots", "Law snapshots", "ad-law"],
    ["admin/law:impact", "Impact simulation", "ad-law"],
    ["admin/law:history", "Law history", "ad-law"],
    ["admin/form:master", "Form master", "ad-form"],
    ["admin/form:versions", "Form versions", "ad-form"],
    ["admin/form:fields", "Fields", "ad-form"],
    ["admin/form:validations", "Validation rules", "ad-form"],
    ["admin/form:linkage-rule", "Linkage rules", "ad-form"],
    ["admin/form:migration", "Form migration", "ad-form"],
    ["admin/form:efile-map", "E-file mapping", "ad-form"],
    ["admin/form:by-set", "Business-year form set", "ad-form"],
    ["admin/form:impact", "Form impact", "ad-form"],
    ["admin/code:manage", "Code management", "ad-menu-fn"],
    ["admin/audit:events", "Audit events", "ad-audit"],
    ["admin/audit:login", "Login history", "ad-audit"],
    ["admin/audit:perm", "Permission audit", "ad-audit"],
    ["admin/audit:settings", "Settings audit", "ad-audit"],
  ].map(([key, title, delegate]) => leafRoute(key, "Administration", title, "admin", delegate))),
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

function route(group, title, layout, delegate) {
  return { group, title, layout, delegate, s1: false };
}

function leafRoute(key, group, title, layout, delegate) {
  return [key, { group, title, layout, delegate, leafKey: key, s1: true }];
}

function legacyLayout(key) {
  if (key.startsWith("ws-")) return "workspace";
  if (key.startsWith("ad-")) return "admin";
  return "plain";
}

const adjustmentModules = [
  ["B1", "통합", "income"],
  ["B2", "기부금", "transactions"],
  ["B3", "접대비", "transactions"],
  ["B4", "감가상각", "assets"],
  ["B5", "인정이자", "transactions"],
  ["B6", "퇴직급여충당금", "assets"],
  ["B7", "대손충당금", "assets"],
  ["B8", "외화평가", "evaluation"],
  ["B9", "재고/증권평가", "transactions"],
  ["B10", "업무용승용차", "assets"],
  ["B11", "이월결손금", "evaluation"],
  ["B12", "세액공제", "tax"],
  ["B13", "최저한세", "tax"],
  ["B14", "가산세", "tax"],
  ["B15", "자본/적립금", "evaluation"],
  ["B16", "외국법인", "special"],
  ["B17", "연결납세", "special"],
];

const adjustmentGridColumns = [
  { key: "source_module", label: "Module" },
  { key: "item_code", label: "Code" },
  { key: "item_name", label: "Item" },
  { key: "direction", label: "Direction" },
  { key: "amount", label: "Amount", format: "money" },
  { key: "disposition", label: "Disposition" },
];

const leafViewState = new Map();

export const leafScreenSpecs = Object.freeze({
  "dashboard:overview": leafSpec("GET", "/api/tenants/{tenant}/dashboard", "dashboard", "READ"),
  "dashboard:duesoon": leafSpec("GET", "/api/tenants/{tenant}/business-years?dueWithinDays=30", "dashboard", "READ"),
  "dashboard:inbox": leafSpec("GET", "/api/tenants/{tenant}/workflow/queue?assignee=me", "workflow", "READ"),
  "dashboard:recent": leafSpec("GET", "/api/tenants/{tenant}/audit-logs?limit=20", "audit", "READ"),
  "dashboard:kpi-tax": leafSpec("GET", "/api/tenants/{tenant}/reports/tax-burden?range=5y", "reports", "READ"),
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
  "ws/adj:B5": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B5", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B6": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B6", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B7": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B7", "adjustment", "CALCULATE", { requires: ["work-context"] }),
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
  "dashboard:overview": (env) => renderLeafScreen(env, "dashboard:overview"),
  "dashboard:duesoon": (env) => renderLeafScreen(env, "dashboard:duesoon"),
  "dashboard:inbox": (env) => renderLeafScreen(env, "dashboard:inbox"),
  "dashboard:recent": (env) => renderLeafScreen(env, "dashboard:recent"),
  "dashboard:kpi-tax": (env) => renderLeafScreen(env, "dashboard:kpi-tax"),
  "ws/start:customer-pick": (env) => renderLeafScreen(env, "ws/start:customer-pick"),
  "ws/start:by-pick": (env) => renderLeafScreen(env, "ws/start:by-pick"),
  "ws/start:snapshot": (env) => renderLeafScreen(env, "ws/start:snapshot"),
  "ws/info:fs": (env) => renderLeafScreen(env, "ws/info:fs"),
  "ws/info:mapping": (env) => renderLeafScreen(env, "ws/info:mapping"),
  "ws/info:assets": (env) => renderLeafScreen(env, "ws/info:assets"),
  "ws/info:transactions": (env) => renderLeafScreen(env, "ws/info:transactions"),
  "ws/info:vehicle": (env) => renderLeafScreen(env, "ws/info:vehicle"),
  "ws/info:consistency": (env) => renderLeafScreen(env, "ws/info:consistency"),
  "ws/adj:B1": (env) => renderLeafScreen(env, "ws/adj:B1"),
  "ws/adj:B2": (env) => renderLeafScreen(env, "ws/adj:B2"),
  "ws/adj:B3": (env) => renderLeafScreen(env, "ws/adj:B3"),
  "ws/adj:B4": (env) => renderLeafScreen(env, "ws/adj:B4"),
  "ws/adj:B5": (env) => renderLeafScreen(env, "ws/adj:B5"),
  "ws/adj:B6": (env) => renderLeafScreen(env, "ws/adj:B6"),
  "ws/adj:B7": (env) => renderLeafScreen(env, "ws/adj:B7"),
  "ws/adj:B8": (env) => renderLeafScreen(env, "ws/adj:B8"),
  "ws/adj:B9": (env) => renderLeafScreen(env, "ws/adj:B9"),
  "ws/adj:B10": (env) => renderLeafScreen(env, "ws/adj:B10"),
  "ws/adj:B11": (env) => renderLeafScreen(env, "ws/adj:B11"),
  "ws/adj:B12": (env) => renderLeafScreen(env, "ws/adj:B12"),
  "ws/adj:B13": (env) => renderLeafScreen(env, "ws/adj:B13"),
  "ws/adj:B14": (env) => renderLeafScreen(env, "ws/adj:B14"),
  "ws/adj:B15": (env) => renderLeafScreen(env, "ws/adj:B15"),
  "ws/adj:B16": (env) => renderLeafScreen(env, "ws/adj:B16"),
  "ws/adj:B17": (env) => renderLeafScreen(env, "ws/adj:B17"),
  "ws/form:form3": (env) => renderLeafScreen(env, "ws/form:form3"),
  "ws/form:attachments": (env) => renderLeafScreen(env, "ws/form:attachments"),
  "ws/form:preview": (env) => renderLeafScreen(env, "ws/form:preview"),
  "ws/form:linkage": (env) => renderLeafScreen(env, "ws/form:linkage"),
  "ws/val:run": (env) => renderLeafScreen(env, "ws/val:run"),
  "ws/val:issues": (env) => renderLeafScreen(env, "ws/val:issues"),
  "ws/val:rules": (env) => renderLeafScreen(env, "ws/val:rules"),
  "ws/appr:request": (env) => renderLeafScreen(env, "ws/appr:request"),
  "ws/appr:inbox": (env) => renderLeafScreen(env, "ws/appr:inbox"),
  "ws/appr:rejected": (env) => renderLeafScreen(env, "ws/appr:rejected"),
  "ws/print:preview": (env) => renderLeafScreen(env, "ws/print:preview"),
  "ws/print:bulk": (env) => renderLeafScreen(env, "ws/print:bulk"),
  "ws/print:history": (env) => renderLeafScreen(env, "ws/print:history"),
  "ws/file:precheck": (env) => renderLeafScreen(env, "ws/file:precheck"),
  "ws/file:generate": (env) => renderLeafScreen(env, "ws/file:generate"),
  "ws/file:submit": (env) => renderLeafScreen(env, "ws/file:submit"),
  "ws/file:done": (env) => renderLeafScreen(env, "ws/file:done"),
  "post/hist:list": (env) => renderLeafScreen(env, "post/hist:list"),
  "post/amend:unlock": (env) => renderLeafScreen(env, "post/amend:unlock"),
  "post/amend:version": (env) => renderLeafScreen(env, "post/amend:version"),
  "post/amend:diff": (env) => renderLeafScreen(env, "post/amend:diff"),
  "post/amend:resubmit": (env) => renderLeafScreen(env, "post/amend:resubmit"),
  "post/correction": (env) => renderLeafScreen(env, "post/correction"),
  "report:year-compare": (env) => renderLeafScreen(env, "report:year-compare"),
  "report:tax-burden": (env) => renderLeafScreen(env, "report:tax-burden"),
  "report:reserve-trend": (env) => renderLeafScreen(env, "report:reserve-trend"),
  "report:loss-expiry": (env) => renderLeafScreen(env, "report:loss-expiry"),
  "report:industry-stats": (env) => renderLeafScreen(env, "report:industry-stats"),
  "report:custom": (env) => renderLeafScreen(env, "report:custom"),
  "admin/tenant:list": (env) => renderAdminTenantLeaf(env),
  "admin/cust:list": (env) => renderLeafScreen(env, "admin/cust:list"),
  "admin/cust:by-master": (env) => renderLeafScreen(env, "admin/cust:by-master"),
  "admin/cust:agent": (env) => renderLeafScreen(env, "admin/cust:agent"),
  "admin/sec:users": (env) => renderLeafScreen(env, "admin/sec:users"),
  "admin/sec:roles": (env) => renderLeafScreen(env, "admin/sec:roles"),
  "admin/sec:matrix": (env) => renderLeafScreen(env, "admin/sec:matrix"),
  "admin/sec:menus": (env) => renderLeafScreen(env, "admin/sec:menus"),
  "admin/sec:functions": (env) => renderLeafScreen(env, "admin/sec:functions"),
  "admin/sec:mask": (env) => renderLeafScreen(env, "admin/sec:mask"),
  "admin/sec:scope": (env) => renderLeafScreen(env, "admin/sec:scope"),
  "admin/cacc:assign": (env) => renderLeafScreen(env, "admin/cacc:assign"),
  "admin/cacc:groups": (env) => renderLeafScreen(env, "admin/cacc:groups"),
  "admin/cacc:rules": (env) => renderLeafScreen(env, "admin/cacc:rules"),
  "admin/cacc:delegate": (env) => renderLeafScreen(env, "admin/cacc:delegate"),
  "admin/cacc:override": (env) => renderLeafScreen(env, "admin/cacc:override"),
  "admin/law:master": (env) => renderLeafScreen(env, "admin/law:master"),
  "admin/law:rates": (env) => renderLeafScreen(env, "admin/law:rates"),
  "admin/law:limits": (env) => renderLeafScreen(env, "admin/law:limits"),
  "admin/law:credits": (env) => renderLeafScreen(env, "admin/law:credits"),
  "admin/law:depr-lives": (env) => renderLeafScreen(env, "admin/law:depr-lives"),
  "admin/law:sme": (env) => renderLeafScreen(env, "admin/law:sme"),
  "admin/law:loss-rule": (env) => renderLeafScreen(env, "admin/law:loss-rule"),
  "admin/law:snapshots": (env) => renderLeafScreen(env, "admin/law:snapshots"),
  "admin/law:impact": (env) => renderLeafScreen(env, "admin/law:impact"),
  "admin/law:history": (env) => renderLeafScreen(env, "admin/law:history"),
  "admin/form:master": (env) => renderLeafScreen(env, "admin/form:master"),
  "admin/form:versions": (env) => renderLeafScreen(env, "admin/form:versions"),
  "admin/form:fields": (env) => renderLeafScreen(env, "admin/form:fields"),
  "admin/form:validations": (env) => renderLeafScreen(env, "admin/form:validations"),
  "admin/form:linkage-rule": (env) => renderLeafScreen(env, "admin/form:linkage-rule"),
  "admin/form:migration": (env) => renderLeafScreen(env, "admin/form:migration"),
  "admin/form:efile-map": (env) => renderLeafScreen(env, "admin/form:efile-map"),
  "admin/form:by-set": (env) => renderLeafScreen(env, "admin/form:by-set"),
  "admin/form:impact": (env) => renderLeafScreen(env, "admin/form:impact"),
  "admin/code:manage": (env) => renderLeafScreen(env, "admin/code:manage"),
  "admin/audit:events": (env) => renderLeafScreen(env, "admin/audit:events"),
  "admin/audit:login": (env) => renderLeafScreen(env, "admin/audit:login"),
  "admin/audit:perm": (env) => renderLeafScreen(env, "admin/audit:perm"),
  "admin/audit:settings": (env) => renderLeafScreen(env, "admin/audit:settings"),
});

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
  const spec = enrichLeafSpec(key, leafScreenSpecs[key]);
  const meta = { ...(env.routeMeta || routeMeta(key)), leafKey: key };
  const roles = env.auth?.user?.roles || [];
  if (!roles.includes("SUPER_ADMIN") && !roles.includes("TENANT_ADMIN")) {
    env.outlet.innerHTML = renderEmptyState(key, {
      kind: "perm",
      title: "권한이 없습니다",
      message: "테넌트 관리는 SUPER_ADMIN 또는 TENANT_ADMIN 권한이 필요합니다.",
    }, meta, spec);
    return;
  }
  await renderAdminTenants(env);
}

async function renderLeafScreen(env, key) {
  const spec = enrichLeafSpec(key, leafScreenSpecs[key]);
  const meta = { ...(env.routeMeta || routeMeta(key)), leafKey: key };
  const gate = leafGate(env, key, spec);
  if (gate) {
    env.outlet.innerHTML = renderEmptyState(key, gate, meta, spec);
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
      title: "데이터를 불러오지 못했습니다",
      message: error.message,
      action: "retry",
    }, meta, spec, primaryApi, actionApi);
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

function enrichLeafSpec(key, spec) {
  return {
    ...spec,
    typology: spec.typology || leafTypology(key),
    rowKey: spec.rowKey || inferRowKey(key),
    update: spec.update || { method: "PATCH", path: "/api/tenants/{tenant}/leaf-records/{recordId}", fallback: "leaf-action" },
    description: spec.description || leafDescription(key),
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

function leafDescription(key) {
  const typology = leafTypology(key);
  if (typology === "grid") return "검색, 필터, 추가, 수정, 삭제를 표 안에서 처리합니다.";
  if (typology === "grid-tree") return "좌측 트리로 범주를 좁히고 우측 표에서 데이터를 관리합니다.";
  if (typology === "dashboard") return "주요 지표와 보조 업무 카드를 한 화면에 표시합니다.";
  if (typology === "wizard") return "단계별 확인과 실행 흐름을 제공합니다.";
  if (typology === "form") return "입력 폼과 미리보기를 나란히 표시합니다.";
  if (typology === "chart") return "지표를 차트와 보조 표로 요약합니다.";
  return "선택한 객체의 상세 정보와 관련 데이터를 표시합니다.";
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
  return `
    <section class="leaf-workbench leaf-typology layout-tree-and-grid" data-typology="grid-tree" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <aside class="panel tree-panel">
        <div class="panel-head"><div><h2>분류</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
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
  const steps = ["대상 확인", "검증", "실행", "결과"];
  const active = wizardActiveStep(state.key);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="wizard" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <ol class="wizard-stepper">
        ${steps.map((step, index) => `<li class="${index + 1 < active ? "done" : index + 1 === active ? "active" : ""}"><span>${index + 1}</span>${escapeHtml(step)}</li>`).join("")}
      </ol>
      <section class="panel wizard-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <button class="secondary-btn compact" type="button" data-step-edit data-row-id="${escapeHtml(firstRowId(state))}">단계 수정</button>
        </div>
        ${renderWizardBody(state, active)}
        <div class="wizard-nav">
          <button class="secondary-btn" type="button" data-wizard-prev ${active === 1 ? "disabled" : ""}>이전</button>
          <button class="primary-btn" type="button" data-wizard-next>${active === steps.length ? "완료" : "다음"}</button>
        </div>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyForm(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = editableLeafColumns(state, row);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="form" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="grid two form-typology-body">
        <article class="panel">
          <div class="panel-head"><div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
          <form class="stack" data-leaf-form data-row-id="${escapeHtml(row.__rowId || "")}">
            ${columns.map((column) => renderEditField(column, row[column.key])).join("")}
            <button class="primary-btn" type="submit">저장</button>
          </form>
        </article>
        <article class="panel form-preview">
          <div class="panel-head"><h2>미리보기</h2></div>
          ${renderObjectTable(row, leafColumns([row], state).slice(0, 6), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyChart(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="chart" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      ${renderLeafSummaryBlock(state, rows)}
      <section class="panel chart-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <div class="panel-head-actions">
            <select data-chart-range aria-label="차트 범위"><option>3y</option><option selected>5y</option><option>10y</option></select>
            <button class="secondary-btn compact" type="button" data-chart-config-edit data-row-id="${escapeHtml(firstRowId(state))}">설정 수정</button>
          </div>
        </div>
        <div class="chart-area" data-chart-target>
          ${renderChartBars(rows)}
        </div>
        ${renderLeafTableShell(state, rows.slice(0, 8), columns)}
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyDetail(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = leafColumns([row], state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="detail" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="panel detail-header">
        <div class="panel-head">
          <div>
            <span class="badge info">Detail</span>
            <h2>${escapeHtml(detailTitle(state, row))}</h2>
            <p>${escapeHtml(state.spec.description)}</p>
          </div>
          <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(row.__rowId || "")}">수정</button>
        </div>
      </section>
      <section class="grid two detail-body">
        <article class="panel">
          <div class="panel-head"><h2>기본 정보</h2></div>
          ${renderObjectTable(row, columns.slice(0, 8), state)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>관련 데이터</h2></div>
          ${renderObjectTable(row, columns.slice(8, 16).length ? columns.slice(8, 16) : columns.slice(0, 4), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderLeafSummaryBlock(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const custom = rows.filter((row) => row.__source === "leaf_records").length;
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
        ["전체", money.format(rows.length)],
        ["활성", money.format(active)],
        ["사용자 추가", money.format(custom)],
        ["권한", `${state.spec.perm.module}:${state.spec.perm.function}`],
      ])}
    </section>`;
}

function renderLeafTableBlock(state, rows, columns = leafColumns(rows, state)) {
  return `
    <section class="panel leaf-table" data-leaf-block="table">
      <div class="panel-head">
        <div><h2>${escapeHtml(state.meta.title || "목록")}</h2><p>${escapeHtml(rows.length)}건 표시 · ${escapeHtml(state.spec.description)}</p></div>
        <div class="panel-head-actions" data-leaf-block="filters">
          ${renderLeafFilterControls(state)}
          <button class="primary-btn compact" type="button" data-leaf-create="${escapeHtml(state.key)}">+ 추가</button>
        </div>
      </div>
      ${renderLeafTableShell(state, rows, columns)}
    </section>`;
}

function renderLeafTableShell(state, rows, columns = leafColumns(rows, state)) {
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${columns.map((column) => `<th class="${escapeHtml(leafHeadClass(column))}">${escapeHtml(column.label)}</th>`).join("")}<th class="row-actions-th">관리</th></tr></thead>
        <tbody data-leaf-table-body>${renderLeafTableRows(state, rows, columns)}</tbody>
      </table>
    </div>`;
}

function renderLeafFilterControls(state) {
  return `
    <label class="inline-control">검색 <input type="search" data-leaf-filter="q" value="${escapeHtml(state.query)}" placeholder="키워드" /></label>
    <label class="inline-control">상태
      <select data-leaf-filter="status">
        ${["ALL", "ACTIVE", "DRAFT", "IN_REVIEW", "APPROVED", "FILED", "SUSPENDED"].map((status) => `<option value="${status}" ${state.status === status ? "selected" : ""}>${status}</option>`).join("")}
      </select>
    </label>
    <button class="secondary-btn compact" type="button" data-leaf-filter-reset>초기화</button>`;
}

function renderLeafTableRows(state, rows, columns = leafColumns(rows, state)) {
  if (!rows.length) {
    return `<tr><td colspan="${columns.length + 1}"><div class="empty-state compact"><strong>데이터가 없습니다</strong><p class="empty">+ 추가 버튼으로 새 행을 만들 수 있습니다.</p></div></td></tr>`;
  }
  return rows.map((item) => `
    <tr data-leaf-row="${escapeHtml(item.__rowId)}">
      ${columns.map((column) => `<td class="${escapeHtml(leafCellClass(column))}" data-format="${escapeHtml(column.format)}">${formatLeafValue(item[column.key], column, item, state)}</td>`).join("")}
      <td class="row-actions" data-format="actions">${renderLeafRowActions(state, item)}</td>
    </tr>`).join("");
}

function renderLeafRowActions(state, item) {
  return `
    ${renderLeafPrimaryRowAction(state, item)}
    <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(item.__rowId)}" title="선택 행 수정">수정</button>
    <button class="danger-btn compact" type="button" data-row-delete data-leaf-row-action="delete" data-row-id="${escapeHtml(item.__rowId)}" title="선택 행 삭제">삭제</button>`;
}

function renderLeafPrimaryRowAction(state, item) {
  if (state.key === "ws/start:customer-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-customer" data-row-id="${escapeHtml(item.__rowId)}">고객사 선택</button>`;
  }
  if (state.key === "ws/start:by-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-by" data-row-id="${escapeHtml(item.__rowId)}">사업연도 선택</button>`;
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
      setLeafActionMessage("삭제되었습니다.");
      rerenderLeaf(env, state);
    }
  } catch (error) {
    setLeafActionMessage(error.message, true);
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
    setLeafActionMessage("저장되었습니다.");
    rerenderLeaf(env, state);
  } catch (error) {
    if (message) message.textContent = error.message;
    setLeafActionMessage(error.message, true);
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
    setLeafActionMessage("추가되었습니다.");
    rerenderLeaf(env, state);
  } catch (error) {
    setLeafActionMessage(error.message, true);
  } finally {
    button.disabled = false;
  }
}

function openEditModal(env, state, row) {
  closeLeafModal(env);
  const columns = editableLeafColumns(state, row);
  env.outlet.insertAdjacentHTML("beforeend", `
    <section class="leaf-modal-backdrop" data-leaf-modal>
      <form class="leaf-edit-modal" data-leaf-edit-form data-row-id="${escapeHtml(row.__rowId || "")}">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)} 수정</h2><p>${escapeHtml(row.__rowId || state.spec.rowKey || "-")}</p></div>
          <button class="secondary-btn compact" type="button" data-edit-close>취소</button>
        </div>
        <div class="form-grid">
          ${columns.map((column) => renderEditField(column, row[column.key])).join("")}
        </div>
        <p class="edit-error" data-edit-error></p>
        <div class="button-row">
          <button class="primary-btn" type="submit">저장</button>
          <button class="secondary-btn" type="button" data-edit-close>취소</button>
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
  if (tableHead) tableHead.textContent = `${rows.length}건 표시 · ${state.spec.description}`;
}

function rerenderLeaf(env, state) {
  env.outlet.innerHTML = renderLeafTemplate(state);
  bindLeafTemplate(env, state);
}

function selectLeafCustomer(env, state, row) {
  const customerId = row.customer_id || row.id;
  if (!customerId) {
    throw new Error("고객사 ID가 없어 선택할 수 없습니다.");
  }
  env.setContext({
    customerId,
    customerName: row.customer_name || row.name || row.customer_code || env.context.customerName,
  });
  setLeafActionMessage("고객사를 선택했습니다. 사업연도를 선택하세요.");
  env.navigate("ws/start:by-pick", { customerId });
}

async function selectLeafBusinessYear(env, state, row) {
  const byId = row.by_id || row.business_year_id || row.id;
  if (!byId || !row.customer_id) {
    throw new Error("사업연도 ID 또는 고객사 ID가 없어 선택할 수 없습니다.");
  }
  const by = { ...row, by_id: byId };
  const customer = await customerForBusinessYear(env, by);
  await refreshContextFromBy(env, by, customer);
  setLeafActionMessage("사업연도를 선택했습니다.");
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
  if (state?.spec?.columns?.length) {
    return state.spec.columns.map((column) => ({ ...column, format: column.format || inferColumnFormat(column.key) }));
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
    label: leafColumnLabel(key),
    format: inferColumnFormat(key),
  }));
}

function prioritizeLeafKeys(keys) {
  const preferred = ["row_id", "record_id", "tenant_code", "customer_code", "customer_name", "login_id", "role_code", "menu_key", "title", "name", "status", "severity", "year_label", "amount", "tax_due", "progress", "biz_reg_no", "corp_reg_no", "email", "phone", "created_at"];
  return [...preferred.filter((key) => keys.includes(key)), ...keys.filter((key) => !preferred.includes(key))];
}

function leafColumnLabel(key) {
  const labels = {
    row_id: "ID",
    record_id: "ID",
    tenant_code: "테넌트",
    customer_code: "고객사 코드",
    customer_name: "고객사",
    login_id: "사용자",
    role_code: "역할",
    menu_key: "메뉴",
    title: "제목",
    name: "이름",
    status: "상태",
    severity: "등급",
    year_label: "사업연도",
    amount: "금액",
    tax_due: "세액",
    progress: "진행률",
    biz_reg_no: "사업자번호",
    corp_reg_no: "법인등록번호",
    email: "이메일",
    phone: "전화",
    created_at: "생성일",
    updated_at: "수정일",
  };
  return labels[key] || key.replaceAll("_", " ");
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
  if (format === "money") return `<span class="num">${escapeHtml(`${money.format(Number(value) || 0)}원`)}</span>`;
  if (format === "bps") return `${((Number(value) || 0) / 100).toFixed(2)}%`;
  if (format === "date") return escapeHtml(formatDate(value));
  if (format === "datetime") return escapeHtml(formatDateTime(value));
  if (format === "biz") return `<span class="code-cell">${escapeHtml(formatBizNo(value))}</span>`;
  if (format === "corp") return `<span class="code-cell">${escapeHtml(formatCorpNo(value))}</span>`;
  if (format === "tags") return renderTags(value);
  if (format === "status") return pill(value);
  if (format === "severity") return `<span class="badge ${escapeHtml(severityClass(value))}">${escapeHtml(value)}</span>`;
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
  return {
    title: `${state.meta.title || state.key} 항목`,
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

function setLeafActionMessage(message, error = false) {
  const result = document.querySelector(".leaf-action-result");
  if (result) result.innerHTML = `<strong>${error ? "액션 실패" : "액션 완료"}</strong><p class="empty">${escapeHtml(message)}</p>`;
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
    { key: "title", label: "제목", format: "text" },
    { key: "status", label: "상태", format: "status" },
  ];
}

function renderEditField(column, value) {
  const inputType = editInputType(column.format);
  if (column.format === "boolean") {
    return `<label class="checkbox-field"><span>${escapeHtml(column.label)}</span><input name="${escapeHtml(column.key)}" type="checkbox" ${value ? "checked" : ""} /></label>`;
  }
  if (column.format === "tags") {
    return `<label>${escapeHtml(column.label)}<input name="${escapeHtml(column.key)}" value="${escapeHtml(asArray(value).join(", "))}" placeholder="쉼표로 구분" /></label>`;
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
  rows.forEach((row) => {
    const raw = row.parent_key || row.menu_key || row.group_code || row.category || row.status || "전체";
    const label = String(raw).split(/[/:.]/)[0] || "전체";
    groups.set(label, (groups.get(label) || 0) + 1);
  });
  if (!groups.size) return `<p class="empty">표시할 분류가 없습니다.</p>`;
  return `<ul class="leaf-tree">${[...groups.entries()].map(([label, count]) => `<li><button type="button" class="secondary-btn compact" data-tree-node="${escapeHtml(label)}">${escapeHtml(label)} <span>${money.format(count)}</span></button></li>`).join("")}</ul>`;
}

function dashboardMetrics(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const warnings = rows.filter((row) => ["WARN", "ERROR"].includes(String(row.severity || "").toUpperCase())).length;
  return [
    ["전체", money.format(rows.length), "info"],
    ["활성", money.format(active), "ok"],
    ["대기", money.format(rows.filter((row) => String(row.status || "").includes("PENDING")).length), "warn"],
    ["주의", money.format(warnings), "warn"],
    ["사용자 추가", money.format(rows.filter((row) => row.__source === "leaf_records").length), "info"],
  ];
}

function dashboardCards(state, rows) {
  const sample = rows.slice(0, 5);
  const list = sample.length
    ? `<ul class="compact-list">${sample.map((row) => `<li><strong>${escapeHtml(detailTitle(state, row))}</strong><span>${escapeHtml(row.status || row.severity || row.created_at || "-")}</span></li>`).join("")}</ul>`
    : `<p class="empty">표시할 항목이 없습니다.</p>`;
  return [
    { title: "업무 현황", caption: state.key, body: list },
    { title: "최근 항목", caption: `${sample.length}건`, body: list },
    { title: "가이드", caption: state.spec.typology, body: `<p class="empty">${escapeHtml(state.spec.description)}</p>` },
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
  return `
    <div class="wizard-body" data-wizard-step="${active}">
      ${metrics([
        ["단계", `${active}/4`],
        ["대상", money.format(state.rows.length)],
        ["상태", state.rows[0]?.status || "READY"],
        ["유형", state.spec.typology],
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

function renderChartBars(rows) {
  const points = rows.slice(0, 8).map((row) => ({ label: chartLabel(row), value: chartValue(row) }));
  const max = Math.max(...points.map((point) => point.value), 1);
  if (!points.length) return `<p class="empty">차트로 표시할 데이터가 없습니다.</p>`;
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
  if (!columns.length) return `<p class="empty">표시할 필드가 없습니다.</p>`;
  return table(["항목", "값"], columns.map((column) => row([
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
  if (spec.requires.includes("work-context") && !hasWorkContext(env.context)) {
    return {
      kind: "ctx",
      title: "작업 컨텍스트가 필요합니다",
      message: "이 메뉴를 열려면 먼저 고객사와 사업연도를 선택해야 합니다.",
      action: "work-start",
    };
  }
  if (!canAccessLeaf(env, spec.perm)) {
    return {
      kind: "perm",
      title: "권한이 없습니다",
      message: `${spec.perm.module}:${spec.perm.function} 권한이 필요합니다.`,
    };
  }
  const flag = spec.featureFlag || env.routeMeta?.feature_flag || null;
  if (flag && !isFeatureEnabled(env, flag)) {
    return {
      kind: "flag",
      title: "기능 플래그가 꺼져 있습니다",
      message: `${flag} 기능이 활성화되어야 사용할 수 있습니다.`,
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

function renderEmptyState(key, gate, meta, spec, primaryApi = "", actionApi = "") {
  return `
    <section class="panel empty-state" data-leaf-key="${escapeHtml(key)}" data-empty-kind="${escapeHtml(gate.kind)}" data-primary-api="${escapeHtml(primaryApi)}" data-action-api="${escapeHtml(actionApi)}">
      <div class="panel-head">
        <div>
          <span class="badge warn">Empty state</span>
          <h2>${escapeHtml(meta.title || key)}</h2>
          <p>${escapeHtml(key)} · ${escapeHtml(spec.perm.module)}:${escapeHtml(spec.perm.function)}</p>
        </div>
      </div>
      <p class="empty">${escapeHtml(gate.message)}</p>
      ${gate.action === "work-start" ? '<button id="goStart" class="primary-btn" type="button">고객사·연도 선택하기</button>' : ""}
      ${gate.action === "retry" ? '<button id="retryLeaf" class="primary-btn" type="button">다시 시도</button>' : ""}
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

export function routeMeta(key) {
  const meta = routes[key] || routes.dashboard;
  return { group: meta.group, title: meta.title, layout: meta.layout, delegate: meta.delegate, s1: meta.s1 };
}

export async function refreshHealth(badge, text, locale = "ko") {
  try {
    await request("/health");
    badge.className = "health-badge ok";
    text.textContent = "정상";
    text.textContent = t(locale, "health.ok");
  } catch {
    badge.className = "health-badge error";
    text.textContent = "오류";
    text.textContent = t(locale, "health.error");
  }
}

export async function renderScreen(env) {
  const meta = routes[env.key] || routes.dashboard;
  const displayMeta = { ...meta, ...(env.routeMeta || {}) };
  const showFlowChrome = shouldShowFlowChrome(env.key, meta);
  if (showFlowChrome) {
    renderLawBanner(env.lawBanner, env.context);
  } else {
    hideLawBanner(env.lawBanner);
  }
  const screen = screenByLeaf[env.key] || screenByDelegate[meta.delegate] || renderDashboard;
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
      <div class="leaf-subnav" aria-label="Sibling leaf navigation">
        ${siblings.map(([siblingKey, siblingMeta]) => `
          <a class="${siblingKey === key ? "active" : ""}" href="${escapeHtml(keyToHash(siblingKey))}" data-leaf-nav="${escapeHtml(siblingKey)}">
            ${escapeHtml(siblingMeta.title)}
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
      <div class="panel-head"><h2>작업 컨텍스트가 필요합니다</h2></div>
      <p class="empty">이 신고 작업 메뉴를 열려면 먼저 고객사와 사업연도를 선택해야 합니다. 아래 버튼을 누르면 작업 시작 화면으로 이동합니다.</p>
      <button id="goStart" class="primary-btn" type="button">고객사·연도 선택하기</button>
    </section>`;
  document.getElementById("goStart").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  return false;
}

function renderLawBanner(container, context) {
  if (!hasWorkContext(context)) {
    container.classList.remove("hidden");
    container.classList.add("empty");
    container.innerHTML = `
      <div>
        <span>업무 흐름</span>
        <strong>고객사와 사업연도를 선택하면 적용 법령과 서식 버전을 표시합니다.</strong>
      </div>
      <button class="secondary-btn compact" type="button" data-flow-start>작업 시작</button>`;
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
    <div><span>고객사</span><strong>${escapeHtml(context.customerName || "-")}</strong></div>
    <div><span>사업연도</span><strong>${escapeHtml(context.fy || "-")}</strong></div>
    <div><span>적용 법령</span><strong>${escapeHtml(lawLabel(data.law_version?.version_code || snapshot.law_version_id || "-"))}</strong></div>
    <div><span>서식 버전</span><strong>${escapeHtml(lawLabel(data.form?.version_no || data.form_version || snapshot.form_version_id || "-"))}</strong></div>
  `;
}

async function appendNextStepCard(outlet, env) {
  const key = env.routeKey || env.leafKey || env.key;
  if (!hasWorkContext(env.context)) {
    outlet.insertAdjacentHTML("beforeend", `
      <section class="flow-next-card" data-flow-card="${escapeHtml(key)}">
        <div class="panel-head">
          <div><h2>다음 단계 추천</h2><p class="empty">고객사와 사업연도를 선택하면 다음 업무 단계가 표시됩니다.</p></div>
          <button class="primary-btn" type="button" data-next-leaf="ws/start:customer-pick">작업 시작</button>
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
  const next = progress.recommendations?.[0] || { leaf_key: progress.next_leaf || "ws/info:fs", label: "Next step", enabled: true };
  outlet.insertAdjacentHTML("beforeend", `
    <section class="flow-next-card" data-flow-card="${escapeHtml(key)}" data-progress-api="${escapeHtml(`${workRoot(env)}/progress`)}">
      <div class="panel-head">
        <div>
          <span class="badge ok">Workflow</span>
          <h2>다음 단계 추천</h2>
          <p>${escapeHtml(progress.status || env.context.status || "DRAFT")} · 진행률 ${escapeHtml(progress.progress ?? env.context.progress ?? 0)}%</p>
        </div>
        <button class="primary-btn" type="button" data-next-leaf="${escapeHtml(next.leaf_key)}" ${next.enabled === false ? "disabled" : ""}>${escapeHtml(next.label || "다음 단계")}</button>
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

function table(headers, rows, empty = "데이터가 없습니다.") {
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${headers.map((head) => `<th>${escapeHtml(head)}</th>`).join("")}</tr></thead>
        <tbody>${rows.length ? rows.join("") : `<tr><td colspan="${headers.length}">${escapeHtml(empty)}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function row(cells) {
  return `<tr>${cells.map((cell) => `<td>${cell}</td>`).join("")}</tr>`;
}

function pill(status) {
  return `<span class="status-pill ${statusClass(status)}">${escapeHtml(status || "-")}</span>`;
}

function renderSnapshotSummary(snapshot) {
  const data = snapshot?.snapshot_data || {};
  const law = data.law || data.law_version || {};
  const form = data.form || {};
  return table(["항목", "값"], [
    row(["스냅샷 ID", escapeHtml(snapshot?.snapshot_id || "-")]),
    row(["법령 버전", escapeHtml(law.version_code || snapshot?.law_version_id || "-")]),
    row(["서식 버전", escapeHtml(form.version_no || form.form_version || snapshot?.form_version_ids || "-")]),
    row(["잠금", snapshot?.locked ? "Y" : "N"]),
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

async function renderDashboard(env) {
  const root = routeRoot(env);
  const [summary, notifications, queue, audit] = await Promise.all([
    request(`${root}/dashboard`),
    request(`${root}/notifications`),
    request(`${root}/workflow/queue?assignee=me`),
    request(`${root}/audit-logs`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([
        ["작성 중", summary.business_year_count - summary.filed_count],
        ["검증/결재 대기", summary.pending_review_count],
        ["승인/신고 완료", summary.filed_count],
        ["마감 임박", summary.due_soon_count],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>내 결재함</h2><button id="dashApproval" class="secondary-btn compact" type="button">열기</button></div>
          ${table(["사업연도", "고객사", "담당자", "대기"], queue.map((item) => row([
            escapeHtml(item.year_label),
            escapeHtml(item.customer_name),
            escapeHtml(item.approver_login_id || "-"),
            `${item.pending_days}일`,
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>최근 알림</h2><button id="dashAlerts" class="secondary-btn compact" type="button">열기</button></div>
          ${table(["등급", "제목", "상태"], notifications.slice(0, 8).map((item) => row([
            `<span class="badge ${item.severity === "WARN" ? "warn" : "info"}">${escapeHtml(item.severity)}</span>`,
            escapeHtml(item.title),
            escapeHtml(item.status),
          ])))}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>최근 활동</h2><button id="dashAudit" class="secondary-btn compact" type="button">감사 로그</button></div>
        ${table(["모듈", "작업", "사용자", "일시"], audit.slice(0, 10).map((item) => row([
          escapeHtml(item.table_name),
          escapeHtml(item.action),
          escapeHtml(item.changed_by),
          escapeHtml(item.changed_at),
        ])))}
      </article>
    </section>`;
  document.getElementById("dashApproval").addEventListener("click", () => env.navigate("ws-appr"));
  document.getElementById("dashAlerts").addEventListener("click", () => env.navigate("rp-alerts"));
  document.getElementById("dashAudit").addEventListener("click", () => env.navigate("ad-audit"));
}

async function renderWorkStart(env) {
  const root = routeRoot(env);
  const [customers, years] = await Promise.all([
    request(`${root}/customers`),
    request(`${root}/business-years`),
  ]);
  const customerOptions = customers
    .map((customer) => `<option value="${customer.customer_id}">${escapeHtml(customer.customer_name)} (${escapeHtml(customer.customer_code)})</option>`)
    .join("");
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>고객사/사업연도</h2></div>
        ${table(["고객사", "사업연도", "상태", "진행", ""], years.map((by) => {
          const customer = customers.find((item) => item.customer_id === by.customer_id);
          return row([
            escapeHtml(customer?.customer_name || by.customer_id),
            escapeHtml(by.year_label),
            pill(by.status),
            `<div class="bar-track"><span style="width:${progressForStatus(by.status)}%"></span></div>`,
            `<button class="primary-btn compact" type="button" data-select-by="${by.by_id}">계속</button>`,
          ]);
        }))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>신규 사업연도</h2></div>
        <form id="businessYearForm" class="stack">
          <label>고객사 <select id="byCustomer">${customerOptions}</select></label>
          <label>사업연도 <input id="byYear" type="number" value="${new Date().getFullYear()}" /></label>
          <div class="form-grid">
            <label>시작일 <input id="byStart" type="date" value="${new Date().getFullYear()}-01-01" /></label>
            <label>종료일 <input id="byEnd" type="date" value="${new Date().getFullYear()}-12-31" /></label>
          </div>
          <button class="primary-btn" type="submit">등록</button>
        </form>
        <div id="snapshotPreview" class="stack"></div>
      </article>
    </section>`;

  document.querySelectorAll("[data-select-by]").forEach((button) => {
    button.addEventListener("click", async () => {
      const by = years.find((item) => String(item.by_id) === button.dataset.selectBy);
      const customer = customers.find((item) => item.customer_id === by.customer_id);
      await refreshContextFromBy(env, by, customer);
      document.getElementById("snapshotPreview").innerHTML = renderSnapshotSummary(env.context.snapshot || {});
      env.navigate("ws-info", { byId: by.by_id, customerId: by.customer_id });
    });
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
      }),
    });
    const customer = customers.find((item) => item.customer_id === by.customer_id);
    await refreshContextFromBy(env, by, customer);
    env.navigate("ws-info", { byId: by.by_id, customerId: by.customer_id });
  });
}

async function renderWorkInfo(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [validation, fs, assets, transactions, vehicleLogs, batches] = await Promise.all([
    request(`${root}/tax-data/validation`),
    request(`${root}/tax-data/financial-statements`),
    request(`${root}/tax-data/assets`),
    request(`${root}/tax-data/transactions`),
    request(`${root}/vehicle-usage-logs`),
    request(`${root}/tax-data/import-batches`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([
        ["재무제표 라인", validation.fs_line_count],
        ["자산", validation.asset_count],
        ["거래", validation.transaction_count],
        ["검증", validation.balanced ? "일치" : "불일치"],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>가져오기</h2></div>
          <form id="importForm" class="stack">
            <label>자료 유형
              <select id="importType">
                <option value="financial-statements">재무제표</option>
                <option value="assets">자산</option>
                <option value="transactions">거래</option>
              </select>
            </label>
            <label>CSV/Excel <input id="importFile" type="file" /></label>
            <button class="primary-btn" type="submit">업로드</button>
          </form>
          ${table(["유형", "파일", "건수", "오류"], batches.map((batch) => row([
            escapeHtml(batch.data_type),
            escapeHtml(batch.source_file_name || "-"),
            money.format(batch.row_count),
            money.format(batch.error_count),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>일관성 검증</h2><button id="taxDataValidate" class="secondary-btn compact" type="button">실행</button></div>
          ${renderTaxDataValidationSummary(validation)}
        </article>
      </section>
      <section class="grid three">
        <article class="panel">${table(["계정", "표준계정", "금액"], fs.slice(0, 10).map((item) => row([escapeHtml(item.account_name), escapeHtml(item.standard_account_name || "-"), money.format(item.amount)])))}</article>
        <article class="panel">${table(["자산", "분류", "취득가"], assets.slice(0, 10).map((item) => row([escapeHtml(item.asset_name), escapeHtml(item.asset_category), money.format(item.acquisition_cost)])))}</article>
        <article class="panel">${table(["거래처", "분류", "금액"], transactions.slice(0, 10).map((item) => row([escapeHtml(item.partner_name), escapeHtml(item.category), money.format(item.amount)])))}</article>
      </section>
      <article class="panel">${table(["자산 ID", "월", "업무사용률"], vehicleLogs.map((item) => row([escapeHtml(item.asset_id), escapeHtml(item.usage_month), `${(item.business_use_bps / 100).toFixed(1)}%`])))}</article>
    </section>`;

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
    await renderWorkInfo(env);
  });
}

async function renderAdjustments(env) {
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
        ${renderDataGrid({ id: "B1", title: "B1 Item Grid", subtitle: "Income add/deduct", rows: b1Items, columns: adjustmentGridColumns, importable: true, runLabel: "Add sample" })}
        ${renderDataGrid({ id: "B4", title: "B4 Item Grid", subtitle: "Depreciation", rows: b4Items, columns: adjustmentGridColumns, importable: true, runLabel: "Add sample" })}
        ${renderDataGrid({ id: "B15", title: "B15 Item Grid", subtitle: "Capital/equity", rows: b15Items, columns: adjustmentGridColumns, importable: true, runLabel: "Add sample" })}
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

async function runAdjustment(root, moduleCode) {
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

async function renderForms(env) {
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

async function renderValidation(env) {
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

function renderValidationResult(root, result) {
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

async function renderApproval(env) {
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

async function renderPrint(env) {
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

async function renderEfiling(env) {
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

async function renderPostHistory(env) {
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

async function renderPostAmend(env) {
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
    env.setContext({ status: by.status, progress: progressForStatus(by.status), lockMode: "OPEN" });
    await renderPostAmend(env);
  });
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
  const planCounts = tenants.reduce((acc, item) => {
    acc[item.plan || "STANDARD"] = (acc[item.plan || "STANDARD"] || 0) + 1;
    return acc;
  }, {});
  env.outlet.innerHTML = `
    <section class="leaf-workbench leaf-typology" data-typology="grid" data-leaf-key="admin/tenant:list">
      <section class="panel leaf-summary" data-leaf-block="summary">
      <div class="panel-head">
        <div><span class="badge info">Leaf workbench</span><h2>테넌트 관리</h2><p>admin/tenant:list · admin:READ</p></div>
      </div>
      ${metrics([
        ["전체 테넌트", tenants.length],
        ["ACTIVE", tenants.filter((item) => item.status === "ACTIVE").length],
        ["SUSPENDED", tenants.filter((item) => item.status === "SUSPENDED").length],
        ["ENTERPRISE", planCounts.ENTERPRISE || 0],
      ])}
      </section>
      <article class="panel leaf-table" data-leaf-block="table">
        <div class="panel-head">
          <div><h2>테넌트</h2><p>${tenants.length}건 표시 · 상태와 요금제를 표 안에서 관리합니다.</p></div>
          <div class="panel-head-actions" data-leaf-block="filters">
          <label>검색 <input type="search" data-tenant-filter="q" placeholder="테넌트 코드/이름" /></label>
          <label>상태 <select data-tenant-filter="status"><option>ALL</option><option>ACTIVE</option><option>SUSPENDED</option><option>CLOSED</option></select></label>
          <button class="secondary-btn compact" type="button" data-tenant-filter-reset>초기화</button>
            <button class="primary-btn compact" type="submit" form="tenantForm" ${canManage ? "" : "disabled"}>+ 추가</button>
          </div>
        </div>
        ${table(["코드", "이름", "상태", "요금제", "최대 사용자", "관리"], tenants.map((item) => row([
          escapeHtml(item.tenant_code),
          escapeHtml(item.tenant_name),
          canManage ? `<select data-tenant-status="${escapeHtml(item.tenant_code)}"><option ${item.status === "ACTIVE" ? "selected" : ""}>ACTIVE</option><option ${item.status === "SUSPENDED" ? "selected" : ""}>SUSPENDED</option><option ${item.status === "CLOSED" ? "selected" : ""}>CLOSED</option></select>` : pill(item.status),
          canManage ? `<select data-tenant-plan="${escapeHtml(item.tenant_code)}"><option ${item.plan === "FREE" ? "selected" : ""}>FREE</option><option ${item.plan === "STANDARD" ? "selected" : ""}>STANDARD</option><option ${item.plan === "PRO" ? "selected" : ""}>PRO</option><option ${item.plan === "ENTERPRISE" ? "selected" : ""}>ENTERPRISE</option></select>` : escapeHtml(item.plan || "STANDARD"),
          escapeHtml(item.max_users),
          canManage ? `<button class="secondary-btn compact" type="button" data-save-tenant="${escapeHtml(item.tenant_code)}">저장</button>` : "",
        ])))}
      </article>
      <article class="panel tenant-create-panel">
        <div class="panel-head"><h2>신규 테넌트</h2><span class="badge info">표 상단 + 추가로 저장</span></div>
        <form id="tenantForm" class="stack">
          <label>코드 <input id="tenantCodeInput" value="tenant${Date.now().toString(36).slice(-4)}" /></label>
          <label>이름 <input id="tenantNameInput" value="신규 테넌트" /></label>
          <label>사업자번호 <input id="tenantBizInput" value="1234567890" /></label>
          <label>요금제 <select id="tenantPlanInput"><option>STANDARD</option><option>PRO</option><option>ENTERPRISE</option><option>FREE</option></select></label>
          <label>Allowed IPs <input id="tenantAllowedIpsInput" placeholder="203.0.113.10/32" /></label>
          <label>계약 시작 <input id="tenantStartInput" type="date" value="${today()}" /></label>
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

async function renderAdminRoles(env) {
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
          `<button class="secondary-btn compact" data-save-menu="${escapeHtml(item.menu_key)}" type="button">저장</button>`,
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
