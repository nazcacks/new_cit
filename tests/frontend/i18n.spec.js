const fs = require("fs");
const assert = require("assert");

const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");
const index = fs.readFileSync("frontend/index.html", "utf8");
const app = fs.readFileSync("frontend/app.js", "utf8");
const menu = fs.readFileSync("frontend/app/menu.js", "utf8");
const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const grid = fs.readFileSync("frontend/app/components/grid.js", "utf8");
const modules = fs.readFileSync("src/modules.rs", "utf8");

function bodyOf(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert(start >= 0, `${name} must exist`);
  const next = source.indexOf("\nfunction ", start + 1);
  return source.slice(start, next > start ? next : undefined);
}

function objectBlock(source, name) {
  const match = source.match(new RegExp(`export const ${name} = Object\\.freeze\\(\\{([\\s\\S]*?)\\n\\}\\);`));
  assert(match, `${name} block is missing`);
  return match[1];
}

for (const namespace of ["common", "route", "field", "status", "modal", "grid", "typology"]) {
  assert(i18n.includes(`"${namespace}.`), `${namespace} namespace must exist in i18n.js`);
}

for (const helper of [
  "export function t(locale, key, params = {})",
  "collectMissingI18nKeys",
  "routeKeyToLabelKey",
  "fieldLabel",
  "statusLabel",
  "localizeRouteMeta",
  "labelForNode",
]) {
  assert(i18n.includes(helper), `${helper} must be exported by i18n.js`);
}

for (const fileCheck of [
  [index, 'data-i18n="app.title"', "index shell title"],
  [index, 'data-i18n="auth.logout"', "index logout"],
  [app, "function renderStaticShell(locale)", "static shell renderer"],
  [app, "function applyLocale(locale)", "locale apply function"],
  [app, "document.documentElement.lang", "document lang update"],
  [app, "renderRoute(currentKey())", "current route rerender"],
  [menu, "statusLabel(status, locale)", "state badge status translation"],
  [screens, "routeKeyToLabelKey(key)", "leaf route label key contract"],
  [screens, "function groupKeyForDelegate(delegate)", "route group label key contract"],
  [screens, 't(locale, "leaf.siblingNavigation")', "localized sibling leaf navigation label"],
  [screens, 't(locale, "common.addPrefix")', "localized add button"],
  [screens, 't(locale, "common.edit")', "localized edit button"],
  [screens, 't(locale, "common.delete")', "localized delete button"],
  [screens, 't(locale, "modal.editTitle"', "localized edit modal title"],
  [grid, "labelKey", "grid label key support"],
  [grid, "statusLabel(value, locale)", "grid status translation"],
  [modules, '"labels": {', "menu node labels response"],
]) {
  assert(fileCheck[0].includes(fileCheck[1]), `${fileCheck[2]} missing`);
}

const leafRoutesBlock = objectBlock(screens, "leafRoutes");
for (const literal of [
  '"Dashboard"',
  '"Tax workspace"',
  '"Post filing"',
  '"Analytics/reports"',
  '"Administration"',
  '"Overview"',
  '"Due soon"',
  '"Settings audit"',
]) {
  assert(!leafRoutesBlock.includes(literal), `leafRoutes must not hardcode ${literal}`);
}
assert(!screens.includes('group === "Dashboard"'), "route group resolution must not depend on English group literals");

const representativeRoutes = [
  ["route.dashboard.overview", "개요", "Overview"],
  ["route.dashboard.duesoon", "마감 임박", "Due soon"],
  ["route.ws.start.customerPick", "고객사 선택", "Customer selection"],
  ["route.ws.info.fs", "재무제표 가져오기", "Financial statements import"],
  ["route.ws.adj.B14", "B14 가산세", "B14 Additional tax"],
  ["route.ws.form.preview", "미리보기", "Preview"],
  ["route.ws.file.generate", "전자신고 파일 생성", "Generate e-file"],
  ["route.post.amend.diff", "수정신고 차이", "Amendment diff"],
  ["route.report.lossExpiry", "결손금 만료", "Loss expiry"],
  ["route.admin.sec.users", "사용자", "Users"],
  ["route.admin.form.linkageRule", "연계 규칙", "Linkage rules"],
  ["route.admin.audit.settings", "설정 변경 감사", "Settings audit"],
];

for (const [key, ko, en] of representativeRoutes) {
  assert(i18n.includes(`"${key}": "${ko}"`), `${key} Korean label missing`);
  assert(i18n.includes(`"${key}": "${en}"`), `${key} English label missing`);
}

const activeI18nBodies = [
  bodyOf(screens, "renderLeafTableBlock"),
  bodyOf(screens, "renderLeafTableShell"),
  bodyOf(screens, "renderLeafFilterControls"),
  bodyOf(screens, "renderLeafTableRows"),
  bodyOf(screens, "renderLeafRowActions"),
  bodyOf(screens, "openEditModal"),
  bodyOf(screens, "renderTypologyWizard"),
  bodyOf(screens, "renderTypologyForm"),
  bodyOf(screens, "renderTypologyChart"),
  bodyOf(screens, "renderTypologyDetail"),
].join("\n");

for (const literal of [">수정<", ">삭제<", ">저장<", ">취소<", ">관리<", ">+ 추가<", ">Edit<", ">Delete<", ">Actions<"]) {
  assert(!activeI18nBodies.includes(literal), `active leaf renderer must not hardcode ${literal}`);
}

console.log("frontend i18n.spec.js passed");
