import { spawn } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";

const baseUrl = process.env.CIT_BASE_URL || "http://localhost:8080";
const edgePath = process.env.EDGE_PATH || "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const port = Number(process.env.CDP_PORT || 9223);
const profileDir = process.env.EDGE_PROFILE || join(process.cwd(), ".tmp", `cit-edge-typology-${Date.now()}`);
const outputDir = process.env.CAPTURE_DIR || "docs/typology";

const shots = [
  ["grid", "admin/cust:list", "#/admin/cust/list", "typology_grid.png"],
  ["grid-tree", "admin/sec:menus", "#/admin/sec/menus", "typology_grid_tree.png"],
  ["dashboard", "dashboard:overview", "#/dashboard/overview", "typology_dashboard.png"],
  ["wizard", "ws/file:precheck", "#/workspace/ws/file/precheck", "typology_wizard.png"],
  ["form", "post/amend:unlock", "#/post/amend/unlock", "typology_form.png"],
  ["chart", "report:tax-burden", "#/report/tax-burden", "typology_chart.png"],
  ["detail", "ws/start:snapshot", "#/workspace/ws/start/snapshot", "typology_detail.png"],
];

class CdpClient {
  constructor(url) {
    this.url = url;
    this.nextId = 1;
    this.pending = new Map();
    this.waiters = new Map();
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
  }

  send(method, params = {}) {
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(payload);
    });
  }

  waitFor(method, timeoutMs = 12000) {
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
}

async function waitForJson(url, timeoutMs = 15000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return response.json();
    } catch {
      // retry until Edge opens the debugging socket
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Unable to connect to ${url}`);
}

async function waitForLoad(client) {
  try {
    await client.waitFor("Page.loadEventFired", 12000);
  } catch {
    await new Promise((resolve) => setTimeout(resolve, 1200));
  }
}

async function waitForTypology(client, typology) {
  const started = Date.now();
  while (Date.now() - started < 15000) {
    const result = await client.send("Runtime.evaluate", {
      expression: `Boolean(document.querySelector('[data-typology="${typology}"]')) && !document.querySelector('#loginView:not(.hidden)')`,
      returnByValue: true,
    });
    if (result.result?.value) return;
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  throw new Error(`Timed out waiting for ${typology}`);
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
    const login = await fetch(`${baseUrl}/api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tenant_code: "demo", login_id: "admin", password: "ChangeMe123!" }),
    }).then((response) => response.json());
    const headers = { Authorization: `Bearer ${login.token}` };
    const [customers, years] = await Promise.all([
      fetch(`${baseUrl}/api/tenants/demo/customers`, { headers }).then((response) => response.json()),
      fetch(`${baseUrl}/api/tenants/demo/business-years`, { headers }).then((response) => response.json()),
    ]);
    const customer = customers[0];
    const by = years.find((item) => item.customer_id === customer.customer_id) || years[0];
    const snapshot = await fetch(`${baseUrl}/api/tenants/demo/business-years/${by.by_id}/snapshot`, { headers }).then((response) => response.json());
    const context = {
      customerId: by.customer_id,
      customerName: customer.customer_name,
      byId: by.by_id,
      fy: String(by.year_label),
      period: `${by.start_date} ~ ${by.end_date}`,
      status: by.status,
      progress: 20,
      snapshot,
      lockMode: by.lock_mode || "OPEN",
    };

    await waitForJson(`http://127.0.0.1:${port}/json/version`);
    const page = await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(`${baseUrl}/`)}`, { method: "PUT" }).then((response) => response.json());
    const client = new CdpClient(page.webSocketDebuggerUrl);
    await client.open();
    await client.send("Page.enable");
    await client.send("Runtime.enable");
    await client.send("Emulation.setDeviceMetricsOverride", { width: 1440, height: 1000, deviceScaleFactor: 1, mobile: false });
    await client.send("Page.navigate", { url: `${baseUrl}/` });
    await waitForLoad(client);
    await client.send("Runtime.evaluate", {
      expression: `
        localStorage.setItem("cit.auth.token", ${JSON.stringify(login.token)});
        localStorage.setItem("cit.work.context", ${JSON.stringify(JSON.stringify(context))});
      `,
      awaitPromise: true,
    });
    await client.send("Page.navigate", { url: `${baseUrl}/?capture=${Date.now()}#/dashboard/overview` });
    await waitForLoad(client);
    await waitForTypology(client, "dashboard");

    for (const [typology, key, hash, fileName] of shots) {
      await client.send("Page.navigate", { url: `${baseUrl}/${hash}` });
      await waitForLoad(client);
      await waitForTypology(client, typology);
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: true, fromSurface: true });
      await writeFile(join(outputDir, fileName), Buffer.from(screenshot.data, "base64"));
      console.log(`${typology}\t${key}\t${join(outputDir, fileName)}`);
    }

    client.close();
  } finally {
    edge.kill();
    await rm(profileDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
