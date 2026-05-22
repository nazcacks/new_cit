import { escapeHtml, request, setTokenGetter, setUnauthorizedHandler } from "/app/api.js";
import { clearContext, loadContext, saveContext } from "/app/context.js";
import { loadLocale, routeLabelsFromMenu, saveLocale, t } from "/app/i18n.js";
import { renderMenu, renderContextBadge, renderStateBadge, renderStepper, renderTenantSwitcher } from "/app/menu.js";
import { currentKey, navigate, onRouteChange } from "/app/router.js";
import { routeMeta, renderScreen, refreshHealth } from "/app/screens.js";

applySmokeBootstrapFromQuery();

const state = {
  token: localStorage.getItem("cit.auth.token") || "",
  auth: null,
  menuTree: null,
  workContext: loadContext(),
  locale: loadLocale(),
};

setTokenGetter(() => state.token);
setUnauthorizedHandler(() => showLogin(t(state.locale, "session.expired")));

const $ = (id) => document.getElementById(id);
const RECENT_TENANTS_KEY = "cit.auth.recentTenants";
let tenantSuggestTimer = null;
let loginOtpChallenge = false;

function applySmokeBootstrapFromQuery() {
  const params = new URLSearchParams(window.location.search);
  const token = params.get("cit_smoke_token");
  const context = params.get("cit_smoke_context");
  if (!token && !context) return;
  if (token) localStorage.setItem("cit.auth.token", token);
  if (context) {
    try {
      const parsed = JSON.parse(context);
      saveContext(parsed);
    } catch {}
  }
  params.delete("cit_smoke_token");
  params.delete("cit_smoke_context");
  const search = params.toString();
  const next = `${window.location.pathname}${search ? `?${search}` : ""}${window.location.hash}`;
  window.history.replaceState({}, document.title, next);
}

function setContext(next) {
  state.workContext = saveContext({ ...state.workContext, ...next });
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
  renderStateBadge($("stateBadge"), state.workContext, state.locale);
  renderNavigation(currentKey());
}

function showLogin(message = "") {
  state.token = "";
  state.auth = null;
  state.menuTree = null;
  localStorage.removeItem("cit.auth.token");
  resetLoginOtpChallenge();
  renderStaticShell(state.locale);
  $("loginView").classList.remove("hidden");
  $("appView").classList.add("hidden");
  $("loginMessage").textContent = message;
  syncLanguageSelect();
  syncTenantDatalist();
}

function showApp(auth) {
  state.token = auth.token;
  state.auth = auth;
  state.menuTree = auth.modules;
  localStorage.setItem("cit.auth.token", state.token);
  saveRecentTenant(auth.user);
  renderStaticShell(state.locale);
  $("loginView").classList.add("hidden");
  $("appView").classList.remove("hidden");
  $("signedTenant").textContent = `${auth.user.tenant_name} / ${auth.user.tenant_code}`;
  syncLanguageSelect();
  syncHealthLabel();
  renderTenantSwitcher($("tenantSwitcher"), state.auth, switchTenant, state.locale);
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
  renderStateBadge($("stateBadge"), state.workContext, state.locale);
  renderNavigation(currentKey());
}

async function renderRoute(key) {
  const meta = displayRouteMeta(key);
  $("routeGroup").textContent = meta.group;
  $("routeTitle").textContent = meta.title;
  if (meta.s1 && !menuTreeContains(state.menuTree, key)) {
    $("stepper").classList.add("hidden");
    $("lawBanner").classList.add("hidden");
    $("lawBanner").innerHTML = "";
    $("cwk-route-outlet").innerHTML = `<section class="panel"><p class="empty">${t(state.locale, "menu.unavailable")}</p></section>`;
    return;
  }
  renderNavigation(key);
  await renderScreen({
    outlet: $("cwk-route-outlet"),
    lawBanner: $("lawBanner"),
    key,
    auth: state.auth,
    context: state.workContext,
    locale: state.locale,
    routeMeta: meta,
    setContext,
    navigate,
  });
}

