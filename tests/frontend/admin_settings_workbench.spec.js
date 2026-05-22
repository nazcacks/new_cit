const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

for (const fn of [
  "renderAdminRolesWorkbench",
  "renderAdminMenusWorkbench",
  "renderAdminCustomerAccessWorkbench",
  "renderAdminLawWorkbench",
  "renderAdminFormsWorkbench",
  "renderAdminAuditWorkbench",
]) {
  assert(screens.includes(`async function ${fn}(env)`), `${fn} must exist`);
}

for (const needle of [
  'data-admin-stage="security"',
  'data-admin-stage="menus"',
  'data-admin-stage="customer-access"',
  'data-admin-stage="law"',
  'data-admin-stage="forms-admin"',
  'data-admin-stage="audit"',
  "Admin settings workbench",
  "Security and permission controls",
  "Menu and function governance",
  "Customer access and assignment",
  "Law and rate version control",
  "Form version administration",
  "Audit and change review",
  'request("/api/admin/field-masking")',
  'request("/api/admin/data-scope")',
  'request("/api/login-history")',
  'request("/api/system-settings")',
  'request("/api/admin/customer-groups")',
  'request("/api/admin/customer-rules")',
  'request("/api/admin/customer-access/override")',
  'request("/api/law-versioning/impact"',
  'request("/api/form-versioning/efile-map")',
  'request("/api/form-versioning/by-set")',
  'request("/api/form-versioning/impact")',
  'request("/api/permission-change-history")',
  "renderAdminRouteButtons(activeLeaf, stageRoutes, env.locale)",
]) {
  assert(screens.includes(needle), `${needle} must be present in the admin workbench implementation`);
}

for (const wrapper of [
  ["renderAdminRoles", "renderAdminRolesWorkbench(env)"],
  ["renderAdminMenus", "renderAdminMenusWorkbench(env)"],
  ["renderAdminCustomerAccess", "renderAdminCustomerAccessWorkbench(env)"],
  ["renderAdminLaw", "renderAdminLawWorkbench(env)"],
  ["renderAdminForms", "renderAdminFormsWorkbench(env)"],
  ["renderAdminAudit", "renderAdminAuditWorkbench(env)"],
]) {
  const [name, call] = wrapper;
  const start = screens.indexOf(`async function ${name}(env) {`);
  assert(start >= 0, `${name} wrapper must exist`);
  const body = screens.slice(start, start + 220);
  assert(body.includes(`return ${call};`), `${name} must delegate to ${call}`);
}

console.log("frontend admin_settings_workbench.spec.js passed");
