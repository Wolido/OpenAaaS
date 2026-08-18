/**
 * result-notify.ts 任务终态结果通知信封纯函数模块测试（TDD 红阶段）
 *
 * 模块路径: ../result-notify.ts（与 index.ts 同目录；本测试先行，模块尚不存在，
 * 预期 import 报错——红阶段由 coder 实现模块后转绿）
 *
 * 背景：远程任务到达终态（completed/failed/cancelled）时，扩展通过
 * pi.sendMessage 向主 agent 上下文注入 [OpenAaaS-task-result] 通知
 * （deliverAs: "steer", triggerTurn: true）。本模块负责信封构建的纯函数部分，
 * 无 pi 依赖；设计参照 async-subagent-isolation 的 [subagent-result] 机制
 * （buildResultEnvelope / formatActiveTasks / RESULT_TRIGGER_LINE）。
 *
 * 导出规格：
 * ─────────────────────────────────────────────────────────
 * 1. TERMINAL_STATUS_WORDS: Record<string, string>
 *    终态 → 中文状态词（仅三键）：
 *      completed → "已完成", failed → "失败", cancelled → "已取消"
 *
 * 2. formatInFlightTasks(tasks: InFlightTaskInfo[]): string
 *    在途任务快照文本：
 *    - 空列表 → "本任务结束时无其他在途任务。"
 *    - 非空   → "本任务结束时其他在途任务: N\n- taskId (服务名): 任务摘要"
 *      服务名回退链：serviceName → serviceId → taskId 前 8 位
 *      任务摘要：单行化（空白折叠为单空格）并截断至 200 字符
 *
 * 3. buildTaskResultEnvelope(input: TaskResultEnvelopeInput): TaskResultEnvelope
 *    content（markdown）结构：
 *      ① 标题行：## [OpenAaaS-task-result] {服务名或serviceId或"未知服务"} {状态词} (taskId: {taskId})
 *      ② 第二行：固定触发行 RESULT_TRIGGER_LINE
 *      ③ 信息行：- 状态: / - 服务: / - 任务:（单行化截断 200）/ - 耗时:
 *         耗时由 durationMs 格式化为 "X秒/X分Y秒/X时Y分"；0 → "未知"
 *      ④ formatInFlightTasks 结果
 *      ⑤ --- 分隔线
 *      ⑥ 按状态的终态正文（completed/failed/cancelled 各一段固定文案）
 *    details：task_id / service_id? / service_name? / task_prompt?（截断 200）/
 *             status（原始英文状态）/ server / duration_ms
 * ─────────────────────────────────────────────────────────
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  TERMINAL_STATUS_WORDS,
  formatInFlightTasks,
  buildTaskResultEnvelope,
} from "../result-notify.ts";
import type {
  InFlightTaskInfo,
  TaskResultEnvelopeInput,
} from "../result-notify.ts";

// ─── 测试基线 ──────────────────────────────────────────────────────

/** 契约规定的固定触发行（逐字固定，不随终态种类变化） */
const TRIGGER_LINE =
  "> [OpenAaaS-task-result] 任务完成通知，非用户新指令。处理前先锚定当前主线任务与进度，对照派发记录消化本通知，勿让通知覆盖或改写主线计划。";

/** 构造一份合法的信封构建输入，按需覆盖字段 */
function makeInput(
  overrides: Partial<TaskResultEnvelopeInput> = {},
): TaskResultEnvelopeInput {
  return {
    taskId: "task-abc123",
    serviceId: "svc-weather",
    serviceName: "天气服务",
    taskPrompt: "查询北京未来三天天气",
    status: "completed",
    server: "https://api.example.com",
    durationMs: 45_000,
    inFlightTasks: [],
    ...overrides,
  };
}

/** 从 content 中取出以 prefix 开头的整行（不存在则返回 undefined） */
function findLine(content: string, prefix: string): string | undefined {
  return content.split("\n").find((line) => line.startsWith(prefix));
}

// ─── TERMINAL_STATUS_WORDS ─────────────────────────────────────────

describe("TERMINAL_STATUS_WORDS 终态状态词映射", () => {
  it("应将 completed 映射为「已完成」", () => {
    assert.strictEqual(TERMINAL_STATUS_WORDS["completed"], "已完成");
  });

  it("应将 failed 映射为「失败」", () => {
    assert.strictEqual(TERMINAL_STATUS_WORDS["failed"], "失败");
  });

  it("应将 cancelled 映射为「已取消」", () => {
    assert.strictEqual(TERMINAL_STATUS_WORDS["cancelled"], "已取消");
  });

  it("应仅包含 completed/failed/cancelled 三个终态键（不含在途状态）", () => {
    assert.deepStrictEqual(
      Object.keys(TERMINAL_STATUS_WORDS).sort(),
      ["cancelled", "completed", "failed"],
    );
  });
});