async function submitLogin(event) {
  event.preventDefault();
  $("loginBtn").disabled = true;
  $("loginMessage").textContent = "";
  const otp = $("loginOtp")?.value.trim() || "";
  try {
    const auth = await request("/api/auth/login", {
      method: "POST",
      skipUnauthorized: true,
      body: JSON.stringify({
        tenant_code: $("loginTenant").value.trim(),
        login_id: $("loginId").value.trim(),
        password: $("loginPassword").value,
        ...(loginOtpChallenge || otp ? { otp } : {}),
      }),
    });
    resetLoginOtpChallenge();
    showApp(auth);
    await refreshHealth($("healthBadge"), $("healthText"), state.locale);
    navigate("dashboard:overview");
  } catch (error) {
    const message = error.message || "";
    if (message.includes("2fa otp is required")) {
      enableLoginOtpChallenge(t(state.locale, "auth.otpRequired"));
    } else if (message.includes("invalid 2fa otp")) {
      enableLoginOtpChallenge(t(state.locale, "auth.otpInvalid"));
    } else if (message.includes("2fa enrollment")) {
      enableLoginOtpChallenge(t(state.locale, "auth.otpEnrollmentRequired"));
    } else if (message.includes("client IP") || message.includes("allowlist")) {
      $("loginMessage").textContent = t(state.locale, "auth.ipBlocked");
    } else if (message.includes("locked") || message.includes("LOCKED")) {
      $("loginMessage").textContent = t(state.locale, "auth.accountLocked");
    } else if (message.includes("expired")) {
      $("loginMessage").textContent = t(state.locale, "auth.passwordExpired");
    } else {
      $("loginMessage").textContent = t(state.locale, "auth.failed");
    }
  } finally {
    $("loginBtn").disabled = false;
  }
}

function enableLoginOtpChallenge(message) {
  loginOtpChallenge = true;
  $("loginOtpWrap")?.classList.remove("hidden");
  $("loginOtpHelp")?.classList.remove("hidden");
  $("loginMessage").textContent = message;
  $("loginOtp")?.focus();
}

function resetLoginOtpChallenge() {
  loginOtpChallenge = false;
  $("loginOtpWrap")?.classList.add("hidden");
  $("loginOtpHelp")?.classList.add("hidden");
  if ($("loginOtp")) $("loginOtp").value = "";
}

function renderNavigation(key) {
  const meta = routeMeta(key, state.locale);
  renderMenu($("moduleMenu"), state.menuTree, state.workContext, key, navigate, state.locale, state.auth);
  if (shouldShowFlowChrome(key, meta)) {
    $("stepper").classList.remove("hidden");
    renderStepper($("stepper"), state.workContext, key, state.locale);
  } else {
    $("stepper").classList.add("hidden");
    $("stepper").innerHTML = "";
  }
}

function shouldShowFlowChrome(key, meta) {
  return meta.layout === "workspace"
    || key.startsWith("post/amend:")
    || key.startsWith("admin/law:")
    || key.startsWith("admin/form:");
}

function displayRouteMeta(key) {
  return routeLabelsFromMenu(state.menuTree, key, state.locale, routeMeta(key, state.locale));
}

function syncLanguageSelect() {
  const select = $("languageSelect");
  if (select) select.value = state.locale;
}

function setupTenantAutocomplete() {
  syncTenantDatalist();
  refreshTenantSuggestions().catch(() => {});
}

function loadRecentTenants() {
  try {
    const items = JSON.parse(localStorage.getItem(RECENT_TENANTS_KEY) || "[]");
    return Array.isArray(items) ? items.slice(0, 5) : [];
  } catch {
    return [];
  }
}

function saveRecentTenant(user) {
  if (!user?.tenant_code) return;
  const next = [
    { tenant_code: user.tenant_code, tenant_name: user.tenant_name || user.tenant_code },
    ...loadRecentTenants().filter((item) => item.tenant_code !== user.tenant_code),
  ].slice(0, 5);
  localStorage.setItem(RECENT_TENANTS_KEY, JSON.stringify(next));
  syncTenantDatalist();
}

function syncTenantDatalist(extra = []) {
  const options = [...loadRecentTenants(), ...extra]
    .filter((item) => item?.tenant_code)
    .reduce((acc, item) => {
      if (!acc.some((existing) => existing.tenant_code === item.tenant_code)) acc.push(item);
      return acc;
    }, [])
    .slice(0, 10);
  const datalist = $("tenantSuggestions");
  if (datalist) {
    datalist.innerHTML = options
      .map((item) => `<option value="${escapeHtml(item.tenant_code)}" label="${escapeHtml(item.tenant_name || item.tenant_code)}"></option>`)
      .join("");
  }
  const recent = $("recentTenants");
  if (recent) {
    const recentItems = loadRecentTenants();
    recent.innerHTML = recentItems
      .map((item) => `<button class="secondary-btn compact" type="button" data-recent-tenant="${escapeHtml(item.tenant_code)}">${escapeHtml(item.tenant_code)}</button>`)
      .join("");
    recent.querySelectorAll("[data-recent-tenant]").forEach((button) => {
      button.addEventListener("click", () => {
        $("loginTenant").value = button.dataset.recentTenant;
      });
    });
  }
}

