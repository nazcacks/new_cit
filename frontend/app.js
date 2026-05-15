const state = {
  token: localStorage.getItem("cit.auth.token") || "",
  user: null,
  moduleTree: null,
  tenantCode: "",
  byId: null,
  efileJobId: "",
  activeLawPath: "/modules/law-versioning/laws",
  activeAdminPath: "/modules/admin/users",
  activeCustomerPath: "/modules/customer/profile",
  activeTaxDataPath: "/modules/tax-data/financial-statements",
  activeAdjustmentPath: "/modules/adjustment/income",
  activeFormPath: "/modules/forms/versions",
  lawVersions: [],
  selectedLawVersionId: null,
  lawSummary: null,
};

const workScopeOptions = [
  ["INFO", "기초정보"],
  ["ADJUST", "세무조정"],
  ["FORM", "서식작성"],
  ["VALIDATE", "검증"],
  ["APPROVE", "결재/검토"],
  ["PRINT", "출력"],
  ["EFILE", "전자신고"],
  ["POST", "사후관리"],
];

const allWorkScopeCodes = workScopeOptions.map(([code]) => code);

function customerAllowedWorkScopes(customer) {
  const scopes = customer?.work_scopes || customer?.workScopes;
  return Array.isArray(scopes) && scopes.length ? scopes : allWorkScopeCodes;
}

function workScopeLabels(codes) {
  const labels = new Map(workScopeOptions);
  return (codes || []).map((code) => labels.get(code) || code);
}

const lawScreens = {
  "/modules/law-versioning/laws": {
    sectionId: "law-screen-laws",
    contentId: "law-screen-laws-body",
    title: "법령 버전 마스터",
    render: renderLawMasterScreen,
  },
  "/modules/law-versioning/rates": {
    sectionId: "law-screen-rates",
    contentId: "law-screen-rates-body",
    title: "법인세율표",
    render: renderTaxRatesScreen,
  },
  "/modules/law-versioning/limits": {
    sectionId: "law-screen-limits",
    contentId: "law-screen-limits-body",
    title: "한도·율표",
    render: () =>
      renderPolicyParameterScreen({
        title: "한도·율표",
        category: "LIMIT",
        defaultItemCode: "ENTERTAINMENT_BASE_LIMIT",
        defaultAmount: 12000000,
        description: "접대비, 기부금, 대손충당금, 업무용승용차 등 한도 파라미터를 실제 DB에 저장합니다.",
      }),
  },
  "/modules/law-versioning/credits": {
    sectionId: "law-screen-credits",
    contentId: "law-screen-credits-body",
    title: "세액공제·감면 율표",
    render: () =>
      renderPolicyParameterScreen({
        title: "세액공제·감면 율표",
        category: "CREDIT",
        defaultItemCode: "RND_CREDIT_BPS",
        defaultAmount: 2500,
        description: "공제율은 bps 단위로 저장합니다. 25%는 2500입니다.",
      }),
  },
  "/modules/law-versioning/depreciation-lives": {
    sectionId: "law-screen-depreciation-lives",
    contentId: "law-screen-depreciation-lives-body",
    title: "기준내용연수표",
    render: () =>
      renderPolicyParameterScreen({
        title: "기준내용연수표",
        category: "DEPRECIATION_LIFE",
        defaultItemCode: "MACHINE_USEFUL_LIFE_YEARS",
        defaultAmount: 5,
        description: "자산 분류별 기준내용연수를 연 단위 금액 필드에 저장합니다.",
      }),
  },
  "/modules/law-versioning/sme-criteria": {
    sectionId: "law-screen-sme-criteria",
    contentId: "law-screen-sme-criteria-body",
    title: "중소기업 판정기준",
    render: () =>
      renderPolicyParameterScreen({
        title: "중소기업 판정기준",
        category: "SME_CRITERIA",
        defaultItemCode: "SME_REVENUE_LIMIT",
        defaultAmount: 12000000000,
        description: "업종별 매출액·자산총액 등 판정 기준을 버전별 파라미터로 저장합니다.",
      }),
  },
  "/modules/law-versioning/loss-rules": {
    sectionId: "law-screen-loss-rules",
    contentId: "law-screen-loss-rules-body",
    title: "결손금 공제규정",
    render: () =>
      renderPolicyParameterScreen({
        title: "결손금 공제규정",
        category: "LOSS_RULE",
        defaultItemCode: "LOSS_CARRYFORWARD_YEARS",
        defaultAmount: 15,
        description: "이월공제 기간과 공제한도율을 사업연도별 규칙으로 관리합니다.",
      }),
  },
  "/modules/law-versioning/snapshots": {
    sectionId: "law-screen-snapshots",
    contentId: "law-screen-snapshots-body",
    title: "사업연도별 적용 스냅샷",
    render: renderSnapshotScreen,
  },
  "/modules/law-versioning/impact": {
    sectionId: "law-screen-impact",
    contentId: "law-screen-impact-body",
    title: "영향 시뮬레이션",
    render: renderImpactScreen,
  },
  "/modules/law-versioning/history": {
    sectionId: "law-screen-history",
    contentId: "law-screen-history-body",
    title: "개정 공지/이력",
    render: renderHistoryScreen,
  },
};

const el = (id) => document.getElementById(id);
const money = new Intl.NumberFormat("ko-KR");

function currentLawScreen() {
  return lawScreens[state.activeLawPath] || lawScreens["/modules/law-versioning/laws"];
}

function setLawScreenHtml(html) {
  const activeScreen = currentLawScreen();
  Object.values(lawScreens).forEach((screen) => {
    const body = el(screen.contentId);
    if (!body || screen.contentId === activeScreen.contentId) {
      return;
    }
    body.innerHTML = `<p class="screen-placeholder">${escapeHtml(screen.title)} 메뉴를 선택하면 이 화면에서 기능이 열립니다.</p>`;
  });
  el(activeScreen.contentId).innerHTML = html;
}

function setActiveLawPanel() {
  document.querySelectorAll(".law-screen-panel").forEach((panel) => {
    panel.classList.toggle("active", panel.dataset.lawPath === state.activeLawPath);
  });
}

function log(message, data) {
  const time = new Date().toLocaleTimeString("ko-KR", { hour12: false });
  const line = data
    ? `[${time}] ${message}\n${JSON.stringify(data, null, 2)}`
    : `[${time}] ${message}`;
  el("logOutput").textContent = `${line}\n\n${el("logOutput").textContent}`;
}

async function request(path, options = {}) {
  const headers = {
    "Content-Type": "application/json",
    ...(state.token ? { Authorization: `Bearer ${state.token}` } : {}),
    ...(options.headers || {}),
  };
  const response = await fetch(path, { headers, ...options });
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    const message = body?.error?.message || response.statusText;
    throw new Error(message);
  }
  return body;
}

async function uploadRequest(path, formData) {
  const headers = {
    ...(state.token ? { Authorization: `Bearer ${state.token}` } : {}),
  };
  const response = await fetch(path, {
    method: "POST",
    headers,
    body: formData,
  });
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    const message = body?.error?.message || response.statusText;
    throw new Error(message);
  }
  return body;
}

function setBusy(isBusy) {
  ["runDemoBtn", "refreshBtn"].forEach((id) => {
    const button = el(id);
    if (button) {
      button.disabled = isBusy;
    }
  });
  document.querySelectorAll(".law-screen-refresh").forEach((button) => {
    button.disabled = isBusy;
  });
}

function numberValue(id) {
  return Number(el(id).value || 0);
}

function setHealth(ok, text) {
  const badge = el("healthBadge");
  badge.classList.remove("pending", "ok", "error");
  badge.classList.add(ok ? "ok" : "error");
  el("healthText").textContent = text;
  el("apiStatus").textContent = ok ? "ok" : "error";
}

function showLogin(message = "") {
  state.token = "";
  state.user = null;
  state.moduleTree = null;
  state.lawVersions = [];
  state.selectedLawVersionId = null;
  localStorage.removeItem("cit.auth.token");
  el("loginView").classList.remove("hidden");
  el("appView").classList.add("hidden");
  el("loginMessage").textContent = message;
}

function showApp(auth) {
  state.token = auth.token;
  state.user = auth.user;
  state.moduleTree = normalizeModuleTree(auth.modules);
  localStorage.setItem("cit.auth.token", state.token);
  el("loginView").classList.add("hidden");
  el("appView").classList.remove("hidden");
  el("signedTenant").textContent = `${auth.user.tenant_name} / ${auth.user.tenant_code}`;
  el("signedUser").textContent = `${auth.user.user_name} (${auth.user.login_id})`;
  el("loginStatus").textContent = "ok";
  renderModuleMenu(state.moduleTree);
  renderModuleCards(state.moduleTree);
}

async function submitLogin(event) {
  event.preventDefault();
  el("loginBtn").disabled = true;
  el("loginMessage").textContent = "";
  try {
    const auth = await request("/api/auth/login", {
      method: "POST",
      body: JSON.stringify({
        tenant_code: el("loginTenant").value,
        login_id: el("loginId").value,
        password: el("loginPassword").value,
      }),
    });
    showApp(auth);
    await refreshDashboard();
    log("로그인 완료", {
      tenant: auth.user.tenant_code,
      login_id: auth.user.login_id,
      modules: topModules(state.moduleTree).length,
      detail_modules: moduleDetailCount(state.moduleTree),
    });
  } catch (error) {
    el("loginMessage").textContent = "로그인 실패";
  } finally {
    el("loginBtn").disabled = false;
  }
}

async function restoreSession() {
  if (!state.token) {
    showLogin();
    return;
  }
  try {
    const auth = await request("/api/auth/me");
    showApp(auth);
    await refreshDashboard();
    log("세션 복원 완료", {
      tenant: auth.user.tenant_code,
      login_id: auth.user.login_id,
    });
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
    showLogin("로그아웃 완료");
  }
}

function normalizeModuleTree(modules) {
  if (Array.isArray(modules)) {
    return { code: "cit-system", display_name: "CIT System", children: modules };
  }
  return modules || { code: "cit-system", display_name: "CIT System", children: [] };
}

function topModules(tree) {
  return Array.isArray(tree?.children) ? tree.children : [];
}

function moduleDetailCount(tree) {
  return topModules(tree).reduce(
    (sum, module) => sum + (Array.isArray(module.children) ? module.children.length : 0),
    0,
  );
}

function nodeLabel(node) {
  if (typeof node === "string") {
    return node;
  }
  return node.display_name || [node.number, node.name].filter(Boolean).join(" ");
}

function nodeName(node) {
  return typeof node === "string" ? node : node.name || nodeLabel(node);
}

function nodeNumber(node, fallback) {
  return typeof node === "string" ? fallback : node.number || fallback;
}

function nodeCode(node, fallback) {
  return String(typeof node === "string" ? fallback : node.code || fallback).replaceAll(".", "-");
}

function renderModuleMenu(tree) {
  const modules = topModules(tree);
  el("moduleMenu").innerHTML = modules
    .map((module, index) => {
      const children = Array.isArray(module.children) ? module.children : [];
      const moduleId = `module-${nodeCode(module, index)}`;
      const path = module.path || "";
      const summary = `
        <summary class="${index === 0 ? "active" : ""}" data-path="${escapeHtml(path)}">
          <span>${escapeHtml(nodeNumber(module, index))}</span>
          ${escapeHtml(nodeName(module))}
        </summary>
      `;

      if (!children.length) {
        return `
          <a href="#${escapeHtml(moduleId)}" data-path="${escapeHtml(path)}" class="menu-link ${index === 0 ? "active" : ""}">
            <span>${escapeHtml(nodeNumber(module, index))}</span>
            ${escapeHtml(nodeName(module))}
          </a>
        `;
      }

      return `
        <details class="menu-group" open>
          ${summary}
          <div class="submenu-list">
            ${children
              .map((child, childIndex) => {
                const childPath = child.path || "";
                return `
                  <a href="#${escapeHtml(nodeCode(child, `${index}-${childIndex}`))}" data-path="${escapeHtml(childPath)}" class="submenu-link">
                    <span>${escapeHtml(nodeNumber(child, `${index}.${childIndex + 1}`))}</span>
                    ${escapeHtml(nodeName(child))}
                  </a>
                `;
              })
              .join("")}
          </div>
        </details>
      `;
    })
    .join("");

  el("moduleMenu").querySelectorAll("[data-path]").forEach((link) => {
    link.addEventListener("click", async (event) => {
      const path = link.dataset.path;
      if (path?.startsWith("/modules/law-versioning")) {
        event.preventDefault();
        await navigateLawRoute(path);
      } else if (path?.startsWith("/modules/admin")) {
        event.preventDefault();
        await navigateAdminRoute(path);
      } else if (path?.startsWith("/modules/customer")) {
        event.preventDefault();
        await navigateCustomerRoute(path);
      } else if (path?.startsWith("/modules/tax-data")) {
        event.preventDefault();
        await navigateTaxDataRoute(path);
      } else if (path?.startsWith("/modules/adjustment")) {
        event.preventDefault();
        await navigateAdjustmentRoute(path);
      } else if (path?.startsWith("/modules/forms")) {
        event.preventDefault();
        await navigateFormRoute(path);
      }
    });
  });
  highlightLawMenu();
  highlightTaxDataMenu();
  highlightAdjustmentMenu();
}

function renderModuleCards(tree) {
  const modules = topModules(tree);
  el("moduleCount").textContent = `${modules.length}개 모듈 / ${moduleDetailCount(tree)}개 상세`;
  el("moduleCards").innerHTML = modules
    .map((module, index) => {
      const children = Array.isArray(module.children) ? module.children : [];

      return `
        <article id="module-${escapeHtml(nodeCode(module, index))}" class="module-card">
          <div class="module-number">${escapeHtml(nodeNumber(module, index))}</div>
          <div>
            <h3>${escapeHtml(nodeLabel(module))}</h3>
            ${
              children.length
                ? `<ul class="module-child-list">
                    ${children
                      .map(
                        (child, childIndex) => `
                          <li id="${escapeHtml(nodeCode(child, `${index}-${childIndex}`))}">
                            <span>${escapeHtml(nodeNumber(child, `${index}.${childIndex + 1}`))}</span>
                            ${escapeHtml(nodeName(child))}
                          </li>
                        `,
                      )
                      .join("")}
                  </ul>`
                : `<p class="module-empty">하위 세부 모듈 없음</p>`
            }
          </div>
        </article>
      `;
    })
    .join("");
}

async function navigateLawRoute(path) {
  state.activeLawPath = lawScreens[path] ? path : "/modules/law-versioning/laws";
  highlightLawMenu();
  if (!state.lawVersions.length) {
    await refreshLawData(false);
  }
  await renderLawScreen();
  el(currentLawScreen().sectionId).scrollIntoView({ behavior: "smooth", block: "start" });
}

async function navigateAdminRoute(path) {
  state.activeAdminPath = path || "/modules/admin/users";
  highlightAdminMenu();
  await renderAdminScreen();
  el("adminWorkspace").scrollIntoView({ behavior: "smooth", block: "start" });
}

