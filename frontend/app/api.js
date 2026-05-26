let tokenGetter = () => "";
let unauthorizedHandler = () => {};

export function setTokenGetter(getter) {
  tokenGetter = getter;
}

export function setUnauthorizedHandler(handler) {
  unauthorizedHandler = handler;
}

function shouldBypassHttpCache(path) {
  return typeof path === "string" && (
    path.includes("/dashboard") ||
    path.includes("/workflow/queue") ||
    path.includes("/notifications")
  );
}

export async function request(path, options = {}) {
  const bypassCache = shouldBypassHttpCache(path);
  const headers = {
    ...(options.body instanceof FormData ? {} : { "Content-Type": "application/json" }),
    ...(bypassCache ? { "Cache-Control": "no-cache" } : {}),
    ...(tokenGetter() ? { Authorization: `Bearer ${tokenGetter()}` } : {}),
    ...(options.headers || {}),
  };
  const fetchOptions = { ...options, headers };
  if (bypassCache) fetchOptions.cache = "no-store";
  const response = await fetch(path, fetchOptions);
  if (response.status === 401 && !options.skipUnauthorized) {
    unauthorizedHandler();
  }
  const text = await response.text();
  const body = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(body?.error?.message || response.statusText);
  }
  return body;
}

export async function downloadBinary(path, fileName) {
  const response = await fetch(path, {
    headers: tokenGetter() ? { Authorization: `Bearer ${tokenGetter()}` } : {},
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || response.statusText);
  }
  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName || "download";
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

export const money = new Intl.NumberFormat("ko-KR");

export function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

export function statusClass(status) {
  return String(status || "").toLowerCase().replaceAll("_", "-");
}

export function today() {
  return new Date().toISOString().slice(0, 10);
}

export function asArray(value) {
  return Array.isArray(value) ? value : [];
}
