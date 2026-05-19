import { asArray, escapeHtml, money } from "/app/api.js";

export function renderDataGrid({
  id,
  title,
  subtitle = "",
  rows = [],
  columns = [],
  importable = false,
  exportable = true,
  runLabel = "",
}) {
  const safeId = escapeHtml(id);
  return `
    <article class="panel data-grid" data-grid="${safeId}">
      <div class="panel-head">
        <div>
          <h2>${escapeHtml(title)}</h2>
          ${subtitle ? `<p class="eyebrow">${escapeHtml(subtitle)}</p>` : ""}
        </div>
        <div class="button-row">
          ${runLabel ? `<button class="primary-btn compact" type="button" data-grid-run="${safeId}">${escapeHtml(runLabel)}</button>` : ""}
          ${exportable ? `<button class="secondary-btn compact" type="button" data-grid-export="${safeId}">Export</button>` : ""}
          ${importable ? `<button class="secondary-btn compact" type="button" data-grid-import="${safeId}">Import JSON</button>` : ""}
        </div>
      </div>
      ${importable ? `<textarea class="grid-paste" data-grid-paste="${safeId}" placeholder="Paste endpoint JSON body"></textarea>` : ""}
      ${gridTable(columns, rows)}
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

function gridTable(columns, rows) {
  const body = asArray(rows)
    .map((item) => `
      <tr>
        ${columns.map((column) => `<td>${formatCell(item, column)}</td>`).join("")}
      </tr>`)
    .join("");
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${columns.map((column) => `<th>${escapeHtml(column.label)}</th>`).join("")}</tr></thead>
        <tbody>${body || `<tr><td colspan="${columns.length}">No rows</td></tr>`}</tbody>
      </table>
    </div>`;
}

function formatCell(item, column) {
  const value = column.value ? column.value(item) : item?.[column.key];
  if (column.format === "money") return money.format(Number(value || 0));
  if (column.format === "json") return escapeHtml(JSON.stringify(value ?? null));
  return escapeHtml(value ?? "-");
}

function exportJson(fileName, rows) {
  const blob = new Blob([JSON.stringify(rows, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