async function navigateCustomerRoute(path) {
  const supported = ["/modules/customer/profile", "/modules/customer/business-years"];
  state.activeCustomerPath = supported.includes(path) ? path : "/modules/customer/profile";
  highlightCustomerMenu();
  await renderCustomerScreen();
  el("customerWorkspace").scrollIntoView({ behavior: "smooth", block: "start" });
}

async function navigateTaxDataRoute(path) {
  const supported = [
    "/modules/tax-data/financial-statements",
    "/modules/tax-data/account-mapping",
    "/modules/tax-data/partners",
    "/modules/tax-data/assets",
  ];
  state.activeTaxDataPath = supported.includes(path) ? path : "/modules/tax-data/financial-statements";
  highlightTaxDataMenu();
  await renderTaxDataScreen();
  el("taxDataWorkspace").scrollIntoView({ behavior: "smooth", block: "start" });
}

async function navigateAdjustmentRoute(path) {
  const supported = [
    "/modules/adjustment/income",
    "/modules/adjustment/donations-entertainment",
    "/modules/adjustment/depreciation",
    "/modules/adjustment/retirement-reserve",
    "/modules/adjustment/bad-debt-reserve",
    "/modules/adjustment/carryforward-loss",
    "/modules/adjustment/tax-credits",
    "/modules/adjustment/penalty-tax",
  ];
  state.activeAdjustmentPath = supported.includes(path) ? path : "/modules/adjustment/income";
  highlightAdjustmentMenu();
  await renderAdjustmentScreen();
  el("adjustmentWorkspace").scrollIntoView({ behavior: "smooth", block: "start" });
}

async function navigateFormRoute(path) {
  const supported = [
    "/modules/forms/versions",
    "/modules/forms/relationships",
    "/modules/forms/migrations",
    "/modules/forms/resolver",
  ];
  state.activeFormPath = supported.includes(path) ? path : "/modules/forms/versions";
  highlightFormMenu();
  await renderFormScreen();
  el("formVersioningWorkspace").scrollIntoView({ behavior: "smooth", block: "start" });
}

function highlightLawMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.path === state.activeLawPath);
  });
  setActiveLawPanel();
}

function highlightFormMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    if (link.dataset.path?.startsWith("/modules/forms")) {
      link.classList.toggle("active", link.dataset.path === state.activeFormPath);
    }
  });
}

function highlightAdminMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    if (link.dataset.path?.startsWith("/modules/admin")) {
      link.classList.toggle("active", link.dataset.path === state.activeAdminPath);
    }
  });
}

function highlightCustomerMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    if (link.dataset.path?.startsWith("/modules/customer")) {
      link.classList.toggle("active", link.dataset.path === state.activeCustomerPath);
    }
  });
}

function highlightTaxDataMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    if (link.dataset.path?.startsWith("/modules/tax-data")) {
      link.classList.toggle("active", link.dataset.path === state.activeTaxDataPath);
    }
  });
}

function highlightAdjustmentMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    if (link.dataset.path?.startsWith("/modules/adjustment")) {
      link.classList.toggle("active", link.dataset.path === state.activeAdjustmentPath);
    }
  });
}

function currentTenantCode() {
  return state.user?.tenant_code || state.tenantCode || "demo";
}

async function optionalRequest(path, fallback) {
  try {
    return await request(path);
  } catch (error) {
    log("선택 데이터 조회 실패", { path, message: error.message });
    return fallback;
  }
}

async function renderAdminScreen() {
  if (state.activeAdminPath === "/modules/admin/roles") {
    await renderAdminRolesScreen();
  } else if (state.activeAdminPath === "/modules/admin/tenants") {
    await renderAdminTenantsScreen();
  } else {
    await renderAdminUsersScreen();
  }
}

function badgeList(items, variant = "") {
  const list = Array.isArray(items) && items.length ? items : ["-"];
  return `<div class="badge-list">${list
    .map((item) => `<span class="badge ${variant}">${escapeHtml(item)}</span>`)
    .join("")}</div>`;
}

function workScopeCheckboxes(prefix, selected = [], allowed = allWorkScopeCodes) {
  return `<div class="scope-grid">${workScopeOptions
    .map(
      ([code, label]) => {
        const isAllowed = allowed.includes(code);
        return `
        <label>
          <input name="${prefix}WorkScope" type="checkbox" value="${code}" ${selected.includes(code) && isAllowed ? "checked" : ""} ${isAllowed ? "" : "disabled"} />
          ${escapeHtml(label)}${isAllowed ? "" : " (미대상)"}
        </label>
      `;
      },
    )
    .join("")}</div>`;
}

function checkedWorkScopes(prefix) {
  return [...document.querySelectorAll(`input[name="${prefix}WorkScope"]:checked:not(:disabled)`)].map((input) => input.value);
}

async function renderAdminUsersScreen() {
  const tenantCode = currentTenantCode();
  el("adminScreenTitle").textContent = "사용자 / 테넌트 / 고객사 업무 권한";
  const [users, roles, customers] = await Promise.all([
    request(`/api/admin/tenants/${tenantCode}/users`),
    request("/api/admin/roles"),
    optionalRequest(`/api/tenants/${tenantCode}/customers`, []),
  ]);
  const roleOptions = roles
    .map((role) => `<option value="${escapeHtml(role.role_code)}">${escapeHtml(role.role_code)}</option>`)
    .join("");
  const customerOptions = customers
    .map(
      (customer) =>
        `<option value="${customer.customer_id}">${escapeHtml(customer.customer_code)} · ${escapeHtml(customer.customer_name)}</option>`,
    )
    .join("");
  const customerById = new Map(customers.map((customer) => [Number(customer.customer_id), customer]));
  const initialCustomer = customers[0] || null;
  const initialAllowedScopes = customerAllowedWorkScopes(initialCustomer);
  const initialSelectedScopes = ["INFO", "ADJUST", "FORM"].filter((scope) =>
    initialAllowedScopes.includes(scope),
  );
  const rows = users.length
    ? users
        .map((user) => {
          const access = user.customer_access || [];
          const scopes = access.flatMap((item) => item.work_scopes || []);
          const accessLabels = access.map((item) => {
            const customer = customerById.get(Number(item.customer_id));
            const allowed = customerAllowedWorkScopes(customer);
            return `${item.customer_id}:${item.access_level} ${item.work_scopes?.length || 0}/${allowed.length}`;
          });
          return `
            <tr>
              <td><strong>${escapeHtml(user.user_name)}</strong><br><span class="muted">${escapeHtml(user.login_id)} · ${escapeHtml(user.email || "-")}</span></td>
              <td>${escapeHtml(user.tenant_code)}</td>
              <td>${badgeList(user.roles)}</td>
              <td>${badgeList(accessLabels, access.some((item) => item.access_level === "BLOCKED") ? "danger" : "")}</td>
              <td>${badgeList(workScopeLabels([...new Set(scopes)]))}</td>
              <td><span class="status-pill ${user.locked ? "dead_letter" : "active"}">${escapeHtml(user.locked ? "LOCKED" : user.status)}</span></td>
              <td class="table-actions">
                <button class="secondary-btn compact" type="button" data-lock-user="${escapeHtml(user.login_id)}">잠금</button>
                <button class="secondary-btn compact" type="button" data-reset-user="${escapeHtml(user.login_id)}">2FA 리셋</button>
              </td>
            </tr>
          `;
        })
        .join("")
    : `<tr><td colspan="7">등록된 사용자가 없습니다.</td></tr>`;

  el("adminScreenBody").innerHTML = `
    <div class="admin-layout">
      <form id="adminUserCreateForm" class="admin-form">
        <h3>사용자 등록</h3>
        <label>로그인 ID<input id="adminLoginId" value="user${Date.now().toString(36).slice(-4)}" /></label>
        <label>이름<input id="adminUserName" value="신규 사용자" /></label>
        <label>이메일<input id="adminEmail" value="user@example.local" /></label>
        <label>초기 비밀번호<input id="adminPassword" type="password" value="ChangeMe123!" /></label>
        <label>역할<select id="adminRole">${roleOptions}</select></label>
        <label>고객사${customers.length ? `<select id="adminCustomerId">${customerOptions}</select>` : `<input id="adminCustomerId" type="number" placeholder="고객사 ID" />`}</label>
        <label>접근 등급
          <select id="adminAccessLevel">
            <option>OWNER</option><option>CO_WORKER</option><option>REVIEWER</option><option>ASSISTANT</option><option>VIEWER</option><option>BLOCKED</option>
          </select>
        </label>
        <div>
          <p class="form-help">고객사별 대상 업무</p>
          <div id="adminCreateScopes">${workScopeCheckboxes("adminCreate", initialSelectedScopes, initialAllowedScopes)}</div>
        </div>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="admin-table-panel">
        <div class="admin-toolbar">
          <label>테넌트<input value="${escapeHtml(tenantCode)}" readonly /></label>
          <label>사용자 수<input value="${users.length}" readonly /></label>
          <label>역할 수<input value="${roles.length}" readonly /></label>
          <label>고객사 수<input value="${customers.length}" readonly /></label>
        </div>
        <div class="table-wrap">
          <table>
            <thead><tr><th>사용자</th><th>테넌트</th><th>역할</th><th>고객사 접근</th><th>대상 업무</th><th>상태</th><th>조치</th></tr></thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;

  const customerInput = el("adminCustomerId");
  if (customerInput && customers.length) {
    customerInput.addEventListener("change", () => {
      const selectedCustomer = customerById.get(Number(customerInput.value));
      const allowed = customerAllowedWorkScopes(selectedCustomer);
      const current = checkedWorkScopes("adminCreate").filter((scope) => allowed.includes(scope));
      const fallback = ["INFO", "ADJUST", "FORM"].filter((scope) => allowed.includes(scope));
      el("adminCreateScopes").innerHTML = workScopeCheckboxes(
        "adminCreate",
        current.length ? current : fallback,
        allowed,
      );
    });
  }

  el("adminUserCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const customerId = Number(el("adminCustomerId").value || 0);
    const body = {
      login_id: el("adminLoginId").value.trim(),
      password: el("adminPassword").value,
      user_name: el("adminUserName").value.trim(),
      email: el("adminEmail").value.trim(),
      use_2fa: true,
      roles: [el("adminRole").value],
      customer_access: customerId
        ? [
            {
              customer_id: customerId,
              access_level: el("adminAccessLevel").value,
              is_primary: true,
              work_scopes: checkedWorkScopes("adminCreate"),
            },
          ]
        : [],
    };
    const created = await request(`/api/admin/tenants/${tenantCode}/users`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    log("사용자 등록 완료", created);
    await renderAdminUsersScreen();
  });

  document.querySelectorAll("[data-lock-user]").forEach((button) => {
    button.addEventListener("click", async () => {
      const loginId = button.dataset.lockUser;
      const updated = await request(`/api/admin/tenants/${tenantCode}/users/${loginId}/status`, {
        method: "POST",
        body: JSON.stringify({ status: "LOCKED", locked: true }),
      });
      log("사용자 잠금 완료", updated);
      await renderAdminUsersScreen();
    });
  });
  document.querySelectorAll("[data-reset-user]").forEach((button) => {
    button.addEventListener("click", async () => {
      const loginId = button.dataset.resetUser;
      const updated = await request(`/api/admin/tenants/${tenantCode}/users/${loginId}/reset-2fa`, {
        method: "POST",
        body: "{}",
      });
      log("2FA 리셋 완료", updated);
      await renderAdminUsersScreen();
    });
  });
}

async function renderAdminRolesScreen() {
  el("adminScreenTitle").textContent = "역할 / 권한 매트릭스";
  const [roles, permissions] = await Promise.all([request("/api/admin/roles"), request("/api/admin/role-permissions")]);
  const roleOptions = roles
    .map((role) => `<option value="${escapeHtml(role.role_code)}">${escapeHtml(role.role_code)} · ${escapeHtml(role.role_name)}</option>`)
    .join("");
  const rows = permissions.length
    ? permissions
        .map(
          (permission) => `
            <tr>
              <td>${escapeHtml(permission.role_code)}</td>
              <td>${escapeHtml(permission.module_code)}</td>
              <td>${escapeHtml(permission.function_code)}</td>
              <td><span class="status-pill ${permission.effect === "DENY" ? "dead_letter" : "active"}">${escapeHtml(permission.effect)}</span></td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="4">등록된 권한이 없습니다.</td></tr>`;
  el("adminScreenBody").innerHTML = `
    <div class="admin-layout">
      <form id="rolePermissionForm" class="admin-form">
        <h3>권한 추가/갱신</h3>
        <label>역할<select id="permissionRole">${roleOptions}</select></label>
        <label>모듈 코드<input id="permissionModule" value="adjustment" /></label>
        <label>기능 코드<input id="permissionFunction" value="READ" /></label>
        <label>효과<select id="permissionEffect"><option>ALLOW</option><option>DENY</option></select></label>
        <button class="primary-btn" type="submit">저장</button>
      </form>
      <div class="admin-table-panel">
        <h3>현재 권한 매트릭스</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>역할</th><th>모듈</th><th>기능</th><th>효과</th></tr></thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
  el("rolePermissionForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const roleCode = el("permissionRole").value;
    const existing = permissions
      .filter((permission) => permission.role_code === roleCode)
      .map((permission) => ({
        module_code: permission.module_code,
        function_code: permission.function_code,
        effect: permission.effect,
      }));
    existing.push({
      module_code: el("permissionModule").value.trim(),
      function_code: el("permissionFunction").value.trim().toUpperCase(),
      effect: el("permissionEffect").value,
    });
    const updated = await request(`/api/admin/roles/${roleCode}/permissions`, {
      method: "PUT",
      body: JSON.stringify({ permissions: existing }),
    });
    log("역할 권한 저장 완료", { roleCode, permissions: updated.length });
    await renderAdminRolesScreen();
  });
}

async function renderAdminTenantsScreen() {
  el("adminScreenTitle").textContent = "테넌트 관리";
  const tenants = await request("/api/tenants");
  el("adminScreenBody").innerHTML = `
    <div class="admin-table-panel">
      <h3>테넌트 목록</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>코드</th><th>이름</th><th>스키마</th><th>사용자 한도</th><th>상태</th></tr></thead>
          <tbody>
            ${tenants
              .map(
                (tenant) => `
                  <tr>
                    <td>${escapeHtml(tenant.tenant_code)}</td>
                    <td>${escapeHtml(tenant.tenant_name)}</td>
                    <td>${escapeHtml(tenant.schema_name)}</td>
                    <td>${tenant.max_users}</td>
                    <td><span class="status-pill active">${escapeHtml(tenant.status)}</span></td>
                  </tr>
                `,
              )
              .join("")}
          </tbody>
        </table>
      </div>
    </div>
  `;
}

function businessYearNextStatuses(status) {
  return {
    DRAFT: ["IN_REVIEW"],
    IN_REVIEW: ["APPROVED", "DRAFT"],
    APPROVED: ["FILED", "IN_REVIEW"],
    FILED: ["AMENDED"],
    AMENDED: ["IN_REVIEW"],
  }[status] || [];
}

async function renderCustomerScreen() {
  if (state.activeCustomerPath === "/modules/customer/business-years") {
    await renderBusinessYearsScreen();
  } else {
    await renderCustomersScreen();
  }
}