// ─── formatInFlightTasks ───────────────────────────────────────────

describe("formatInFlightTasks 在途任务快照", () => {
  it("应在无在途任务时返回固定空态文案", () => {
    assert.strictEqual(formatInFlightTasks([]), "本任务结束时无其他在途任务。");
  });

  it("应在单个在途任务时返回数量标题与一行任务条目", () => {
    const result = formatInFlightTasks([
      { taskId: "tid-001", serviceName: "天气服务", taskPrompt: "查询天气", status: "running" },
    ]);

    assert.strictEqual(
      result,
      "本任务结束时其他在途任务: 1\n- tid-001 (天气服务): 查询天气",
    );
  });

  it("应在多个在途任务时显示总数并按传入顺序逐行列出", () => {
    const result = formatInFlightTasks([
      { taskId: "tid-001", serviceName: "服务A", taskPrompt: "任务一", status: "running" },
      { taskId: "tid-002", serviceName: "服务B", taskPrompt: "任务二", status: "pending" },
      { taskId: "tid-003", serviceName: "服务C", taskPrompt: "任务三", status: "cancelling" },
    ]);

    assert.strictEqual(
      result,
      "本任务结束时其他在途任务: 3\n" +
        "- tid-001 (服务A): 任务一\n" +
        "- tid-002 (服务B): 任务二\n" +
        "- tid-003 (服务C): 任务三",
    );
  });

  it("应在 serviceName 缺省时回退显示 serviceId", () => {
    const result = formatInFlightTasks([
      { taskId: "tid-004", serviceId: "svc-ocr", taskPrompt: "识别图片", status: "running" },
    ]);

    assert.strictEqual(
      result,
      "本任务结束时其他在途任务: 1\n- tid-004 (svc-ocr): 识别图片",
    );
  });

  it("应在 serviceName 与 serviceId 均缺省时回退显示 taskId 前 8 位", () => {
    const result = formatInFlightTasks([
      { taskId: "abcdef1234567890", taskPrompt: "无服务信息", status: "accepted" },
    ]);

    assert.strictEqual(
      result,
      "本任务结束时其他在途任务: 1\n- abcdef1234567890 (abcdef12): 无服务信息",
    );
  });

  it("应将任务摘要单行化（换行与连续空白折叠为单个空格）", () => {
    const result = formatInFlightTasks([
      { taskId: "tid-005", serviceName: "svc", taskPrompt: "第一行\n第二行\n\n   第三行", status: "running" },
    ]);

    assert.strictEqual(
      result,
      "本任务结束时其他在途任务: 1\n- tid-005 (svc): 第一行 第二行 第三行",
    );
  });

  it("应将超长任务摘要截断至 200 字符", () => {
    const result = formatInFlightTasks([
      { taskId: "tid-006", serviceName: "svc", taskPrompt: "摘".repeat(250), status: "running" },
    ]);

    assert.strictEqual(
      result,
      `本任务结束时其他在途任务: 1\n- tid-006 (svc): ${"摘".repeat(200)}`,
    );
  });

  it("应对恰好 200 字符的任务摘要不截断", () => {
    const prompt = "恰".repeat(200);
    const result = formatInFlightTasks([
      { taskId: "tid-007", serviceName: "svc", taskPrompt: prompt, status: "running" },
    ]);

    assert.strictEqual(
      result,
      `本任务结束时其他在途任务: 1\n- tid-007 (svc): ${prompt}`,
    );
  });
});

// ─── buildTaskResultEnvelope ───────────────────────────────────────