async function refreshTenantSuggestions() {
  const q = $("loginTenant")?.value?.trim() || "";
  const suggestions = await request(`/api/public/tenant-suggest?q=${encodeURIComponent(q)}`);
  syncTenantDatalist(suggestions);
}

function renderStaticShell(locale) {
  const normalized = saveLocale(locale);
  document.documentElement.lang = normalized === "en" ? "en" : "ko";
  document.title = t(normalized, "app.title");
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    element.textContent = t(normalized, element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => {
    element.setAttribute("placeholder", t(normalized, element.dataset.i18nPlaceholder));
  });
  document.querySelectorAll("[data-i18n-title]").forEach((element) => {
    element.setAttribute("title", t(normalized, element.dataset.i18nTitle));
  });
  document.querySelectorAll("[data-i18n-attr]").forEach((element) => {
    element.dataset.i18nAttr.split(";").forEach((entry) => {
      const [attr, key] = entry.split(":");
      if (attr && key) element.setAttribute(attr, t(normalized, key));
    });
  });
}

function applyLocale(locale) {
  state.locale = saveLocale(locale);
  renderStaticShell(state.locale);
  syncLanguageSelect();
  syncHealthLabel();
  if (!state.auth) return;
  document.querySelector("[data-leaf-modal]")?.remove();
  renderTenantSwitcher($("tenantSwitcher"), state.auth, switchTenant, state.locale);
  renderStateBadge($("stateBadge"), state.workContext, state.locale);
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
  renderRoute(currentKey()).catch((error) => {
    $("cwk-route-outlet").innerHTML = `<section class="panel"><p class="empty">${error.message}</p></section>`;
  });
}

function changeLanguage(locale) {
  applyLocale(locale);
}

async function switchTenant(tenantCode) {
  if (!tenantCode || tenantCode === state.auth?.user?.tenant_code) return;
  const auth = await request("/api/auth/switch-tenant", {
    method: "POST",
    body: JSON.stringify({ tenant_code: tenantCode }),
  });
  state.workContext = clearContext();
  showApp(auth);
  await refreshHealth($("healthBadge"), $("healthText"), state.locale);
  navigate("dashboard:overview");
}

function syncHealthLabel() {
  const badge = $("healthBadge");
  if (!badge) return;
  if (badge.classList.contains("ok")) {
    $("healthText").textContent = t(state.locale, "health.ok");
  } else if (badge.classList.contains("error")) {
    $("healthText").textContent = t(state.locale, "health.error");
  } else {
    $("healthText").textContent = t(state.locale, "health.pending");
  }
}

function menuTreeContains(node, key) {
  if (!node) return false;
  if (node.code === key || node.key === key) return true;
  const children = Array.isArray(node.children) ? node.children : [];
  return children.some((child) => menuTreeContains(child, key));
}

async function restoreSession() {
  renderStaticShell(state.locale);
  syncLanguageSelect();
  setupTenantAutocomplete();
  if (!state.token) {
    showLogin();
    return;
  }
  try {
    const auth = await request("/api/auth/me");
    showApp(auth);
    await refreshHealth($("healthBadge"), $("healthText"), state.locale);
    navigate(currentKey() || "dashboard");
  } catch {
    showLogin();
  }
}

async function logout() {
  try {
    if (state.token) {
      await request("/api/auth/logout", { method: "POST", body: "{}" });
    }
  } finally {
    showLogin(t(state.locale, "auth.loggedOut"));
  }
}

$("loginForm").addEventListener("submit", submitLogin);
$("loginBtn").addEventListener("click", submitLogin);
$("logoutBtn").addEventListener("click", logout);
$("languageSelect").addEventListener("change", (event) => changeLanguage(event.target.value));
$("tenantSearchBtn")?.addEventListener("click", refreshTenantSuggestions);
$("loginTenant")?.addEventListener("input", () => {
  clearTimeout(tenantSuggestTimer);
  tenantSuggestTimer = setTimeout(refreshTenantSuggestions, 180);
});

onRouteChange((key) => {
  if (!state.auth) {
    return;
  }
  renderRoute(key).catch((error) => {
    $("cwk-route-outlet").innerHTML = `<section class="panel"><p class="empty">${error.message}</p></section>`;
  });
});

restoreSession();
