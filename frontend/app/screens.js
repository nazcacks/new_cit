import { request, downloadBinary, escapeHtml, money, statusClass, today, asArray, jsonBlock } from "/app/api.js";
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
  };
}

async function renderLeafScreen(env, key) {
  const spec = leafScreenSpecs[key];
  const meta = { ...(env.routeMeta || routeMeta(key)), leafKey: key };
  const gate = leafGate(env, key, spec);
  if (gate) {
    env.outlet.innerHTML = renderEmptyState(key, gate, meta, spec);
    bindEmptyStateActions(env, gate);
    return;
  }

  const primaryApi = resolveApiPath(spec.primary.path, env);
  const actionApi = resolveApiPath(spec.action.path, env);
  let payload;
  try {
    payload = await request(primaryApi, apiOptions(spec.primary, key, primaryApi, env));
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

  env.outlet.innerHTML = `
    <section class="panel leaf-screen" data-leaf-key="${escapeHtml(key)}" data-primary-api="${escapeHtml(primaryApi)}" data-action-api="${escapeHtml(actionApi)}">
      <div class="panel-head">
        <div>
          <span class="badge info">Leaf screen</span>
          <h2>${escapeHtml(meta.title || key)}</h2>
          <p>${escapeHtml(key)} · ${escapeHtml(spec.perm.module)}:${escapeHtml(spec.perm.function)}</p>
        </div>
        <button class="primary-btn" type="button" data-leaf-action="${escapeHtml(key)}">기능 실행</button>
      </div>
      <div class="leaf-api-summary">
        <span>1차 API</span><strong>${escapeHtml(spec.primary.method)} ${escapeHtml(primaryApi)}</strong>
        <span>액션 API</span><strong>${escapeHtml(spec.action.method)} ${escapeHtml(actionApi)}</strong>
      </div>
      ${renderPayload(payload)}
      <div class="leaf-action-result" aria-live="polite"></div>
    </section>`;
  bindLeafAction(env, key, spec, primaryApi, actionApi);
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

function bindLeafAction(env, key, spec, primaryApi, actionApi) {
  document.querySelector(`[data-leaf-action="${cssEscape(key)}"]`)?.addEventListener("click", async (event) => {
    const button = event.currentTarget;
    const resultEl = document.querySelector(".leaf-action-result");
    button.disabled = true;
    try {
      const result = await request(actionApi, apiOptions(spec.action, key, primaryApi, env));
      resultEl.innerHTML = `<strong>액션 완료</strong>${jsonBlock(result)}`;
    } catch (error) {
      resultEl.innerHTML = `<strong>액션 실패</strong><p class="empty">${escapeHtml(error.message)}</p>`;
    } finally {
      button.disabled = false;
    }
  });
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

function renderPayload(payload) {
  const rows = payloadRows(payload);
  return `
    <div class="leaf-response">
      ${metrics(payloadMetrics(payload))}
      ${rows.length ? table(payloadHeaders(rows), rows.slice(0, 8).map((item) => row(payloadHeaders(rows).map((key) => escapeHtml(item[key])))), "응답 행이 없습니다.") : ""}
      <details open>
        <summary>1차 API 응답</summary>
        ${jsonBlock(payload)}
      </details>
    </div>`;
}

function payloadRows(payload) {
  if (Array.isArray(payload)) return payload.filter((item) => item && typeof item === "object");
  if (!payload || typeof payload !== "object") return [];
  for (const key of ["rows", "items", "fields", "events", "differences", "issues", "validations", "history"]) {
    if (Array.isArray(payload[key])) return payload[key].filter((item) => item && typeof item === "object");
  }
  return [payload];
}

function payloadHeaders(rows) {
  const keys = new Set();
  rows.slice(0, 5).forEach((item) => Object.keys(item).slice(0, 6).forEach((key) => keys.add(key)));
  return [...keys].slice(0, 6);
}

function payloadMetrics(payload) {
  const rows = payloadRows(payload);
  const type = Array.isArray(payload) ? "array" : typeof payload;
  const keys = payload && typeof payload === "object" && !Array.isArray(payload) ? Object.keys(payload).length : rows.length;
  return [
    ["응답 타입", type],
    ["행 수", String(rows.length)],
    ["필드 수", String(keys)],
    ["반영 상태", "OK"],
  ];
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
  if (meta.layout === "workspace") {
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
    <section class="panel work-context-empty" data-leaf-key="${escapeHtml(env.routeKey || env.leafKey || env.key)}">
      <div class="panel-head"><h2>작업 컨텍스트가 필요합니다</h2></div>
      <p class="empty">이 신고 작업 메뉴를 열려면 먼저 고객사와 사업연도를 선택해야 합니다. 아래 버튼을 누르면 작업 시작 화면으로 이동합니다.</p>
      <button id="goStart" class="primary-btn" type="button">고객사·연도 선택하기</button>
    </section>`;
  document.getElementById("goStart").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  return false;
}

function renderLawBanner(container, context) {
  if (!hasWorkContext(context)) {
    container.classList.add("hidden");
    container.innerHTML = "";
    return;
  }
  const snapshot = context.snapshot || {};
  const data = snapshot.snapshot_data || {};
  container.classList.remove("hidden");
  container.innerHTML = `
    <div><span>고객사</span><strong>${escapeHtml(context.customerName || "-")}</strong></div>
    <div><span>사업연도</span><strong>${escapeHtml(context.fy || "-")}</strong></div>
    <div><span>상태</span><strong>${escapeHtml(context.status || "-")}</strong></div>
    <div><span>적용 법령</span><strong>${escapeHtml(data.law_version?.version_code || snapshot.law_version_id || "-")}</strong></div>
  `;
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

function bindJsonDump(id, value) {
  const el = document.getElementById(id);
  if (el) el.textContent = JSON.stringify(value, null, 2);
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
      document.getElementById("snapshotPreview").innerHTML = jsonBlock(env.context.snapshot || {});
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
          ${jsonBlock(validation)}
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
        <div id="validationResult">${jsonBlock({ tax_data: taxData, efiling: efile })}</div>
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
    <pre>${escapeHtml(JSON.stringify({ pass: result.pass }, null, 2))}</pre>`;
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
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>테넌트</h2></div>
        ${table(["코드", "이름", "상태", "최대 사용자"], tenants.map((item) => row([escapeHtml(item.tenant_code), escapeHtml(item.tenant_name), escapeHtml(item.status), escapeHtml(item.max_users)])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>등록</h2></div>
        <form id="tenantForm" class="stack">
          <label>코드 <input id="tenantCodeInput" value="tenant${Date.now().toString(36).slice(-4)}" /></label>
          <label>이름 <input id="tenantNameInput" value="신규 테넌트" /></label>
          <label>사업자번호 <input id="tenantBizInput" value="1234567890" /></label>
          <label>Allowed IPs <input id="tenantAllowedIpsInput" placeholder="203.0.113.10/32" /></label>
          <label>계약 시작 <input id="tenantStartInput" type="date" value="${today()}" /></label>
          <button class="primary-btn" type="submit">등록</button>
        </form>
      </article>
    </section>`;
  document.getElementById("tenantForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/tenants", {
      method: "POST",
      body: JSON.stringify({ tenant_code: document.getElementById("tenantCodeInput").value, tenant_name: document.getElementById("tenantNameInput").value, biz_reg_no: document.getElementById("tenantBizInput").value, contract_start: document.getElementById("tenantStartInput").value, contract_end: null, allowed_ips: document.getElementById("tenantAllowedIpsInput").value || null, max_users: 10 }),
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
      <article class="panel">${jsonBlock({ customers: customers.map((item) => ({ customer_id: item.customer_id, work_scopes: item.work_scopes })) })}</article>
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
