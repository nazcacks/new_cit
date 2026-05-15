const state = {
  token: localStorage.getItem("cit.auth.token") || "",
  user: null,
  moduleTree: null,
  tenantCode: "",
  byId: null,
  efileJobId: "",
  activeLawPath: "/modules/law-versioning/laws",
  lawVersions: [],
  selectedLawVersionId: null,
  lawSummary: null,
};

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
      }
    });
  });
  highlightLawMenu();
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

function highlightLawMenu() {
  document.querySelectorAll("#moduleMenu .submenu-link, #moduleMenu .menu-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.path === state.activeLawPath);
  });
  setActiveLawPanel();
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
document.querySelectorAll(".law-screen-refresh").forEach((button) => {
  button.addEventListener("click", () => {
  refreshLawData().catch((error) => log("법령·세율 새로고침 실패", { message: error.message }));
  });
});
el("clearLogBtn").addEventListener("click", () => {
  el("logOutput").textContent = "";
});

restoreSession();
