/**
 * polling.ts — 任务状态轮询策略纯函数模块
 *
 * 与 index.ts 解耦，便于独立单元测试（见 test/polling.test.ts）。
 */

/**
 * 按任务运行时长（毫秒，从服务端 created_at 算起）返回下次轮询间隔：
 * - 0 ≤ runtime < 120_000（< 2 分钟）     → 10_000
 * - 120_000 ≤ runtime < 600_000（2~10 分钟）→ 20_000
 * - runtime ≥ 600_000（≥ 10 分钟）         → 30_000
 */
export function nextPollIntervalMs(taskRuntimeMs: number): number {
  if (taskRuntimeMs < 120_000) return 10_000;
  if (taskRuntimeMs < 600_000) return 20_000;
  return 30_000;
}

/**
 * 连续失败退避间隔（毫秒），用于网络错误 / 5xx / 无 Retry-After 的 429：
 * - 1 → 60_000
 * - 2 → 120_000
 * - 3 → 240_000
 * - ≥ 4 → 300_000（封顶）
 */
export function nextBackoffMs(consecutiveFailures: number): number {
  if (consecutiveFailures <= 1) return 60_000;
  if (consecutiveFailures === 2) return 120_000;
  if (consecutiveFailures === 3) return 240_000;
  return 300_000;
}

/**
 * 解析 HTTP 429 Retry-After 头，仅支持秒数格式（非负整数）。
 * 含前导/尾随空白会先 trim；HTTP 日期格式、非数字、空字符串、null 均返回 null。
 */
export function parseRetryAfterSeconds(headerValue: string | null): number | null {
  if (headerValue === null) return null;
  const trimmed = headerValue.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  return Number(trimmed);
}

/** 单次轮询延迟下限（毫秒）：防止 Retry-After 过小导致紧循环 */
export const MIN_RETRY_MS = 1_000;

/** 单次轮询延迟上限（毫秒）：防止 Retry-After 过大导致任务长时间挂起 */
export const MAX_RETRY_MS = 300_000;

/**
 * nextPollDelay 的决策输入：状态查询结果分类。
 * index.ts 中 getTaskStatus 返回的本地联合类型（额外携带 data/message 等字段）
 * 在结构上与本接口兼容，可直接传入。
 */
export interface TaskStatusQuery {
  kind: "ok" | "rate_limited" | "server_error" | "network_error" | "permanent";
}

/** 一次轮询后的调度决策 */
export interface PollDecision {
  /** 距下一次轮询的延迟（毫秒）；stop=true 时为 0 */
  delayMs: number;
  /** 是否停止轮询（仅 permanent 错误为 true） */
  stop: boolean;
  /** 更新后的连续失败计数 */
  consecutiveFailures: number;
}

/**
 * 单次轮询的统一决策函数：
 * - ok                                → 失败计数清零，按任务运行时长分档（nextPollIntervalMs）
 * - rate_limited + Retry-After ≥ 1    → 尊重服务端等待，钳制到 [MIN_RETRY_MS, MAX_RETRY_MS]，
 *                                       受控等待不算失败，计数不递增
 * - rate_limited 无可用 Retry-After（null/0）→ 视为失败，退避（计数 +1）
 * - server_error / network_error      → 退避（计数 +1，nextBackoffMs）
 * - permanent                         → stop=true，delayMs=0，计数保持不变
 */
export function nextPollDelay(
  query: TaskStatusQuery,
  taskRuntimeMs: number,
  consecutiveFailures: number,
  retryAfterSeconds: number | null
): PollDecision {
  switch (query.kind) {
    case "ok":
      // 成功：失败计数清零，回到按任务运行时长分档的正常间隔
      return { delayMs: nextPollIntervalMs(taskRuntimeMs), stop: false, consecutiveFailures: 0 };
    case "rate_limited":
      if (retryAfterSeconds !== null && retryAfterSeconds >= 1) {
        return {
          delayMs: Math.min(Math.max(retryAfterSeconds * 1000, MIN_RETRY_MS), MAX_RETRY_MS),
          stop: false,
          consecutiveFailures,
        };
      }
      // 无可用 Retry-After（null/0）的 429 视为失败，走退避
      {
        const next = consecutiveFailures + 1;
        return { delayMs: nextBackoffMs(next), stop: false, consecutiveFailures: next };
      }
    case "server_error":
    case "network_error": {
      const next = consecutiveFailures + 1;
      return { delayMs: nextBackoffMs(next), stop: false, consecutiveFailures: next };
    }
    case "permanent":
      // 永久错误：停止轮询，计数保持不变（便于诊断）
      return { delayMs: 0, stop: true, consecutiveFailures };
  }
}
