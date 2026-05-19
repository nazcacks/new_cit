let routeHandler = () => {};

export function currentKey() {
  return parseHash(window.location.hash).key;
}

export function currentRoute() {
  return parseHash(window.location.hash);
}

export function navigate(key, params = {}) {
  const query = new URLSearchParams(params);
  const suffix = query.toString() ? `?${query}` : "";
  const target = `${keyToHash(key)}${suffix}`;
  if (window.location.hash === target) {
    routeHandler(currentKey());
  } else {
    window.location.hash = target;
  }
}

export function onRouteChange(handler) {
  routeHandler = handler;
  window.addEventListener("hashchange", () => routeHandler(currentKey()));
}

export function keyToHash(key) {
  if (key.includes(":")) {
    const [scope, suffix] = key.split(":");
    if (scope === "dashboard") return `#/dashboard/${suffix}`;
    if (scope.startsWith("ws/")) return `#/workspace/${scope}/${suffix}`;
    if (scope.startsWith("post/")) return `#/post/${scope.slice("post/".length)}/${suffix}`;
    if (scope === "report") return `#/report/${suffix}`;
    if (scope.startsWith("admin/")) return `#/admin/${scope.slice("admin/".length)}/${suffix}`;
  }
  if (key === "post/correction") return "#/post/correction";
  return `#/${key}`;
}

function parseHash(hash) {
  const raw = hash.replace(/^#\/?/, "");
  const [path, query = ""] = raw.split("?");
  const parts = path.split("/").filter(Boolean);
  return {
    path,
    query,
    params: Object.fromEntries(new URLSearchParams(query)),
    key: normalizeParts(parts) || "dashboard:overview",
  };
}

function normalizeParts(parts) {
  if (!parts.length) return "";
  if (parts[0] === "dashboard") return `dashboard:${parts[1] || "overview"}`;
  if (parts[0] === "workspace" && parts[1] === "ws" && parts.length >= 4) {
    return `ws/${parts[2]}:${parts.slice(3).join("/")}`;
  }
  if (parts[0] === "post") {
    if (parts.length === 2 && parts[1] === "correction") return "post/correction";
    if (parts.length >= 3) return `post/${parts[1]}:${parts.slice(2).join("/")}`;
  }
  if (parts[0] === "report") return `report:${parts[1] || "year-compare"}`;
  if (parts[0] === "admin" && parts.length >= 3) {
    return `admin/${parts[1]}:${parts.slice(2).join("/")}`;
  }
  return parts.join("/");
}
