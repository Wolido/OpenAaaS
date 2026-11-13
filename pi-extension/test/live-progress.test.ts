/**
 * get_task「执行器内进度（live）」展示测试（TDD 红阶段）
 *
 * 被测入口: ../index.ts 的 default 导出工厂 `(pi: ExtensionAPI) => void`
 *
 * 背景（新协议：facts 整个删除）：服务端 GET /api/v1/client/tasks/{id}
 * 响应新增可选字段 live，只含：
 *   {
 *     text: string,       // 原始进度日志（多行）
 *     stale: boolean,     // true 表示进度可能已过期
 *     source, truncated,  // 其它元信息
 *   }
 * facts 已由协议删除；updated_at / elapsed_secs / idle_secs 亦已删除，
 * 客户端不得依赖。无进度时该字段缺席。
 *
 * 期望契约（本测试定义，供 coder 实现）：
 *   1. live 存在且 stale !== true → 进度块首行严格为「进度：」，其后只有 text。
 *   2. live 存在且 stale === true → 进度块首行严格为「进度（可能不是最新）：」，其后只有 text；
 *      输出不得出现「可能已过期」，也不得泄露协议已删除的 source 字段。
 *   3. live 缺席 → 无进度块，原有状态行仍在。
 *   4. live.text 为空（"" 或缺省）→ 无进度块（即使负载里仍带着 facts 也不展示）。
 *   5. 输出中不得出现「facts」字样；也不得含 undefined / NaN 等占位文本。
 *   6. 旧 server 负载带额外历史字段（facts、updated_at 等）不报错，也不展示这些字段。
 *   7. 进度块（首行 + text）超过 800 个字符时按码点截断：前 799 个字符 + 「…」，
 *      总长 ≤ 800，且不得切坏 emoji（不得出现半个代理对）。
 *
 * 测试方式：参考 test/tool-exposure.test.ts，用 stub pi 捕获 registerTool 的
 * 工具定义并真实调用 execute；config.json 通过临时 HOME 注入；safeFetch 底层用
 * 全局 fetch，故替换 globalThis.fetch 返回构造好的 GET /tasks/{id} 响应。
 */

import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

interface CapturedToolDefinition {
  name?: string;
  execute?: (
    toolCallId: string,
    params: Record<string, unknown>,
    signal?: unknown,
    onUpdate?: unknown,
    ctx?: unknown,
  ) => Promise<unknown>;
  [key: string]: unknown;
}

function makeStubPi(capturedTools: CapturedToolDefinition[]) {
  const noop = () => {};
  const target = {
    on: noop,
    registerTool: (definition: CapturedToolDefinition) => {
      capturedTools.push(definition);
    },
    registerCommand: noop,
    sendMessage: noop,
    appendEntry: noop,
  };

  return new Proxy(target, {
    get(obj, prop, receiver) {
      if (prop in obj) {
        return Reflect.get(obj, prop, receiver);
      }
      return noop;
    },
  }) as never;
}

/** 承载临时 HOME 与被替换的全局 fetch，测试结束后统一还原 */
let tempHome: string;
let originalHome: string | undefined;
let originalFetch: typeof globalThis.fetch;

/** 已导入的扩展 execute 入口（模块级缓存，避免重复导入/重复配置） */
let executeTool: CapturedToolDefinition["execute"];

before(async () => {
  // Arrange：临时 HOME + 一份带 api_key 的 config.json（index.ts 在模块加载期解析该路径）
  originalHome = process.env.HOME;
  tempHome = mkdtempSync(join(tmpdir(), "openaaas-live-progress-"));
  const configDir = join(tempHome, ".pi", "agent", "openaaas");
  mkdirSync(configDir, { recursive: true });
  writeFileSync(
    join(configDir, "config.json"),
    JSON.stringify({
      servers: {
        default: {
          server_url: "https://api.example.com",
          api_key: "test-key",
        },
      },
      default_server: "default",
    }),
    "utf8",
  );
  process.env.HOME = tempHome;

  const capturedTools: CapturedToolDefinition[] = [];
  const plugin = (await import("../index.ts")).default as unknown as (
    pi: unknown,
  ) => void;
  plugin(makeStubPi(capturedTools));

  const tool = capturedTools.find((t) => t.name === "OpenAaaS");
  assert.ok(tool, "未捕获到 name 为 \"OpenAaaS\" 的工具注册定义");
  assert.ok(typeof tool.execute === "function", "OpenAaaS 工具定义缺少 execute");
  executeTool = tool.execute;
});

