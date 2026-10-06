/**
 * index.ts 扩展入口「工具注册定义」契约 / 回归测试
 *
 * 被测入口: ../index.ts 的 default 导出工厂 `(pi: ExtensionAPI) => void`
 *
 * 契约：工厂被调用时，通过 pi.registerTool 注册的 OpenAaaS 工具定义必须声明
 *   exposure: "model-only"
 *
 * 背景：OpenAaaS 是异步工具（submit_task 立即返回，真实结果稍后以
 * [OpenAaaS-task-result] 通知到达）。pi 启用 codemode 后，脚本可通过
 * ctx.executeTool() 调用已注册工具，但脚本里调用异步工具必然拿不到结果、
 * 造成静默错误。exposure: "model-only" 的语义是：仍声明给模型可直接调用，
 * 但永不被脚本通过 ctx.executeTool() 调用。
 *
 * 历史说明：index.ts 的 OpenAaaS 注册定义加入 exposure 字段前，第二条断言为
 * 红（expected "model-only"，actual undefined）；加入该字段后转绿，并作为契约 /
 * 回归测试长期保留，防止该字段被误删或改回。
 *
 * 测试方式：真实调用扩展入口工厂（而非正则/文本扫描源码），用 stub pi 捕获
 * registerTool 收到的工具定义。工厂在注册期只用到 pi.on / pi.registerTool /
 * pi.registerCommand（sendMessage、appendEntry 等只在 handler 内部触发，调用时
 * 不会执行）；stub 以显式方法覆盖已知 API，未知属性经 Proxy 兜底为 no-op 函数。
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

/** 捕获到的工具注册定义（只关心 name / exposure，其余字段原样保留） */
interface CapturedToolDefinition {
  name?: string;
  exposure?: unknown;
  [key: string]: unknown;
}

/**
 * 构造真实入口工厂可安全调用的 stub pi：
 * - registerTool 真实捕获工具定义到 capturedTools；
 * - on / registerCommand / sendMessage / appendEntry 为 no-op；
 * - 未知属性经 Proxy 兜底为 no-op 函数（注册期实际不会用到）。
 */
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
  });
}

describe("OpenAaaS 工具注册定义的 exposure 契约", () => {
  it('应注册 name 为 "OpenAaaS" 且 exposure 为 "model-only" 的工具定义', async () => {
    // Arrange：捕获容器 + stub pi
    const capturedTools: CapturedToolDefinition[] = [];
    const stubPi = makeStubPi(capturedTools);
    const plugin = (await import("../index.ts")).default as unknown as (
      pi: unknown,
    ) => void;

    // Act：真实调用扩展入口工厂
    plugin(stubPi);

    // Assert 1：先确认捕获到 OpenAaaS 定义（用于区分「stub 写错」与「字段缺失」）
    const openAaaSTool = capturedTools.find((tool) => tool.name === "OpenAaaS");
    assert.ok(
      openAaaSTool,
      `未捕获到 name 为 "OpenAaaS" 的工具注册定义，实际捕获 ${capturedTools.length} 个: [${
        capturedTools.map((tool) => String(tool.name)).join(", ")
      }]`,
    );

    // Assert 2：异步工具必须为 model-only，禁止 codemode 脚本经 ctx.executeTool() 调用
    assert.equal(
      openAaaSTool.exposure,
      "model-only",
      `OpenAaaS 工具定义曝光级别错误：expected "model-only"，actual ${String(
        openAaaSTool.exposure,
      )}（异步工具不可被 codemode 脚本通过 ctx.executeTool() 调用）`,
    );
  });
});
