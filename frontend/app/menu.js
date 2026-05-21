import { escapeHtml } from "/app/api.js";
import { hasWorkContext } from "/app/context.js";
import { labelForNode, t } from "/app/i18n.js";

const workspaceSteps = [
  "ws-start",
  "ws-info",
  "ws-adj",
  "ws-form",
  "ws-val",
  "ws-appr",
  "ws-print",
  "ws-file",
];

export function renderMenu(container, tree, context, activeKey, navigate, locale = "ko", auth = null) {
  const roots = Array.isArray(tree?.children) ? tree.children : [];
  container.innerHTML = roots
    .map((root, rootIndex) => renderNode(root, rootIndex + 1, context, activeKey, 0, locale, auth))
    .join("");

  container.querySelectorAll("[data-menu-key]").forEach((link) => {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      navigate(link.dataset.menuKey);
    });
  });
}

export function renderTenantSwitcher(container, auth, onSwitch, locale = "ko") {
  const tenants = Array.isArray(auth?.accessible_tenants) ? auth.accessible_tenants : [];
  const current = tenants.find((tenant) => tenant.current) || {
    tenant_code: auth?.user?.tenant_code,
    tenant_name: auth?.user?.tenant_name,
    role: auth?.user?.roles?.[0] || "USER",
  };
  const canSwitch = tenants.length >= 2 || auth?.user?.roles?.includes("SUPER_ADMIN");
  if (!container) return;
  if (!canSwitch) {
    container.innerHTML = `<span class="tenant-label">${escapeHtml(current.tenant_name || "-")} · ${escapeHtml(current.tenant_code || "-")}</span>`;
    return;
  }
  container.innerHTML = `
    <label class="tenant-switch-label">
      <span>${t(locale, "tenant.working")}</span>
      <select id="tenantSwitchSelect" aria-label="${escapeHtml(t(locale, "tenant.switch"))}">
        ${tenants.map((tenant) => `
          <option value="${escapeHtml(tenant.tenant_code)}" ${tenant.current ? "selected" : ""}>
            ${escapeHtml(tenant.tenant_name)} · ${escapeHtml(tenant.tenant_code)} · ${escapeHtml(tenant.role)}
          </option>`).join("")}
      </select>
    </label>`;
  container.querySelector("#tenantSwitchSelect")?.addEventListener("change", (event) => {
    onSwitch(event.target.value);
  });
}

export function renderContextBadge(container, context, locale = "ko") {
  const ready = hasWorkContext(context);
  container.innerHTML = ready
    ? `
      <strong>${escapeHtml(context.customerName || "-")}</strong>
      <span>${escapeHtml(context.fy || "-")} / ${escapeHtml(context.status || "DRAFT")}</span>
      <div class="bar-track"><span style="width:${Number(context.progress || 0)}%"></span></div>
      <span>${context.lockMode === "LOCKED" ? t(locale, "context.locked") : t(locale, "context.editable")}</span>
    `
    : `<strong>${t(locale, "context.none")}</strong><span>${t(locale, "context.select")}</span>`;
}

export function renderStateBadge(container, context, locale = "ko") {
  if (!container) return;
  const status = context?.status || "NO_CONTEXT";
  const locked = context?.lockMode === "LOCKED" || context?.locked === true;
  container.innerHTML = `
    <span class="state-pill ${escapeHtml(status.toLowerCase().replaceAll("_", "-"))}">${escapeHtml(status)}</span>
    <span class="lock-badge ${locked ? "locked" : "open"}" title="${escapeHtml(locked ? t(locale, "context.locked") : t(locale, "context.editable"))}">${locked ? "LOCKED" : "OPEN"}</span>`;
}

