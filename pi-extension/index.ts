import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { matchesKey, truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import { Type } from "typebox";
import {
  readFileSync,
  writeFileSync,
  existsSync,
  lstatSync,
  realpathSync,
  mkdirSync,
  readdirSync,
  rmSync,
  renameSync,
} from "node:fs";
import { resolve, basename, sep, relative, isAbsolute } from "node:path";
import { homedir } from "node:os";
import AdmZip from "adm-zip";
import { lookup } from "mime-types";
import { gunzipSync, inflateSync, inflateRawSync, brotliDecompressSync } from "node:zlib";
import { nextPollDelay, parseRetryAfterSeconds } from "./polling.ts";

const CONFIG_DIR = resolve(homedir(), ".pi/agent/openaaas");
const CONFIG_PATH = resolve(CONFIG_DIR, "config.json");
const MAX_DOWNLOAD_SIZE = 100 * 1024 * 1024;
const MAX_UPLOAD_SIZE = 100 * 1024 * 1024;

interface ServerConfig {
  server_url: string;
  api_key?: string;
  client_id?: string;
  name?: string;
}

interface AppConfig {
  servers: Record<string, ServerConfig>;
  default_server: string;
}

function stripTrailingSlash(url: string): string {
  try {
    const parsed = new URL(url);
    if (parsed.pathname === "/" && url.endsWith("/")) {
      return url.slice(0, -1);
    }
    return url.replace(/\/+$/, "");
  } catch {
    const stripped = url.replace(/\/+$/, "");
    if (/^https?:$/i.test(stripped)) {
      return url;
    }
    return stripped;
  }
}

function sanitizeFilename(filename: string, fallbackExt: string): string {
  let safe = basename(filename);
  if (!safe || safe === "." || safe === ".." || safe === "/" || safe === "\\") {
    safe = `result.${fallbackExt}`;
  }
  return safe;
}

function ensureConfigDir() {
  if (!existsSync(CONFIG_DIR)) {
    mkdirSync(CONFIG_DIR, { recursive: true });
  }
}

function loadConfig(): AppConfig {
  ensureConfigDir();
  const defaultConfig: AppConfig = {
    servers: { default: { server_url: "https://api.open-aaas.com" } },
    default_server: "default",
  };
  if (!existsSync(CONFIG_PATH)) {
    return defaultConfig;
  }
  let raw: string;
  try {
    raw = readFileSync(CONFIG_PATH, "utf8");
  } catch (e) {
    const code = e && typeof e === "object" ? (e as NodeJS.ErrnoException).code : "";
    if (code === "EACCES") {
      throw new Error("无法读取配置文件（权限不足）");
    }
    if (code === "ENOENT") {
      return defaultConfig;
    }
    console.error("[OpenAaaS] 警告: 无法读取配置文件，使用默认配置");
    return defaultConfig;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    console.error("[OpenAaaS] 警告: 配置文件 JSON 格式错误，使用默认配置");
    return defaultConfig;
  }
  if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
    return parsed as AppConfig;
  }
  console.error("[OpenAaaS] 警告: 配置文件格式错误，使用默认配置");
  return defaultConfig;
}

function saveConfig(config: AppConfig) {
  ensureConfigDir();
  const tmpPath = CONFIG_PATH + ".tmp";
  try {
    writeFileSync(tmpPath, JSON.stringify(config, null, 2), "utf8");
    renameSync(tmpPath, CONFIG_PATH);
  } catch (e) {
    try { rmSync(tmpPath, { force: true }); } catch {}
    const code = e && typeof e === "object" ? (e as NodeJS.ErrnoException).code : "";
    if (code === "ENOSPC") {
      throw new Error("无法保存配置文件: 磁盘空间不足");
    }
    throw new Error(`无法保存配置文件: ${e instanceof Error ? e.message : String(e)}`);
  }
}

function getServerConfig(server?: string): ServerConfig & { alias: string } {
  const config = loadConfig();
  const alias = server || config.default_server || "default";
  const sc = config.servers?.[alias];
  if (!sc) {
    const available = Object.keys(config.servers || {}).join(", ");
    throw new Error(`服务器别名 "${alias}" 不存在。可用服务器: ${available || "无"}`);
  }
  return { ...sc, alias };
}

function requireApiKey(server?: string): string {
  const sc = getServerConfig(server);
  if (!sc.api_key) {
    throw new Error(`服务器 "${sc.alias}" 缺少 API Key，请先运行 register 进行注册`);
  }
  return sc.api_key;
}

function listServerConfigs(): Record<string, ServerConfig> {
  const config = loadConfig();
  return config.servers || {};
}

function saveServerConfig(alias: string, serverConfig: ServerConfig) {
  const config = loadConfig();
  if (!config.servers) config.servers = {};
  config.servers[alias] = serverConfig;
  saveConfig(config);
}

function removeServerConfig(alias: string) {
  const config = loadConfig();
  if (!config.servers?.[alias]) {
    throw new Error(`服务器别名 "${alias}" 不存在`);
  }
  if (config.default_server === alias) {
    throw new Error(`不能删除默认服务器 "${alias}"，请先使用 set_default_server 切换默认服务器`);
  }
  delete config.servers[alias];
  saveConfig(config);
}

function getDefaultServer(): string {
  const config = loadConfig();
  return config.default_server || "default";
}

function setDefaultServer(alias: string) {
  const config = loadConfig();
  if (!config.servers?.[alias]) {
    throw new Error(`服务器别名 "${alias}" 不存在`);
  }
  config.default_server = alias;
  saveConfig(config);
}

function combineSignals(signals: AbortSignal[]): AbortSignal {
  return AbortSignal.any(signals);
}

async function maybeDecompress(response: Response): Promise<Response> {
  const encoding = response.headers.get("content-encoding")?.toLowerCase() || "";
  const contentType = response.headers.get("content-type")?.toLowerCase() || "";
  const isJson = contentType.includes("application/json");

  let body: Buffer;
  try {
    body = Buffer.from(await response.arrayBuffer());
  } catch {
    return response;
  }
  if (body.length === 0) {
    return response;
  }

  // Fast-path: skip leading whitespace and check if body already looks like plain JSON.
  let idx = 0;
  while (idx < body.length && (body[idx] === 0x20 || body[idx] === 0x09 || body[idx] === 0x0a || body[idx] === 0x0d)) {
    idx++;
  }
  if (isJson && (body[idx] === 0x7b || body[idx] === 0x5b)) {
    const headers = new Headers(response.headers);
    headers.delete("content-encoding");
    headers.set("content-length", String(body.length));
    return new Response(body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    });
  }

  let decompressed: Buffer | undefined;

  if (encoding.includes("gzip") || (body[0] === 0x1f && body[1] === 0x8b)) {
    try {
      decompressed = gunzipSync(body);
    } catch {
      // not actually gzip
    }
  } else if (encoding.includes("deflate")) {
    try {
      decompressed = inflateSync(body);
    } catch {
      // not actually zlib-wrapped deflate; try raw deflate
      try {
        decompressed = inflateRawSync(body);
      } catch {
        // not deflate either
      }
    }
  } else if (encoding.includes("br")) {
    try {
      decompressed = brotliDecompressSync(body);
    } catch {
      // not actually brotli
    }
  }

  // Fallback: if decompression failed but it's supposed to be JSON, try raw body
  if (!decompressed && isJson) {
    try {
      JSON.parse(body.toString("utf8"));
      decompressed = body; // already valid JSON, use as-is
    } catch {
      // not JSON
    }
  }

  const finalBody = decompressed ?? body;

  const headers = new Headers(response.headers);
  headers.delete("content-encoding");
  headers.set("content-length", String(finalBody.length));

  return new Response(finalBody, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

async function safeFetch(
  url: string,
  init?: RequestInit,
  timeoutMs = 30000,
  maxRedirects = 3,
): Promise<Response> {
  const timeoutSignal = AbortSignal.timeout(timeoutMs);
  const userSignal = init?.signal;

  let signal: AbortSignal;
  if (userSignal) {
    signal = combineSignals([timeoutSignal, userSignal]);
  } else {
    signal = timeoutSignal;
  }

  let currentUrl = url;
  let remainingRedirects = maxRedirects;
  const originalHost = new URL(url).host;

  while (remainingRedirects >= 0) {
    try {
      // Build headers: strip Authorization on cross-domain redirect
      const headers: Record<string, string> = {};
      const initHeaders = init?.headers;
      if (initHeaders) {
        if (initHeaders instanceof Headers) {
          initHeaders.forEach((v, k) => { headers[k] = v; });
        } else if (Array.isArray(initHeaders)) {
          initHeaders.forEach(([k, v]) => { headers[k] = v; });
        } else {
          Object.assign(headers, initHeaders);
        }
      }
      const currentHost = new URL(currentUrl).host;
      if (currentHost !== originalHost) {
        delete headers["Authorization"];
      }

      const response = await fetch(currentUrl, {
        ...init,
        signal,
        redirect: "manual",
        headers,
      });

      if (response.status >= 300 && response.status < 400) {
        const location = response.headers.get("Location");
        if (location) {
          if (remainingRedirects <= 0) {
            throw new Error("请求被多次重定向，请直接使用 HTTPS URL");
          }
          currentUrl = new URL(location, currentUrl).toString();
          remainingRedirects--;
          continue;
        }
      }
      return await maybeDecompress(response);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (err instanceof TypeError) {
        if (msg.includes("Invalid URL")) {
          throw new Error("服务端地址格式错误: 请检查 server_url 配置");
        }
        throw new Error("连接失败: 无法连接到服务端，请检查 server_url 是否正确");
      }
      const NETWORK_ERRORS = ["fetch failed", "ECONNREFUSED", "ENOTFOUND", "ETIMEDOUT", "ECONNRESET", "EPIPE", "EHOSTUNREACH"];
      if (NETWORK_ERRORS.some(kw => msg.includes(kw))) {
        throw new Error("连接失败: 无法连接到服务端，请检查 server_url 是否正确");
      }
      throw err;
    }
  }

}

function stringifyValue(val: unknown): string {
  if (val === null || val === undefined) return "未知错误";
  if (typeof val === "string") return val;
  if (typeof val === "number" || typeof val === "boolean") return String(val);
  try {
    return JSON.stringify(val);
  } catch {
    return String(val);
  }
}

async function readErrorBody(response: Response): Promise<string> {
  const text = await response.text().catch(() => "");
  try {
    const data = JSON.parse(text);
    if (data && typeof data === "object") {
      if ("error" in data) return stringifyValue((data as Record<string, unknown>).error);
      if ("message" in data) return stringifyValue((data as Record<string, unknown>).message);
    }
    return text || response.statusText;
  } catch {
    return text || response.statusText;
  }
}

async function downloadSingleFile(
  serverUrl: string,
  apiKey: string,
  fileId: string,
  safeName: string,
  extractDir: string
): Promise<{ filePath: string; fileSize: number }> {
  const downloadUrl = `${serverUrl}/api/v1/client/files/${encodeURIComponent(fileId)}/download`;
  const downloadResponse = await safeFetch(downloadUrl, {
    method: "GET",
    headers: { Authorization: `Bearer ${apiKey}` },
  }, 60000);

  if (!downloadResponse.ok) {
    const msg = await readErrorBody(downloadResponse);
    if (downloadResponse.status === 401) {
      throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
    }
    if (downloadResponse.status === 403) {
      throw new Error("权限不足 (403): 无法下载该文件");
    }
    throw new Error(`下载文件失败 (HTTP ${downloadResponse.status}): ${msg}`);
  }

  const contentLength = downloadResponse.headers.get("Content-Length");
  if (contentLength) {
    const len = Number(contentLength);
    if (!isNaN(len) && len > MAX_DOWNLOAD_SIZE) {
      throw new Error(`下载文件过大: ${len} bytes，超过 ${MAX_DOWNLOAD_SIZE} bytes 限制`);
    }
  }
  const contentBuffer = Buffer.from(await downloadResponse.arrayBuffer());
  if (contentBuffer.length > MAX_DOWNLOAD_SIZE) {
    throw new Error(`下载文件过大: ${contentBuffer.length} bytes，超过 ${MAX_DOWNLOAD_SIZE} bytes 限制`);
  }

  const filePath = resolve(extractDir, safeName);
  try {
    writeFileSync(filePath, contentBuffer);
  } catch (e) {
    const code = e && typeof e === "object" ? (e as NodeJS.ErrnoException).code : "";
    if (code === "ENOSPC") {
      throw new Error("保存文件失败: 磁盘空间不足");
    }
    throw new Error(`保存文件失败: ${e instanceof Error ? e.message : String(e)}`);
  }

  return { filePath, fileSize: contentBuffer.length };
}

function ensureUtcTimestamp(ts: string): string {
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?$/.test(ts)) {
    return ts + "Z";
  }
  return ts;
}