async function renderCustomersScreen() {
  const tenantCode = currentTenantCode();
  el("customerScreenTitle").textContent = "고객사 관리";
  const customers = await optionalRequest(`/api/tenants/${tenantCode}/customers`, []);
  el("customerScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="customerCreateForm" class="law-form">
        <h3>고객사 등록</h3>
        <label>고객사 코드<input id="customerCode" value="CUST${Date.now().toString(36).slice(-4)}" /></label>
        <label>고객사명<input id="customerName" value="신규 고객사" /></label>
        <label>사업자번호<input id="customerBizNo" value="2208112345" /></label>
        <label>법인등록번호<input id="customerCorpNo" value="1101111234567" /></label>
        <label>업종코드<input id="customerIndustry" value="62010" /></label>
        <div>
          <p class="form-help">고객사 대상 업무</p>
          ${workScopeCheckboxes("customerCreate", ["INFO", "ADJUST", "FORM", "VALIDATE", "PRINT"])}
        </div>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="law-table-panel">
        <div class="table-wrap">
          <table>
            <thead><tr><th>고객사</th><th>사업자번호</th><th>대상 업무</th><th>상태</th></tr></thead>
            <tbody>${customers
              .map(
                (customer) => `
                  <tr>
                    <td>${escapeHtml(customer.customer_code)}<br><span class="muted">${escapeHtml(customer.customer_name)}</span></td>
                    <td>${escapeHtml(customer.biz_reg_no)}</td>
                    <td>${badgeList(workScopeLabels(customerAllowedWorkScopes(customer)))}</td>
                    <td><span class="status-pill active">${escapeHtml(customer.status)}</span></td>
                  </tr>
                `,
              )
              .join("")}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
  el("customerCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const created = await request(`/api/tenants/${tenantCode}/customers`, {
      method: "POST",
      body: JSON.stringify({
        customer_code: el("customerCode").value.trim(),
        customer_name: el("customerName").value.trim(),
        biz_reg_no: el("customerBizNo").value.trim(),
        corp_reg_no: el("customerCorpNo").value.trim(),
        industry_code: el("customerIndustry").value.trim(),
        is_sme: true,
        work_scopes: checkedWorkScopes("customerCreate"),
      }),
    });
    log("고객사 등록 완료", created);
    await renderCustomersScreen();
  });
}

async function renderBusinessYearsScreen() {
  const tenantCode = currentTenantCode();
  el("customerScreenTitle").textContent = "사업연도 관리 / 적용 스냅샷";
  const [customers, years] = await Promise.all([
    optionalRequest(`/api/tenants/${tenantCode}/customers`, []),
    optionalRequest(`/api/tenants/${tenantCode}/business-years`, []),
  ]);
  const customerOptions = customers
    .map((customer) => `<option value="${customer.customer_id}">${escapeHtml(customer.customer_code)} · ${escapeHtml(customer.customer_name)}</option>`)
    .join("");
  el("customerScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="businessYearCreateForm" class="law-form">
        <h3>사업연도 생성</h3>
        <label>고객사${customers.length ? `<select id="byCustomerId">${customerOptions}</select>` : `<input id="byCustomerId" type="number" />`}</label>
        <label>사업연도<input id="byYearLabel" type="number" value="${new Date().getFullYear()}" /></label>
        <label>시작일<input id="byStartDate" type="date" value="${new Date().getFullYear()}-01-01" /></label>
        <label>종료일<input id="byEndDate" type="date" value="${new Date().getFullYear()}-12-31" /></label>
        <button class="primary-btn" type="submit">생성 및 스냅샷 적용</button>
      </form>
      <div class="law-table-panel">
        <div class="table-wrap">
          <table>
            <thead><tr><th>사업연도</th><th>고객사</th><th>기간</th><th>상태</th><th>조치</th></tr></thead>
            <tbody>${years
              .map((year) => {
                const actions = businessYearNextStatuses(year.status)
                  .map((status) => `<button class="secondary-btn compact" type="button" data-by-id="${year.by_id}" data-by-status="${status}">${status}</button>`)
                  .join("");
                return `
                  <tr>
                    <td>${year.year_label}<br><span class="muted">BY ${year.by_id}</span></td>
                    <td>${year.customer_id}</td>
                    <td>${formatDate(year.start_date)} ~ ${formatDate(year.end_date)}</td>
                    <td><span class="status-pill ${year.status.toLowerCase()}">${escapeHtml(year.status)}</span></td>
                    <td class="table-actions">${actions}<button class="secondary-btn compact" type="button" data-snapshot-by-id="${year.by_id}">스냅샷</button></td>
                  </tr>
                `;
              })
              .join("")}</tbody>
          </table>
        </div>
        <pre id="businessYearSnapshotOutput" class="json-result">{}</pre>
      </div>
    </div>
  `;
  el("businessYearCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const created = await request(`/api/tenants/${tenantCode}/business-years`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: Number(el("byCustomerId").value),
        year_label: Number(el("byYearLabel").value),
        start_date: el("byStartDate").value,
        end_date: el("byEndDate").value,
      }),
    });
    log("사업연도 생성 및 자동 스냅샷 완료", created);
    await renderBusinessYearsScreen();
  });
  document.querySelectorAll("[data-by-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const updated = await request(`/api/tenants/${tenantCode}/business-years/${button.dataset.byId}/status`, {
        method: "POST",
        body: JSON.stringify({ status: button.dataset.byStatus }),
      });
      log("사업연도 상태 변경", updated);
      await renderBusinessYearsScreen();
    });
  });
  document.querySelectorAll("[data-snapshot-by-id]").forEach((button) => {
    button.addEventListener("click", async () => {
      const snapshot = await request(`/api/tenants/${tenantCode}/business-years/${button.dataset.snapshotById}/snapshot`);
      el("businessYearSnapshotOutput").textContent = JSON.stringify(snapshot, null, 2);
      log("사업연도 적용 스냅샷 조회", snapshot);
    });
  });
}

async function loadTaxDataContext() {
  const tenantCode = currentTenantCode();
  const [customers, years] = await Promise.all([
    optionalRequest(`/api/tenants/${tenantCode}/customers`, []),
    optionalRequest(`/api/tenants/${tenantCode}/business-years`, []),
  ]);
  if (!state.byId && years.length) {
    state.byId = Number(years[0].by_id);
  }
  if (state.byId && !years.some((year) => Number(year.by_id) === Number(state.byId))) {
    state.byId = years.length ? Number(years[0].by_id) : null;
  }
  const byId = state.byId ? Number(state.byId) : null;
  const year = years.find((item) => Number(item.by_id) === byId) || null;
  const customer = customers.find((item) => Number(item.customer_id) === Number(year?.customer_id)) || customers[0] || null;
  return { tenantCode, customers, years, byId, year, customer };
}

function taxDataYearSelector(context) {
  if (!context.years.length) {
    return `<div class="screen-placeholder">먼저 고객사와 사업연도를 생성하세요.</div>`;
  }
  return `
    <div class="admin-toolbar">
      <label>테넌트<input value="${escapeHtml(context.tenantCode)}" readonly /></label>
      <label>사업연도
        <select id="taxDataById">
          ${context.years
            .map(
              (year) =>
                `<option value="${year.by_id}" ${Number(year.by_id) === Number(context.byId) ? "selected" : ""}>${year.year_label} · BY ${year.by_id} · 고객사 ${year.customer_id}</option>`,
            )
            .join("")}
        </select>
      </label>
      <label>고객사<input value="${escapeHtml(context.customer?.customer_name || "-")}" readonly /></label>
      <label>상태<input value="${escapeHtml(context.year?.status || "-")}" readonly /></label>
    </div>
  `;
}

function attachTaxDataYearSelector() {
  const select = el("taxDataById");
  if (select) {
    select.addEventListener("change", async () => {
      state.byId = Number(select.value);
      await renderTaxDataScreen();
    });
  }
}

function taxDataUploadForm(id, dataType, label) {
  const tenantCode = currentTenantCode();
  return `
    <form id="${id}" class="law-form">
      <h3>${escapeHtml(label)} 업로드</h3>
      <label>CSV/XLSX 파일<input id="${id}File" type="file" accept=".csv,.xlsx" /></label>
      <div class="table-actions">
        <button class="primary-btn" type="submit">업로드/검증/적재</button>
        <a class="secondary-btn compact" href="/api/tenants/${escapeHtml(tenantCode)}/tax-data/templates/${escapeHtml(dataType)}">템플릿</a>
      </div>
    </form>
  `;
}

async function uploadTaxDataFile(dataType, inputId) {
  const file = el(inputId).files[0];
  if (!file || !state.byId) {
    throw new Error("업로드할 파일과 사업연도를 선택하세요.");
  }
  const form = new FormData();
  form.append("file", file);
  const result = await uploadRequest(
    `/api/tenants/${currentTenantCode()}/business-years/${state.byId}/tax-data/${dataType}/import`,
    form,
  );
  log("세무정보 임포트 완료", result.batch);
  if (result.errors?.length) {
    log("세무정보 임포트 오류", result.errors);
  }
  await renderTaxDataScreen();
}

function renderImportBatchesTable(batches) {
  const rows = batches.length
    ? batches
        .map(
          (batch) => `
            <tr>
              <td>${batch.batch_id}</td>
              <td>${escapeHtml(batch.data_type)}</td>
              <td>${escapeHtml(batch.source_file_name || "-")}</td>
              <td>${batch.row_count} / ${batch.valid_count}</td>
              <td>${batch.auto_mapped_count}</td>
              <td><span class="status-pill ${batch.error_count ? "error" : "active"}">${escapeHtml(batch.status)}</span></td>
              <td class="table-actions"><button class="secondary-btn compact" type="button" data-import-error-batch="${batch.batch_id}">오류</button></td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="7">임포트 이력이 없습니다.</td></tr>`;
  return `
    <div class="table-wrap">
      <table>
        <thead><tr><th>배치</th><th>유형</th><th>파일</th><th>행/적재</th><th>자동매핑</th><th>상태</th><th>오류</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
    <pre id="taxDataErrorOutput" class="json-result">{}</pre>
  `;
}

function attachImportErrorButtons() {
  document.querySelectorAll("[data-import-error-batch]").forEach((button) => {
    button.addEventListener("click", async () => {
      const errors = await request(
        `/api/tenants/${currentTenantCode()}/business-years/${state.byId}/tax-data/import-batches/${button.dataset.importErrorBatch}/errors`,
      );
      el("taxDataErrorOutput").textContent = JSON.stringify(errors, null, 2);
    });
  });
}

async function renderTaxDataScreen() {
  if (state.activeTaxDataPath === "/modules/tax-data/account-mapping") {
    await renderAccountMappingScreen();
  } else if (state.activeTaxDataPath === "/modules/tax-data/assets") {
    await renderAssetsScreen();
  } else if (state.activeTaxDataPath === "/modules/tax-data/partners") {
    await renderTransactionsScreen();
  } else {
    await renderFinancialStatementsScreen();
  }
}

async function renderFinancialStatementsScreen() {
  const context = await loadTaxDataContext();
  el("taxDataScreenTitle").textContent = "재무제표 입력 / 임포트";
  if (!context.byId) {
    el("taxDataScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const [lines, batches, validation] = await Promise.all([
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/financial-statements`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/import-batches`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/validation`, null),
  ]);
  const lineRows = lines.length
    ? lines
        .map(
          (line) => `
            <tr>
              <td>${escapeHtml(line.statement_type)} / ${line.row_no || "-"}</td>
              <td>${escapeHtml(line.account_code)}<br><span class="muted">${escapeHtml(line.account_name)}</span></td>
              <td>${escapeHtml(line.standard_account_code || "-")}<br><span class="muted">${escapeHtml(line.standard_account_name || "-")}</span></td>
              <td>${money.format(line.debit)}</td>
              <td>${money.format(line.credit)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">재무제표 라인이 없습니다.</td></tr>`;
  el("taxDataScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-summary-grid compact-grid">
      <article><span>차변</span><strong>${money.format(validation?.debit_total || 0)}</strong></article>
      <article><span>대변</span><strong>${money.format(validation?.credit_total || 0)}</strong></article>
      <article><span>잔액검증</span><strong>${validation?.balanced ? "일치" : "불일치"}</strong></article>
      <article><span>미매핑</span><strong>${validation?.unresolved_mapping_count || 0}</strong></article>
    </div>
    <div class="law-layout">
      ${taxDataUploadForm("fsImportForm", "financial-statements", "재무제표")}
      <div class="law-table-panel">
        <h3>임포트 배치</h3>
        ${renderImportBatchesTable(batches)}
      </div>
    </div>
    <div class="law-table-panel">
      <h3>재무제표 라인</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>구분/행</th><th>원천 계정</th><th>표준 계정</th><th>차변</th><th>대변</th></tr></thead>
          <tbody>${lineRows}</tbody>
        </table>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  attachImportErrorButtons();
  el("fsImportForm").addEventListener("submit", (event) => {
    event.preventDefault();
    uploadTaxDataFile("financial-statements", "fsImportFormFile").catch((error) =>
      log("재무제표 임포트 실패", { message: error.message }),
    );
  });
}

async function renderAccountMappingScreen() {
  const context = await loadTaxDataContext();
  el("taxDataScreenTitle").textContent = "계정과목 매핑 학습";
  const customerId = Number(context.customer?.customer_id || 0);
  const mappings = customerId
    ? await optionalRequest(`/api/tenants/${context.tenantCode}/customers/${customerId}/account-mappings`, [])
    : [];
  const rows = mappings.length
    ? mappings
        .map(
          (mapping) => `
            <tr>
              <td>${escapeHtml(mapping.statement_type)}</td>
              <td>${escapeHtml(mapping.source_account_code)}<br><span class="muted">${escapeHtml(mapping.source_account_name)}</span></td>
              <td>${escapeHtml(mapping.standard_account_code)}<br><span class="muted">${escapeHtml(mapping.standard_account_name)}</span></td>
              <td>${mapping.use_count}</td>
              <td>${formatDateTime(mapping.last_used_at)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">학습된 매핑이 없습니다.</td></tr>`;
  el("taxDataScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-layout">
      <form id="accountMappingForm" class="law-form">
        <h3>수동 매핑 등록</h3>
        <label>구분<select id="mappingStatementType"><option>BS</option><option>PL</option></select></label>
        <label>원천 계정코드<input id="mappingSourceCode" value="10100" /></label>
        <label>원천 계정명<input id="mappingSourceName" value="Cash" /></label>
        <label>표준 계정코드<input id="mappingStandardCode" value="STD_CASH" /></label>
        <label>표준 계정명<input id="mappingStandardName" value="Cash" /></label>
        <button class="primary-btn" type="submit" ${customerId ? "" : "disabled"}>저장</button>
      </form>
      <div class="law-table-panel">
        <h3>고객사별 매핑</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>구분</th><th>원천 계정</th><th>표준 계정</th><th>사용</th><th>최근 사용</th></tr></thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  el("accountMappingForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const mapping = await request(`/api/tenants/${context.tenantCode}/customers/${customerId}/account-mappings`, {
      method: "POST",
      body: JSON.stringify({
        statement_type: el("mappingStatementType").value,
        source_account_code: el("mappingSourceCode").value.trim(),
        source_account_name: el("mappingSourceName").value.trim(),
        standard_account_code: el("mappingStandardCode").value.trim(),
        standard_account_name: el("mappingStandardName").value.trim(),
      }),
    });
    log("계정 매핑 저장 완료", mapping);
    await renderAccountMappingScreen();
  });
}

