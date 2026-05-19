import { escapeHtml } from "/app/api.js";

const LANGUAGE_KEY = "cit.ui.language";
const TOP_LEVEL_KEYS = new Set(["dashboard", "workspace", "post", "reports", "admin"]);

const strings = Object.freeze({
  ko: {
    "health.ok": "정상",
    "health.error": "오류",
    "session.expired": "세션이 만료되었습니다.",
    "auth.failed": "로그인 실패",
    "auth.loggedOut": "로그아웃되었습니다.",
    "menu.unavailable": "로그인한 사용자에게 제공되지 않는 메뉴입니다.",
    "context.locked": "잠금",
    "context.editable": "편집 가능",
    "context.none": "작업 컨텍스트 없음",
    "context.select": "작업 시작에서 고객사와 사업연도를 선택하세요.",
    "leaf.badge": "세부 메뉴",
    "leaf.prefix": "현재 선택한 메뉴",
    "leaf.middle": "는",
    "leaf.suffix": "통합 기능 화면으로 열렸습니다.",
    "step.ws-start": "0 시작",
    "step.ws-info": "1 입력",
    "step.ws-adj": "2 조정",
    "step.ws-form": "3 서식",
    "step.ws-val": "4 검증",
    "step.ws-appr": "5 결재",
    "step.ws-print": "6 출력",
    "step.ws-file": "7 전자신고",
  },
  en: {
    "health.ok": "OK",
    "health.error": "Error",
    "session.expired": "Session expired.",
    "auth.failed": "Login failed",
    "auth.loggedOut": "Logged out.",
    "menu.unavailable": "This menu is not available for the signed-in user.",
    "context.locked": "Locked",
    "context.editable": "Editable",
    "context.none": "No work context",
    "context.select": "Select a customer and business year from Start.",
    "leaf.badge": "Leaf route",
    "leaf.prefix": "The selected menu",
    "leaf.middle": "is opened in",
    "leaf.suffix": "the integrated function screen.",
    "step.ws-start": "0 Start",
    "step.ws-info": "1 Input",
    "step.ws-adj": "2 Adjust",
    "step.ws-form": "3 Forms",
    "step.ws-val": "4 Validate",
    "step.ws-appr": "5 Approve",
    "step.ws-print": "6 Print",
    "step.ws-file": "7 E-file",
  },
});

export function loadLocale() {
  return normalizeLocale(localStorage.getItem(LANGUAGE_KEY));
}

export function saveLocale(locale) {
  const next = normalizeLocale(locale);
  localStorage.setItem(LANGUAGE_KEY, next);
  return next;
}

export function normalizeLocale(locale) {
  return locale === "en" ? "en" : "ko";
}

export function t(locale, key) {
  const normalized = normalizeLocale(locale);
  return strings[normalized]?.[key] || strings.ko[key] || key;
}

export function labelForNode(node, locale) {
  const normalized = normalizeLocale(locale);
  const labels = node?.labels || {};
  if (normalized === "en") {
    return labels.en || node?.label_en || node?.name_en || node?.label || node?.display_name || node?.name || node?.code || "";
  }
  return labels.ko || node?.label_ko || node?.label || node?.display_name || node?.name || node?.code || "";
}

export function routeLabelsFromMenu(tree, key, locale, fallbackMeta) {
  const path = findMenuPath(tree, key);
  if (!path.length) return fallbackMeta;
  const activeNode = path[path.length - 1];
  const rootNode = path.find((node) => TOP_LEVEL_KEYS.has(node.code)) || path[1] || activeNode;
  return {
    ...fallbackMeta,
    group: labelForNode(rootNode, locale) || fallbackMeta.group,
    title: labelForNode(activeNode, locale) || fallbackMeta.title,
  };
}

export function leafFocusText(locale, key, delegate) {
  if (normalizeLocale(locale) === "en") {
    return `The selected menu ${escapeHtml(key)} opened the ${escapeHtml(delegate)} integrated function screen.`;
  }
  return `현재 선택한 메뉴 ${escapeHtml(key)}는 ${escapeHtml(delegate)} 통합 기능 화면으로 열렸습니다.`;
}

function findMenuPath(node, key, ancestors = []) {
  if (!node) return [];
  const path = [...ancestors, node];
  if (node.code === key || node.key === key) return path;
  const children = Array.isArray(node.children) ? node.children : [];
  for (const child of children) {
    const found = findMenuPath(child, key, path);
    if (found.length) return found;
  }
  return [];
}
