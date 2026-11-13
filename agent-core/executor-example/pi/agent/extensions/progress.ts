/**
 * Progress Extension - 执行器内进度上报
 *
 * 向查看进度的人暴露一个 `aaas_progress` 工具，把模型的一句话进度追加写入
 * /workspace/.open-aaas/progress.jsonl。
 *
 * 刻意不 import 任何包（包括 pi 自身），只用 node 内置模块，避免与镜像里
 * 全局安装的 pi 版本产生类型/运行时耦合。所有写入失败都被吞掉，不影响任务。
 */

import { appendFileSync, mkdirSync } from "node:fs";

const PROGRESS_DIR = "/workspace/.open-aaas";
const PROGRESS_FILE = `${PROGRESS_DIR}/progress.jsonl`;

const MAX_NOTE_LENGTH = 60;

const NoteParams = {
	type: "object",
	properties: {
		note: { type: "string" },
	},
	required: ["note"],
};

const DESCRIPTION = `用途：向查看本次任务进度的人报告一句话进度，写入任务 workspace 的进度文件。

示例：
- aaas_progress note="已读取输入文件，开始整理数据结构"
- aaas_progress note="数据清洗完成，接下来生成汇总表"
- aaas_progress note="汇总表已写入 output，开始写最终说明"`;

const PROMPT_SNIPPET = "用 aaas_progress 一句话汇报进度（最多 60 字，做完什么 + 下一步做什么）";

const PROMPT_GUIDELINES = [
	"aaas_progress: 开始动手前先报一次，让查看进度的人知道任务已启动。",
	"aaas_progress: 每完成一个有意义的阶段报一次，看起来小的阶段也要报。",
	"aaas_progress: 一句话最多 60 字，只写「做完什么 + 下一步做什么」。",
	"aaas_progress: 不写工具名、命令、文件内容或推理过程。",
	"aaas_progress: 不要把它当输出通道，任务结果仍然写入 /workspace/output/。",
	"aaas_progress: 同一个意思不要重复报，进度有推进时再报。",
	"aaas_progress: 它没有副作用，写入失败会被静默吞掉，不必为此重试或检查。",
];

const ISO = () => new Date().toISOString();

export default function (pi: any) {
	try {
		let lastNote = "";

		const ensureDir = () => {
			mkdirSync(PROGRESS_DIR, { recursive: true });
		};

		pi.registerTool({
			name: "aaas_progress",
			label: "Progress",
			description: DESCRIPTION,
			parameters: NoteParams,
			promptSnippet: PROMPT_SNIPPET,
			promptGuidelines: PROMPT_GUIDELINES,

			async execute(_toolCallId: string, params: any, _signal: any, _onUpdate: any, _ctx: any) {
				const ok = { content: [{ type: "text", text: "ok" }], details: {} };
				try {
					const raw = typeof params?.note === "string" ? params.note : "";
					const flat = raw.replace(/\s+/g, " ").trim();
					const chars = Array.from(flat);
					const note = chars.length > MAX_NOTE_LENGTH
						? chars.slice(0, MAX_NOTE_LENGTH - 1).join("") + "…"
						: flat;
					if (!note) return ok;
					if (note === lastNote) return ok;
					ensureDir();
					appendFileSync(PROGRESS_FILE, `${ISO()} note=${note}\n`);
					lastNote = note;
				} catch {
					// 进度写失败不影响任务
				}
				return ok;
			},
		});
	} catch {
		// 扩展加载失败不影响任务
	}
}