async function renderAssetsScreen() {
  const context = await loadTaxDataContext();
  el("taxDataScreenTitle").textContent = "자산 / 감가상각 정보";
  if (!context.byId) {
    el("taxDataScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const [assets, batches, validation] = await Promise.all([
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/assets`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/import-batches`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/validation`, null),
  ]);
  const rows = assets.length
    ? assets
        .map(
          (asset) => `
            <tr>
              <td>${escapeHtml(asset.asset_code)}<br><span class="muted">${escapeHtml(asset.asset_name)}</span></td>
              <td>${escapeHtml(asset.asset_category)}</td>
              <td>${asset.is_business_vehicle ? "업무용차" : "-"}</td>
              <td>${formatDate(asset.acquisition_date)}</td>
              <td>${money.format(asset.acquisition_cost)}</td>
              <td>${asset.useful_life_years}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="6">자산대장 라인이 없습니다.</td></tr>`;
  el("taxDataScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-summary-grid compact-grid">
      <article><span>자산</span><strong>${validation?.asset_count || 0}</strong></article>
      <article><span>업무용차</span><strong>${validation?.business_vehicle_count || 0}</strong></article>
    </div>
    <div class="law-layout">
      ${taxDataUploadForm("assetImportForm", "assets", "자산대장")}
      <div class="law-table-panel">
        <h3>임포트 배치</h3>
        ${renderImportBatchesTable(batches)}
      </div>
    </div>
    <div class="law-table-panel">
      <h3>자산대장</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>자산</th><th>분류</th><th>업무용차</th><th>취득일</th><th>취득가액</th><th>내용연수</th></tr></thead>
          <tbody>${rows}</tbody>
        </table>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  attachImportErrorButtons();
  el("assetImportForm").addEventListener("submit", (event) => {
    event.preventDefault();
    uploadTaxDataFile("assets", "assetImportFormFile").catch((error) =>
      log("자산대장 임포트 실패", { message: error.message }),
    );
  });
}

async function renderTransactionsScreen() {
  const context = await loadTaxDataContext();
  el("taxDataScreenTitle").textContent = "거래 명세";
  if (!context.byId) {
    el("taxDataScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const [transactions, batches, validation] = await Promise.all([
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/transactions`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/import-batches`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/tax-data/validation`, null),
  ]);
  const rows = transactions.length
    ? transactions
        .map(
          (transaction) => `
            <tr>
              <td>${formatDate(transaction.tx_date)}</td>
              <td>${escapeHtml(transaction.partner_name)}</td>
              <td>${escapeHtml(transaction.category)}</td>
              <td>${escapeHtml(transaction.account_code || "-")}</td>
              <td>${escapeHtml(transaction.description || "-")}</td>
              <td>${money.format(transaction.amount)}</td>
              <td>${escapeHtml(transaction.evidence_type || "-")}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="7">거래 명세가 없습니다.</td></tr>`;
  el("taxDataScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-summary-grid compact-grid">
      <article><span>거래 건수</span><strong>${validation?.transaction_count || 0}</strong></article>
    </div>
    <div class="law-layout">
      ${taxDataUploadForm("transactionImportForm", "transactions", "거래 명세")}
      <div class="law-table-panel">
        <h3>임포트 배치</h3>
        ${renderImportBatchesTable(batches)}
      </div>
    </div>
    <div class="law-table-panel">
      <h3>기부금 / 접대비 / 지급이자 거래</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>일자</th><th>거래처</th><th>분류</th><th>계정</th><th>내용</th><th>금액</th><th>증빙</th></tr></thead>
          <tbody>${rows}</tbody>
        </table>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  attachImportErrorButtons();
  el("transactionImportForm").addEventListener("submit", (event) => {
    event.preventDefault();
    uploadTaxDataFile("transactions", "transactionImportFormFile").catch((error) =>
      log("거래 명세 임포트 실패", { message: error.message }),
    );
  });
}

function incomeAdjustmentRows() {
  const rows = [
    ["GROSS_INCLUSION", "B1_GROSS_ADD", "익금산입", 10000000, "RESERVE", true, "법인세법 제15조"],
    ["GROSS_EXCLUSION", "B1_GROSS_DEDUCT", "익금불산입", 2000000, "OTHER", false, "법인세법 제18조"],
    ["LOSS_INCLUSION", "B1_LOSS_DEDUCT", "손금산입", 0, "RESERVE", true, "법인세법 제19조"],
    ["LOSS_DISALLOWANCE", "B1_LOSS_ADD", "손금불산입", 0, "OUTFLOW", false, "법인세법 제19조"],
  ];
  return rows
    .map(
      ([section, code, name, amount, disposition, temporary, lawRef], index) => `
        <tr class="b1-item-row" data-index="${index}">
          <td>
            <select data-b1-field="section">
              ${["GROSS_INCLUSION", "GROSS_EXCLUSION", "LOSS_INCLUSION", "LOSS_DISALLOWANCE"]
                .map((value) => `<option value="${value}" ${value === section ? "selected" : ""}>${value}</option>`)
                .join("")}
            </select>
          </td>
          <td><input data-b1-field="item_code" value="${escapeHtml(code)}" /></td>
          <td><input data-b1-field="item_name" value="${escapeHtml(name)}" /></td>
          <td><input data-b1-field="amount" type="number" value="${amount}" /></td>
          <td>
            <select data-b1-field="disposition">
              ${["RESERVE", "OUTFLOW", "OTHER", "INTERNAL"]
                .map((value) => `<option value="${value}" ${value === disposition ? "selected" : ""}>${value}</option>`)
                .join("")}
            </select>
          </td>
          <td><input data-b1-field="temporary" type="checkbox" ${temporary ? "checked" : ""} /></td>
          <td><input data-b1-field="law_ref" value="${escapeHtml(lawRef)}" /></td>
        </tr>
      `,
    )
    .join("");
}

function collectIncomeAdjustmentItems() {
  return [...document.querySelectorAll(".b1-item-row")]
    .map((row) => {
      const value = (field) => row.querySelector(`[data-b1-field="${field}"]`);
      return {
        section: value("section").value,
        item_code: value("item_code").value.trim(),
        item_name: value("item_name").value.trim(),
        amount: Number(value("amount").value || 0),
        disposition: value("disposition").value,
        temporary: value("temporary").checked,
        law_ref: value("law_ref").value.trim(),
      };
    })
    .filter((item) => item.amount > 0 && item.item_code && item.item_name);
}

async function renderAdjustmentScreen() {
  if (state.activeAdjustmentPath === "/modules/adjustment/income") {
    await renderIncomeAdjustmentScreen();
    return;
  }
  if (state.activeAdjustmentPath === "/modules/adjustment/donations-entertainment") {
    await renderTransactionBasedAdjustmentScreen();
    return;
  }
  if (evaluationModuleForPath(state.activeAdjustmentPath)) {
    await renderEvaluationAdjustmentScreen();
    return;
  }
  await renderAssetBasedAdjustmentScreen();
}

function evaluationModuleForPath(path) {
  return {
    "/modules/adjustment/fx-valuation": ["B7", "B-7 외화평가"],
    "/modules/adjustment/inventory-valuation": ["B8", "B-8 재고·유가증권 평가"],
    "/modules/adjustment/carryforward-loss": ["B11", "B-11 이월결손금"],
    "/modules/adjustment/capital-reserves": ["B15", "B-15 자본금과 적립금"],
  }[path];
}

function assetModuleForPath(path) {
  return {
    "/modules/adjustment/depreciation": ["B4", "B-4 감가상각"],
    "/modules/adjustment/retirement-reserve": ["B5", "B-5 퇴직급여충당금"],
    "/modules/adjustment/bad-debt-reserve": ["B6", "B-6 대손충당금"],
    "/modules/adjustment/tax-credits": ["B6", "세액공제"],
    "/modules/adjustment/penalty-tax": ["B6", "가산세"],
  }[path] || ["B10", "B-10 업무용승용차"];
}

async function renderTransactionBasedAdjustmentScreen() {
  const context = await loadTaxDataContext();
  el("adjustmentScreenTitle").textContent = "B-2/B-3/B-9 거래 기반 세무조정";
  if (!context.byId) {
    el("adjustmentScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const root = `/api/tenants/${context.tenantCode}/business-years/${context.byId}`;
  const [b2Items, b3Items, b9Items, transactions] = await Promise.all([
    optionalRequest(`${root}/adjustments/transactions/B2`, []),
    optionalRequest(`${root}/adjustments/transactions/B3`, []),
    optionalRequest(`${root}/adjustments/transactions/B9`, []),
    optionalRequest(`${root}/tax-data/transactions`, []),
  ]);
  const adjustmentItems = [...b2Items, ...b3Items, ...b9Items];
  const itemRows = adjustmentItems.length
    ? adjustmentItems
        .map(
          (item) => `
            <tr>
              <td>${escapeHtml(item.source_module)}</td>
              <td>${escapeHtml(item.item_code)}<br><span class="muted">${escapeHtml(item.item_name)}</span></td>
              <td>${money.format(item.amount)}</td>
              <td>${escapeHtml(item.direction)}</td>
              <td>${escapeHtml(item.disposition)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">저장된 거래 기반 조정 항목이 없습니다.</td></tr>`;
  const filteredTransactions = transactions.filter((row) =>
    ["DONATION", "ENTERTAINMENT", "INTEREST"].includes((row.category || "").toUpperCase()),
  );
  const transactionRows = filteredTransactions.length
    ? filteredTransactions
        .map(
          (row) => `
            <tr>
              <td>${formatDate(row.tx_date)}</td>
              <td>${escapeHtml(row.category)}</td>
              <td>${escapeHtml(row.partner_name)}</td>
              <td>${escapeHtml(row.description || "-")}</td>
              <td>${escapeHtml(row.evidence_type || "-")}</td>
              <td>${money.format(row.amount)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="6">거래 명세가 없습니다.</td></tr>`;
  el("adjustmentScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-layout">
      <div class="law-form">
        <form id="b2TransactionForm">
          <h3>B-2 기부금</h3>
          <label>기부금 차감 전 기준소득<input id="b2BaseIncome" type="number" value="500000000" /></label>
          <button class="primary-btn" type="submit">기부금 한도/10년 이월 계산</button>
        </form>
        <form id="b3TransactionForm">
          <h3>B-3 접대비</h3>
          <label>제품매출<input id="b3ProductRevenue" type="number" value="2000000000" /></label>
          <label>용역매출<input id="b3ServiceRevenue" type="number" value="1000000000" /></label>
          <button class="primary-btn" type="submit">접대비 한도 계산</button>
        </form>
        <form id="b9TransactionForm">
          <h3>B-9 지급이자</h3>
          <label>가지급금 적수/평균잔액<input id="b9LoanBalance" type="number" value="100000000" /></label>
          <label>가중평균 이자율(bps)<input id="b9RateBps" type="number" value="460" /></label>
          <label>수동 손금불산입<input id="b9Manual" type="number" value="0" /></label>
          <div class="progress-bar"><span id="b9LoanBar" style="width:46%"></span></div>
          <button class="primary-btn" type="submit">지급이자 손금불산입 계산</button>
        </form>
      </div>
      <div class="law-table-panel">
        <h3>조정 결과</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>모듈</th><th>항목</th><th>금액</th><th>방향</th><th>처분</th></tr></thead>
            <tbody>${itemRows}</tbody>
          </table>
        </div>
        <pre id="transactionAdjustmentResult" class="json-result">{}</pre>
      </div>
    </div>
    <div class="law-table-panel">
      <h3>기부금 / 접대비 / 지급이자 거래 명세</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>일자</th><th>구분</th><th>거래처</th><th>설명</th><th>증빙</th><th>금액</th></tr></thead>
          <tbody>${transactionRows}</tbody>
        </table>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  el("b9RateBps").addEventListener("input", () => {
    el("b9LoanBar").style.width = `${Math.min(100, Math.max(0, Number(el("b9RateBps").value || 0) / 10))}%`;
  });
  el("b2TransactionForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await postTransactionAdjustment(root, "B2", {
      taxable_income_before_donation: Number(el("b2BaseIncome").value || 0),
    });
  });
  el("b3TransactionForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await postTransactionAdjustment(root, "B3", {
      revenue_breakdowns: [
        { revenue_category: "PRODUCT", amount: Number(el("b3ProductRevenue").value || 0) },
        { revenue_category: "SERVICE", amount: Number(el("b3ServiceRevenue").value || 0) },
      ],
    });
  });
  el("b9TransactionForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await postTransactionAdjustment(root, "B9", {
      weighted_average_loan_balance: Number(el("b9LoanBalance").value || 0),
      weighted_average_interest_rate_bps: Number(el("b9RateBps").value || 0),
      manual_interest_disallowance: Number(el("b9Manual").value || 0),
    });
  });
}

async function postTransactionAdjustment(root, moduleCode, body) {
  const result = await request(`${root}/adjustments/transactions/${moduleCode}`, {
    method: "POST",
    body: JSON.stringify(body),
  });
  el("transactionAdjustmentResult").textContent = JSON.stringify(result, null, 2);
  log(`${moduleCode} 거래 기반 조정 완료`, { addbacks: result.addbacks, deductions: result.deductions });
  await renderTransactionBasedAdjustmentScreen();
}