function formatDuration(
  startedAt: string | undefined | null,
  completedAt: string | undefined | null,
  status: string
): string {
  if (!startedAt) return "";
  const start = new Date(ensureUtcTimestamp(startedAt));
  if (isNaN(start.getTime())) return "";

  const end =
    ["running", "cancelling"].includes(status)
      ? new Date()
      : completedAt
      ? new Date(ensureUtcTimestamp(completedAt))
      : null;
  if (!end || isNaN(end.getTime())) return "";

  const totalSeconds = Math.floor((end.getTime() - start.getTime()) / 1000);
  if (totalSeconds < 0) return "";

  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) return `${hours}小时${minutes}分钟${seconds}秒`;
  if (minutes > 0) return `${minutes}分钟${seconds}秒`;
  return `${seconds}秒`;
}

function formatDurationShort(
  startedAt: string | undefined | null,
  completedAt: string | undefined | null,
  status: string
): string {
  if (!startedAt) return "";
  const start = new Date(ensureUtcTimestamp(startedAt));
  if (isNaN(start.getTime())) return "";

  const end =
    ["running", "cancelling"].includes(status)
      ? new Date()
      : completedAt
      ? new Date(ensureUtcTimestamp(completedAt))
      : null;
  if (!end || isNaN(end.getTime())) return "";

  const totalSeconds = Math.floor((end.getTime() - start.getTime()) / 1000);
  if (totalSeconds < 0) return "";

  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) return `${hours}h${minutes}m${seconds}s`;
  if (minutes > 0) return `${minutes}m${seconds}s`;
  return `${seconds}s`;
}

interface MonitoredTask {
  taskId: string;
  serviceId?: string;
  serviceName?: string;
  taskPrompt?: string;
  status: string;
  server: string;
  createdAt: string;
  startedAt?: string;
  completedAt?: string;
  lastCheckedAt: number;
  intervalId?: ReturnType<typeof setTimeout>;
  pollingStopped?: boolean;
  consecutiveFailures?: number;
  currentGeneration?: number;
  lastError?: string;
}

const activeTasks = new Map<string, MonitoredTask>();
const MAX_RETAINED_TASKS = 20;

interface ToastInfo {
  taskId: string;
  titleText: string;
  icon: string;
  titleColor: "success" | "error";
  serviceName: string;
  promptText: string;
  duration: string;
  timeoutId: ReturnType<typeof setTimeout>;
}

const MAX_TOASTS = 5; // 最多同时显示的通知数

const activeToasts = new Map<string, ToastInfo>();
let toastOverlayRequestRender: (() => void) | null = null;
let toastOverlayDone: (() => void) | null = null;

function removeToast(taskId: string): void {
  const toast = activeToasts.get(taskId);
  if (toast) {
    clearTimeout(toast.timeoutId);
    activeToasts.delete(taskId);
  }

  if (activeToasts.size === 0) {
    if (toastOverlayDone) {
      toastOverlayDone();
      toastOverlayDone = null;
      toastOverlayRequestRender = null;
    }
  } else {
    if (toastOverlayRequestRender) {
      toastOverlayRequestRender();
    }
  }
}

function createToastOverlay(ctx: ExtensionContext): void {
  ctx.ui.custom<void>(
    (tui, theme, _kb, done) => {
      toastOverlayDone = done;
      toastOverlayRequestRender = () => tui.requestRender();
      return {
        render(width: number): string[] {
          const th = theme;
          const innerW = width - 2;
          const pad = (s: string, len: number) => {
            const vis = visibleWidth(s);
            if (vis > len) {
              return truncateToWidth(s, len, "...", true);
            }
            return s + " ".repeat(Math.max(0, len - vis));
          };

          const allLines: string[] = [];
          let isFirst = true;

          for (const toast of activeToasts.values()) {
            if (!isFirst) {
              allLines.push("");
            }
            isFirst = false;

            const row = (content: string) => th.fg(toast.titleColor, "│") + pad(content, innerW) + th.fg(toast.titleColor, "│");
            allLines.push(th.fg(toast.titleColor, `╭${"─".repeat(innerW)}╮`));
            allLines.push(row(` ${th.fg(toast.titleColor, toast.icon)} ${th.fg("accent", toast.titleText)}`));
            allLines.push(row(""));
            allLines.push(row(` 服务: ${toast.serviceName}`));
            allLines.push(row(` 摘要: ${toast.promptText}`));
            allLines.push(row(` 时长: ${toast.duration}`));
            allLines.push(th.fg(toast.titleColor, `╰${"─".repeat(innerW)}╯`));
          }

          return allLines;
        },
        invalidate(): void {},
        dispose(): void {
          for (const toast of activeToasts.values()) {
            clearTimeout(toast.timeoutId);
          }
          activeToasts.clear();
          toastOverlayDone = null;
          toastOverlayRequestRender = null;
        },
      };
    },
    {
      overlay: true,
      overlayOptions: {
        anchor: "top-right",
        width: 45,
        margin: { top: 1, right: 2 },
        nonCapturing: true,
      },
    },
  ).catch((err) => {
    console.error("[OpenAaaS] toast overlay error:", err instanceof Error ? err.message : String(err));
  });
}

function addToast(ctx: ExtensionContext, task: MonitoredTask, newStatus: string, duration: string): void {
  // 去重
  removeToast(task.taskId);

  // 限制数量
  if (activeToasts.size >= MAX_TOASTS) {
    const firstKey = activeToasts.keys().next().value as string;
    removeToast(firstKey);
  }

  const titleText = newStatus === "completed" ? "OpenAaaS 任务已完成" : newStatus === "failed" ? "OpenAaaS 任务失败" : "OpenAaaS 任务已取消";
  const icon = newStatus === "completed" ? "✅" : newStatus === "failed" ? "❌" : "🚫";
  const titleColor = newStatus === "completed" ? "success" : "error";

  const toastInfo: ToastInfo = {
    taskId: task.taskId,
    titleText,
    icon,
    titleColor,
    serviceName: (task.serviceName || task.serviceId || "未知服务").replace(/\r?\n/g, " "),
    promptText: (task.taskPrompt || "无").replace(/\r?\n/g, " "),
    duration,
    timeoutId: setTimeout(() => removeToast(task.taskId), 30000),
  };

  activeToasts.set(task.taskId, toastInfo);

  if (toastOverlayRequestRender) {
    toastOverlayRequestRender();
  } else {
    createToastOverlay(ctx);
  }
}


