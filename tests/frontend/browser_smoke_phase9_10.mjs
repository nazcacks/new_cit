import { spawn } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";

const baseUrl = process.env.CIT_BASE_URL || "http://localhost:8080";
const edgePath = process.env.EDGE_PATH || "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const port = Number(process.env.CDP_PORT || 9231);
const profileDir = process.env.EDGE_PROFILE || join(process.cwd(), ".tmp", `cit-edge-phase9-10-${Date.now()}`);
const outputDir = process.env.CAPTURE_DIR || "docs/phase9_10";

const shots = [
  { name: "dashboard_overview", hash: "#/dashboard/overview", selector: '[data-dashboard="overview"]' },
  { name: "work_start", hash: "#/workspace/ws/start/customer-pick", selector: '[data-work-start-stage="customer-pick"]' },
  { name: "tax_data_fs", hash: "#/workspace/ws/info/fs", selector: '[data-tax-data-stage="financial-statements"]' },
  { name: "std_fs_mapping", hash: "#/workspace/ws/info/mapping", selector: '[data-std-fs-workbench]' },
  { name: "tax_data_consistency", hash: "#/workspace/ws/info/consistency", selector: '[data-tax-data-stage="consistency"]' },
  { name: "workflow_validation", hash: "#/workspace/ws/val/run", selector: '[data-validation-stage="run"]' },
  { name: "efiling_precheck", hash: "#/workspace/ws/file/precheck", selector: '[data-efile-stage="precheck"]' },
  { name: "workflow_post_amend", hash: "#/post/amend/unlock", selector: '[data-amend-stage="unlock"]' },
  { name: "admin_security", hash: "#/admin/sec/roles", selector: '[data-admin-stage="security-roles"]' },
  { name: "admin_law", hash: "#/admin/law/master", selector: '[data-admin-stage="law-master"]' },
  { name: "admin_forms", hash: "#/admin/form/master", selector: '[data-admin-stage="form-master"]' },
  { name: "admin_audit", hash: "#/admin/audit/events", selector: '[data-admin-stage="audit-events"]' },
];

class CdpClient {
  constructor(url) {
    this.url = url;
    this.nextId = 1;
    this.pending = new Map();
    this.waiters = new Map();
    this.errors = [];
    this.ws = new WebSocket(url);
    this.ws.addEventListener("message", (event) => this.onMessage(event));
  }

  async open() {
    if (this.ws.readyState === WebSocket.OPEN) return;
    await new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", reject, { once: true });
    });
  }

  onMessage(event) {
    const message = JSON.parse(event.data);
    if (message.id && this.pending.has(message.id)) {
      const { resolve, reject } = this.pending.get(message.id);
      this.pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result || {});
      return;
    }
    if (message.method && this.waiters.has(message.method)) {
      const waiters = this.waiters.get(message.method);
      this.waiters.delete(message.method);
      waiters.forEach((resolve) => resolve(message.params || {}));
    }
    if (message.method === "Runtime.exceptionThrown") {
      const details = message.params?.exceptionDetails;
      this.errors.push({
        method: message.method,
        text: details?.exception?.description || details?.text || "runtime exception",
      });
    }
    if (message.method === "Runtime.consoleAPICalled" && message.params?.type === "error") {
      this.errors.push({
        method: message.method,
        text: (message.params.args || []).map((arg) => arg.value || arg.description || arg.type).join(" "),
      });
    }
    if (message.method === "Log.entryAdded" && message.params?.entry?.level === "error") {
      this.errors.push({
        method: message.method,
        text: message.params.entry.text,
      });
    }
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  waitFor(method, timeoutMs = 15000) {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error(`Timed out waiting for ${method}`)), timeoutMs);
      const wrapped = (params) => {
        clearTimeout(timeout);
        resolve(params);
      };
      const waiters = this.waiters.get(method) || [];
      waiters.push(wrapped);
      this.waiters.set(method, waiters);
    });
  }

  close() {
    this.ws.close();
  }

  drainErrors() {
    const errors = this.errors;
    this.errors = [];
    return errors;
  }
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForJson(url, timeoutMs = 15000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return response.json();
    } catch {}
    await delay(250);
  }
  throw new Error(`Unable to connect to ${url}`);
}

async function waitForLoad(client) {
  try {
    await client.waitFor("Page.loadEventFired", 12000);
  } catch {
    await delay(1500);
  }
}

async function waitForSelector(client, selector, timeoutMs = 20000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const result = await client.send("Runtime.evaluate", {
      expression: `Boolean(document.querySelector(${JSON.stringify(selector)})) && !document.querySelector('#loginView:not(.hidden)')`,
      returnByValue: true,
    });
    if (result.result?.value) return;
    await delay(400);
  }
  throw new Error(`Timed out waiting for ${selector}`);
}