async function renderEvaluationAdjustmentScreen() {
  const context = await loadTaxDataContext();
  const [moduleCode, title] = evaluationModuleForPath(state.activeAdjustmentPath);
  el("adjustmentScreenTitle").textContent = title;
  if (!context.byId) {
    el("adjustmentScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const root = `/api/tenants/${context.tenantCode}/business-years/${context.byId}`;
  const [items, reserves] = await Promise.all([
    optionalRequest(`${root}/adjustments/evaluation/${moduleCode}`, []),
    optionalRequest(`${root}/reserves`, []),
  ]);
  const itemRows = items.length
    ? items
        .map(
          (item) => `
            <tr>
              <td>${escapeHtml(item.item_code)}<br><span class="muted">${escapeHtml(item.item_name)}</span></td>
              <td>${money.format(item.amount)}</td>
              <td>${escapeHtml(item.direction)}</td>
              <td>${escapeHtml(item.disposition)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="4">저장된 평가·이월·유보 조정 항목이 없습니다.</td></tr>`;
  const form = evaluationFormMarkup(moduleCode);
  el("adjustmentScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-layout">
      <form id="evaluationAdjustmentForm" class="law-form">
        <h3>${escapeHtml(title)} 계산</h3>
        ${form}
        <button class="primary-btn" type="submit">계산/저장</button>
      </form>
      <div class="law-table-panel">
        <h3>조정 결과</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>항목</th><th>금액</th><th>방향</th><th>처분</th></tr></thead>
            <tbody>${itemRows}</tbody>
          </table>
        </div>
        <pre id="evaluationAdjustmentResult" class="json-result">${JSON.stringify({ reserves }, null, 2)}</pre>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  el("evaluationAdjustmentForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = collectEvaluationPayload(moduleCode);
    const result = await request(`${root}/adjustments/evaluation/${moduleCode}`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    el("evaluationAdjustmentResult").textContent = JSON.stringify(result, null, 2);
    log(`${moduleCode} 평가·이월·유보 조정 완료`, { addbacks: result.addbacks, deductions: result.deductions });
    await renderEvaluationAdjustmentScreen();
  });
}

function evaluationFormMarkup(moduleCode) {
  if (moduleCode === "B11") {
    return `
      <label>공제 전 소득금액<input id="evalTaxableIncome" type="number" value="300000000" /></label>
      <label>결손 발생연도<input id="lossOriginYear" type="number" value="2025" /></label>
      <label>이월결손금 잔액<input id="lossRemainingAmount" type="number" value="120000000" /></label>
      <label>만료연도<input id="lossExpiresYear" type="number" value="2026" /></label>
    `;
  }
  if (moduleCode === "B15") {
    return `
      <label>변동일<input id="capitalChangeDate" type="date" value="2026-06-30" /></label>
      <label>변동 유형<input id="capitalChangeType" value="PAID_IN_CAPITAL" /></label>
      <label>변동 금액<input id="capitalChangeAmount" type="number" value="50000000" /></label>
      <label>설명<input id="capitalChangeDescription" value="유상증자" /></label>
    `;
  }
  return `
    <label>항목 코드<input id="valuationItemCode" value="${moduleCode === "B7" ? "USD_AR" : "INV_FINISHED"}" /></label>
    <label>항목명<input id="valuationItemName" value="${moduleCode === "B7" ? "USD receivable" : "Finished goods"}" /></label>
    <label>장부금액<input id="valuationBookAmount" type="number" value="120000000" /></label>
    <label>세법평가액<input id="valuationTaxAmount" type="number" value="100000000" /></label>
    <label>평가방법<input id="valuationMethod" value="${moduleCode === "B7" ? "CLOSING_RATE" : "LOWER_OF_COST_OR_MARKET"}" /></label>
  `;
}

function collectEvaluationPayload(moduleCode) {
  if (moduleCode === "B11") {
    return {
      taxable_income_before_loss: Number(el("evalTaxableIncome").value || 0),
      loss_carryforwards: [
        {
          origin_year: Number(el("lossOriginYear").value || 0),
          original_amount: Number(el("lossRemainingAmount").value || 0),
          remaining_amount: Number(el("lossRemainingAmount").value || 0),
          expires_year: Number(el("lossExpiresYear").value || 0),
        },
      ],
    };
  }
  if (moduleCode === "B15") {
    return {
      capital_changes: [
        {
          change_date: el("capitalChangeDate").value,
          change_type: el("capitalChangeType").value,
          amount: Number(el("capitalChangeAmount").value || 0),
          description: el("capitalChangeDescription").value,
        },
      ],
    };
  }
  return {
    positions: [
      {
        item_code: el("valuationItemCode").value,
        item_name: el("valuationItemName").value,
        position_type: moduleCode === "B7" ? "MONETARY" : "INVENTORY",
        monetary: moduleCode === "B7",
        valuation_method: el("valuationMethod").value,
        book_amount: Number(el("valuationBookAmount").value || 0),
        tax_amount: Number(el("valuationTaxAmount").value || 0),
      },
    ],
  };
}

async function renderAssetBasedAdjustmentScreen() {
  const context = await loadTaxDataContext();
  const [moduleCode, title] = assetModuleForPath(state.activeAdjustmentPath);
  el("adjustmentScreenTitle").textContent = title;
  if (!context.byId) {
    el("adjustmentScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const [items, reserves] = await Promise.all([
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/adjustments/assets/${moduleCode}`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/reserves`, []),
  ]);
  const itemRows = items.length
    ? items
        .map(
          (item) => `
            <tr>
              <td>${escapeHtml(item.item_code)}<br><span class="muted">${escapeHtml(item.item_name)}</span></td>
              <td>${money.format(item.amount)}</td>
              <td>${escapeHtml(item.direction)}</td>
              <td>${escapeHtml(item.disposition)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="4">저장된 조정 항목이 없습니다.</td></tr>`;
  el("adjustmentScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-layout">
      <form id="assetAdjustmentForm" class="law-form">
        <h3>${escapeHtml(title)} 계산</h3>
        <label>장부 충당금/상각비<input id="assetBookReserve" type="number" value="30000000" /></label>
        <label>추계액/채권잔액<input id="assetBaseAmount" type="number" value="200000000" /></label>
        <label>외부적립/실적대손<input id="assetExternalAmount" type="number" value="50000000" /></label>
        <label>율 또는 업무사용비율(bps)<input id="assetRateBps" type="number" value="${moduleCode === "B10" ? 8000 : 100}" /></label>
        <button class="primary-btn" type="submit">계산/저장</button>
      </form>
      <div class="law-table-panel">
        <h3>조정 결과</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>항목</th><th>금액</th><th>방향</th><th>처분</th></tr></thead>
            <tbody>${itemRows}</tbody>
          </table>
        </div>
        <pre id="assetAdjustmentResult" class="json-result">{}</pre>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  el("assetAdjustmentForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = {
      book_reserve: Number(el("assetBookReserve").value || 0),
      estimated_liability: Number(el("assetBaseAmount").value || 0),
      receivable_balance: Number(el("assetBaseAmount").value || 0),
      external_fund: Number(el("assetExternalAmount").value || 0),
      actual_bad_debt: Number(el("assetExternalAmount").value || 0),
      rate_bps: Number(el("assetRateBps").value || 0),
      business_use_bps: Number(el("assetRateBps").value || 10000),
    };
    const result = await request(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/adjustments/assets/${moduleCode}`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    el("assetAdjustmentResult").textContent = JSON.stringify({ result, reserves }, null, 2);
    log(`${title} 계산 완료`, { addbacks: result.addbacks, deductions: result.deductions });
    await renderAssetBasedAdjustmentScreen();
  });
}

async function renderIncomeAdjustmentScreen() {
  const context = await loadTaxDataContext();
  el("adjustmentScreenTitle").textContent = "B-1 소득금액조정";
  if (!context.byId) {
    el("adjustmentScreenBody").innerHTML = taxDataYearSelector(context);
    return;
  }
  const [snapshot, items, reserves] = await Promise.all([
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/snapshot`, null),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/adjustments/income`, []),
    optionalRequest(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/reserves`, []),
  ]);
  const law = snapshot?.snapshot_data?.law || {};
  const itemRows = items.length
    ? items
        .map(
          (item) => `
            <tr>
              <td>${escapeHtml(item.section)}</td>
              <td>${escapeHtml(item.item_code)}<br><span class="muted">${escapeHtml(item.item_name)}</span></td>
              <td>${money.format(item.amount)}</td>
              <td>${escapeHtml(item.direction)}</td>
              <td>${escapeHtml(item.disposition)}</td>
              <td>${escapeHtml(item.law_ref || "-")}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="6">저장된 B-1 조정 항목이 없습니다.</td></tr>`;
  const reserveRows = reserves.length
    ? reserves
        .map(
          (reserve) => `
            <tr>
              <td>${escapeHtml(reserve.reserve_code)}</td>
              <td>${money.format(reserve.amount)}</td>
              <td>${escapeHtml(reserve.direction)}</td>
              <td>${escapeHtml(reserve.carryforward_to || "-")}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="4">자동 생성된 유보가 없습니다.</td></tr>`;
  el("adjustmentScreenBody").innerHTML = `
    ${taxDataYearSelector(context)}
    <div class="law-summary-grid compact-grid">
      <article><span>적용 법령</span><strong>${escapeHtml(law.version_code || "-")}</strong></article>
      <article><span>스냅샷</span><strong>${snapshot?.snapshot_id || "-"}</strong></article>
      <article><span>잠금</span><strong>${snapshot?.locked ? "LOCKED" : "OPEN"}</strong></article>
    </div>
    <form id="incomeAdjustmentForm" class="law-table-panel">
      <div class="admin-toolbar">
        <label>결산서 당기순이익<input id="b1AccountingIncome" type="number" placeholder="비워두면 재무제표 NET_INCOME 사용" /></label>
        <label>항목 수<input value="4" readonly /></label>
      </div>
      <div class="table-wrap">
        <table>
          <thead><tr><th>섹션</th><th>코드</th><th>항목</th><th>금액</th><th>처분</th><th>유보</th><th>법조항</th></tr></thead>
          <tbody>${incomeAdjustmentRows()}</tbody>
        </table>
      </div>
      <div class="table-actions">
        <button class="primary-btn" type="submit">B-1 계산/저장</button>
      </div>
    </form>
    <div class="law-layout">
      <div class="law-table-panel">
        <h3>B-1 조정 항목</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>섹션</th><th>항목</th><th>금액</th><th>방향</th><th>처분</th><th>법조항</th></tr></thead>
            <tbody>${itemRows}</tbody>
          </table>
        </div>
      </div>
      <div class="law-table-panel">
        <h3>유보 자동 생성</h3>
        <div class="table-wrap">
          <table>
            <thead><tr><th>코드</th><th>금액</th><th>방향</th><th>이월연도</th></tr></thead>
            <tbody>${reserveRows}</tbody>
          </table>
        </div>
        <pre id="incomeAdjustmentResult" class="json-result">{}</pre>
      </div>
    </div>
  `;
  attachTaxDataYearSelector();
  el("incomeAdjustmentForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const accountingIncomeRaw = el("b1AccountingIncome").value.trim();
    const result = await request(`/api/tenants/${context.tenantCode}/business-years/${context.byId}/adjustments/income`, {
      method: "POST",
      body: JSON.stringify({
        accounting_income: accountingIncomeRaw ? Number(accountingIncomeRaw) : null,
        items: collectIncomeAdjustmentItems(),
      }),
    });
    el("incomeAdjustmentResult").textContent = JSON.stringify(result, null, 2);
    log("B-1 소득금액조정 완료", {
      taxable_income: result.taxable_income,
      reserves: result.reserves_created.length,
    });
    await renderIncomeAdjustmentScreen();
  });
}

async function renderFormScreen() {
  if (state.activeFormPath === "/modules/forms/relationships") {
    await renderFormRelationshipsScreen();
  } else if (state.activeFormPath === "/modules/forms/migrations") {
    await renderFormMigrationScreen();
  } else if (state.activeFormPath === "/modules/forms/resolver") {
    await renderFormResolverScreen();
  } else {
    await renderFormVersionsScreen();
  }
}

async function renderFormVersionsScreen() {
  el("formScreenTitle").textContent = "서식 버전 관리";
  const [forms, versions] = await Promise.all([
    request("/api/form-versioning/forms"),
    request("/api/form-versioning/versions"),
  ]);
  el("formScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="formVersionCreateForm" class="law-form">
        <h3>서식 버전 등록</h3>
        <label>서식 코드<input id="formCode" value="FORM3" /></label>
        <label>서식명<input id="formName" value="과세표준 및 세액조정계산서" /></label>
        <label>버전<input id="formVersionNo" value="2026.${Date.now().toString(36).slice(-2)}" /></label>
        <label>시행일<input id="formEffectiveFrom" type="date" value="2026-01-01" /></label>
        <label>필드 CSV<input id="formFields" value="taxable_income,corporate_tax,total_tax_due" /></label>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="law-table-panel">
        <div class="admin-toolbar">
          <label>서식 수<input value="${forms.length}" readonly /></label>
          <label>버전 수<input value="${versions.length}" readonly /></label>
        </div>
        <div class="table-wrap">
          <table>
            <thead><tr><th>서식</th><th>버전</th><th>기간</th><th>상태</th><th>조치</th></tr></thead>
            <tbody>${versions
              .map(
                (version) => `
                  <tr>
                    <td>${escapeHtml(version.form_code)}<br><span class="muted">${escapeHtml(version.form_name)}</span></td>
                    <td>${escapeHtml(version.version_no)}</td>
                    <td>${formatDate(version.effective_from)} ~ ${formatDate(version.effective_to)}</td>
                    <td><span class="status-pill ${version.status.toLowerCase()}">${escapeHtml(version.status)}</span></td>
                    <td class="table-actions">
                      <button class="secondary-btn compact" type="button" data-form-version-id="${version.form_version_id}" data-form-status="APPROVED">APPROVED</button>
                      <button class="secondary-btn compact" type="button" data-form-version-id="${version.form_version_id}" data-form-status="ACTIVE">ACTIVE</button>
                    </td>
                  </tr>
                `,
              )
              .join("")}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
  el("formVersionCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const fields = el("formFields").value.split(",").map((field) => field.trim()).filter(Boolean);
    const created = await request("/api/form-versioning/versions", {
      method: "POST",
      body: JSON.stringify({
        form_code: el("formCode").value.trim(),
        form_name: el("formName").value.trim(),
        version_no: el("formVersionNo").value.trim(),
        effective_from: el("formEffectiveFrom").value,
        effective_to: null,
        template_json: { fields },
      }),
    });
    log("서식 버전 등록 완료", created);
    await renderFormVersionsScreen();
  });
  document.querySelectorAll("[data-form-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const updated = await request(`/api/form-versioning/versions/${button.dataset.formVersionId}/status`, {
        method: "POST",
        body: JSON.stringify({ status: button.dataset.formStatus }),
      });
      log("서식 버전 상태 변경", updated);
      await renderFormVersionsScreen();
    });
  });
}

async function renderFormRelationshipsScreen() {
  el("formScreenTitle").textContent = "서식 항목 매핑";
  const relationships = await request("/api/form-versioning/relationships");
  el("formScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="formRelationshipCreateForm" class="law-form">
        <h3>항목 매핑 등록</h3>
        <label>원천 서식<input id="relSourceForm" value="FORM15" /></label>
        <label>원천 필드<input id="relSourceField" value="taxable_income" /></label>
        <label>대상 서식<input id="relTargetForm" value="FORM3" /></label>
        <label>대상 필드<input id="relTargetField" value="taxable_income" /></label>
        <label>시행일<input id="relEffectiveFrom" type="date" value="2026-01-01" /></label>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="law-table-panel">
        <div class="table-wrap">
          <table>
            <thead><tr><th>원천</th><th>대상</th><th>규칙</th><th>기간</th></tr></thead>
            <tbody>${relationships
              .map(
                (rel) => `
                  <tr>
                    <td>${escapeHtml(rel.source_form)}.${escapeHtml(rel.source_field)}</td>
                    <td>${escapeHtml(rel.target_form)}.${escapeHtml(rel.target_field)}</td>
                    <td><code>${escapeHtml(JSON.stringify(rel.rule_json))}</code></td>
                    <td>${formatDate(rel.effective_from)} ~ ${formatDate(rel.effective_to)}</td>
                  </tr>
                `,
              )
              .join("")}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;
  el("formRelationshipCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const created = await request("/api/form-versioning/relationships", {
      method: "POST",
      body: JSON.stringify({
        source_form: el("relSourceForm").value.trim(),
        source_field: el("relSourceField").value.trim(),
        target_form: el("relTargetForm").value.trim(),
        target_field: el("relTargetField").value.trim(),
        rule_json: { operation: "copy_latest" },
        effective_from: el("relEffectiveFrom").value,
        effective_to: null,
      }),
    });
    log("서식 항목 매핑 등록 완료", created);
    await renderFormRelationshipsScreen();
  });
}

