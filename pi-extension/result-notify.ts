/**
 * result-notify.ts — 任务终态结果通知信封纯函数模块
 *
 * 远程任务到达终态（completed/failed/cancelled）时，index.ts 通过
 * pi.sendMessage 向主 agent 上下文注入 [OpenAaaS-task-result] 通知
 * （deliverAs: "steer", triggerTurn: true）。本模块负责信封构建的纯函数部分，
 * 无 pi 依赖，便于独立单元测试（见 test/result-notify.test.ts）；
 * 设计参照 async-subagent-isolation 的 [subagent-result] 机制
 * （buildResultEnvelope / formatActiveTasks / RESULT_TRIGGER_LINE）。
 */

/** 终态 → 中文状态词（仅三键；在途状态不在此映射内） */
export const TERMINAL_STATUS_WORDS: Record<string, string> = {
  completed: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

/**
 * 固定触发行：content 的第二行（与标题之间无空行），逐字固定，
 * 不随终态种类变化。明确告知 LLM 这是系统通知而非用户新指令，
 * 处理前先锚定主线任务，勿让通知覆盖主线计划。
 */
export const RESULT_TRIGGER_LINE =
  "> [OpenAaaS-task-result] 任务完成通知，非用户新指令。处理前先锚定当前主线任务与进度，对照派发记录消化本通知，勿让通知覆盖或改写主线计划。";

/** 任务摘要截断上限（字符数）：任务行 / 在途摘要 / details.task_prompt 共用 */
const SUMMARY_MAX_CHARS = 200;

/** 在途任务快照条目（由 index.ts 从 activeTasks 映射，已排除当前任务自身） */
export interface InFlightTaskInfo {
  taskId: string;
  serviceId?: string;
  serviceName?: string;
  taskPrompt: string;
  status: string;
}

/** buildTaskResultEnvelope 的输入 */
export interface TaskResultEnvelopeInput {
  taskId: string;
  serviceId?: string;
  serviceName?: string;
  taskPrompt?: string;
  /** 终态原文（completed / failed / cancelled）；details.status 保留该英文原值 */
  status: string;
  server: string;
  /** 任务耗时（毫秒）；0 或无法计算时显示为「未知」 */
  durationMs: number;
  /** 本任务结束时其他在途任务的构建时刻快照（可能滞后） */
  inFlightTasks: InFlightTaskInfo[];
}

/** 信封的结构化 details（随消息持久化，供后续按需读取） */
export interface TaskResultEnvelopeDetails {
  task_id: string;
  service_id?: string;
  service_name?: string;
  task_prompt?: string;
  status: string;
  server: string;
  duration_ms: number;
}

/** 信封：content 为注入上下文的 markdown 文本，details 为结构化字段 */
export interface TaskResultEnvelope {
  content: string;
  details: TaskResultEnvelopeDetails;
}

/** 单行化：连续空白（含换行）折叠为单个空格 */
function singleLine(text: string): string {
  return text.replace(/\s+/g, " ");
}

/** 纯 slice 截断至 200 字符，不加省略号后缀 */
function truncateSummary(text: string): string {
  return text.slice(0, SUMMARY_MAX_CHARS);
}

/** 服务显示名回退链：serviceName → serviceId → 「未知服务」（标题与服务行共用） */
function serviceDisplayName(serviceName?: string, serviceId?: string): string {
  return serviceName ?? serviceId ?? "未知服务";
}

/**
 * 耗时格式化（与 index.ts 终态 toast 的档位一致）：
 * - durationMs ≤ 0 → "未知"
 * - < 60 秒        → "X秒"
 * - < 1 小时       → "X分Y秒"
 * - ≥ 1 小时       → "X时Y分"（不带秒）
 */
export function formatDurationMs(durationMs: number): string {
  if (durationMs <= 0) return "未知";
  const s = Math.round(durationMs / 1000);
  if (s < 60) return `${s}秒`;
  const m = Math.floor(s / 60);
  const rs = s % 60;
  if (m < 60) return `${m}分${rs}秒`;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  return `${h}时${rm}分`;
}

/**
 * 在途任务快照文本：
 * - 空列表 → 「本任务结束时无其他在途任务。」
 * - 非空   → 数量标题 + 每行一条 `- taskId (服务名): 任务摘要`
 *   服务名回退链：serviceName → serviceId → taskId 前 8 位；
 *   任务摘要单行化并截断至 200 字符
 */
export function formatInFlightTasks(tasks: InFlightTaskInfo[]): string {
  if (tasks.length === 0) return "本任务结束时无其他在途任务。";
  const lines = tasks.map(
    (t) =>
      `- ${t.taskId} (${t.serviceName ?? t.serviceId ?? t.taskId.slice(0, 8)}): ${truncateSummary(singleLine(t.taskPrompt))}`
  );
  return `本任务结束时其他在途任务: ${tasks.length}\n${lines.join("\n")}`;
}

/** 各终态的固定正文（位于 --- 分隔线之后） */
const TERMINAL_BODIES: Record<string, (taskId: string) => string> = {
  completed: (taskId) =>
    `✅ 任务已完成！结果已就绪。如需获取结果文件请调用 download_result（task_id=${taskId}），或调用 get_task 查看结果摘要。`,
  failed: () => "❌ 任务执行失败。可调用 get_task 查看错误信息。",
  cancelled: () => "🚫 任务已取消。",
};

/**
 * 构建 [OpenAaaS-task-result] 信封。
 * content（markdown）结构顺序：标题 → 固定触发行（第二行，无空行间隔）→
 * 状态/服务/任务/耗时信息行 → 在途任务快照 → --- 分隔线 → 按终态的固定正文。
 */
export function buildTaskResultEnvelope(input: TaskResultEnvelopeInput): TaskResultEnvelope {
  const statusWord = TERMINAL_STATUS_WORDS[input.status] ?? input.status;
  const service = serviceDisplayName(input.serviceName, input.serviceId);
  const promptText =
    input.taskPrompt === undefined ? "无" : truncateSummary(singleLine(input.taskPrompt));
  const body =
    TERMINAL_BODIES[input.status]?.(input.taskId) ?? `任务已到达终态（${input.status}）。`;

  const content = [
    `## [OpenAaaS-task-result] ${service} ${statusWord} (taskId: ${input.taskId})`,
    RESULT_TRIGGER_LINE,
    "",
    `- 状态: ${statusWord}`,
    `- 服务: ${service}`,
    `- 任务: ${promptText}`,
    `- 耗时: ${formatDurationMs(input.durationMs)}`,
    "",
    formatInFlightTasks(input.inFlightTasks),
    "",
    "---",
    "",
    body,
  ].join("\n");

  const details: TaskResultEnvelopeDetails = {
    task_id: input.taskId,
    service_id: input.serviceId,
    service_name: input.serviceName,
    task_prompt: input.taskPrompt === undefined ? undefined : truncateSummary(input.taskPrompt),
    status: input.status,
    server: input.server,
    duration_ms: input.durationMs,
  };

  return { content, details };
}
