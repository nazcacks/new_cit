import { request, setTokenGetter, setUnauthorizedHandler } from "/app/api.js";
import { loadContext, saveContext } from "/app/context.js";
import { loadLocale, routeLabelsFromMenu, saveLocale, t } from "/app/i18n.js";
import { renderMenu, renderContextBadge, renderStepper } from "/app/menu.js";
import { currentKey, navigate, onRouteChange } from "/app/router.js";
import { routeMeta, renderScreen, refreshHealth } from "/app/screens.js";

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

function setContext(next) {
  state.workContext = saveContext({ ...state.workContext, ...next });
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
  renderNavigation(currentKey());
}

function showLogin(message = "") {
  state.token = "";
  state.auth = null;
  state.menuTree = null;
  localStorage.removeItem("cit.auth.token");
  $("loginView").classList.remove("hidden");
  $("appView").classList.add("hidden");
  $("loginMessage").textContent = message;
  syncLanguageSelect();
}

function showApp(auth) {
  state.token = auth.token;
  state.auth = auth;
  state.menuTree = auth.modules;
  localStorage.setItem("cit.auth.token", state.token);
  $("loginView").classList.add("hidden");
  $("appView").classList.remove("hidden");
  $("signedTenant").textContent = `${auth.user.tenant_name} / ${auth.user.tenant_code}`;
  syncLanguageSelect();
  syncHealthLabel();
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
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
  try {
    const auth = await request("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({
        tenant_code: $("loginTenant").value.trim(),
        login_id: $("loginId").value.trim(),
        password: $("loginPassword").value,
      }),
    });
    showApp(auth);
    await refreshHealth($("healthBadge"), $("healthText"), state.locale);
    navigate("dashboard:overview");
  } catch {
    $("loginMessage").textContent = t(state.locale, "auth.failed");
  } finally {
    $("loginBtn").disabled = false;
  }
}

function renderNavigation(key) {
  const meta = routeMeta(key);
  renderMenu($("moduleMenu"), state.menuTree, state.workContext, key, navigate, state.locale);
  if (meta.layout === "workspace") {
    $("stepper").classList.remove("hidden");
    renderStepper($("stepper"), state.workContext, key, state.locale);
  } else {
    $("stepper").classList.add("hidden");
    $("stepper").innerHTML = "";
  }
}

function displayRouteMeta(key) {
  return routeLabelsFromMenu(state.menuTree, key, state.locale, routeMeta(key));
}

function syncLanguageSelect() {
  const select = $("languageSelect");
  if (select) select.value = state.locale;
}

function changeLanguage(locale) {
  state.locale = saveLocale(locale);
  syncLanguageSelect();
  syncHealthLabel();
  if (!state.auth) return;
  renderContextBadge($("contextBadge"), state.workContext, state.locale);
  renderRoute(currentKey()).catch((error) => {
    $("cwk-route-outlet").innerHTML = `<section class="panel"><p class="empty">${error.message}</p></section>`;
  });
}

function syncHealthLabel() {
  const badge = $("healthBadge");
  if (!badge) return;
  if (badge.classList.contains("ok")) {
    $("healthText").textContent = t(state.locale, "health.ok");
  } else if (badge.classList.contains("error")) {
    $("healthText").textContent = t(state.locale, "health.error");
  }
}

function menuTreeContains(node, key) {
  if (!node) return false;
  if (node.code === key || node.key === key) return true;
  const children = Array.isArray(node.children) ? node.children : [];
  return children.some((child) => menuTreeContains(child, key));
}

async function restoreSession() {
  syncLanguageSelect();
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

onRouteChange((key) => {
  if (!state.auth) {
    return;
  }
  renderRoute(key).catch((error) => {
    $("cwk-route-outlet").innerHTML = `<section class="panel"><p class="empty">${error.message}</p></section>`;
  });
});

restoreSession();