async function renderFormMigrationScreen() {
  el("formScreenTitle").textContent = "서식 데이터 마이그레이션";
  const versions = await request("/api/form-versioning/versions");
  el("formScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="formMigrationForm" class="law-form">
        <h3>Dry-run / Execute / Rollback</h3>
        <label>테넌트<input id="migTenant" value="${escapeHtml(currentTenantCode())}" /></label>
        <label>사업연도 ID<input id="migById" type="number" value="${state.byId || ""}" /></label>
        <label>서식 코드<input id="migFormCode" value="FORM3" /></label>
        <label>대상 버전<select id="migToVersion">${versions.map((version) => `<option value="${version.form_version_id}">${escapeHtml(version.form_code)} ${escapeHtml(version.version_no)} ${escapeHtml(version.status)}</option>`).join("")}</select></label>
        <div class="button-row">
          <button class="secondary-btn" type="button" id="formDryRunBtn">Dry-run</button>
          <button class="primary-btn" type="button" id="formExecuteBtn">Execute</button>
          <button class="secondary-btn" type="button" id="formRollbackBtn">Rollback</button>
        </div>
      </form>
      <pre id="formMigrationOutput" class="json-result">{}</pre>
    </div>
  `;
  const body = () => ({
    tenant_code: el("migTenant").value.trim(),
    by_id: Number(el("migById").value),
    form_code: el("migFormCode").value.trim(),
    to_version_id: Number(el("migToVersion").value),
  });
  const runMigration = async (path) => {
    const result = await request(path, { method: "POST", body: JSON.stringify(body()) });
    el("formMigrationOutput").textContent = JSON.stringify(result, null, 2);
    log("서식 마이그레이션 실행", result);
  };
  el("formDryRunBtn").addEventListener("click", () => runMigration("/api/form-versioning/migrations/dry-run"));
  el("formExecuteBtn").addEventListener("click", () => runMigration("/api/form-versioning/migrations/execute"));
  el("formRollbackBtn").addEventListener("click", () => runMigration("/api/form-versioning/migrations/rollback"));
}

async function renderFormResolverScreen() {
  el("formScreenTitle").textContent = "사업연도 적용 서식";
  el("formScreenBody").innerHTML = `
    <div class="law-layout">
      <form id="formResolverForm" class="law-form">
        <h3>적용 서식 버전 조회</h3>
        <label>테넌트<input id="resolverTenant" value="${escapeHtml(currentTenantCode())}" /></label>
        <label>사업연도 ID<input id="resolverById" type="number" value="${state.byId || ""}" /></label>
        <label>서식 코드<input id="resolverFormCode" value="FORM3" /></label>
        <button class="primary-btn" type="submit">조회</button>
      </form>
      <pre id="formResolverOutput" class="json-result">{}</pre>
    </div>
  `;
  el("formResolverForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const params = new URLSearchParams({
      tenant_code: el("resolverTenant").value.trim(),
      by_id: el("resolverById").value,
      form_code: el("resolverFormCode").value.trim(),
    });
    const result = await request(`/api/form-versioning/resolve?${params.toString()}`);
    el("formResolverOutput").textContent = JSON.stringify(result, null, 2);
    log("적용 서식 버전 조회", result);
  });
}

async function refreshHealth() {
  try {
    const health = await request("/health");
    setHealth(health.status === "ok", `${health.service}: ${health.status}`);
    return health;
  } catch (error) {
    setHealth(false, "health error");
    log("상태 체크 실패", { message: error.message });
    return null;
  }
}

async function refreshTenants() {
  const tenants = await request("/api/tenants");
  el("tenantCount").textContent = tenants.length.toString();
  el("tenantRows").innerHTML = tenants
    .map(
      (tenant) => `
        <tr>
          <td>${escapeHtml(tenant.tenant_code)}</td>
          <td>${escapeHtml(tenant.tenant_name)}</td>
          <td>${escapeHtml(tenant.schema_name)}</td>
          <td><span class="status-pill active">${escapeHtml(tenant.status)}</span></td>
        </tr>
      `,
    )
    .join("");
}

async function refreshJobs() {
  const jobs = await request("/api/jobs");
  const dead = await request("/api/jobs?status=dead_letter");
  el("dlqCount").textContent = dead.length.toString();
  el("jobRows").innerHTML = jobs
    .slice(0, 12)
    .map(
      (job) => `
        <tr>
          <td title="${escapeHtml(job.job_id)}">${escapeHtml(job.job_id.slice(0, 8))}</td>
          <td>${escapeHtml(job.job_type)}</td>
          <td><span class="status-pill ${escapeHtml(job.status)}">${escapeHtml(job.status)}</span></td>
          <td>${job.attempts}/${job.max_attempts}</td>
        </tr>
      `,
    )
    .join("");
}

async function refreshDashboard() {
  await refreshHealth();
  await Promise.all([refreshTenants(), refreshJobs(), refreshLawData(false)]);
}

async function refreshLawData(rerender = true) {
  const [summary, laws] = await Promise.all([
    request("/api/law-versioning/summary"),
    request("/api/tax-laws"),
  ]);
  state.lawSummary = summary;
  state.lawVersions = laws;
  if (!state.selectedLawVersionId && laws.length) {
    state.selectedLawVersionId = laws[0].law_version_id;
  }
  if (state.selectedLawVersionId && !laws.some((law) => law.law_version_id === state.selectedLawVersionId)) {
    state.selectedLawVersionId = laws[0]?.law_version_id || null;
  }
  renderLawSummary();
  if (rerender) {
    await renderLawScreen();
  }
}

function renderLawSummary() {
  {
    const summary = state.lawSummary || {};
    const latest = summary.latest_law;
    let cards = [
      ["법령 버전", summary.laws ?? "-"],
      ["세율 구간", summary.rates ?? "-"],
      ["한도·공제", summary.limits ?? "-"],
      ["개정 이력", summary.amendments ?? "-"],
    ]
      .map(
        ([label, value]) => `
        <article>
          <span>${escapeHtml(label)}</span>
          <strong>${escapeHtml(value)}</strong>
        </article>
      `,
      )
      .join("");
    if (latest) {
      cards += `<article class="wide">
        <span>최신 적용 버전</span>
        <strong>${escapeHtml(latest.version_code)} / ${escapeHtml(latest.status)}</strong>
      </article>`;
    }
    document.querySelectorAll("[data-law-summary]").forEach((summaryNode) => {
      summaryNode.innerHTML = cards;
    });
    return;
  }
  const summary = state.lawSummary || {};
  const latest = summary.latest_law;
  el("lawSummaryCards").innerHTML = [
    ["법령 버전", summary.laws ?? "-"],
    ["세율 구간", summary.rates ?? "-"],
    ["한도·율", summary.limits ?? "-"],
    ["개정 이력", summary.amendments ?? "-"],
  ]
    .map(
      ([label, value]) => `
        <article>
          <span>${escapeHtml(label)}</span>
          <strong>${escapeHtml(value)}</strong>
        </article>
      `,
    )
    .join("");
  if (latest) {
    el("lawSummaryCards").insertAdjacentHTML(
      "beforeend",
      `<article class="wide">
        <span>최신 적용 버전</span>
        <strong>${escapeHtml(latest.version_code)} / ${escapeHtml(latest.status)}</strong>
      </article>`,
    );
  }
}

async function renderLawScreen() {
  const screen = currentLawScreen();
  setActiveLawPanel();
  await screen.render();
}

function lawVersionOptions(selectedId = state.selectedLawVersionId) {
  return state.lawVersions
    .map(
      (law) => `
        <option value="${law.law_version_id}" ${law.law_version_id === selectedId ? "selected" : ""}>
          ${escapeHtml(law.version_code)} · ${escapeHtml(law.law_name)}
        </option>
      `,
    )
    .join("");
}

function selectedLawVersion() {
  return state.lawVersions.find((law) => law.law_version_id === state.selectedLawVersionId) || state.lawVersions[0];
}

function legacyRenderLawSelector() {
  return `
    <label>
      법령 버전
      <select id="lawVersionSelect">${lawVersionOptions()}</select>
    </label>
  `;
}

function attachLawVersionSelect(onChange = renderLawScreen) {
  const select = el("lawVersionSelect");
  if (select) {
    select.addEventListener("change", async () => {
      state.selectedLawVersionId = Number(select.value);
      await onChange();
    });
  }
}

async function legacyRenderLawMasterScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="lawCreateForm" class="law-form">
        <h3>신규 법령 버전 등록</h3>
        <label>버전 코드<input id="lawVersionCode" value="CIT-${new Date().getFullYear()}-${Date.now().toString(36).slice(-4).toUpperCase()}" /></label>
        <label>법령명<input id="lawName" value="법인세법 ${new Date().getFullYear()} 개정" /></label>
        <label>적용 시작일<input id="lawEffectiveFrom" type="date" value="${new Date().getFullYear()}-01-01" /></label>
        <label>적용 종료일<input id="lawEffectiveTo" type="date" /></label>
        <label>변경 요약<input id="lawChangeSummary" value="법령·세율 버전 관리 화면에서 등록" /></label>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="law-table-panel">
        <h3>법령 버전 목록</h3>
        ${renderLawTable()}
      </div>
    </div>
  `);

  el("lawCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = {
      version_code: el("lawVersionCode").value.trim(),
      law_name: el("lawName").value.trim(),
      effective_from: el("lawEffectiveFrom").value,
      effective_to: el("lawEffectiveTo").value || null,
      metadata: {
        change_summary: el("lawChangeSummary").value.trim(),
        source: "law-versioning-ui",
      },
    };
    const law = await request("/api/tax-laws", { method: "POST", body: JSON.stringify(body) });
    state.selectedLawVersionId = law.law_version_id;
    await request("/api/law-amendments", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: law.law_version_id,
        change_summary: body.metadata.change_summary,
        approved_by: state.user?.login_id || "web",
      }),
    });
    await refreshLawData();
    log("법령 버전 등록 완료", law);
  });

  document.querySelectorAll("[data-law-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const lawVersionId = Number(button.dataset.lawId);
      const status = button.dataset.lawStatus;
      const currentLaw = state.lawVersions.find((law) => law.law_version_id === lawVersionId);
      if (status === "ACTIVE" && currentLaw?.status === "DRAFT") {
        await request(`/api/tax-laws/${lawVersionId}/status`, {
          method: "POST",
          body: JSON.stringify({
            status: "REVIEWED",
            change_summary: "?곹깭 蹂寃? REVIEWED",
            approved_by: state.user?.login_id || "web",
          }),
        });
      }
      const law = await request(`/api/tax-laws/${lawVersionId}/status`, {
        method: "POST",
        body: JSON.stringify({
          status,
          change_summary: `상태 변경: ${status}`,
          approved_by: state.user?.login_id || "web",
        }),
      });
      await refreshLawData();
      log("법령 상태 변경 완료", law);
    });
  });
}