async function describeRoute(client) {
  const result = await client.send("Runtime.evaluate", {
    expression: `({
      hash: location.hash,
      title: document.getElementById("routeTitle")?.textContent || null,
      group: document.getElementById("routeGroup")?.textContent || null,
      appVisible: Boolean(document.querySelector("#appView") && !document.querySelector("#appView").classList.contains("hidden")),
      loginVisible: Boolean(document.querySelector("#loginView") && !document.querySelector("#loginView").classList.contains("hidden")),
      outletText: document.getElementById("cwk-route-outlet")?.innerText?.slice(0, 1200) || null,
      outletHtml: document.getElementById("cwk-route-outlet")?.innerHTML?.slice(0, 1200) || null,
      hasEfiling: Boolean(document.querySelector('[data-stage="efiling"]')),
      hasPostAmend: Boolean(document.querySelector('[data-stage="post-amend"]')),
    })`,
    returnByValue: true,
  });
  return result.result?.value || null;
}

async function waitForApp(client, timeoutMs = 20000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const result = await client.send("Runtime.evaluate", {
      expression: `Boolean(document.querySelector('#appView') && !document.querySelector('#appView').classList.contains('hidden'))`,
      returnByValue: true,
    });
    if (result.result?.value) return;
    await delay(400);
  }
  throw new Error("Timed out waiting for app shell");
}

async function loginAndContext() {
  const auth = await fetch(`${baseUrl}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tenant_code: "demo", login_id: "admin", password: "ChangeMe123!" }),
  }).then((response) => response.json());
  const headers = { Authorization: `Bearer ${auth.token}` };
  const [customers, years] = await Promise.all([
    fetch(`${baseUrl}/api/tenants/demo/customers`, { headers }).then((response) => response.json()),
    fetch(`${baseUrl}/api/tenants/demo/business-years`, { headers }).then((response) => response.json()),
  ]);
  const customer = customers[0];
  const by = years.find((item) => item.status === "IN_REVIEW") || years.find((item) => item.status === "APPROVED") || years[0];
  return {
    token: auth.token,
    context: {
      customerId: by.customer_id,
      customerName: customer.customer_name,
      byId: by.by_id,
      fy: String(by.year_label),
      status: by.status,
      progress: 92,
      lockMode: by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN"),
      selectedFormCode: "FORM3",
      selectedPrintFormCode: "FORM3",
    },
  };
}

async function main() {
  await mkdir(outputDir, { recursive: true });
  await rm(profileDir, { recursive: true, force: true });
  await mkdir(profileDir, { recursive: true });

  const edge = spawn(edgePath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profileDir}`,
    "about:blank",
  ], { stdio: "ignore" });

  try {
    const { token, context } = await loginAndContext();
    await waitForJson(`http://127.0.0.1:${port}/json/version`);
    const targets = await waitForJson(`http://127.0.0.1:${port}/json/list`);
    const page = targets.find((target) => target.type === "page");
    if (!page?.webSocketDebuggerUrl) throw new Error("page target not available");

    const client = new CdpClient(page.webSocketDebuggerUrl);
    await client.open();
    await client.send("Page.enable");
    await client.send("Runtime.enable");
    await client.send("Log.enable");
    await client.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 1200, deviceScaleFactor: 1, mobile: false });

    const manifest = [];
    for (const shot of shots) {
      client.drainErrors();
      const url = `${baseUrl}/?cit_smoke_token=${encodeURIComponent(token)}&cit_smoke_context=${encodeURIComponent(JSON.stringify(context))}${shot.hash}`;
      await client.send("Page.navigate", { url });
      await waitForLoad(client);
      await waitForApp(client);
      try {
        await waitForSelector(client, shot.selector);
      } catch (error) {
        const diagnostic = await describeRoute(client);
        console.error(JSON.stringify({ shot: shot.name, selector: shot.selector, diagnostic }, null, 2));
        throw error;
      }
      const errors = client.drainErrors();
      if (errors.length) {
        const diagnostic = await describeRoute(client);
        console.error(JSON.stringify({ shot: shot.name, errors, diagnostic }, null, 2));
        throw new Error(`Browser console/runtime errors on ${shot.name}`);
      }
      const screenshot = await client.send("Page.captureScreenshot", {
        format: "png",
        captureBeyondViewport: true,
        fromSurface: true,
      });
      const file = join(outputDir, `${shot.name}.png`);
      await writeFile(file, Buffer.from(screenshot.data, "base64"));
      manifest.push({ name: shot.name, selector: shot.selector, file });
      console.log(`${shot.name}\t${file}`);
    }
    await writeFile(join(outputDir, "manifest.json"), JSON.stringify(manifest, null, 2));
    client.close();
  } finally {
    edge.kill();
    await delay(500);
    await rm(profileDir, { recursive: true, force: true }).catch(() => {});
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