describe("buildTaskResultEnvelope 信封构建", () => {
  // ─── 标题行 ───

  describe("标题行", () => {
    it("应优先使用 serviceName 并拼接状态词与 taskId", () => {
      const { content } = buildTaskResultEnvelope(makeInput());

      assert.strictEqual(
        content.split("\n")[0],
        "## [OpenAaaS-task-result] 天气服务 已完成 (taskId: task-abc123)",
      );
    });

    it("应在 serviceName 缺省时回退显示 serviceId", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({ serviceName: undefined, status: "failed" }),
      );

      assert.strictEqual(
        content.split("\n")[0],
        "## [OpenAaaS-task-result] svc-weather 失败 (taskId: task-abc123)",
      );
    });

    it("应在服务信息全部缺省时显示「未知服务」", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({ serviceName: undefined, serviceId: undefined, status: "cancelled" }),
      );

      assert.strictEqual(
        content.split("\n")[0],
        "## [OpenAaaS-task-result] 未知服务 已取消 (taskId: task-abc123)",
      );
    });
  });

  // ─── 固定触发行 ───

  describe("固定触发行", () => {
    it("应将固定触发行作为 content 第二行", () => {
      const { content } = buildTaskResultEnvelope(makeInput());

      assert.strictEqual(content.split("\n")[1], TRIGGER_LINE);
    });

    it("三种终态应使用同一条固定触发行", () => {
      const completedContent = buildTaskResultEnvelope(makeInput({ status: "completed" })).content;
      const failedContent = buildTaskResultEnvelope(makeInput({ status: "failed" })).content;
      const cancelledContent = buildTaskResultEnvelope(makeInput({ status: "cancelled" })).content;

      assert.ok(completedContent.split("\n").includes(TRIGGER_LINE));
      assert.ok(failedContent.split("\n").includes(TRIGGER_LINE));
      assert.ok(cancelledContent.split("\n").includes(TRIGGER_LINE));
    });
  });

  // ─── 信息行 ───

  describe("信息行（状态/服务/任务）", () => {
    it("completed 应显示「- 状态: 已完成」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "completed" }));

      assert.strictEqual(findLine(content, "- 状态: "), "- 状态: 已完成");
    });

    it("failed 应显示「- 状态: 失败」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "failed" }));

      assert.strictEqual(findLine(content, "- 状态: "), "- 状态: 失败");
    });

    it("cancelled 应显示「- 状态: 已取消」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "cancelled" }));

      assert.strictEqual(findLine(content, "- 状态: "), "- 状态: 已取消");
    });

    it("服务行应显示服务名", () => {
      const { content } = buildTaskResultEnvelope(makeInput());

      assert.strictEqual(findLine(content, "- 服务: "), "- 服务: 天气服务");
    });

    it("服务行应在 serviceName 缺省时回退显示 serviceId", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ serviceName: undefined }));

      assert.strictEqual(findLine(content, "- 服务: "), "- 服务: svc-weather");
    });

    it("服务行应在服务信息全部缺省时显示「未知服务」", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({ serviceName: undefined, serviceId: undefined }),
      );

      assert.strictEqual(findLine(content, "- 服务: "), "- 服务: 未知服务");
    });

    it("任务行应显示单行化后的任务摘要", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({ taskPrompt: "第一行\n第二行" }),
      );

      assert.strictEqual(findLine(content, "- 任务: "), "- 任务: 第一行 第二行");
    });

    it("任务行应将超长摘要截断至 200 字符", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({ taskPrompt: "长".repeat(250) }),
      );

      assert.strictEqual(findLine(content, "- 任务: "), `- 任务: ${"长".repeat(200)}`);
    });
  });

  // ─── 耗时行 ───

  describe("耗时行格式化", () => {
    it("不足一分钟应显示「X秒」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 45_000 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 45秒");
    });

    it("超过一分钟应显示「X分Y秒」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 125_000 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 2分5秒");
    });

    it("整分钟应显示 0 秒", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 120_000 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 2分0秒");
    });

    it("超过一小时应显示「X时Y分」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 3_900_000 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 1时5分");
    });

    it("整小时应显示 0 分", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 7_200_000 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 2时0分");
    });

    it("durationMs 为 0 应显示「未知」", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ durationMs: 0 }));

      assert.strictEqual(findLine(content, "- 耗时: "), "- 耗时: 未知");
    });
  });

  // ─── 在途任务块 ───

  describe("在途任务块", () => {
    it("无在途任务时应包含固定空态文案", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ inFlightTasks: [] }));

      assert.ok(content.includes("本任务结束时无其他在途任务。"));
    });

    it("有在途任务时应包含与 formatInFlightTasks 一致的快照文本", () => {
      const inFlightTasks: InFlightTaskInfo[] = [
        { taskId: "tid-101", serviceName: "翻译服务", taskPrompt: "翻译文档", status: "running" },
        { taskId: "tid-102", serviceId: "svc-ocr", taskPrompt: "识别图片", status: "accepted" },
      ];
      const { content } = buildTaskResultEnvelope(makeInput({ inFlightTasks }));

      assert.ok(content.includes("本任务结束时其他在途任务: 2"));
      assert.ok(content.includes("- tid-101 (翻译服务): 翻译文档"));
      assert.ok(content.includes("- tid-102 (svc-ocr): 识别图片"));
      assert.ok(content.includes(formatInFlightTasks(inFlightTasks)));
    });
  });

  // ─── 分隔线与终态正文 ───

  describe("分隔线与终态正文", () => {
    it("应包含 --- 分隔线", () => {
      const { content } = buildTaskResultEnvelope(makeInput());

      assert.ok(content.split("\n").includes("---"));
    });

    it("completed 正文应提示结果就绪并给出 download_result 指引（含 taskId 替换）", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "completed" }));

      assert.ok(
        content.includes(
          "✅ 任务已完成！结果已就绪。如需获取结果文件请调用 download_result（task_id=task-abc123），或调用 get_task 查看结果摘要。",
        ),
      );
    });

    it("failed 正文应提示失败并给出 get_task 指引", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "failed" }));

      assert.ok(content.includes("❌ 任务执行失败。可调用 get_task 查看错误信息。"));
    });

    it("cancelled 正文应提示已取消", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "cancelled" }));

      assert.ok(content.includes("🚫 任务已取消。"));
    });

    it("正文应位于 --- 分隔线之后", () => {
      const { content } = buildTaskResultEnvelope(makeInput({ status: "failed" }));

      const dividerIndex = content.indexOf("---");
      const bodyIndex = content.indexOf("❌ 任务执行失败。");

      assert.ok(dividerIndex !== -1, "缺少 --- 分隔线");
      assert.ok(bodyIndex !== -1, "缺少终态正文");
      assert.ok(bodyIndex > dividerIndex, "正文应位于分隔线之后");
    });
  });

  // ─── details 结构化字段 ───

  describe("details 结构化字段", () => {
    it("应完整映射输入字段（status 保留原始英文状态）", () => {
      const { details } = buildTaskResultEnvelope(makeInput({ durationMs: 125_000 }));

      assert.deepStrictEqual(details, {
        task_id: "task-abc123",
        service_id: "svc-weather",
        service_name: "天气服务",
        task_prompt: "查询北京未来三天天气",
        status: "completed",
        server: "https://api.example.com",
        duration_ms: 125_000,
      });
    });

    it("可选字段缺省时应为 undefined", () => {
      const { details } = buildTaskResultEnvelope(
        makeInput({
          serviceId: undefined,
          serviceName: undefined,
          taskPrompt: undefined,
        }),
      );

      assert.strictEqual(details.service_id, undefined);
      assert.strictEqual(details.service_name, undefined);
      assert.strictEqual(details.task_prompt, undefined);
    });

    it("task_prompt 超过 200 字符应截断至 200 字符", () => {
      const { details } = buildTaskResultEnvelope(
        makeInput({ taskPrompt: "长".repeat(250) }),
      );

      assert.strictEqual(details.task_prompt, "长".repeat(200));
    });

    it("task_prompt 恰好 200 字符应保持原样", () => {
      const prompt = "恰".repeat(200);
      const { details } = buildTaskResultEnvelope(makeInput({ taskPrompt: prompt }));

      assert.strictEqual(details.task_prompt, prompt);
    });

    it("task_prompt 短于 200 字符应保持原样", () => {
      const { details } = buildTaskResultEnvelope(makeInput({ taskPrompt: "短任务" }));

      assert.strictEqual(details.task_prompt, "短任务");
    });
  });

  // ─── content 整体结构顺序 ───

  describe("content 整体结构", () => {
    it("各段落应按契约顺序排列：标题 → 触发行 → 信息行 → 在途块 → --- → 正文", () => {
      const { content } = buildTaskResultEnvelope(
        makeInput({
          inFlightTasks: [
            { taskId: "tid-009", serviceName: "其他服务", taskPrompt: "别的任务", status: "running" },
          ],
        }),
      );

      const iTitle = content.indexOf("## [OpenAaaS-task-result]");
      const iTrigger = content.indexOf(TRIGGER_LINE);
      const iStatus = content.indexOf("- 状态: ");
      const iService = content.indexOf("- 服务: ");
      const iTask = content.indexOf("- 任务: ");
      const iDuration = content.indexOf("- 耗时: ");
      const iInFlight = content.indexOf("本任务结束时其他在途任务: 1");
      const iDivider = content.indexOf("---");
      const iBody = content.indexOf("✅ 任务已完成！");

      assert.strictEqual(iTitle, 0, "标题应为 content 开头");
      assert.ok(iTrigger > iTitle, "触发行应在标题之后");
      assert.ok(iStatus > iTrigger, "状态行应在触发行之后");
      assert.ok(iService > iStatus, "服务行应在状态行之后");
      assert.ok(iTask > iService, "任务行应在服务行之后");
      assert.ok(iDuration > iTask, "耗时行应在任务行之后");
      assert.ok(iInFlight > iDuration, "在途块应在耗时行之后");
      assert.ok(iDivider > iInFlight, "分隔线应在在途块之后");
      assert.ok(iBody > iDivider, "正文应在分隔线之后");
    });
  });
});