function legacyRenderLawTable() {
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>버전</th>
            <th>법령명</th>
            <th>적용기간</th>
            <th>상태</th>
            <th>조치</th>
          </tr>
        </thead>
        <tbody>
          ${state.lawVersions
            .map(
              (law) => `
                <tr>
                  <td>${escapeHtml(law.version_code)}</td>
                  <td>${escapeHtml(law.law_name)}</td>
                  <td>${formatDate(law.effective_from)} ~ ${formatDate(law.effective_to)}</td>
                  <td><span class="status-pill ${escapeHtml(law.status.toLowerCase())}">${escapeHtml(law.status)}</span></td>
                  <td class="table-actions">
                    <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="REVIEWED">검토</button>
                    <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="APPROVED">승인</button>
                    <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="ACTIVE">활성</button>
                  </td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

async function legacyRenderTaxRatesScreen() {
  const law = selectedLawVersion();
  const rates = law ? await request(`/api/tax-rates?law_version_id=${law.law_version_id}`) : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="taxRateForm" class="law-form">
        <h3>세율 구간 등록</h3>
        ${renderLawSelector()}
        <label>항목 코드<input id="rateItemCode" value="CORPORATE_TAX" /></label>
        <label>과세표준 From<input id="rateFrom" type="number" value="0" /></label>
        <label>과세표준 To<input id="rateTo" type="number" /></label>
        <label>세율 bps<input id="rateBps" type="number" value="900" /></label>
        <label>누진공제<input id="rateDeduction" type="number" value="0" /></label>
        <button class="primary-btn" type="submit">세율 저장</button>
      </form>
      <div class="law-table-panel">
        <h3>세율표</h3>
        <div class="law-inline-tools">
          <label>계산 미리보기<input id="ratePreviewIncome" type="number" value="300000000" /></label>
          <button id="ratePreviewBtn" class="secondary-btn" type="button">계산</button>
          <strong id="ratePreviewResult">-</strong>
        </div>
        ${renderRateTable(rates)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("taxRateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = {
      law_version_id: Number(el("lawVersionSelect").value),
      item_code: el("rateItemCode").value.trim().toUpperCase(),
      taxable_from: Number(el("rateFrom").value || 0),
      taxable_to: el("rateTo").value ? Number(el("rateTo").value) : null,
      base_tax: 0,
      rate_bps: Number(el("rateBps").value || 0),
      progressive_deduction: Number(el("rateDeduction").value || 0),
      effective_from: law?.effective_from || new Date().toISOString().slice(0, 10),
      effective_to: law?.effective_to || null,
      metadata: { source: "law-versioning-ui" },
    };
    const rate = await request("/api/tax-rates", { method: "POST", body: JSON.stringify(body) });
    await renderLawScreen();
    log("세율 저장 완료", rate);
  });
  el("ratePreviewBtn").addEventListener("click", () => {
    const income = Number(el("ratePreviewIncome").value || 0);
    el("ratePreviewResult").textContent = `${money.format(calculateRatePreview(income, rates))} 원`;
  });
}

function legacyRenderRateTable(rates) {
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>항목</th>
            <th>과세표준</th>
            <th>세율</th>
            <th>누진공제</th>
            <th>기간</th>
          </tr>
        </thead>
        <tbody>
          ${rates
            .map(
              (rate) => `
                <tr>
                  <td>${escapeHtml(rate.item_code)}</td>
                  <td>${money.format(rate.taxable_from)} ~ ${rate.taxable_to ? money.format(rate.taxable_to) : "상한 없음"}</td>
                  <td>${(rate.rate_bps / 100).toFixed(2)}%</td>
                  <td>${money.format(rate.progressive_deduction)}</td>
                  <td>${formatDate(rate.effective_from)} ~ ${formatDate(rate.effective_to)}</td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

function calculateRatePreview(income, rates) {
  const rate = rates.find((row) => income >= row.taxable_from && (row.taxable_to === null || income <= row.taxable_to));
  if (!rate) {
    return 0;
  }
  return Math.max(0, Math.floor((income * rate.rate_bps) / 10000) + rate.base_tax - rate.progressive_deduction);
}

async function legacyRenderPolicyParameterScreen(config) {
  const law = selectedLawVersion();
  const params = law
    ? await request(
        `/api/tax-limits?law_version_id=${law.law_version_id}&category=${encodeURIComponent(config.category)}`,
      )
    : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="policyParamForm" class="law-form">
        <h3>${escapeHtml(config.title)} 등록</h3>
        <p class="form-help">${escapeHtml(config.description)}</p>
        ${renderLawSelector()}
        <label>항목 코드<input id="paramItemCode" value="${escapeHtml(config.defaultItemCode)}" /></label>
        <label>값<input id="paramAmount" type="number" value="${config.defaultAmount}" /></label>
        <label>설명<input id="paramDescription" value="${escapeHtml(config.title)} 파라미터" /></label>
        <button class="primary-btn" type="submit">저장</button>
      </form>
      <div class="law-table-panel">
        <h3>${escapeHtml(config.title)} 목록</h3>
        ${renderLimitTable(params)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("policyParamForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const selectedLaw = selectedLawVersion();
    const body = {
      law_version_id: Number(el("lawVersionSelect").value),
      item_code: el("paramItemCode").value.trim().toUpperCase(),
      amount: Number(el("paramAmount").value || 0),
      effective_from: selectedLaw?.effective_from || new Date().toISOString().slice(0, 10),
      effective_to: selectedLaw?.effective_to || null,
      metadata: {
        category: config.category,
        description: el("paramDescription").value.trim(),
        source: "law-versioning-ui",
      },
    };
    const limit = await request("/api/tax-limits", { method: "POST", body: JSON.stringify(body) });
    await renderLawScreen();
    log(`${config.title} 저장 완료`, limit);
  });
}

function legacyRenderLimitTable(params) {
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>항목</th>
            <th>값</th>
            <th>분류</th>
            <th>설명</th>
            <th>기간</th>
          </tr>
        </thead>
        <tbody>
          ${params
            .map(
              (param) => `
                <tr>
                  <td>${escapeHtml(param.item_code)}</td>
                  <td>${money.format(param.amount)}</td>
                  <td>${escapeHtml(param.metadata?.category || param.metadata?.group || "-")}</td>
                  <td>${escapeHtml(param.metadata?.description || "-")}</td>
                  <td>${formatDate(param.effective_from)} ~ ${formatDate(param.effective_to)}</td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

async function legacyRenderSnapshotScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="snapshotForm" class="law-form">
        <h3>스냅샷 조회/생성</h3>
        <label>테넌트 코드<input id="snapshotTenantCode" value="${escapeHtml(state.tenantCode || "demo")}" /></label>
        <label>사업연도 ID<input id="snapshotById" type="number" value="${state.byId || ""}" /></label>
        <div class="button-row">
          <button id="snapshotCreateBtn" class="primary-btn" type="button">생성</button>
          <button id="snapshotGetBtn" class="secondary-btn" type="button">조회</button>
        </div>
      </form>
      <div class="law-table-panel">
        <h3>스냅샷 결과</h3>
        <pre id="snapshotResult" class="json-result"></pre>
      </div>
    </div>
  `);
  const run = async (method) => {
    const tenantCode = el("snapshotTenantCode").value.trim();
    const byId = Number(el("snapshotById").value || 0);
    if (!tenantCode || !byId) {
      el("snapshotResult").textContent = "테넌트 코드와 사업연도 ID가 필요합니다. 데모 실행 후 자동 입력됩니다.";
      return;
    }
    const snapshot = await request(`/api/tenants/${tenantCode}/business-years/${byId}/snapshot`, {
      method,
      body: method === "POST" ? "{}" : undefined,
    });
    el("snapshotResult").textContent = JSON.stringify(snapshot, null, 2);
    log(`스냅샷 ${method === "POST" ? "생성" : "조회"} 완료`, {
      tenantCode,
      byId,
      snapshot_id: snapshot.snapshot_id,
    });
  };
  el("snapshotCreateBtn").addEventListener("click", () => run("POST").catch((error) => log("스냅샷 생성 실패", { message: error.message })));
  el("snapshotGetBtn").addEventListener("click", () => run("GET").catch((error) => log("스냅샷 조회 실패", { message: error.message })));
}

async function legacyRenderImpactScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="impactForm" class="law-form">
        <h3>영향 시뮬레이션 실행</h3>
        ${renderLawSelector()}
        <label class="checkbox-row">
          <input id="impactIncludeLocked" type="checkbox" />
          잠금 스냅샷 포함
        </label>
        <button class="primary-btn" type="submit">시뮬레이션</button>
      </form>
      <div class="law-table-panel">
        <h3>시뮬레이션 결과</h3>
        <div id="impactResult"></div>
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("impactForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const impact = await request("/api/law-versioning/impact", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(el("lawVersionSelect").value),
        include_locked: el("impactIncludeLocked").checked,
      }),
    });
    renderImpactResult(impact);
    log("영향 시뮬레이션 완료", impact.summary);
  });
}

function legacyRenderImpactResult(impact) {
  el("impactResult").innerHTML = `
    <div class="law-summary-grid compact-grid">
      <article><span>대상 사업연도</span><strong>${impact.summary.business_years}</strong></article>
      <article><span>잠금 스냅샷</span><strong>${impact.summary.locked_snapshots}</strong></article>
      <article><span>세율 행</span><strong>${impact.summary.rate_rows}</strong></article>
      <article><span>한도 행</span><strong>${impact.summary.limit_rows}</strong></article>
    </div>
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>테넌트</th><th>스키마</th><th>대상 사업연도</th><th>잠금</th></tr>
        </thead>
        <tbody>
          ${(impact.tenant_impacts || [])
            .map(
              (tenant) => `
                <tr>
                  <td>${escapeHtml(tenant.tenant_code)}</td>
                  <td>${escapeHtml(tenant.schema_name)}</td>
                  <td>${tenant.business_years}</td>
                  <td>${tenant.locked_snapshots}</td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

async function legacyRenderHistoryScreen() {
  const law = selectedLawVersion();
  const histories = law ? await request(`/api/law-amendments?law_version_id=${law.law_version_id}`) : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="historyForm" class="law-form">
        <h3>개정 이력 등록</h3>
        ${renderLawSelector()}
        <label>개정 요약<input id="historySummary" value="법령 개정 공지 등록" /></label>
        <label>승인자<input id="historyApprovedBy" value="${escapeHtml(state.user?.login_id || "admin")}" /></label>
        <button class="primary-btn" type="submit">이력 저장</button>
      </form>
      <div class="law-table-panel">
        <h3>개정 이력</h3>
        ${renderHistoryTable(histories)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("historyForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const history = await request("/api/law-amendments", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(el("lawVersionSelect").value),
        change_summary: el("historySummary").value.trim(),
        approved_by: el("historyApprovedBy").value.trim(),
      }),
    });
    await refreshLawData();
    log("개정 이력 저장 완료", history);
  });
}

function legacyRenderHistoryTable(histories) {
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>ID</th><th>요약</th><th>승인자</th><th>승인일시</th></tr>
        </thead>
        <tbody>
          ${histories
            .map(
              (history) => `
                <tr>
                  <td>${history.amendment_id}</td>
                  <td>${escapeHtml(history.change_summary)}</td>
                  <td>${escapeHtml(history.approved_by)}</td>
                  <td>${formatDateTime(history.approved_at)}</td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

async function createDemoCase() {
  setBusy(true);
  try {
    const suffix = Date.now().toString(36).slice(-8);
    const tenantCode = `ui${suffix}`;
    log("데모 테넌트 생성 시작", { tenantCode });

    const tenant = await request("/api/tenants", {
      method: "POST",
      body: JSON.stringify({
        tenant_code: tenantCode,
        tenant_name: "UI Demo Tax Firm",
        biz_reg_no: "1234567890",
        contract_start: "2026-01-01",
        contract_end: null,
        max_users: 20,
      }),
    });

    const customer = await request(`/api/tenants/${tenantCode}/customers`, {
      method: "POST",
      body: JSON.stringify({
        customer_code: "CUST001",
        customer_name: "서울테크 주식회사",
        biz_reg_no: "2208112345",
        corp_reg_no: "1101111234567",
        industry_code: "62010",
        is_sme: true,
      }),
    });

    const businessYear = await request(`/api/tenants/${tenantCode}/business-years`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: customer.customer_id,
        year_label: 2026,
        start_date: "2026-01-01",
        end_date: "2026-12-31",
      }),
    });

    await request(`/api/tenants/${tenantCode}/business-years/${businessYear.by_id}/snapshot`, {
      method: "POST",
      body: "{}",
    });

    const calculation = await request(
      `/api/tenants/${tenantCode}/business-years/${businessYear.by_id}/adjustments`,
      {
        method: "POST",
        body: JSON.stringify({
          accounting_income: numberValue("accountingIncome"),
          gross_revenue: numberValue("grossRevenue"),
          donations: numberValue("donations"),
          entertainment_expense: numberValue("entertainmentExpense"),
          depreciation_book: numberValue("depreciationBook"),
          depreciation_tax_limit: numberValue("depreciationTaxLimit"),
          carryforward_loss: numberValue("carryforwardLoss"),
          tax_credits: numberValue("taxCredits"),
        }),
      },
    );

    const form = await request(
      `/api/tenants/${tenantCode}/business-years/${businessYear.by_id}/forms/FORM3`,
      { method: "POST", body: "{}" },
    );

    const job = await request(
      `/api/tenants/${tenantCode}/business-years/${businessYear.by_id}/efilings`,
      { method: "POST", body: JSON.stringify({ max_attempts: 3 }) },
    );

    state.tenantCode = tenant.tenant_code;
    state.byId = businessYear.by_id;
    state.efileJobId = job.job_id;

    el("tenantCode").textContent = state.tenantCode;
    el("businessYearId").textContent = state.byId.toString();
    el("taxableIncome").textContent = `${money.format(calculation.taxable_income)} 원`;
    el("totalTaxDue").textContent = `${money.format(form.data_json.total_tax_due)} 원`;
    el("efileJob").textContent = job.job_id;
    log("데모 세무조정 완료", { calculation, form, efile_job: job });

    await refreshDashboard();
    if (state.activeLawPath === "/modules/law-versioning/snapshots") {
      await renderLawScreen();
    }
  } catch (error) {
    log("데모 실행 실패", { message: error.message });
  } finally {
    setBusy(false);
  }
}

function formatDate(value) {
  return value || "현행";
}

function formatDateTime(value) {
  if (!value) {
    return "-";
  }
  return new Date(value).toLocaleString("ko-KR", { hour12: false });
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

const cleanLawScreenDefinitions = {
  "/modules/law-versioning/laws": {
    title: "법령 버전 마스터",
    render: renderLawMasterScreen,
  },
  "/modules/law-versioning/rates": {
    title: "법인세율표",
    render: renderTaxRatesScreen,
  },
  "/modules/law-versioning/limits": {
    title: "한도·공제표",
    render: () =>
      renderPolicyParameterScreen({
        title: "한도·공제표",
        category: "LIMIT",
        defaultItemCode: "ENTERTAINMENT_BASE_LIMIT",
        defaultAmount: 12000000,
        description: "접대비, 기부금, 과소자본세제 등 한도성 세무 파라미터를 법령 버전별로 관리합니다.",
      }),
  },
  "/modules/law-versioning/credits": {
    title: "세액공제·감면 표",
    render: () =>
      renderPolicyParameterScreen({
        title: "세액공제·감면 표",
        category: "CREDIT",
        defaultItemCode: "RND_CREDIT_BPS",
        defaultAmount: 2500,
        description: "세액공제율과 감면율을 bps 단위로 저장합니다. 25%는 2500으로 입력합니다.",
      }),
  },
  "/modules/law-versioning/depreciation-lives": {
    title: "기준내용연수표",
    render: () =>
      renderPolicyParameterScreen({
        title: "기준내용연수표",
        category: "DEPRECIATION_LIFE",
        defaultItemCode: "MACHINE_USEFUL_LIFE_YEARS",
        defaultAmount: 5,
        description: "자산 분류별 기준내용연수를 법령 버전별 파라미터로 관리합니다.",
      }),
  },
  "/modules/law-versioning/sme-criteria": {
    title: "중소기업 판정기준",
    render: () =>
      renderPolicyParameterScreen({
        title: "중소기업 판정기준",
        category: "SME_CRITERIA",
        defaultItemCode: "SME_REVENUE_LIMIT",
        defaultAmount: 12000000000,
        description: "업종별 매출액 등 중소기업 판정 기준을 법령 버전별로 관리합니다.",
      }),
  },
  "/modules/law-versioning/loss-rules": {
    title: "결손금 공제규정",
    render: () =>
      renderPolicyParameterScreen({
        title: "결손금 공제규정",
        category: "LOSS_RULE",
        defaultItemCode: "LOSS_CARRYFORWARD_YEARS",
        defaultAmount: 15,
        description: "결손금 이월공제 기간과 공제 한도를 사업연도별 규칙으로 관리합니다.",
      }),
  },
  "/modules/law-versioning/snapshots": {
    title: "사업연도별 적용 스냅샷",
    render: renderSnapshotScreen,
  },
  "/modules/law-versioning/impact": {
    title: "영향 시뮬레이션",
    render: renderImpactScreen,
  },
  "/modules/law-versioning/history": {
    title: "개정 공지/이력",
    render: renderHistoryScreen,
  },
};

Object.entries(cleanLawScreenDefinitions).forEach(([path, definition]) => {
  Object.assign(lawScreens[path], definition);
});

function renderLawSelector() {
  return `
    <label>
      법령 버전
      <select id="lawVersionSelect">${lawVersionOptions()}</select>
    </label>
  `;
}

function formatDate(value) {
  return value || "현재";
}

function lawStatusActionButtons(law) {
  const nextStatuses = {
    DRAFT: ["REVIEWED"],
    REVIEWED: ["ACTIVE", "DRAFT"],
    APPROVED: ["ACTIVE", "RETIRED"],
    ACTIVE: ["RETIRED"],
    RETIRED: [],
  }[law.status] || [];
  return nextStatuses.length
    ? nextStatuses
        .map(
          (status) =>
            `<button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="${status}">${status}</button>`,
        )
        .join("")
    : `<span class="muted">-</span>`;
}

function renderLawTable() {
  const rows = state.lawVersions.length
    ? state.lawVersions
        .map(
          (law) => `
            <tr>
              <td>${escapeHtml(law.version_code)}</td>
              <td>${escapeHtml(law.law_name)}</td>
              <td>${formatDate(law.effective_from)} ~ ${formatDate(law.effective_to)}</td>
              <td><span class="status-pill ${escapeHtml(law.status.toLowerCase())}">${escapeHtml(law.status)}</span></td>
              <td class="table-actions">
                <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="REVIEWED">검토</button>
                <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="APPROVED">승인</button>
                <button class="secondary-btn compact" type="button" data-law-id="${law.law_version_id}" data-law-status="ACTIVE">활성</button>
              </td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">등록된 법령 버전이 없습니다.</td></tr>`;
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>버전</th>
            <th>법령명</th>
            <th>적용기간</th>
            <th>상태</th>
            <th>조치</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
  `;
}

async function renderLawMasterScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="lawCreateForm" class="law-form">
        <h3>신규 법령 버전 등록</h3>
        <label>버전 코드<input id="lawVersionCode" value="CIT-${new Date().getFullYear()}-${Date.now().toString(36).slice(-4).toUpperCase()}" /></label>
        <label>법령명<input id="lawName" value="법인세법 ${new Date().getFullYear()} 개정" /></label>
        <label>적용 시작일<input id="lawEffectiveFrom" type="date" value="${new Date().getFullYear()}-01-01" /></label>
        <label>적용 종료일<input id="lawEffectiveTo" type="date" /></label>
        <label>변경 요약<input id="lawChangeSummary" value="법령·세율 버전 관리 화면에서 등록" /></label>
        <button class="primary-btn" type="submit">등록</button>
      </form>
      <div class="law-table-panel">
        <h3>법령 버전 목록</h3>
        ${renderLawTable()}
      </div>
    </div>
  `);

  el("lawCreateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = {
      version_code: el("lawVersionCode").value.trim(),
      law_name: el("lawName").value.trim(),
      effective_from: el("lawEffectiveFrom").value,
      effective_to: el("lawEffectiveTo").value || null,
      metadata: {
        change_summary: el("lawChangeSummary").value.trim(),
        source: "law-versioning-ui",
      },
    };
    const law = await request("/api/tax-laws", { method: "POST", body: JSON.stringify(body) });
    state.selectedLawVersionId = law.law_version_id;
    await request("/api/law-amendments", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: law.law_version_id,
        change_summary: body.metadata.change_summary,
        approved_by: state.user?.login_id || "web",
      }),
    });
    await refreshLawData();
    log("법령 버전 등록 완료", law);
  });

  document.querySelectorAll("[data-law-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const lawVersionId = Number(button.dataset.lawId);
      const status = button.dataset.lawStatus;
      const currentLaw = state.lawVersions.find((law) => law.law_version_id === lawVersionId);
      if (status === "ACTIVE" && currentLaw?.status === "DRAFT") {
        await request(`/api/tax-laws/${lawVersionId}/status`, {
          method: "POST",
          body: JSON.stringify({
            status: "REVIEWED",
            change_summary: "?곹깭 蹂寃? REVIEWED",
            approved_by: state.user?.login_id || "web",
          }),
        });
      }
      const law = await request(`/api/tax-laws/${lawVersionId}/status`, {
        method: "POST",
        body: JSON.stringify({
          status,
          change_summary: `상태 변경: ${status}`,
          approved_by: state.user?.login_id || "web",
        }),
      });
      await refreshLawData();
      log("법령 상태 변경 완료", law);
    });
  });
}