export default function (pi: ExtensionAPI) {
  // ==================== 任务监控逻辑 ====================

  // getTaskStatus 查询结果分类：
  // - ok: 正常返回任务数据
  // - rate_limited: HTTP 429（可能携带 Retry-After 秒数），非永久错误
  // - server_error: HTTP 5xx，退避重试
  // - network_error: 网络异常，退避重试
  // - permanent: 401/404 等永久错误，停止轮询
  type TaskStatusQuery =
    | { kind: "ok"; data: Record<string, unknown> }
    | { kind: "rate_limited"; retryAfterSeconds: number | null }
    | { kind: "server_error" }
    | { kind: "network_error" }
    | { kind: "permanent"; message: string };

  const getTaskStatus = async (taskId: string, server: string): Promise<TaskStatusQuery> => {
    const { server_url: serverUrl, api_key: apiKey } = getServerConfig(server);
    if (!apiKey) {
      console.error(`[OpenAaaS] 服务器 "${server}" 缺少 API Key，无法查询任务状态`);
      return { kind: "permanent", message: `服务器 "${server}" 缺少 API Key` };
    }
    const url = `${serverUrl}/api/v1/client/tasks/${encodeURIComponent(taskId)}`;
    let response: Response;
    try {
      response = await safeFetch(url, {
        method: "GET",
        headers: { Authorization: `Bearer ${apiKey}` },
      });
    } catch (err) {
      console.error(`[OpenAaaS] 查询任务状态失败: ${err instanceof Error ? err.message : String(err)}`);
      return { kind: "network_error" };
    }
    if (response.ok) {
      return { kind: "ok", data: (await response.json()) as Record<string, unknown> };
    }
    const msg = await readErrorBody(response);
    if (response.status === 429) {
      console.error(`[OpenAaaS] 查询任务状态被限流 (HTTP 429): ${msg}`);
      return {
        kind: "rate_limited",
        retryAfterSeconds: parseRetryAfterSeconds(response.headers.get("Retry-After")),
      };
    }
    if (response.status === 401 || response.status === 404) {
      console.error(
        response.status === 401
          ? `[OpenAaaS] 认证失败 (401): 无法查询任务状态，请检查 api_key`
          : `[OpenAaaS] 查询任务状态失败 (HTTP 404): 任务不存在`
      );
      return { kind: "permanent", message: `HTTP ${response.status}` };
    }
    console.error(`[OpenAaaS] 查询任务状态失败 (HTTP ${response.status}): ${msg}`);
    if (response.status >= 500) {
      return { kind: "server_error" };
    }
    // 其他 4xx 同样视为永久错误，避免无限重试
    return { kind: "permanent", message: `HTTP ${response.status}: ${msg}` };
  };

  const updateTUI = (ctx: ExtensionContext) => {
    // 始终清空 footer
    ctx.ui.setStatus("OpenAaaS", undefined);

    // 单次遍历统计所有状态 + 收集 widget 需要显示的任务
    let pending = 0, accepted = 0, running = 0, completed = 0, failed = 0, cancelled = 0, cancelling = 0;
    const widgetTasks: MonitoredTask[] = [];
    for (const task of activeTasks.values()) {
      switch (task.status) {
        case "pending": pending++; break;
        case "accepted": accepted++; break;
        case "running": running++; break;
        case "completed": completed++; break;
        case "failed": failed++; break;
        case "cancelled": cancelled++; break;
        case "cancelling": cancelling++; break;
      }
      if (["pending", "accepted", "running", "cancelling"].includes(task.status)) {
        widgetTasks.push(task);
      }
    }

    const parts: string[] = [];
    if (pending > 0) parts.push(`⏳${pending} pending`);
    if (accepted > 0) parts.push(`📋${accepted} accepted`);
    if (running > 0) parts.push(`🟢${running} running`);
    if (cancelling > 0) parts.push(`🚫${cancelling} cancelling`);
    if (completed > 0) parts.push(`✅${completed} done`);
    if (failed > 0) parts.push(`❌${failed} failed`);
    if (cancelled > 0) parts.push(`🚫${cancelled} cancelled`);

    if (parts.length === 0) {
      ctx.ui.setWidget("OpenAaaS", undefined);
      return;
    }

    const widgetLines: string[] = [];
    widgetLines.push(`OpenAaaS: ${parts.join(" | ")}`);

    // 按状态优先级排序: running > accepted > pending > cancelling
    const statusPriority: Record<string, number> = {
      running: 0,
      accepted: 1,
      pending: 2,
      cancelling: 3,
    };
    widgetTasks.sort((a, b) => (statusPriority[a.status] ?? 99) - (statusPriority[b.status] ?? 99));

    const displayTasks = widgetTasks.slice(0, 5);
    for (const task of displayTasks) {
      const statusIcon: Record<string, string> = {
        pending: "⏳",
        accepted: "📋",
        running: "🟢",
        cancelling: "⏹️",
      };
      const icon = statusIcon[task.status] ?? "⚪";
      const nameBase = (task.serviceName || task.taskPrompt || "").replace(/\r?\n/g, " ").slice(0, 12) || task.taskId.slice(0, 8);
      const name = task.server ? `[${task.server}] ${nameBase}` : nameBase;
      let duration = "";
      if (task.status === "running" && task.startedAt) {
        duration = formatDurationShort(task.startedAt, null, "running");
      } else if (task.status === "cancelling" && task.startedAt) {
        duration = formatDurationShort(task.startedAt, null, "cancelling");
      }
      widgetLines.push(`${icon} ${name}${duration ? ` ${duration}` : ""}`);
    }
    if (widgetTasks.length > 5) {
      widgetLines.push(`... ${widgetTasks.length - 5} more`);
    }

    ctx.ui.setWidget("OpenAaaS", widgetLines, { placement: "belowEditor" });
  };

  const stopPolling = (taskId: string) => {
    const task = activeTasks.get(taskId);
    if (task?.intervalId) {
      clearTimeout(task.intervalId);
      task.intervalId = undefined;
    }
    if (task) {
      task.pollingStopped = true;
      // 递增 generation，使进行中的轮询响应被丢弃（取消竞态防护）
      task.currentGeneration = (task.currentGeneration ?? 0) + 1;
    }
  };

  const trimTerminalTasks = () => {
    const terminalEntries = Array.from(activeTasks.entries()).filter(([, t]) =>
      ["completed", "failed", "cancelled"].includes(t.status)
    );
    if (terminalEntries.length > MAX_RETAINED_TASKS) {
      terminalEntries.sort((a, b) => {
        const timeA = new Date(a[1].createdAt).getTime() || 0;
        const timeB = new Date(b[1].createdAt).getTime() || 0;
        return timeA - timeB;
      });
      const toRemove = terminalEntries.length - MAX_RETAINED_TASKS;
      for (let i = 0; i < toRemove; i++) {
        activeTasks.delete(terminalEntries[i][0]);
      }
    }
  };

  // 任务年龄基准：服务端 created_at（或本地 createdAt），会话重建任务沿用持久化值
  const getTaskAgeMs = (task: MonitoredTask): number => {
    const createdMs = new Date(ensureUtcTimestamp(task.createdAt)).getTime();
    if (isNaN(createdMs)) return 0;
    return Math.max(0, Date.now() - createdMs);
  };

  const startMonitoring = (task: MonitoredTask, ctx: ExtensionContext) => {
    // 处理一次成功的状态查询；返回 true 表示任务到达终态（已停止轮询）
    const handleStatusUpdate = (result: Record<string, unknown>): boolean => {
      const newStatus = (result.status as string) || "unknown";
      const prevStatus = task.status;

      if (newStatus !== prevStatus) {
        task.status = newStatus;
        if (result.started_at) task.startedAt = result.started_at as string;
        if (result.completed_at) task.completedAt = result.completed_at as string;

        if (newStatus === "completed" || newStatus === "failed" || newStatus === "cancelled") {
          const startMs = new Date(ensureUtcTimestamp(task.startedAt || task.createdAt)).getTime();
          const endMs = task.completedAt
            ? new Date(ensureUtcTimestamp(task.completedAt)).getTime()
            : NaN;
          const durationMs = !isNaN(startMs) && !isNaN(endMs) ? endMs - startMs : 0;
          const duration =
            durationMs > 0
              ? (() => {
                  const s = Math.round(durationMs / 1000);
                  if (s < 60) return `${s}秒`;
                  const m = Math.floor(s / 60);
                  const rs = s % 60;
                  if (m < 60) return `${m}分${rs}秒`;
                  const h = Math.floor(m / 60);
                  const rm = m % 60;
                  return `${h}时${rm}分`;
                })()
              : "未知";
          addToast(ctx, task, newStatus, duration);
        }

        // 持久化到 session
        pi.appendEntry("OpenAaaS-task", {
          task_id: task.taskId,
          service_id: task.serviceId,
          service_name: task.serviceName,
          task_prompt: task.taskPrompt,
          status: newStatus,
          server: task.server,
          created_at: task.createdAt,
          started_at: task.startedAt,
          completed_at: task.completedAt,
          updated_at: new Date().toISOString(),
        });

        // 终态停止轮询但保留在 Map 中
        if (["completed", "failed", "cancelled"].includes(newStatus)) {
          if (result.completed_at) task.completedAt = result.completed_at as string;
          stopPolling(task.taskId);

          trimTerminalTasks();

          updateTUI(ctx);
          return true;
        }
      }

      task.lastCheckedAt = Date.now();
      updateTUI(ctx);
      return false;
    };

    const runPoll = async () => {
      // generation 快照：响应返回时若已变化（任务被取消/到达终态），丢弃该响应
      const generation = task.currentGeneration ?? 0;

      let query: TaskStatusQuery;
      try {
        query = await getTaskStatus(task.taskId, task.server);
      } catch (err) {
        console.error("[OpenAaaS] poll error:", err instanceof Error ? err.message : String(err));
        query = { kind: "network_error" };
      }

      // 过期响应：不更新状态、不持久化、不调度下一轮
      if ((task.currentGeneration ?? 0) !== generation) return;

      if (query.kind === "ok") {
        task.lastError = undefined;
        if (handleStatusUpdate(query.data)) return; // 终态：已停止轮询
      }

      // 统一决策：延迟钳制（[MIN_RETRY_MS, MAX_RETRY_MS]）与退避规则由 nextPollDelay 保证
      const decision = nextPollDelay(
        query,
        getTaskAgeMs(task),
        task.consecutiveFailures ?? 0,
        query.kind === "rate_limited" ? query.retryAfterSeconds : null
      );
      task.consecutiveFailures = decision.consecutiveFailures;

      if (decision.stop) {
        // 永久错误（401/404 等）：停止轮询并记录错误，不无限重试
        task.lastError = query.kind === "permanent" ? query.message : "未知错误";
        stopPolling(task.taskId);
        // 持久化永久错误（status 保持原值），防止“假活”：任务看起来仍在监控实则已停止。
        // polling_stopped=true 标记轮询已永久停止，供 session 重建（reconstructTasks）识别，
        // 避免对非终态但已停止的任务重新发起轮询；last_error 供 /OpenAaaS-tasks 面板展示
        pi.appendEntry("OpenAaaS-task", {
          task_id: task.taskId,
          service_id: task.serviceId,
          service_name: task.serviceName,
          task_prompt: task.taskPrompt,
          status: task.status,
          server: task.server,
          created_at: task.createdAt,
          started_at: task.startedAt,
          completed_at: task.completedAt,
          updated_at: new Date().toISOString(),
          last_error: task.lastError,
          polling_stopped: true,
        });
        updateTUI(ctx);
        return;
      }

      // 检查任务是否仍在监控中且未停止轮询
      const currentTask = activeTasks.get(task.taskId);
      if (currentTask && !currentTask.pollingStopped) {
        task.intervalId = setTimeout(() => runPoll(), decision.delayMs);
      }
    };

    // 立即执行一次
    runPoll();
    activeTasks.set(task.taskId, task);
  };

  // 从 session entries 重建任务状态
  const reconstructTasks = (ctx: ExtensionContext) => {
    const entries = ctx.sessionManager.getEntries();
    // 反向遍历，确保每个任务使用最新的 entry
    for (let i = entries.length - 1; i >= 0; i--) {
      const entry = entries[i];
      if (entry.type !== "custom") continue;
      if (entry.customType !== "OpenAaaS-task") continue;

      const data = entry.data as Record<string, unknown>;
      const taskId = data.task_id as string;
      const status = (data.status as string) || "unknown";

      if (!taskId || activeTasks.has(taskId)) continue;

      const config = loadConfig();
      const server = (data.server as string) || config.default_server || "default";
      const isTerminal = ["completed", "failed", "cancelled"].includes(status);
      const task: MonitoredTask = {
        taskId,
        serviceId: data.service_id as string,
        serviceName: data.service_name as string,
        taskPrompt: data.task_prompt as string,
        status,
        server,
        createdAt: (data.created_at as string) || new Date().toISOString(),
        startedAt: data.started_at as string,
        completedAt: (data.completed_at as string) || (data.completedAt as string),
        lastCheckedAt: Date.now(),
        lastError: (data.last_error as string) || undefined,
      };

      // polling_stopped=true：上一轮 session 已因永久错误停止轮询（status 仍为非终态），
      // 恢复为“已停止”状态放入 Map（保留 lastError 供面板展示），不再 startMonitoring，
      // 否则每次 session 重建都会对已永久停止的任务重新发起轮询
      if (isTerminal || data.polling_stopped === true) {
        task.pollingStopped = true;
        activeTasks.set(taskId, task);
      } else {
        startMonitoring(task, ctx);
      }
    }

    trimTerminalTasks();

    updateTUI(ctx);
  };

  const getTaskHistory = (ctx: ExtensionContext): { total: number; tasks: Array<Record<string, unknown>>; text: string } => {
    const taskMap = new Map<string, Record<string, unknown>>();

    for (const entry of ctx.sessionManager.getEntries()) {
      if (entry.type !== "custom") continue;
      if (entry.customType !== "OpenAaaS-task") continue;

      const data = entry.data as Record<string, unknown>;
      const taskId = data.task_id as string;
      if (!taskId) continue;

      const existing = taskMap.get(taskId);
      if (!existing) {
        taskMap.set(taskId, data);
      } else {
        const existingTime = new Date((existing.updated_at as string) || 0).getTime();
        const newTime = new Date((data.updated_at as string) || 0).getTime();
        if (newTime > existingTime) {
          taskMap.set(taskId, data);
        }
      }
    }

    if (taskMap.size === 0) {
      return { total: 0, tasks: [], text: "当前 Session 没有 OpenAaaS 任务历史" };
    }

    const tasks = Array.from(taskMap.values()).map((t) => ({
      task_id: t.task_id,
      status: t.status || "unknown",
      server: t.server,
      service_id: t.service_id,
      service_name: t.service_name,
      task_prompt: ((t.task_prompt as string) || "").replace(/\r?\n/g, " ").slice(0, 50),
      created_at: t.created_at,
      updated_at: t.updated_at,
    }));

    const lines = [`当前 Session 共有 ${tasks.length} 个 OpenAaaS 任务：`];
    for (const task of tasks) {
      const statusIcon: Record<string, string> = {
        pending: "⏳",
        accepted: "📋",
        running: "🟢",
        completed: "✅",
        failed: "❌",
        cancelled: "🚫",
        cancelling: "⏹️",
      };
      const icon = statusIcon[task.status as string] ?? "⚪";
      const name = ((task.service_name as string) || (task.task_prompt as string) || "").replace(/\r?\n/g, " ");
      lines.push(`- ${icon} ${task.task_id} | ${task.status} | ${task.server}${name ? ` | ${name}` : ""}`);
    }

    return { total: tasks.length, tasks, text: lines.join("\n") };
  };

  // Session 事件监听
  pi.on("session_start", async (_event, ctx) => {
    reconstructTasks(ctx);
    const result = getTaskHistory(ctx);
    if (result.total > 0) {
      pi.sendMessage({
        customType: "OpenAaaS-rebuild",
        content: `[OpenAaaS] Session 重建完成\n${result.text}`,
        display: true,
        details: { task_count: result.total },
      }, { triggerTurn: false });
    }
  });

  pi.on("session_tree", async (_event, ctx) => {
    reconstructTasks(ctx);
    const result = getTaskHistory(ctx);
    if (result.total > 0) {
      pi.sendMessage({
        customType: "OpenAaaS-rebuild",
        content: `[OpenAaaS] Session 重建完成\n${result.text}`,
        display: true,
        details: { task_count: result.total },
      }, { triggerTurn: false });
    }
  });

  pi.on("session_shutdown", async () => {
    // stopPolling 会清理定时器并递增 generation，
    // 防止关机瞬间在途请求返回后回写已清空的状态
    for (const [taskId] of activeTasks) {
      stopPolling(taskId);
    }
    activeTasks.clear();

    // 清理 toast
    for (const toast of activeToasts.values()) {
      clearTimeout(toast.timeoutId);
    }
    activeToasts.clear();
    if (toastOverlayDone) {
      toastOverlayDone();
      toastOverlayDone = null;
      toastOverlayRequestRender = null;
    }
  });

  // 监听 tool_result，自动捕获 submit_task
  pi.on("tool_result", async (event, ctx) => {
    if (event.toolName !== "OpenAaaS") return;
    const input = event.input as Record<string, unknown>;
    if (input?.action !== "submit_task") return;

    const details = event.details as Record<string, unknown>;
    const taskId = (details.task_id as string) || (details.id as string);
    if (!taskId || activeTasks.has(taskId)) return;

    const server = (input.server as string) || getDefaultServer();
    const task: MonitoredTask = {
      taskId,
      serviceId: (input.service_id as string) || (details.service_id as string) || undefined,
      taskPrompt: (input.task_prompt as string) || undefined,
      status: (details.status as string) || "pending",
      server,
      createdAt: (details.created_at as string) || new Date().toISOString(),
      completedAt: undefined,
      lastCheckedAt: Date.now(),
    };

    // 获取服务名称（从 list_services 缓存或 API）
    // 简化：先不获取服务名称，后续轮询时如果有需要再补充

    startMonitoring(task, ctx);
    updateTUI(ctx);

    // 持久化
    pi.appendEntry("OpenAaaS-task", {
      task_id: task.taskId,
      service_id: task.serviceId,
      service_name: task.serviceName,
      task_prompt: task.taskPrompt,
      status: task.status,
      server: task.server,
      created_at: task.createdAt,
      started_at: task.startedAt,
      completed_at: task.completedAt,
      updated_at: new Date().toISOString(),
    });
  });

  // ==================== 命令注册 ====================
  pi.registerCommand("OpenAaaS-tasks", {
    description: "List all OpenAaaS tasks in the current session",
    handler: async (_args, ctx) => {
      if (!ctx.hasUI) {
        ctx.ui.notify("/OpenAaaS-tasks requires interactive mode", "error");
        return;
      }

      const tasks = Array.from(activeTasks.values());
      if (tasks.length === 0) {
        ctx.ui.notify("No OpenAaaS tasks in current session", "info");
        return;
      }

      await ctx.ui.custom<void>((_tui, _theme, _kb, done) => {
        const timeoutId = setTimeout(() => done(), 30000);
        return {
          render(_width: number): string[] {
            const lines: string[] = [];
            lines.push(" OpenAaaS Tasks ");
            lines.push("");

            const statusIcon: Record<string, string> = {
              pending: "⏳",
              accepted: "📋",
              running: "🟢",
              completed: "✅",
              failed: "❌",
              cancelled: "🚫",
              cancelling: "⏹️",
            };

            for (const task of tasks.slice(0, 20)) {
              const icon = statusIcon[task.status] ?? "⚪";
              const nameBase = (task.serviceName || task.taskPrompt || "").replace(/\r?\n/g, " ").slice(0, 30) || task.taskId.slice(0, 8);
              const name = task.server ? `[${task.server}] ${nameBase}` : nameBase;
              let duration = "";
              if (["running", "cancelling"].includes(task.status) && task.startedAt) {
                duration = formatDuration(task.startedAt, null, task.status);
              } else if (["completed", "failed", "cancelled"].includes(task.status) && task.startedAt) {
                duration = formatDuration(task.startedAt, task.completedAt || null, task.status);
              }
              lines.push(` ${icon} ${name}`);
              lines.push(`    Status: ${task.status}${duration ? ` | Duration: ${duration}` : ""}`);
              if (task.lastError) {
                lines.push(`    Last check failed: ${task.lastError.replace(/\r?\n/g, " ").slice(0, 60)}`);
              }
            }

            if (tasks.length > 20) {
              lines.push(` ... and ${tasks.length - 20} more`);
            }

            lines.push("");
            lines.push(" Press Escape to close ");
            return lines;
          },
          handleInput(data: string): void {
            if (matchesKey(data, "escape") || matchesKey(data, "ctrl+c")) {
              clearTimeout(timeoutId);
              done();
            }
          },
          invalidate(): void {},
          dispose(): void {
            clearTimeout(timeoutId);
          },
        };
      });
    },
  });

  // ==================== 原有工具注册逻辑 ====================

  // @ts-expect-error TS2589: Type instantiation is excessively deep and possibly infinite.
  pi.registerTool({
    name: "OpenAaaS",
    label: "OpenAaaS",
    description:
      "支持多服务器配置。所有 action 可通过 server 参数指定目标服务器别名，不传则使用 default_server。\n\n" +
      "OpenAaaS 统一入口工具，用于将任务提交给远程 Agent 异步执行。\n\n" +
      "任务提交后会自动进入后台执行，widget 会实时显示进度（pending → accepted → running → completed）。你（LLM）不需要主动轮询查询状态，widget 会自动监控并在任务完成时通过 UI 通知用户。\n\n" +
      "信息获取遵循渐进式披露原则：不要一次性获取所有服务的完整信息。先用 list_services 获取轻量列表（name + description + status），根据描述筛选出候选服务，再对目标服务调用 get_service_usage 获取详细用法说明（能力范围、调用规范、返回格式、限制条件）。usage 通常很长，只应在确定使用该服务时获取。\n\n" +
      "标准使用流程：\n" +
      "1. set_server_url — 设置服务端地址（如未设置，默认连接 localhost）\n" +
      "2. register — 注册获取 api_key（仅需一次）\n" +
      "3. list_services — 获取轻量服务列表（name/description/status），浏览并筛选候选服务\n" +
      "4. get_service_usage — 对筛选出的候选服务，按需获取详细 usage（能力范围、调用规范、返回格式、限制条件）\n" +
      "5. 根据 usage 内容，构造正确的 task_prompt 和 output_prompt\n" +
      "6. submit_task — 提交任务（可附带文件），保存返回的 task_id\n" +
      "7. list_history — 查看当前 Session 中所有任务历史（上下文压缩后可用来恢复记忆）\n" +
      "8. 等待用户告知任务完成，或用 get_task 查询最终结果（仅在用户明确要求时调用，不要主动轮询）\n" +
      "9. download_result — 任务完成后下载结果文件\n\n" +
      "重要：widget 实时显示的任务状态仅对用户可见，你无法直接看到。如果你需要回答用户关于某个任务当前状态的任何问题（例如\"任务现在是什么状态\"\"完成了吗\"），必须先调用 get_task 重新查询最新状态，不要引用之前调用返回的旧状态。\n\n" +
      "注意：如果当前服务器（default_server 或指定的 server）已有 api_key，说明已完成注册，请勿重复调用 register。\n\n" +
      "支持的 action：\n" +
      "- discover: 发现服务端 API 信息\n" +
      "- set_server_url: 设置服务器地址并保存到 config.json。已有注册信息（api_key）的服务器不会被覆盖，如需修改地址请先使用 remove_server 删除旧配置。\n" +
      "- register: 注册客户端账号，获取 api_key（仅需一次）。如果当前服务器已有 api_key，请勿重复注册。\n" +
      "- update_profile: 修改当前客户端用户名\n" +
      "- list_services: 列出可用的 Agent 服务（返回轻量摘要：name/description/status/agent_status/access_type/has_permission 等，不含 usage 长文本）\n" +
      "- get_service_usage: 获取指定服务的详细 usage（能力范围、调用规范、返回格式、限制条件）。这是渐进式披露的关键步骤：先 list_services 筛选，再对目标服务获取 usage\n" +
      "- list_history: 列出当前 Session 中所有 OpenAaaS 任务历史\n" +
      "- submit_task: 提交任务到远程 Agent\n" +
      "- get_task: 查询任务状态和最终结果（仅在用户要求时调用，不要主动轮询）\n" +
      "- cancel_task: 取消执行中的任务\n" +
      "- list_files: 列出任务的结果文件列表\n" +
      "- download_result: 下载任务结果文件（支持 file_id 单选或 download_all 全选）\n" +
      "- list_servers: 列出所有已配置的服务器\n" +
      "- set_default_server: 切换默认服务器\n" +
      "- remove_server: 删除指定服务器的配置（不能删除默认服务器）",
    parameters: Type.Object({
      action: Type.String({
        description: "操作类型",
        enum: [
          "discover",
          "set_server_url",
          "register",
          "update_profile",
          "list_services",
          "get_service_usage",
          "list_history",
          "submit_task",
          "get_task",
          "cancel_task",
          "list_files",
          "download_result",
          "list_servers",
          "set_default_server",
          "remove_server",
        ],
      }),
      server: Type.Optional(Type.String({ description: "服务器别名，默认使用 default_server" })),
      server_url: Type.Optional(Type.String({ description: "服务端地址" })),
      name: Type.Optional(Type.String({ description: "客户端名称/用户名" })),
      task_id: Type.Optional(Type.String({ description: "任务 ID" })),
      service_id: Type.Optional(Type.String({ description: "目标服务 ID" })),
      task_prompt: Type.Optional(Type.String({ description: "任务描述/提示词" })),
      output_prompt: Type.Optional(Type.String({ description: "输出格式要求" })),
      session_id: Type.Optional(Type.String({ description: "会话 ID，用于保持对话上下文" })),
      file_id: Type.Optional(Type.String({ description: "指定要下载的文件 ID（用于 download_result），不指定则默认下载第一个文件" })),
      download_all: Type.Optional(Type.Boolean({ description: "是否下载该任务的所有结果文件（用于 download_result），默认 false" })),
      input_files: Type.Optional(
        Type.Array(Type.String(), { description: "输入文件路径列表" })
      ),
    }),
    async execute(_toolCallId, params, signal, _onUpdate, ctx) {
      switch (params.action) {
        case "discover": {
          const serverUrl = stripTrailingSlash(params.server_url || getServerConfig(params.server).server_url);
          const url = `${serverUrl}/api/v1/discovery`;

          const response = await safeFetch(url, { method: "GET" });
          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
            }
            throw new Error(`请求失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const apiInfo = (result.api as Record<string, unknown>) || {};
          const version = (apiInfo.version as string) ?? "unknown";
          const baseUrl = (apiInfo.base_url as string) ?? params.server_url ?? getServerConfig(params.server).server_url;
          const auth = (() => {
            const formatAuth = (v: unknown): string | undefined => {
              if (typeof v === "string") return v;
              if (v && typeof v === "object" && !Array.isArray(v)) {
                const o = v as Record<string, unknown>;
                if (typeof o.type === "string") return o.type;
                if (typeof o.method === "string") return o.method;
                return JSON.stringify(v);
              }
              return undefined;
            };
            return formatAuth(result.authentication) ?? formatAuth(result.auth) ?? "Bearer Token (通过 register 获取 api_key)";
          })();

          let text = `成功获取服务端 API 信息\n服务端版本: ${version}\nBase URL: ${baseUrl}\n认证方式: ${auth}`;

          const endpoints = result.endpoints;
          if (endpoints && Array.isArray(endpoints)) {
            text += "\n\n可用端点:";
            for (const ep of endpoints) {
              if (ep && typeof ep === "object") {
                const e = ep as Record<string, unknown>;
                const epName = (e.name as string) ?? "unnamed";
                const epMethod = (e.method as string) ?? "?";
                const epPath = (e.path as string) ?? "";
                text += `\n  - ${epName}: [${epMethod}] ${epPath}`;
              }
            }
          }

          const services = result.services;
          if (services && Array.isArray(services)) {
            text += `\n\n已注册服务 (${services.length} 个):`;
            for (const svc of services) {
              if (svc && typeof svc === "object") {
                const s = svc as Record<string, unknown>;
                text += `\n  - ${String(s.name ?? s.id ?? "unnamed")}`;
              }
            }
          }

          return {
            content: [{ type: "text", text }],
            details: result,
          };
        }

        case "set_server_url": {
          const serverUrl = (params.server_url || "").trim();
          if (!serverUrl) {
            throw new Error("缺少必填参数: server_url");
          }
          let parsed: URL;
          try {
            parsed = new URL(serverUrl);
          } catch {
            throw new Error("参数错误: server_url 必须是有效的 URL，如 http://example.com 或 https://example.com");
          }
          if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
            throw new Error("参数错误: server_url 必须以 http:// 或 https:// 开头");
          }
          if (!parsed.hostname) {
            throw new Error("参数错误: server_url 必须包含有效的主机地址");
          }

          const config = loadConfig();
          const alias = params.server || config.default_server || "default";

          const existing = config.servers?.[alias];
          if (existing && existing.api_key) {
            const oldUrl = stripTrailingSlash(existing.server_url || "");
            const newUrl = stripTrailingSlash(serverUrl);
            if (oldUrl && oldUrl !== newUrl) {
              throw new Error(
                `服务器 "${alias}" 已有注册信息（地址: ${existing.server_url}，api_key 存在）。\n` +
                `直接修改会清除原有注册信息。\n\n` +
                `如需添加新服务器，请使用新的 server 别名，例如：\n` +
                `  server="new-alias" server_url="${serverUrl}"\n\n` +
                `如需修改该服务器地址，请先使用 remove_server 删除旧配置，或使用其他别名。`
              );
            }
          }

          const targetUrl = stripTrailingSlash(serverUrl);
          if (config.servers) {
            for (const [otherAlias, otherServer] of Object.entries(config.servers)) {
              if (otherAlias === alias) continue;
              const otherUrl = stripTrailingSlash(otherServer.server_url || "");
              if (otherUrl === targetUrl && typeof otherServer.api_key === "string" && otherServer.api_key) {
                throw new Error(
                  `该服务器地址已被其他别名注册过。\n` +
                  `服务器别名: ${otherAlias}\n` +
                  `服务器地址: ${otherServer.server_url}\n\n` +
                  `如需使用此别名，请先使用 remove_server 删除已有配置。`
                );
              }
            }
          }

          if (!config.servers) config.servers = {};
          if (!config.servers[alias]) config.servers[alias] = { server_url: "" };
          const oldUrl = stripTrailingSlash(config.servers[alias].server_url || "");
          config.servers[alias].server_url = targetUrl;

          let warning = "";
          if (oldUrl && oldUrl !== config.servers[alias].server_url && !config.servers[alias].api_key) {
            warning =
              `\n\n⚠️ 服务器 "${alias}" 地址已从 ${oldUrl} 更换为 ${config.servers[alias].server_url}。`;
          }

          saveConfig(config);

          return {
            content: [
              {
                type: "text",
                text: `服务器 "${alias}" 地址设置成功！已保存到 config.json: ${config.servers[alias].server_url}${warning}`,
              },
            ],
            details: {
              server_alias: alias,
              server_url: config.servers[alias].server_url,
              server_changed: !!warning,
              warning: warning || undefined,
            },
          };
        }

        case "register": {
          const name = (params.name || "").trim();
          if (!name) {
            throw new Error("缺少必填参数: name");
          }
          if (name.length > 64) {
            throw new Error("参数错误: name 长度不能超过64字符");
          }
          if (/[\x00-\x1f\/\\<>|&;$]/.test(name)) {
            throw new Error("参数错误: name 包含非法字符");
          }

          // 检查是否已注册
          const sc = getServerConfig(params.server);
          if (typeof sc.api_key === "string" && sc.api_key) {
            const clientId = sc.client_id || "unknown";
            const savedName = sc.name || "unknown";
            return {
              content: [
                {
                  type: "text",
                  text:
                    `已注册，无需重复注册。\n` +
                    `服务器别名: ${sc.alias}\n` +
                    `服务器地址: ${sc.server_url}\n` +
                    `客户端 ID: ${clientId}\n` +
                    `用户名: ${savedName}\n\n` +
                    `如需修改用户名，请使用 update_profile。\n` +
                    `如需切换到其他服务器，请先用 set_server_url 修改地址，再重新注册。`,
                },
              ],
              details: {
                already_registered: true,
                server_alias: sc.alias,
                server_url: sc.server_url,
                client_id: clientId,
                name: savedName,
              },
            };
          }

          // 检查是否有其他别名已注册到同一服务器地址
          const config = loadConfig();
          const currentUrl = stripTrailingSlash(sc.server_url || "");
          if (config.servers) {
            for (const [otherAlias, otherServer] of Object.entries(config.servers)) {
              if (otherAlias === sc.alias) continue;
              const otherUrl = stripTrailingSlash(otherServer.server_url || "");
              if (otherUrl === currentUrl && typeof otherServer.api_key === "string" && otherServer.api_key) {
                return {
                  content: [
                    {
                      type: "text",
                      text:
                        `该服务器地址已被其他别名注册过。\n` +
                        `服务器别名: ${otherAlias}\n` +
                        `服务器地址: ${otherServer.server_url}\n` +
                        `客户端 ID: ${otherServer.client_id || "unknown"}\n` +
                        `用户名: ${otherServer.name || "unknown"}\n\n` +
                        `如需使用其他别名，请先使用 remove_server 删除已有配置。`,
                    },
                  ],
                  details: {
                    error: "duplicate_server_url",
                    server_alias: otherAlias,
                    server_url: otherServer.server_url,
                    client_id: otherServer.client_id || "unknown",
                    name: otherServer.name || "unknown",
                  },
                };
              }
            }
          }

          const { server_url: serverUrl, alias } = getServerConfig(params.server);
          const url = `${serverUrl}/api/v1/client/auth/register`;

          const response = await safeFetch(url, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ name }),
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            throw new Error(`注册失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const apiKey = (result.api_key as string) || (result.token as string);
          const clientId =
            (result.client_id as string) || (result.id as string);

          if (typeof apiKey === "string" && apiKey) {
            const config = loadConfig();
            if (!config.servers) config.servers = {};
            if (!config.servers[alias]) config.servers[alias] = { server_url: serverUrl };
            config.servers[alias].api_key = apiKey;
            if (typeof clientId === "string" && clientId) {
              config.servers[alias].client_id = clientId;
            }
            config.servers[alias].name = name;
            saveConfig(config);
          }

          return {
            content: [
              {
                type: "text",
                text: `注册成功！服务器: ${alias}，客户端 ID: ${clientId}。API Key 已${
                  apiKey ? "自动保存" : "返回，请手动保存"
                }到 config.json`,
              },
            ],
            details: {
              server_alias: alias,
              client_id: clientId,
              saved_to_config: !!apiKey,
            },
          };
        }

        case "update_profile": {
          const name = (params.name || "").trim();
          if (!name) {
            throw new Error("参数错误: name 不能为空");
          }
          if (name.length > 64) {
            throw new Error("参数错误: name 长度不能超过64字符");
          }
          if (/[\x00-\x1f\/\\<>|&;$]/.test(name)) {
            throw new Error("参数错误: name 包含非法字符");
          }

          const { server_url: serverUrl, alias } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/profile`;

          const response = await safeFetch(url, {
            method: "PUT",
            headers: {
              "Content-Type": "application/json",
              Authorization: `Bearer ${apiKey}`,
            },
            body: JSON.stringify({ name }),
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error(
                "认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确"
              );
            } else if (response.status === 409) {
              throw new Error(
                "用户名已存在 (409): 该用户名已被其他用户使用，请选择其他名称"
              );
            }
            throw new Error(`更新失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const clientId =
            (result.client_id as string) || (result.id as string);
          const updatedName = (result.name as string) || name;

          const config = loadConfig();
          if (!config.servers) config.servers = {};
          if (!config.servers[alias]) config.servers[alias] = { server_url: serverUrl };
          config.servers[alias].name = updatedName;
          saveConfig(config);

          return {
            content: [
              {
                type: "text",
                text: `用户名更新成功！新用户名: ${updatedName}`,
              },
            ],
            details: {
              client_id: clientId,
              name: updatedName,
              saved_to_config: true,
            },
          };
        }

        case "list_services": {
          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/services`;

          const response = await safeFetch(url, {
            method: "GET",
            headers: { Authorization: `Bearer ${apiKey}` },
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error(
                "认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确"
              );
            }
            throw new Error(`请求失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as
            | Array<Record<string, unknown>>
            | { services?: Array<Record<string, unknown>> };
          const services = Array.isArray(result) ? result : result.services || [];
          const formatted = services.map((svc) => ({
            id: (svc.id as string) ?? "unknown",
            name: (svc.name as string) ?? "未命名",
            description: (svc.description as string) ?? "无描述",
            agent_status: (svc.agent_status as string) || "unknown",
            access_type: (svc.access_type as string) || "unknown",
            has_permission: svc.has_permission === true,
            registration_status: (svc.registration_status as string) ?? undefined,
          }));

          if (formatted.length === 0) {
            return {
              content: [{ type: "text", text: "暂无可用的 Agent 服务" }],
              details: { total: 0, services: [] },
            };
          }

          const statusIcon: Record<string, string> = {
            online: "🟢",
            offline: "🔴",
            unknown: "⚪",
          };

          const lines = [`找到 ${formatted.length} 个可用服务：`];
          for (let i = 0; i < formatted.length; i++) {
            const svc = formatted[i];
            const icon = statusIcon[svc.agent_status] ?? "⚪";
            const perm = svc.has_permission ? "✅ 有权限" : "❌ 无权限";
            lines.push(
              `${i + 1}. ${svc.name ?? "未命名"}`,
              `   ID: ${svc.id ?? "unknown"}`,
              `   状态: ${icon} ${svc.agent_status}`,
              `   访问类型: ${svc.access_type}`,
              `   权限: ${perm}`,
              `   描述: ${svc.description ?? "无描述"}`
            );
            if (svc.registration_status) {
              lines.push(`   注册状态: ${svc.registration_status}`);
            }
            if (i < formatted.length - 1) {
              lines.push("");
            }
          }

          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: {
              total: formatted.length,
              services: formatted,
            },
          };
        }

        case "get_service_usage": {
          const serviceId = params.service_id;
          if (!serviceId) throw new Error("缺少必填参数: service_id");

          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/services/${encodeURIComponent(serviceId)}/usage`;

          const response = await safeFetch(url, {
            method: "GET",
            headers: { Authorization: `Bearer ${apiKey}` },
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
            } else if (response.status === 403) {
              throw new Error("权限不足 (403): 您没有权限查看该服务的 usage");
            } else if (response.status === 404) {
              throw new Error("服务不存在 (404): 请检查 service_id 是否正确");
            }
            throw new Error(`请求失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const usage = (result.usage as string) || "无 usage 说明";
          const name = (result.name as string) || "未命名";

          return {
            content: [
              {
                type: "text",
                text: `服务: ${name}\n\nUsage:\n${usage}`,
              },
            ],
            details: result,
          };
        }

        case "list_history": {
          const result = getTaskHistory(ctx);
          return {
            content: [{ type: "text", text: result.text }],
            details: { total: result.total, tasks: result.tasks },
          };
        }

        case "submit_task": {
          const { service_id, task_prompt, output_prompt, session_id, input_files } =
            params;

          if (!service_id) throw new Error("缺少必填参数: service_id");
          if (!task_prompt) throw new Error("缺少必填参数: task_prompt");

          const { server_url: serverUrl, alias } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/tasks`;

          const formData = new FormData();
          formData.append("service_id", service_id);
          formData.append("task_prompt", task_prompt);
          formData.append("output_prompt", output_prompt || "");
          if (session_id) formData.append("session_id", session_id);

          if (input_files && Array.isArray(input_files)) {
            if (input_files.length > 10) {
              throw new Error("文件上传失败: 最多支持 10 个文件");
            }
            let cwdResolved: string;
            try {
              cwdResolved = realpathSync(resolve(ctx.cwd));
            } catch {
              cwdResolved = resolve(ctx.cwd);
            }
            for (const filePath of input_files) {
              const absolutePath = resolve(cwdResolved, filePath);
              let rawStats;
              try {
                rawStats = lstatSync(absolutePath);
              } catch (e) {
                throw new Error(`文件上传失败: 文件不存在: ${absolutePath}`);
              }
              if (rawStats.isSymbolicLink()) {
                throw new Error(`文件上传失败: 不支持符号链接: ${absolutePath}`);
              }

              let realPath: string;
              try {
                realPath = realpathSync(absolutePath);
              } catch {
                throw new Error(`文件上传失败: 无法解析文件路径: ${absolutePath}`);
              }
              const rel = relative(cwdResolved, realPath);
              if (rel === ".." || rel.startsWith(".." + sep) || isAbsolute(rel)) {
                throw new Error(`文件上传失败: 只能上传当前工作目录下的文件`);
              }

              const stats = lstatSync(realPath);
              if (!stats.isFile()) {
                throw new Error(`文件上传失败: 路径不是文件: ${absolutePath}`);
              }
              if (stats.size > MAX_UPLOAD_SIZE) {
                throw new Error(`文件上传失败: 文件过大: ${stats.size} bytes，超过 ${MAX_UPLOAD_SIZE} bytes 限制`);
              }
              let fileBuffer: Buffer;
              try {
                fileBuffer = readFileSync(realPath);
              } catch (e) {
                const code = e && typeof e === "object" ? (e as NodeJS.ErrnoException).code : "";
                if (code === "ENOENT") {
                  throw new Error(`文件上传失败: 文件不存在: ${realPath}`);
                }
                if (code === "EACCES") {
                  throw new Error(`文件上传失败: 没有权限读取文件: ${realPath}`);
                }
                throw new Error(`文件上传失败: 无法读取文件: ${e instanceof Error ? e.message : String(e)}`);
              }
              // 二次校验：防御 lstatSync 检查与 readFileSync 之间的 TOCTOU 竞态
              if (fileBuffer.length > MAX_UPLOAD_SIZE) {
                throw new Error(`文件上传失败: 文件过大: ${fileBuffer.length} bytes，超过 ${MAX_UPLOAD_SIZE} bytes 限制`);
              }
              const mimeType = lookup(realPath) || "application/octet-stream";
              const blob = new Blob([fileBuffer], { type: mimeType });
              formData.append("files", blob, basename(realPath));
            }
          }

          const response = await safeFetch(url, {
            method: "POST",
            headers: { Authorization: `Bearer ${apiKey}` },
            body: formData,
            signal,
          }, 60000);

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error(
                "认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确"
              );
            } else if (response.status === 404) {
              throw new Error(
                "服务不存在 (404): 请检查 service_id 是否正确，可通过 list_services 获取可用服务列表"
              );
            }
            throw new Error(`提交失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const taskId = (result.id as string) || (result.task_id as string);
          const status = (result.status as string) || "unknown";

          // 启动任务监控
          if (taskId && !activeTasks.has(taskId)) {
            const task: MonitoredTask = {
              taskId,
              serviceId: params.service_id,
              taskPrompt: params.task_prompt,
              status: status || "pending",
              server: alias || getDefaultServer(),
              createdAt: (result.created_at as string) || new Date().toISOString(),
              completedAt: undefined,
              lastCheckedAt: Date.now(),
            };
            startMonitoring(task, ctx);
            updateTUI(ctx);
            pi.appendEntry("OpenAaaS-task", {
              task_id: task.taskId,
              service_id: task.serviceId,
              service_name: task.serviceName,
              task_prompt: task.taskPrompt,
              status: task.status,
              server: task.server,
              created_at: task.createdAt,
              started_at: task.startedAt,
              completed_at: task.completedAt,
              updated_at: new Date().toISOString(),
            });
          }

          return {
            content: [
              {
                type: "text",
                text: `任务提交成功！任务 ID: ${taskId}，状态: ${status}\n\nwidget 正在自动监控进度，请勿轮询查询。等待用户告知任务完成后再获取结果。`,
              },
            ],
            details: {
              task_id: taskId,
              status,
              service_id,
              created_at: result.created_at,
              full_response: result,
            },
          };
        }

        case "get_task": {
          const taskId = params.task_id;
          if (!taskId) throw new Error("缺少必填参数: task_id");

          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/tasks/${encodeURIComponent(taskId)}`;

          const response = await safeFetch(url, {
            method: "GET",
            headers: { Authorization: `Bearer ${apiKey}` },
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error(
                "认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确"
              );
            } else if (response.status === 404) {
              throw new Error("任务不存在 (404): 请检查 task_id 是否正确");
            }
            throw new Error(`查询失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const status = (result.status as string) || "unknown";
          const statusDesc: Record<string, string> = {
            pending: "等待中",
            accepted: "已接受",
            running: "执行中",
            completed: "已完成",
            failed: "失败",
            cancelled: "已取消",
            cancelling: "取消中",
          };

          const startTime =
            (result.started_at as string) || (result.created_at as string);
          const durationStr = formatDuration(
            startTime,
            (result.completed_at as string) || null,
            status
          );

          let content = `任务状态: ${statusDesc[status] || status}\n任务 ID: ${
            (result.id as string) || (result.task_id as string)
          }`;

          if (status === "completed") {
            content +=
              "\n\n✅ 任务已完成！可以使用 download_result 工具下载结果文件。";
            const resultData = result.result;
            if (resultData && typeof resultData === "object") {
              content += "\n\n执行结果摘要:";
              const r = resultData as Record<string, unknown>;
              if (r.summary) {
                content += `\n${r.summary}`;
              } else if (typeof r.output === "string") {
                const out = r.output;
                content += `\n${out.slice(0, 500)}${out.length > 500 ? "..." : ""}`;
              }
            }
          } else if (status === "failed") {
            content += "\n\n❌ 任务执行失败";
            const resultData = result.result;
            if (resultData && typeof resultData === "object") {
              const r = resultData as Record<string, unknown>;
              const errMsg = (r.error as string) || (r.message as string);
              if (errMsg) {
                content += `\n错误信息: ${errMsg}`;
              }
            }
          } else if (["pending", "accepted", "running"].includes(status)) {
            content += "\n\n⏳ 任务正在执行中，widget 正在自动监控进度。请勿轮询，等待完成后再查询。";
          }

          if (durationStr) {
            content += `\n⏱️ 运行时长: ${durationStr}`;
          }

          return {
            content: [{ type: "text", text: content }],
            details: {
              task_id: result.id || result.task_id,
              status,
              service_id: result.service_id,
              task_prompt: result.task_prompt,
              output_prompt: result.output_prompt,
              result: result.result,
              created_at: result.created_at,
              updated_at: result.updated_at,
              completed_at: result.completed_at,
              started_at: result.started_at,
              full_response: result,
            },
          };
        }

        case "cancel_task": {
          const taskId = params.task_id;
          if (!taskId) throw new Error("缺少必填参数: task_id");

          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/tasks/${encodeURIComponent(taskId)}/cancel`;

          const response = await safeFetch(url, {
            method: "POST",
            headers: {
              Authorization: `Bearer ${apiKey}`,
              "Content-Type": "application/json",
            },
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) {
              throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
            } else if (response.status === 403) {
              throw new Error("权限不足 (403): 只能取消自己创建的任务");
            } else if (response.status === 404) {
              throw new Error("任务不存在 (404): 请检查 task_id 是否正确");
            } else if (response.status === 400) {
              throw new Error(`无法取消 (400): ${msg}`);
            }
            throw new Error(`取消失败 (HTTP ${response.status}): ${msg}`);
          }

          const result = (await response.json()) as Record<string, unknown>;
          const status = (result.status as string) || "unknown";

          if (activeTasks.has(taskId)) {
            const task = activeTasks.get(taskId)!;
            task.status = status;
            if (result.completed_at) task.completedAt = result.completed_at as string;
            // 注意：status 为 "cancelling"（非终态）时有意不递增 generation——
            // 容忍在途轮询响应短暂回写旧状态，下一轮轮询会自愈为最新状态
            if (["cancelled", "completed", "failed"].includes(status)) {
              stopPolling(taskId);
              trimTerminalTasks();
            }
            updateTUI(ctx);
            pi.appendEntry("OpenAaaS-task", {
              task_id: task.taskId,
              service_id: task.serviceId,
              service_name: task.serviceName,
              task_prompt: task.taskPrompt,
              status: task.status,
              server: task.server,
              created_at: task.createdAt,
              started_at: task.startedAt,
              completed_at: task.completedAt,
              updated_at: new Date().toISOString(),
            });
          }

          let text: string;
          if (status === "cancelled") {
            text = `✅ 任务已取消\n任务 ID: ${taskId}\n状态: 已取消`;
          } else if (status === "cancelling") {
            text = `⏳ 任务正在取消中\n任务 ID: ${taskId}\n状态: 取消中（Agent 将收到取消信号）`;
          } else {
            text = `任务状态: ${status}\n任务 ID: ${taskId}`;
          }

          return {
            content: [{ type: "text", text }],
            details: {
              task_id: taskId,
              status,
              full_response: result,
            },
          };
        }

        case "list_files": {
          const taskId = params.task_id;
          if (!taskId) throw new Error("缺少必填参数: task_id");

          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);
          const url = `${serverUrl}/api/v1/client/files/list/${encodeURIComponent(taskId)}`;

          const response = await safeFetch(url, {
            method: "GET",
            headers: { Authorization: `Bearer ${apiKey}` },
          });

          if (!response.ok) {
            const msg = await readErrorBody(response);
            if (response.status === 401) throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
            throw new Error(`获取文件列表失败 (HTTP ${response.status}): ${msg}`);
          }

          const listResult = (await response.json()) as Array<Record<string, unknown>> | { files?: Array<Record<string, unknown>> };
          const files = Array.isArray(listResult) ? listResult : listResult.files || [];

          if (files.length === 0) {
            return {
              content: [{ type: "text", text: `任务 ${taskId} 没有结果文件` }],
              details: { task_id: taskId, files: [] },
            };
          }

          const lines = [`任务 ${taskId} 共有 ${files.length} 个结果文件：`];
          for (let i = 0; i < files.length; i++) {
            const f = files[i];
            const filename = (f.filename as string) ?? (f.name as string) ?? "unnamed";
            const fileId = (f.id as string) ?? (f.file_id as string) ?? "";
            const size = f.size ?? f.file_size ?? "未知";
            lines.push(`${i + 1}. ${filename} (ID: ${fileId}, 大小: ${size})`);
          }

          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: { task_id: taskId, files },
          };
        }

        case "download_result": {
          const taskId = params.task_id;
          if (!taskId) throw new Error("缺少必填参数: task_id");

          const { server_url: serverUrl } = getServerConfig(params.server);
          const apiKey = requireApiKey(params.server);

          const listUrl = `${serverUrl}/api/v1/client/files/list/${encodeURIComponent(taskId)}`;
          const listResponse = await safeFetch(listUrl, {
            method: "GET",
            headers: { Authorization: `Bearer ${apiKey}` },
          });

          if (!listResponse.ok) {
            const msg = await readErrorBody(listResponse);
            if (listResponse.status === 401) {
              throw new Error("认证失败 (401): API Key 无效，请检查 config.json 中的 api_key 是否正确");
            }
            throw new Error(`获取文件列表失败 (HTTP ${listResponse.status}): ${msg}`);
          }

          const listResult = (await listResponse.json()) as
            | Array<Record<string, unknown>>
            | { files?: Array<Record<string, unknown>> };
          const files = Array.isArray(listResult)
            ? listResult
            : listResult.files || [];

          if (!files || files.length === 0) {
            throw new Error(`任务 ${taskId} 没有可下载的结果文件`);
          }

          const safeTaskId = taskId.replace(/[\\/]/g, "_").replace(/\.{2,}/g, "_");
          const downloadDir = resolve(ctx.cwd, ".OpenAaaS", "downloads", safeTaskId);
          const parentDir = resolve(ctx.cwd, ".OpenAaaS", "downloads");
          if (!downloadDir.startsWith(parentDir + sep) && downloadDir !== parentDir) {
            throw new Error("任务 ID 包含非法路径字符");
          }
          if (!existsSync(downloadDir)) mkdirSync(downloadDir, { recursive: true });

          let targetFiles: Array<Record<string, unknown>>;
          if (params.download_all) {
            targetFiles = files;
          } else if (params.file_id) {
            const matched = files.find((f) => (f.id as string) === params.file_id || (f.file_id as string) === params.file_id);
            if (!matched) throw new Error(`找不到指定的文件 ID: ${params.file_id}`);
            targetFiles = [matched];
          } else {
            const zipFiles = files.filter((f) => typeof f.filename === "string" && f.filename.toLowerCase().endsWith(".zip"));
            targetFiles = zipFiles.length > 0 ? [zipFiles[0]] : [files[0]];
          }

          for (const targetFile of targetFiles) {
            const fileId = (targetFile.id as string) || (targetFile.file_id as string);
            const filename = (targetFile.filename as string) || (targetFile.name as string) || `${fileId}.download`;
            if (!fileId) {
              throw new Error(`文件缺少 ID，无法下载: ${filename}`);
            }

            const isZip = filename.toLowerCase().endsWith(".zip");
            const safeName = sanitizeFilename(filename, isZip ? "zip" : "download");
            let uniqueSafeName = safeName;
            let counter = 1;
            const extIndex = safeName.lastIndexOf(".");
            const baseName = extIndex > 0 ? safeName.slice(0, extIndex) : safeName;
            const ext = extIndex > 0 ? safeName.slice(extIndex) : "";
            while (existsSync(resolve(downloadDir, uniqueSafeName))) {
              uniqueSafeName = `${baseName}_${counter}${ext}`;
              counter++;
            }
            const { filePath } = await downloadSingleFile(serverUrl, apiKey, fileId, uniqueSafeName, downloadDir);

            if (isZip) {
              try {
                const zip = new AdmZip(filePath);
                const MAX_EXTRACT_SIZE = MAX_DOWNLOAD_SIZE * 3;
                let totalExtractSize = 0;
                for (const entry of zip.getEntries()) {
                  const entryPath = resolve(downloadDir, entry.entryName);
                  if (entryPath !== downloadDir && !entryPath.startsWith(downloadDir + sep)) {
                    throw new Error(`解压失败: 压缩包包含非法路径: ${entry.entryName}`);
                  }
                  if (!entry.isDirectory) {
                    totalExtractSize += entry.header.size;
                    if (totalExtractSize > MAX_EXTRACT_SIZE) {
                      throw new Error(`解压失败: 解压后总大小超过 ${MAX_EXTRACT_SIZE} bytes 限制，可能存在 Zip 炸弹`);
                    }
                  }
                }
                zip.extractAllTo(downloadDir, true);
              } catch (e) {
                const msg = e instanceof Error ? e.message : String(e);
                throw new Error(`下载成功但解压失败: ${msg}`);
              }
              try { rmSync(filePath, { force: true }); } catch {}
            }
          }

          let extractedFiles: string[];
          try {
            extractedFiles = readdirSync(downloadDir);
          } catch {
            extractedFiles = [];
          }

          const fileList = extractedFiles.length > 0
            ? extractedFiles.map((f) => `  - ${f}`).join("\n")
            : "（目录为空）";

          return {
            content: [{
              type: "text",
              text: `结果下载成功！\n任务 ID: ${taskId}\n文件夹路径: ${downloadDir}\n文件列表:\n${fileList}\n\n💡 提示: 同一任务多次下载会覆盖到同一目录。`,
            }],
            details: { task_id: taskId, folder_path: downloadDir, files: extractedFiles },
          };
        }

        case "list_servers": {
          const config = loadConfig();
          const servers = config.servers || {};
          const lines = [`已配置 ${Object.keys(servers).length} 个服务器：`];
          for (const [alias, sc] of Object.entries(servers)) {
            const isDefault = alias === config.default_server;
            const hasKey = !!sc.api_key;
            lines.push(`${isDefault ? "★ " : "  "}${alias}: ${sc.server_url}${hasKey ? " (已注册)" : " (未注册)"}`);
          }
          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: { servers, default_server: config.default_server },
          };
        }

        case "set_default_server": {
          const alias = (params.server || "").trim();
          if (!alias) throw new Error("缺少必填参数: server");
          setDefaultServer(alias);
          return {
            content: [{ type: "text", text: `默认服务器已切换为: ${alias}` }],
            details: { default_server: alias },
          };
        }

        case "remove_server": {
          const alias = (params.server || "").trim();
          if (!alias) throw new Error("缺少必填参数: server（要删除的服务器别名）");
          removeServerConfig(alias);
          return {
            content: [{ type: "text", text: `服务器 "${alias}" 已删除` }],
            details: { removed: true, server_alias: alias },
          };
        }

        default: {
          throw new Error(`未知的 action: ${params.action}`);
        }
      }
    },
  });
}