after(() => {
  if (originalHome === undefined) {
    delete process.env.HOME;
  } else {
    process.env.HOME = originalHome;
  }
  if (originalFetch) {
    globalThis.fetch = originalFetch;
  }
  rmSync(tempHome, { recursive: true, force: true });
});

/**
 * 以给定任务响应体执行 get_task，返回输出文本。
 * 替换 globalThis.fetch 模拟 GET /api/v1/client/tasks/{id}。
 */
async function runGetTask(
  taskPayload: Record<string, unknown>,
): Promise<string> {
  originalFetch = globalThis.fetch;
  globalThis.fetch = (async () =>
    new Response(JSON.stringify(taskPayload), {
      status: 200,
      headers: { "content-type": "application/json" },
    })) as unknown as typeof globalThis.fetch;

  try {
    const result = (await executeTool!(
      "call-live-progress",
      { action: "get_task", task_id: "task-live-1" },
      undefined,
      undefined,
      {},
    )) as { content: Array<{ type: string; text: string }> };

    const textBlock = result.content.find((b) => b.type === "text");
    assert.ok(textBlock, "get_task 输出中没有 text content");
    return textBlock.text;
  } finally {
    globalThis.fetch = originalFetch;
  }
}

/** 定位进度块首行（输出首行是「任务状态: …」，进度标题行以「进度」开头） */
function progressHeader(text: string): string {
  const line = text.split(/\r?\n/).find((l) => l.trim().startsWith("进度"));
  return line ? line.trim() : "";
}

/** 整体输出中不得出现的占位文本 */
const PLACEHOLDERS = ["undefined", "NaN"] as const;

function assertNoPlaceholder(output: string) {
  for (const forbidden of PLACEHOLDERS) {
    assert.ok(
      !output.includes(forbidden),
      `输出不应含「${forbidden}」占位文本\n实际输出:\n${output}`,
    );
  }
}

/** 新协议已删除 facts：任何情况下输出都不得出现该字样 */
function assertNoFactsWord(output: string) {
  assert.ok(
    !output.includes("facts"),
    `输出不得出现「facts」字样（该字段已从协议删除）\n实际输出:\n${output}`,
  );
}

/** 新协议已删除 source 字段，且 stale 文案改为「可能不是最新」 */
function assertNoDeprecatedStaleWordingAndNoSource(output: string) {
  assert.ok(
    !output.includes("可能已过期"),
    `输出不得出现「可能已过期」（stale 文案应为「可能不是最新」）\n实际输出:\n${output}`,
  );
  assert.ok(
    !output.includes("source"),
    `输出不得出现「source」字样（该字段已从协议删除）\n实际输出:\n${output}`,
  );
}

/** 取进度块（进度标题行 + 其后正文）；找不到标题行时断言失败 */
function progressBlock(text: string): string {
  const lines = text.split(/\r?\n/);
  const index = lines.findIndex((l) => l.trim().startsWith("进度"));
  assert.notStrictEqual(
    index,
    -1,
    `未找到进度标题行\n实际输出:\n${text}`,
  );
  return lines.slice(index).join("\n");
}

const LEGACY_FACTS = "阶段 step1 · 输出 3 个文件 · 容器在跑";
const TEXT_LINE_1 = "phase=step1 开始解析";
const TEXT_LINE_2 = "note=已解析完 12 个 CSV";
const LIVE_TEXT = `${TEXT_LINE_1}\n${TEXT_LINE_2}`;