function renderRateTable(rates) {
  const rows = rates.length
    ? rates
        .map(
          (rate) => `
            <tr>
              <td>${escapeHtml(rate.item_code)}</td>
              <td>${money.format(rate.taxable_from)} ~ ${rate.taxable_to ? money.format(rate.taxable_to) : "상한 없음"}</td>
              <td>${(rate.rate_bps / 100).toFixed(2)}%</td>
              <td>${money.format(rate.progressive_deduction)}</td>
              <td>${formatDate(rate.effective_from)} ~ ${formatDate(rate.effective_to)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">등록된 세율 구간이 없습니다.</td></tr>`;
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>항목</th>
            <th>과세표준</th>
            <th>세율</th>
            <th>누진공제</th>
            <th>기간</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
  `;
}

async function renderTaxRatesScreen() {
  const law = selectedLawVersion();
  const rates = law ? await request(`/api/tax-rates?law_version_id=${law.law_version_id}`) : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="taxRateForm" class="law-form">
        <h3>세율 구간 등록</h3>
        ${renderLawSelector()}
        <label>항목 코드<input id="rateItemCode" value="CORPORATE_TAX" /></label>
        <label>과세표준 From<input id="rateFrom" type="number" value="0" /></label>
        <label>과세표준 To<input id="rateTo" type="number" /></label>
        <label>세율 bps<input id="rateBps" type="number" value="900" /></label>
        <label>누진공제<input id="rateDeduction" type="number" value="0" /></label>
        <button class="primary-btn" type="submit">세율 저장</button>
      </form>
      <div class="law-table-panel">
        <h3>세율표</h3>
        <div class="law-inline-tools">
          <label>계산 미리보기<input id="ratePreviewIncome" type="number" value="300000000" /></label>
          <button id="ratePreviewBtn" class="secondary-btn" type="button">계산</button>
          <strong id="ratePreviewResult">-</strong>
        </div>
        ${renderRateTable(rates)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("taxRateForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = {
      law_version_id: Number(el("lawVersionSelect").value),
      item_code: el("rateItemCode").value.trim().toUpperCase(),
      taxable_from: Number(el("rateFrom").value || 0),
      taxable_to: el("rateTo").value ? Number(el("rateTo").value) : null,
      base_tax: 0,
      rate_bps: Number(el("rateBps").value || 0),
      progressive_deduction: Number(el("rateDeduction").value || 0),
      effective_from: law?.effective_from || new Date().toISOString().slice(0, 10),
      effective_to: law?.effective_to || null,
      metadata: { source: "law-versioning-ui" },
    };
    const rate = await request("/api/tax-rates", { method: "POST", body: JSON.stringify(body) });
    await renderLawScreen();
    log("세율 저장 완료", rate);
  });
  el("ratePreviewBtn").addEventListener("click", () => {
    const income = Number(el("ratePreviewIncome").value || 0);
    el("ratePreviewResult").textContent = `${money.format(calculateRatePreview(income, rates))} 원`;
  });
}

function renderLimitTable(params) {
  const rows = params.length
    ? params
        .map(
          (param) => `
            <tr>
              <td>${escapeHtml(param.item_code)}</td>
              <td>${money.format(param.amount)}</td>
              <td>${escapeHtml(param.metadata?.category || param.metadata?.group || "-")}</td>
              <td>${escapeHtml(param.metadata?.description || "-")}</td>
              <td>${formatDate(param.effective_from)} ~ ${formatDate(param.effective_to)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="5">등록된 파라미터가 없습니다.</td></tr>`;
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>항목</th>
            <th>값</th>
            <th>분류</th>
            <th>설명</th>
            <th>기간</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
  `;
}

async function renderPolicyParameterScreen(config) {
  const law = selectedLawVersion();
  const params = law
    ? await request(
        `/api/tax-limits?law_version_id=${law.law_version_id}&category=${encodeURIComponent(config.category)}`,
      )
    : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="policyParamForm" class="law-form">
        <h3>${escapeHtml(config.title)} 등록</h3>
        <p class="form-help">${escapeHtml(config.description)}</p>
        ${renderLawSelector()}
        <label>항목 코드<input id="paramItemCode" value="${escapeHtml(config.defaultItemCode)}" /></label>
        <label>값<input id="paramAmount" type="number" value="${config.defaultAmount}" /></label>
        <label>설명<input id="paramDescription" value="${escapeHtml(config.title)} 파라미터" /></label>
        <button class="primary-btn" type="submit">저장</button>
      </form>
      <div class="law-table-panel">
        <h3>${escapeHtml(config.title)} 목록</h3>
        ${renderLimitTable(params)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("policyParamForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const selectedLaw = selectedLawVersion();
    const body = {
      law_version_id: Number(el("lawVersionSelect").value),
      item_code: el("paramItemCode").value.trim().toUpperCase(),
      amount: Number(el("paramAmount").value || 0),
      effective_from: selectedLaw?.effective_from || new Date().toISOString().slice(0, 10),
      effective_to: selectedLaw?.effective_to || null,
      metadata: {
        category: config.category,
        description: el("paramDescription").value.trim(),
        source: "law-versioning-ui",
      },
    };
    const limit = await request("/api/tax-limits", { method: "POST", body: JSON.stringify(body) });
    await renderLawScreen();
    log(`${config.title} 저장 완료`, limit);
  });
}

async function renderSnapshotScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="snapshotForm" class="law-form">
        <h3>스냅샷 조회/생성</h3>
        <label>테넌트 코드<input id="snapshotTenantCode" value="${escapeHtml(state.tenantCode || "demo")}" /></label>
        <label>사업연도 ID<input id="snapshotById" type="number" value="${state.byId || ""}" /></label>
        <div class="button-row">
          <button id="snapshotCreateBtn" class="primary-btn" type="button">생성</button>
          <button id="snapshotGetBtn" class="secondary-btn" type="button">조회</button>
        </div>
      </form>
      <div class="law-table-panel">
        <h3>스냅샷 결과</h3>
        <pre id="snapshotResult" class="json-result"></pre>
      </div>
    </div>
  `);
  const run = async (method) => {
    const tenantCode = el("snapshotTenantCode").value.trim();
    const byId = Number(el("snapshotById").value || 0);
    if (!tenantCode || !byId) {
      el("snapshotResult").textContent = "테넌트 코드와 사업연도 ID가 필요합니다. 데모 실행 후 자동 입력할 수 있습니다.";
      return;
    }
    const snapshot = await request(`/api/tenants/${tenantCode}/business-years/${byId}/snapshot`, {
      method,
      body: method === "POST" ? "{}" : undefined,
    });
    el("snapshotResult").textContent = JSON.stringify(snapshot, null, 2);
    log(`스냅샷 ${method === "POST" ? "생성" : "조회"} 완료`, {
      tenantCode,
      byId,
      snapshot_id: snapshot.snapshot_id,
    });
  };
  el("snapshotCreateBtn").addEventListener("click", () => run("POST").catch((error) => log("스냅샷 생성 실패", { message: error.message })));
  el("snapshotGetBtn").addEventListener("click", () => run("GET").catch((error) => log("스냅샷 조회 실패", { message: error.message })));
}

function renderImpactResult(impact) {
  el("impactResult").innerHTML = `
    <div class="law-summary-grid compact-grid">
      <article><span>대상 사업연도</span><strong>${impact.summary.business_years}</strong></article>
      <article><span>잠금 스냅샷</span><strong>${impact.summary.locked_snapshots}</strong></article>
      <article><span>세율 행</span><strong>${impact.summary.rate_rows}</strong></article>
      <article><span>한도 행</span><strong>${impact.summary.limit_rows}</strong></article>
    </div>
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>테넌트</th><th>스키마</th><th>대상 사업연도</th><th>잠금</th></tr>
        </thead>
        <tbody>
          ${(impact.tenant_impacts || [])
            .map(
              (tenant) => `
                <tr>
                  <td>${escapeHtml(tenant.tenant_code)}</td>
                  <td>${escapeHtml(tenant.schema_name)}</td>
                  <td>${tenant.business_years}</td>
                  <td>${tenant.locked_snapshots}</td>
                </tr>
              `,
            )
            .join("")}
        </tbody>
      </table>
    </div>
  `;
}

async function renderImpactScreen() {
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="impactForm" class="law-form">
        <h3>영향 시뮬레이션 실행</h3>
        ${renderLawSelector()}
        <label class="checkbox-row">
          <input id="impactIncludeLocked" type="checkbox" />
          잠금 스냅샷 포함
        </label>
        <button class="primary-btn" type="submit">시뮬레이션</button>
      </form>
      <div class="law-table-panel">
        <h3>시뮬레이션 결과</h3>
        <div id="impactResult"></div>
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("impactForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const impact = await request("/api/law-versioning/impact", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(el("lawVersionSelect").value),
        include_locked: el("impactIncludeLocked").checked,
      }),
    });
    renderImpactResult(impact);
    log("영향 시뮬레이션 완료", impact.summary);
  });
}

function renderHistoryTable(histories) {
  const rows = histories.length
    ? histories
        .map(
          (history) => `
            <tr>
              <td>${history.amendment_id}</td>
              <td>${escapeHtml(history.change_summary)}</td>
              <td>${escapeHtml(history.approved_by)}</td>
              <td>${formatDateTime(history.approved_at)}</td>
            </tr>
          `,
        )
        .join("")
    : `<tr><td colspan="4">등록된 개정 이력이 없습니다.</td></tr>`;
  return `
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>ID</th><th>요약</th><th>승인자</th><th>승인일시</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
  `;
}

async function renderHistoryScreen() {
  const law = selectedLawVersion();
  const histories = law ? await request(`/api/law-amendments?law_version_id=${law.law_version_id}`) : [];
  setLawScreenHtml(`
    <div class="law-layout">
      <form id="historyForm" class="law-form">
        <h3>개정 이력 등록</h3>
        ${renderLawSelector()}
        <label>개정 요약<input id="historySummary" value="법령 개정 공지 등록" /></label>
        <label>승인자<input id="historyApprovedBy" value="${escapeHtml(state.user?.login_id || "admin")}" /></label>
        <button class="primary-btn" type="submit">이력 저장</button>
      </form>
      <div class="law-table-panel">
        <h3>개정 이력</h3>
        ${renderHistoryTable(histories)}
      </div>
    </div>
  `);
  attachLawVersionSelect();
  el("historyForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const history = await request("/api/law-amendments", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(el("lawVersionSelect").value),
        change_summary: el("historySummary").value.trim(),
        approved_by: el("historyApprovedBy").value.trim(),
      }),
    });
    await refreshLawData();
    log("개정 이력 저장 완료", history);
  });
}

el("loginForm").addEventListener("submit", submitLogin);
el("logoutBtn").addEventListener("click", logout);
el("runDemoBtn").addEventListener("click", createDemoCase);
el("refreshBtn").addEventListener("click", () => {
  refreshDashboard().catch((error) => log("새로고침 실패", { message: error.message }));
});
el("adminRefreshBtn").addEventListener("click", () => {
  renderAdminScreen().catch((error) => log("시스템 관리 새로고침 실패", { message: error.message }));
});
el("customerRefreshBtn").addEventListener("click", () => {
  renderCustomerScreen().catch((error) => log("고객사/사업연도 새로고침 실패", { message: error.message }));
});
el("taxDataRefreshBtn").addEventListener("click", () => {
  renderTaxDataScreen().catch((error) => log("세무정보 입력 새로고침 실패", { message: error.message }));
});
el("adjustmentRefreshBtn").addEventListener("click", () => {
  renderAdjustmentScreen().catch((error) => log("세무조정 새로고침 실패", { message: error.message }));
});
el("formRefreshBtn").addEventListener("click", () => {
  renderFormScreen().catch((error) => log("서식 버전 관리 새로고침 실패", { message: error.message }));
});
document.querySelectorAll(".law-screen-refresh").forEach((button) => {
  button.addEventListener("click", () => {
  refreshLawData().catch((error) => log("법령·세율 새로고침 실패", { message: error.message }));
  });
});
el("clearLogBtn").addEventListener("click", () => {
  el("logOutput").textContent = "";
});

restoreSession();
