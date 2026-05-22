import { asArray, escapeHtml, money } from "/app/api.js";
import { fieldLabel, statusLabel, t } from "/app/i18n.js";

export function renderDataGrid({
  id,
  title,
  subtitle = "",
  rows = [],
  columns = [],
  importable = false,
  exportable = true,
  runLabel = "",
  runLabelKey = "",
  locale = "ko",
}) {
  const safeId = escapeHtml(id);
  const actionLabel = runLabelKey ? t(locale, runLabelKey) : runLabel;
  return `
    <article class="panel data-grid" data-grid="${safeId}">
      <div class="panel-head">
        <div>
          <h2>${escapeHtml(title)}</h2>
          ${subtitle ? `<p class="eyebrow">${escapeHtml(subtitle)}</p>` : ""}
        </div>
        <div class="button-row">
          ${actionLabel ? `<button class="primary-btn compact" type="button" data-grid-run="${safeId}">${escapeHtml(actionLabel)}</button>` : ""}
          ${exportable ? `<button class="secondary-btn compact" type="button" data-grid-export="${safeId}">${escapeHtml(t(locale, "grid.export"))}</button>` : ""}
          ${importable ? `<button class="secondary-btn compact" type="button" data-grid-import="${safeId}">${escapeHtml(t(locale, "grid.importJson"))}</button>` : ""}
        </div>
      </div>
      ${importable ? `<textarea class="grid-paste" data-grid-paste="${safeId}" placeholder="${escapeHtml(t(locale, "grid.pasteJson"))}"></textarea>` : ""}
      ${gridTable(columns, rows, locale)}
    </article>`;
}

export function bindDataGridActions({ grids, onRun, onImport }) {
  document.querySelectorAll("[data-grid-run]").forEach((button) => {
    button.addEventListener("click", async () => {
      await onRun?.(button.dataset.gridRun);
    });
  });
  document.querySelectorAll("[data-grid-export]").forEach((button) => {
    button.addEventListener("click", () => {
      const grid = grids[button.dataset.gridExport] || {};
      exportJson(`${button.dataset.gridExport}.json`, grid.rows || []);
    });
  });
  document.querySelectorAll("[data-grid-import]").forEach((button) => {
    button.addEventListener("click", async () => {
      const gridId = button.dataset.gridImport;
      const input = document.querySelector(`[data-grid-paste="${CSS.escape(gridId)}"]`);
      const value = input?.value.trim();
      if (!value) return;
      await onImport?.(gridId, JSON.parse(value));
    });
  });
}

function gridTable(columns, rows, locale) {
  const body = asArray(rows)
    .map((item) => `
      <tr>
        ${columns.map((column) => `<td>${formatCell(item, column, locale)}</td>`).join("")}
      </tr>`)
    .join("");
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${columns.map((column) => `<th>${escapeHtml(columnLabel(column, locale))}</th>`).join("")}</tr></thead>
        <tbody>${body || `<tr><td colspan="${columns.length}">${escapeHtml(t(locale, "grid.empty"))}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function columnLabel(column, locale) {
  if (column.labelKey) return t(locale, column.labelKey);
  if (column.labels?.[locale]) return column.labels[locale];
  if (column.key) return fieldLabel(column.key, locale);
  return column.label || "";
}

function formatCell(item, column, locale) {
  const value = column.value ? column.value(item) : item?.[column.key];
  if (column.format === "money") return money.format(Number(value || 0));
  if (column.format === "json") return escapeHtml(JSON.stringify(value ?? null));
  if (column.format === "status") return escapeHtml(statusLabel(value, locale));
  return escapeHtml(value ?? "-");
}

function exportJson(fileName, rows) {
  const blob = new Blob([JSON.stringify(rows, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName || "download";
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
