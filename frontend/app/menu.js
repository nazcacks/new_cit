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

export function renderMenu(container, tree, context, activeKey, navigate, locale = "ko") {
  const roots = Array.isArray(tree?.children) ? tree.children : [];
  container.innerHTML = roots
    .map((root, rootIndex) => renderNode(root, rootIndex + 1, context, activeKey, 0, locale))
    .join("");

  container.querySelectorAll("[data-menu-key]").forEach((link) => {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      navigate(link.dataset.menuKey);
    });
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

function renderNode(node, index, context, activeKey, depth, locale) {
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
      </div>
      <div class="submenu depth-${depth + 1}">
        ${children.map((child, childIndex) => renderNode(child, childIndex + 1, context, activeKey, depth + 1, locale)).join("")}
      </div>
    </section>
  `;
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

function stepProgress(index) {
  return [0, 20, 45, 60, 70, 85, 92, 100][index] || 0;
}