export function renderStepper(container, context, activeKey, locale = "ko") {
  const ready = hasWorkContext(context);
  const activeStep = stepKeyFor(activeKey);
  container.innerHTML = workspaceSteps
    .map((key, index) => {
      const done = ready && Number(context.progress || 0) >= stepProgress(index);
      const locked = !ready && key !== "ws-start";
      const label = t(locale, `step.${key}`);
      return `<a class="step ${activeStep === key ? "active" : ""} ${done ? "done" : ""} ${locked ? "disabled" : ""}" href="#/${key}">${escapeHtml(label)}</a>`;
    })
    .join("");
}

export function stepKeyFor(key) {
  if (key.startsWith("ws/start:")) return "ws-start";
  if (key.startsWith("ws/info:")) return "ws-info";
  if (key.startsWith("ws/adj:")) return "ws-adj";
  if (key.startsWith("ws/form:")) return "ws-form";
  if (key.startsWith("ws/val:")) return "ws-val";
  if (key.startsWith("ws/appr:")) return "ws-appr";
  if (key.startsWith("ws/print:")) return "ws-print";
  if (key.startsWith("ws/file:")) return "ws-file";
  return key;
}

function renderNode(node, index, context, activeKey, depth, locale, auth) {
  if (!canShowNode(node, auth)) return "";
  const children = Array.isArray(node.children) ? node.children : [];
  if (!children.length) {
    return menuAnchor(node, indexLabel(node, index), context, activeKey, depth, locale);
  }
  const active = nodeActive(node, activeKey);
  return `
    <section class="menu-root depth-${depth}">
      <div class="menu-parent ${active ? "active" : ""}" data-depth="${depth}">
        <span class="menu-index">${escapeHtml(indexLabel(node, index))}</span>
        <span>${escapeHtml(labelForNode(node, locale))}</span>
        <span class="menu-progress-dot ${active ? "active" : ""}" style="--menu-progress:${groupProgress(node, context, active)}%;" aria-hidden="true"></span>
      </div>
      <div class="submenu depth-${depth + 1}">
        ${children.map((child, childIndex) => renderNode(child, childIndex + 1, context, activeKey, depth + 1, locale, auth)).join("")}
      </div>
    </section>
  `;
}

function canShowNode(node, auth) {
  if (node.code === "admin/tenant" || node.code === "admin/tenant:list") {
    return auth?.user?.roles?.some((role) => role === "SUPER_ADMIN" || role === "TENANT_ADMIN");
  }
  return true;
}

function menuAnchor(node, index, context, activeKey, depth, locale) {
  const requiresContext = Array.isArray(node.requires_context) && node.requires_context.length > 0;
  const needsContext = requiresContext && !hasWorkContext(context);
  const active = activeKey === node.code || activeKey === node.key;
  return `
    <a class="menu-link ${active ? "active" : ""} ${needsContext ? "needs-context" : ""}" href="${escapeHtml(node.path || `#/${node.code}`)}" data-menu-key="${escapeHtml(node.code)}" data-depth="${depth}">
      <span class="menu-index">${escapeHtml(index)}</span>
      <span>${escapeHtml(labelForNode(node, locale))}</span>
    </a>
  `;
}

function nodeActive(node, activeKey) {
  if (activeKey === node.code || activeKey === node.key) return true;
  const children = Array.isArray(node.children) ? node.children : [];
  return children.some((child) => nodeActive(child, activeKey));
}

function indexLabel(node, index) {
  if (node.code?.startsWith("ws/adj:")) return node.code.split(":")[1];
  if (node.code?.startsWith("admin/")) return node.code.split(":").pop().slice(0, 3).toUpperCase();
  return String(index);
}

function groupProgress(node, context, active) {
  if (!active) return 0;
  if (node.code === "workspace" || node.code?.startsWith("ws-") || node.code?.startsWith("ws/")) {
    return Math.max(0, Math.min(100, Number(context?.progress || 0)));
  }
  return 100;
}

function stepProgress(index) {
  return [0, 20, 45, 60, 70, 85, 92, 100][index] || 0;
}