describe("get_task 执行器内进度（live）展示", () => {
  it("应在 live 存在且非 stale 时以「进度：」为首行并原样展示多行 text", async () => {
    // Arrange：新协议 live，只含 text / stale / source / truncated
    const payload = {
      id: "task-live-1",
      status: "running",
      live: {
        text: LIVE_TEXT,
        stale: false,
        source: "live",
        truncated: false,
      },
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：进度块首行严格为「进度：」
    assert.strictEqual(
      progressHeader(output),
      "进度：",
      `非 stale 时进度块首行应严格为「进度：」\n实际首行: ${progressHeader(output)}\n实际输出:\n${output}`,
    );

    // Assert 2：text 的多行内容原样出现
    assert.ok(
      output.includes(LIVE_TEXT),
      `输出应按原样包含 live.text 多行内容\n实际输出:\n${output}`,
    );

    // Assert 3：不得出现 facts 字样、占位文本
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });

  it("应在 live.stale 为 true 时以「进度（可能不是最新）：」为首行并仍展示 text", async () => {
    // Arrange
    const payload = {
      id: "task-live-2",
      status: "running",
      live: {
        text: LIVE_TEXT,
        stale: true,
        source: "live",
        truncated: false,
      },
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：首行严格为「进度（可能不是最新）：」
    assert.strictEqual(
      progressHeader(output),
      "进度（可能不是最新）：",
      `stale 时进度块首行应严格为「进度（可能不是最新）：」\n实际首行: ${progressHeader(output)}\n实际输出:\n${output}`,
    );

    // Assert 2：text 仍展示
    assert.ok(
      output.includes(TEXT_LINE_1) && output.includes(TEXT_LINE_2),
      `stale 时应仍展示 live.text 全部行\n实际输出:\n${output}`,
    );

    // Assert 3：不得出现 facts 字样、占位文本
    assertNoFactsWord(output);
    assertNoPlaceholder(output);

    // Assert 4：不得出现旧文案「可能已过期」，也不得展示负载里的 source 字段
    assertNoDeprecatedStaleWordingAndNoSource(output);
  });

  it("应在 live 缺席时保持原有状态输出且不含任何进度块", async () => {
    // Arrange：无 live 字段
    const payload = {
      id: "task-live-3",
      status: "running",
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：原有状态行仍在
    assert.ok(
      output.includes("任务状态: 执行中"),
      `缺少原有状态行「任务状态: 执行中」\n实际输出:\n${output}`,
    );

    // Assert 2：不含任何进度块
    assert.strictEqual(
      progressHeader(output),
      "",
      `live 缺席时不应出现进度标题行\n实际输出:\n${output}`,
    );

    // Assert 3：不得出现 facts 字样、占位文本
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });

  it("应在 live.text 为空时不显示进度块，即使负载仍带着 facts", async () => {
    // Arrange：text 为空串，旧 server 还带着 facts
    const payload = {
      id: "task-live-empty-text",
      status: "running",
      live: {
        text: "",
        stale: false,
        source: "live",
        truncated: false,
        facts: LEGACY_FACTS,
      },
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：无进度块
    assert.strictEqual(
      progressHeader(output),
      "",
      `live.text 为空时不应出现进度标题行\n实际输出:\n${output}`,
    );

    // Assert 2：历史字段 facts 不展示
    assert.ok(
      !output.includes(LEGACY_FACTS),
      `live.text 为空时不应展示 facts 内容\n实际输出:\n${output}`,
    );

    // Assert 3：原有的状态行仍在，且不含 facts 字样、占位文本
    assert.ok(
      output.includes("任务状态: 执行中"),
      `缺少原有状态行「任务状态: 执行中」\n实际输出:\n${output}`,
    );
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });

  it("应在 live.text 缺省时不显示进度块，即使负载仍带着 facts", async () => {
    // Arrange：text 字段缺省，旧 server 还带着 facts
    const payload = {
      id: "task-live-missing-text",
      status: "running",
      live: {
        stale: false,
        source: "live",
        truncated: false,
        facts: LEGACY_FACTS,
      },
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：无进度块
    assert.strictEqual(
      progressHeader(output),
      "",
      `live.text 缺省时不应出现进度标题行\n实际输出:\n${output}`,
    );

    // Assert 2：历史字段 facts 不展示
    assert.ok(
      !output.includes(LEGACY_FACTS),
      `live.text 缺省时不应展示 facts 内容\n实际输出:\n${output}`,
    );

    // Assert 3：不得出现 facts 字样、占位文本
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });

  it("应在旧 server 负载带额外历史字段时不报错且只展示 text", async () => {
    // Arrange：旧 server 额外带 facts 与 updated_at，新协议客户端不得依赖或展示它们
    const payload = {
      id: "task-live-legacy",
      status: "running",
      live: {
        text: LIVE_TEXT,
        stale: false,
        source: "live",
        truncated: false,
        facts: LEGACY_FACTS,
        updated_at: "2024-01-01T12:00:00Z",
      },
    };

    // Act（不得抛异常）
    const output = await runGetTask(payload);

    // Assert 1：仍以「进度：」为首行，且 text 展示
    assert.strictEqual(
      progressHeader(output),
      "进度：",
      `旧 server 负载也应输出「进度：」首行\n实际首行: ${progressHeader(output)}\n实际输出:\n${output}`,
    );
    assert.ok(
      output.includes(TEXT_LINE_2),
      `输出缺少 live.text 内容\n实际输出:\n${output}`,
    );

    // Assert 2：历史字段不展示，且不含 facts 字样、占位文本
    assert.ok(
      !output.includes(LEGACY_FACTS),
      `不应展示旧 server 的历史字段 facts 内容\n实际输出:\n${output}`,
    );
    assert.ok(
      !output.includes("2024-01-01T12:00:00Z"),
      `不应展示旧 server 的历史字段 updated_at\n实际输出:\n${output}`,
    );
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });

  it("应在 live.text 超过 800 字符时按码点截断且不切坏 emoji", async () => {
    // Arrange：两个 emoji 紧邻第 800 个 UTF-16 码元，按码元 slice 会切出半个代理对
    const longText = `${"x".repeat(793)}🙂🙂${"y".repeat(200)}`;
    assert.ok(
      Array.from(longText).length > 800,
      "用例前置条件：live.text 的码点数应明显超过 800",
    );
    const payload = {
      id: "task-live-long",
      status: "running",
      live: {
        text: longText,
        stale: false,
        source: "live",
        truncated: true,
      },
    };

    // Act
    const output = await runGetTask(payload);

    // Assert 1：进度块按码点计数不超过 800 个字符（Array.from 保证按码点而非码元计数）
    const block = progressBlock(output);
    const blockChars = Array.from(block).length;
    assert.ok(
      blockChars <= 800,
      `进度块应按码点截断到 ≤ 800 个字符，实际 ${blockChars} 个\n块尾: ${block.slice(-20)}`,
    );

    // Assert 2：截断后以省略号结尾
    assert.ok(
      block.endsWith("…"),
      `截断后的进度块应以「…」结尾\n块尾: ${block.slice(-20)}`,
    );

    // Assert 3：不得出现半个代理对（for...of 按码点迭代，孤立代理码元会单独取出）
    for (const ch of block) {
      const codePoint = ch.codePointAt(0) ?? 0;
      assert.ok(
        codePoint < 0xd800 || codePoint > 0xdfff,
        `进度块出现半个代理对（孤立码元 U+${codePoint.toString(16)}）\n块尾: ${block.slice(-20)}`,
      );
    }

    // Assert 4：不得出现 facts 字样、占位文本
    assertNoFactsWord(output);
    assertNoPlaceholder(output);
  });
});
