/**
 * Subagent Tool - Delegate tasks to specialized agents
 *
 * Spawns a separate `pi` process for each subagent invocation,
 * giving it an isolated context window.
 *
 * Supports two modes:
 *   - Single: { agent: "name", task: "..." }
 *   - Chain: { chain: [{ agent: "name", task: "... {previous} ..." }, ...] }
 *
 * Uses JSON mode to capture structured output from subagents.
 *
 * Modified: per-agent skill directory isolation via --no-skills --skill args.
 */

import { spawn } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { AgentToolResult } from "@mariozechner/pi-agent-core";
import type { Message } from "@mariozechner/pi-ai";
import { StringEnum } from "@mariozechner/pi-ai";
import {
	type ExtensionAPI,
	getMarkdownTheme,
	withFileMutationQueue,
	getAgentDir,
	parseFrontmatter,
} from "@mariozechner/pi-coding-agent";
import { Container, Markdown, Spacer, Text } from "@mariozechner/pi-tui";
import { Type } from "@sinclair/typebox";

// ===== Inlined agents.ts with skills support =====

type AgentScope = "user" | "project" | "both";

interface AgentConfig {
	name: string;
	description: string;
	tools?: string[];
	model?: string;
	skills?: string[];
	canDelegate?: boolean;
	systemPrompt: string;
	source: "user" | "project";
	filePath: string;
}

interface AgentDiscoveryResult {
	agents: AgentConfig[];
	projectAgentsDir: string | null;
}

function loadAgentsFromDir(dir: string, source: "user" | "project"): AgentConfig[] {
	const agents: AgentConfig[] = [];
	if (!fs.existsSync(dir)) return agents;
	let entries: fs.Dirent[];
	try {
		entries = fs.readdirSync(dir, { withFileTypes: true });
	} catch {
		return agents;
	}
	for (const entry of entries) {
		if (!entry.name.endsWith(".md")) continue;
		if (!entry.isFile() && !entry.isSymbolicLink()) continue;
		const filePath = path.join(dir, entry.name);
		let content: string;
		try {
			content = fs.readFileSync(filePath, "utf-8");
		} catch {
			continue;
		}
		const { frontmatter, body } = parseFrontmatter<Record<string, string>>(content);
		if (!frontmatter.name || !frontmatter.description) continue;
		const tools = frontmatter.tools
			?.split(",")
			.map((t: string) => t.trim())
			.filter(Boolean);
		const hasSkills = "skills" in frontmatter;
		const skills = hasSkills
			? (frontmatter.skills
				?.split(",")
				.map((s: string) => s.trim())
				.filter(Boolean) ?? [])
			: undefined;
		const canDelegate =
			frontmatter.canDelegate !== undefined
				? String(frontmatter.canDelegate).toLowerCase().trim() !== "false"
				: undefined;
		agents.push({
			name: frontmatter.name,
			description: frontmatter.description,
			tools: tools && tools.length > 0 ? tools : undefined,
			model: frontmatter.model,
			skills,
			canDelegate,
			systemPrompt: body,
			source,
			filePath,
		});
	}
	return agents;
}

function isDirectory(p: string): boolean {
	try {
		return fs.statSync(p).isDirectory();
	} catch {
		return false;
	}
}

function findNearestProjectAgentsDir(cwd: string): string | null {
	let currentDir = cwd;
	while (true) {
		const candidate = path.join(currentDir, ".pi", "agents");
		if (isDirectory(candidate)) return candidate;
		const parentDir = path.dirname(currentDir);
		if (parentDir === currentDir) return null;
		currentDir = parentDir;
	}
}

function discoverAgents(cwd: string, scope: AgentScope): AgentDiscoveryResult {
	const userDir = path.join(getAgentDir(), "agents");
	const projectAgentsDir = findNearestProjectAgentsDir(cwd);
	const userAgents = scope === "project" ? [] : loadAgentsFromDir(userDir, "user");
	const projectAgents =
		scope === "user" || !projectAgentsDir ? [] : loadAgentsFromDir(projectAgentsDir, "project");
	const agentMap = new Map<string, AgentConfig>();
	if (scope === "both") {
		for (const agent of userAgents) agentMap.set(agent.name, agent);
		for (const agent of projectAgents) agentMap.set(agent.name, agent);
	} else if (scope === "user") {
		for (const agent of userAgents) agentMap.set(agent.name, agent);
	} else {
		for (const agent of projectAgents) agentMap.set(agent.name, agent);
	}
	return { agents: Array.from(agentMap.values()), projectAgentsDir };
}

function formatAgentList(
	agents: AgentConfig[],
	maxItems: number,
): { text: string; remaining: number } {
	if (agents.length === 0) return { text: "none", remaining: 0 };
	const listed = agents.slice(0, maxItems);
	const remaining = agents.length - listed.length;
	return {
		text: listed.map((a) => `${a.name} (${a.source}): ${a.description}`).join("; "),
		remaining,
	};
}

// ===== Original index.ts =====

const COLLAPSED_ITEM_COUNT = 10;

function formatTokens(count: number): string {
	if (count < 1000) return count.toString();
	if (count < 10000) return `${(count / 1000).toFixed(1)}k`;
	if (count < 1000000) return `${Math.round(count / 1000)}k`;
	return `${(count / 1000000).toFixed(1)}M`;
}

function formatPhase(phase: string): string {
	if (phase === "thinking") return "🤔 thinking...";
	if (phase === "waiting") return "⏳ waiting for next step...";
	if (phase.startsWith("tooling:")) {
		const tool = phase.slice(8);
		return `⚡ ${tool}...`;
	}
	return "(running...)";
}

function formatUsageStats(
	usage: {
		input: number;
		output: number;
		cacheRead: number;
		cacheWrite: number;
		cost: number;
		contextTokens?: number;
		turns?: number;
	},
	model?: string,
): string {
	const parts: string[] = [];
	if (usage.turns) parts.push(`${usage.turns} turn${usage.turns > 1 ? "s" : ""}`);
	if (usage.input) parts.push(`↑${formatTokens(usage.input)}`);
	if (usage.output) parts.push(`↓${formatTokens(usage.output)}`);
	if (usage.cacheRead) parts.push(`R${formatTokens(usage.cacheRead)}`);
	if (usage.cacheWrite) parts.push(`W${formatTokens(usage.cacheWrite)}`);
	if (usage.cost) parts.push(`$${usage.cost.toFixed(4)}`);
	if (usage.contextTokens && usage.contextTokens > 0) {
		parts.push(`ctx:${formatTokens(usage.contextTokens)}`);
	}
	if (model) parts.push(model);
	return parts.join(" ");
}

function formatToolCall(
	toolName: string,
	args: Record<string, unknown>,
	themeFg: (color: any, text: string) => string,
): string {
	const shortenPath = (p: string) => {
		const home = os.homedir();
		return p.startsWith(home) ? `~${p.slice(home.length)}` : p;
	};

	switch (toolName) {
		case "bash": {
			const command = (args.command as string) || "...";
			const preview = command.length > 60 ? `${command.slice(0, 60)}...` : command;
			return themeFg("muted", "$ ") + themeFg("toolOutput", preview);
		}
		case "read": {
			const rawPath = (args.file_path || args.path || "...") as string;
			const filePath = shortenPath(rawPath);
			const offset = args.offset as number | undefined;
			const limit = args.limit as number | undefined;
			let text = themeFg("accent", filePath);
			if (offset !== undefined || limit !== undefined) {
				const startLine = offset ?? 1;
				const endLine = limit !== undefined ? startLine + limit - 1 : "";
				text += themeFg("warning", `:${startLine}${endLine ? `-${endLine}` : ""}`);
			}
			return themeFg("muted", "read ") + text;
		}
		case "write": {
			const rawPath = (args.file_path || args.path || "...") as string;
			const filePath = shortenPath(rawPath);
			const content = (args.content || "") as string;
			const lines = content.split("\n").length;
			let text = themeFg("muted", "write ") + themeFg("accent", filePath);
			if (lines > 1) text += themeFg("dim", ` (${lines} lines)`);
			return text;
		}
		case "edit": {
			const rawPath = (args.file_path || args.path || "...") as string;
			return themeFg("muted", "edit ") + themeFg("accent", shortenPath(rawPath));
		}
		case "ls": {
			const rawPath = (args.path || ".") as string;
			return themeFg("muted", "ls ") + themeFg("accent", shortenPath(rawPath));
		}
		case "find": {
			const pattern = (args.pattern || "*") as string;
			const rawPath = (args.path || ".") as string;
			return (
				themeFg("muted", "find ") +
				themeFg("accent", pattern) +
				themeFg("dim", ` in ${shortenPath(rawPath)}`)
			);
		}
		case "grep": {
			const pattern = (args.pattern || "") as string;
			const rawPath = (args.path || ".") as string;
			return (
				themeFg("muted", "grep ") +
				themeFg("accent", `/${pattern}/`) +
				themeFg("dim", ` in ${shortenPath(rawPath)}`)
			);
		}
		default: {
			let argsStr: string;
			try {
				argsStr = JSON.stringify(args);
			} catch {
				argsStr = "[unserializable]";
			}
			const preview = argsStr.length > 50 ? `${argsStr.slice(0, 50)}...` : argsStr;
			return themeFg("accent", toolName) + themeFg("dim", ` ${preview}`);
		}
	}
}

interface UsageStats {
	input: number;
	output: number;
	cacheRead: number;
	cacheWrite: number;
	cost: number;
	contextTokens: number;
	turns: number;
}

interface SingleResult {
	agent: string;
	agentSource: "user" | "project" | "unknown";
	task: string;
	exitCode: number;
	messages: Message[];
	stderr: string;
	usage: UsageStats;
	model?: string;
	stopReason?: string;
	errorMessage?: string;
	step?: number;
	phase: "idle" | "thinking" | `tooling:${string}` | "waiting";
	lastPhaseChange: number;
	thinkingBuffer?: string;
}

interface SubagentDetails {
	mode: "single" | "chain";
	agentScope: AgentScope;
	projectAgentsDir: string | null;
	results: SingleResult[];
}

function getFinalOutput(messages: Message[]): string {
	for (let i = messages.length - 1; i >= 0; i--) {
		const msg = messages[i];
		if (msg.role === "assistant") {
			for (const part of msg.content) {
				if (part.type === "text") return part.text;
			}
		}
	}
	return "";
}

type DisplayItem = { type: "text"; text: string } | { type: "toolCall"; name: string; args: Record<string, any> };

function getDisplayItems(messages: Message[]): DisplayItem[] {
	const items: DisplayItem[] = [];
	for (const msg of messages) {
		if (msg.role === "assistant") {
			for (const part of msg.content) {
				if (part.type === "text") items.push({ type: "text", text: part.text });
				else if (part.type === "toolCall")
					items.push({ type: "toolCall", name: part.name, args: part.arguments });
			}
		}
	}
	return items;
}

function formatSubagentDiagnostics(result: SingleResult, maxTraceItems = 15): string {
	const lines: string[] = [];
	lines.push(`Agent: ${result.agent} (${result.agentSource})`);
	lines.push(`Exit code: ${result.exitCode}`);
	if (result.stopReason) lines.push(`Stop reason: ${result.stopReason}`);
	if (result.errorMessage) lines.push(`Error message: ${result.errorMessage}`);

	const stderr = result.stderr.trim();
	if (stderr) {
		lines.push(`Stderr:\n${stderr.slice(0, 800)}${stderr.length > 800 ? "\n..." : ""}`);
	}

	const items = getDisplayItems(result.messages);
	if (items.length > 0) {
		lines.push("Execution trace:");
		const start = Math.max(0, items.length - maxTraceItems);
		if (start > 0) lines.push(`  ... ${start} earlier items omitted`);
		for (let i = start; i < items.length; i++) {
			const item = items[i];
			if (item.type === "text") {
				const text = item.text.trim();
				if (text) {
					const firstLine = text.split("\n")[0];
					const preview = firstLine.slice(0, 120);
					lines.push(`  [text] ${preview}${firstLine.length > 120 ? "..." : ""}`);
				}
			} else {
				let argsStr: string;
				try {
					argsStr = JSON.stringify(item.args);
				} catch {
					argsStr = "[unserializable]";
				}
				const preview = argsStr.slice(0, 100);
				lines.push(`  [tool] ${item.name}: ${preview}${argsStr.length > 100 ? "..." : ""}`);
			}
		}
	}

	const finalOutput = getFinalOutput(result.messages);
	if (finalOutput && finalOutput !== result.errorMessage) {
		lines.push(`Final output:\n${finalOutput.trim().slice(0, 500)}${finalOutput.length > 500 ? "\n..." : ""}`);
	}

	return lines.join("\n");
}

async function writePromptToTempFile(
	agentName: string,
	prompt: string,
): Promise<{ dir: string; filePath: string }> {
	const tmpDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), "pi-subagent-"));
	const safeName = agentName.replace(/[^\w.-]+/g, "_");
	const filePath = path.join(tmpDir, `prompt-${safeName}.md`);
	await withFileMutationQueue(filePath, async () => {
		await fs.promises.writeFile(filePath, prompt, { encoding: "utf-8", mode: 0o600 });
	});
	return { dir: tmpDir, filePath };
}

function getPiInvocation(args: string[]): { command: string; args: string[] } {
	const currentScript = process.argv[1];
	if (currentScript && fs.existsSync(currentScript)) {
		return { command: process.execPath, args: [currentScript, ...args] };
	}

	const execName = path.basename(process.execPath).toLowerCase();
	const isGenericRuntime = /^(node|bun)(\.exe)?$/.test(execName);
	if (!isGenericRuntime) {
		return { command: process.execPath, args };
	}

	return { command: "pi", args };
}

type OnUpdateCallback = (partial: AgentToolResult<SubagentDetails>) => void;

async function runSingleAgent(
	defaultCwd: string,
	agents: AgentConfig[],
	agentName: string,
	task: string,
	cwd: string | undefined,
	step: number | undefined,
	signal: AbortSignal | undefined,
	onUpdate: OnUpdateCallback | undefined,
	makeDetails: (results: SingleResult[]) => SubagentDetails,
): Promise<SingleResult> {
	const agent = agents.find((a) => a.name === agentName);

	if (!agent) {
		const available = agents.map((a) => `"${a.name}"`).join(", ") || "none";
		return {
			agent: agentName,
			agentSource: "unknown",
			task,
			exitCode: 1,
			messages: [],
			stderr: `Unknown agent: "${agentName}". Available agents: ${available}.`,
			usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, contextTokens: 0, turns: 0 },
			step,
			phase: "idle",
			lastPhaseChange: Date.now(),
		};
	}

	const args: string[] = ["--mode", "json", "-p", "--no-session"];
	if (agent.model) args.push("--model", agent.model);
	if (agent.tools && agent.tools.length > 0) args.push("--tools", agent.tools.join(","));

	// MODIFIED: inject per-agent skill isolation
	if (agent.skills !== undefined) {
		args.push("--no-skills");
		if (agent.skills.length > 0) {
			const baseDir = cwd ?? defaultCwd;
			for (const skillPath of agent.skills) {
				const resolved = skillPath.startsWith("~/")
				    ? path.join(os.homedir(), skillPath.slice(2))
				    : path.isAbsolute(skillPath)
				        ? skillPath
				        : path.resolve(baseDir, skillPath);
				args.push("--skill", resolved);
			}
		}
	}

	let tmpPromptDir: string | null = null;
	let tmpPromptPath: string | null = null;

	const currentResult: SingleResult = {
		agent: agentName,
		agentSource: agent.source,
		task,
		exitCode: 0,
		messages: [],
		stderr: "",
		usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, contextTokens: 0, turns: 0 },
		model: agent.model,
		step,
		phase: "idle",
		lastPhaseChange: Date.now(),
	};

	const emitUpdate = () => {
		if (emitTimer) { clearTimeout(emitTimer); emitTimer = null; }
		if (onUpdate) {
			onUpdate({
				content: [{ type: "text", text: getFinalOutput(currentResult.messages) || formatPhase(currentResult.phase) }],
				details: makeDetails([currentResult]),
			});
		}
	};

	let emitTimer: ReturnType<typeof setTimeout> | null = null;
	const throttledEmitUpdate = () => {
		if (emitTimer) return;
		emitTimer = setTimeout(() => {
			emitTimer = null;
			emitUpdate();
		}, 100);
	};

	try {
		if (agent.systemPrompt.trim()) {
			const tmp = await writePromptToTempFile(agent.name, agent.systemPrompt);
			tmpPromptDir = tmp.dir;
			tmpPromptPath = tmp.filePath;
			args.push("--append-system-prompt", tmpPromptPath);
		}

		args.push(`Task: ${task}`);
		let wasAborted = false;

		const POST_EXIT_GRACE_MS = 500;
		const ABORT_FORCE_TIMEOUT_MS = 2000;
		const DEFAULT_ACTIVITY_TIMEOUT_MS = 600_000;
		const DEFAULT_HARD_TIMEOUT_MS = 1_800_000;

		const parseEnvInt = (raw: string | undefined, fallback: number): number => {
			const parsed = parseInt(raw || String(fallback), 10);
			return Number.isNaN(parsed) ? fallback : parsed;
		};

		const exitCode = await new Promise<number>((resolve) => {
			const invocation = getPiInvocation(args);
			const currentDepth = parseInt(process.env.PI_SUBAGENT_DEPTH || "0", 10);
			const proc = spawn(invocation.command, invocation.args, {
				cwd: cwd ?? defaultCwd,
				shell: false,
				stdio: ["ignore", "pipe", "pipe"],
				env: {
					...process.env,
					PI_SUBAGENT_DEPTH: String(currentDepth + 1),
					PI_CAN_DELEGATE: String(agent.canDelegate === true),
					PI_CURRENT_AGENT_NAME: agent.name,
				},
			});
			let buffer = "";
			let resolved = false;
			let exitCodeValue: number | null = null;
			let stdoutEnded = false;
			let postExitTimer: ReturnType<typeof setTimeout> | undefined;
			let sigkillTimer: ReturnType<typeof setTimeout> | undefined;
			let activityTimer: ReturnType<typeof setTimeout> | undefined;
			let hardTimer: ReturnType<typeof setTimeout> | undefined;
			let abortForceTimer: ReturnType<typeof setTimeout> | undefined;
			let killProc: (() => void) | undefined;

			const finalize = (code: number) => {
				if (resolved) return;
				resolved = true;
				if (postExitTimer) {
					clearTimeout(postExitTimer);
					postExitTimer = undefined;
				}
				if (sigkillTimer) {
					clearTimeout(sigkillTimer);
					sigkillTimer = undefined;
				}
				if (activityTimer) {
					clearTimeout(activityTimer);
					activityTimer = undefined;
				}
				if (hardTimer) {
					clearTimeout(hardTimer);
					hardTimer = undefined;
				}
				if (abortForceTimer) {
					clearTimeout(abortForceTimer);
					abortForceTimer = undefined;
				}
				if (emitTimer) {
					clearTimeout(emitTimer);
					emitTimer = null;
				}
				proc.stdout?.removeAllListeners();
				proc.stderr?.removeAllListeners();
				proc.removeAllListeners();
				if (signal && killProc) {
					signal.removeEventListener("abort", killProc);
				}
				if (buffer.trim()) processLineRaw(buffer);
				const effectiveCode =
					currentResult.stopReason === "error" || currentResult.errorMessage ? 1 : code;
				resolve(effectiveCode);
			};

			const maybeFinalizeAfterExit = () => {
				if (exitCodeValue !== null && stdoutEnded) {
					finalize(exitCodeValue);
				}
			};

			const processLineRaw = (line: string) => {
				if (!line.trim()) return;
				let event: any;
				try {
					event = JSON.parse(line);
				} catch {
					return;
				}

				if (event.type === "turn_start") {
					currentResult.phase = "thinking";
					currentResult.lastPhaseChange = Date.now();
					currentResult.thinkingBuffer = "";
					resetActivityTimer();
					emitUpdate();
				}

				// message_start: assistant message begins; typically no text yet, so just update phase
				if (event.type === "message_start") {
					currentResult.phase = "thinking";
					currentResult.lastPhaseChange = Date.now();
					resetActivityTimer();
					emitUpdate();
				}

				if (event.type === "message_update" && event.assistantMessageEvent?.type === "text_delta") {
					const delta = event.assistantMessageEvent.delta as string;
					if (delta) {
						currentResult.thinkingBuffer = (currentResult.thinkingBuffer || "") + delta;
					}
					if (currentResult.thinkingBuffer && currentResult.thinkingBuffer.length > 2048) {
						// Keep last 2048 chars; try to keep whole lines if possible
						const idx = currentResult.thinkingBuffer.indexOf("\n", currentResult.thinkingBuffer.length - 2048);
						currentResult.thinkingBuffer = currentResult.thinkingBuffer.slice(idx >= 0 ? idx + 1 : -2048);
					}
					currentResult.phase = "thinking";
					currentResult.lastPhaseChange = Date.now();
					resetActivityTimer();
					throttledEmitUpdate();
				}

				if (event.type === "tool_execution_start") {
					currentResult.phase = `tooling:${event.toolName}`;
					currentResult.lastPhaseChange = Date.now();
					resetActivityTimer();
					emitUpdate();
				}

				if (event.type === "tool_execution_update") {
					currentResult.phase = `tooling:${event.toolName}`;
					resetActivityTimer();
					throttledEmitUpdate();
				}

				if (event.type === "tool_execution_end") {
					if (event.message) {
						currentResult.messages.push(event.message as Message);
					}
					currentResult.phase = "waiting";
					currentResult.lastPhaseChange = Date.now();
					resetActivityTimer();
					emitUpdate();
				}

				if (event.type === "turn_end") {
					currentResult.phase = "idle";
					currentResult.lastPhaseChange = Date.now();
					resetActivityTimer();
					emitUpdate();
				}

				if (event.type === "message_end" && event.message) {
					const msg = event.message as Message;
					currentResult.messages.push(msg);
					resetActivityTimer();

					if (msg.role === "assistant") {
						currentResult.usage.turns++;
						const usage = msg.usage;
						if (usage) {
							currentResult.usage.input += usage.input || 0;
							currentResult.usage.output += usage.output || 0;
							currentResult.usage.cacheRead += usage.cacheRead || 0;
							currentResult.usage.cacheWrite += usage.cacheWrite || 0;
							currentResult.usage.cost += usage.cost?.total || 0;
							currentResult.usage.contextTokens = usage.totalTokens || 0;
						}
						if (!currentResult.model && msg.model) currentResult.model = msg.model;
						if (msg.stopReason) currentResult.stopReason = msg.stopReason;
						if (msg.errorMessage) currentResult.errorMessage = msg.errorMessage;
						if (msg.stopReason === "error" || msg.errorMessage) {
							try {
								proc.kill("SIGKILL");
							} catch {
								/* ignore ESRCH */
							}
							emitUpdate();
							finalize(1);
							return;
						}
					}
					emitUpdate();
				}
			};
			const processLine = (line: string) => {
				if (resolved) return;
				processLineRaw(line);
			};

			const resetActivityTimer = () => {
				if (resolved) return;
				if (activityTimer) clearTimeout(activityTimer);
				const activityMs = parseEnvInt(
					process.env.PI_SUBAGENT_ACTIVITY_TIMEOUT_MS,
					DEFAULT_ACTIVITY_TIMEOUT_MS,
				);
				if (activityMs > 0) {
					activityTimer = setTimeout(() => {
						const elapsed = Date.now() - currentResult.lastPhaseChange;
						const phase = currentResult.phase;
						const turns = currentResult.usage.turns;
						currentResult.stderr += `[subagent-isolation] activity timeout exceeded after ${Math.round(elapsed / 1000)}s idle (phase: ${phase}, turns: ${turns}), killing...\n`;
						try {
							proc.kill("SIGKILL");
						} catch {
							/* ignore ESRCH */
						}
						finalize(1);
					}, activityMs);
				}
			};

			const setupHardTimer = () => {
				const hardMs = parseEnvInt(
					process.env.PI_SUBAGENT_HARD_TIMEOUT_MS,
					DEFAULT_HARD_TIMEOUT_MS,
				);
				if (hardMs > 0) {
					hardTimer = setTimeout(() => {
						const turns = currentResult.usage.turns;
						const phase = currentResult.phase;
						currentResult.stderr += `[subagent-isolation] hard timeout exceeded (phase: ${phase}, turns: ${turns}), killing...\n`;
						try {
							proc.kill("SIGKILL");
						} catch {
							/* ignore ESRCH */
						}
						finalize(1);
					}, hardMs);
				}
			};

			proc.stdout.on("data", (data) => {
				resetActivityTimer();
				buffer += data.toString();
				const lines = buffer.split("\n");
				buffer = lines.pop() || "";
				for (const line of lines) processLine(line);
			});

			proc.stdout.on("end", () => {
				stdoutEnded = true;
				maybeFinalizeAfterExit();
			});

			proc.stderr.on("data", (data) => {
				currentResult.stderr += data.toString();
			});

			proc.on("exit", (code, signal) => {
				exitCodeValue = signal ? 1 : (code ?? 0);
				maybeFinalizeAfterExit();
				if (!resolved) {
					postExitTimer = setTimeout(() => finalize(exitCodeValue ?? 0), POST_EXIT_GRACE_MS);
				}
			});

			proc.on("close", (code, signal) => {
				finalize(signal ? 1 : (code ?? 0));
			});

			proc.on("error", (err) => {
				currentResult.stderr += `[subagent-isolation] process error: ${err?.message ?? String(err)}\n`;
				finalize(1);
			});

			if (signal) {
				killProc = () => {
					wasAborted = true;
					try {
						proc.kill("SIGTERM");
					} catch {
						/* ignore ESRCH */
					}
					if (exitCodeValue !== null || proc.exitCode !== null || proc.signalCode !== null) {
						finalize(1);
						return;
					}
					sigkillTimer = setTimeout(() => {
						try {
							if (proc.exitCode === null && proc.signalCode === null) {
								proc.kill("SIGKILL");
								abortForceTimer = setTimeout(() => {
									finalize(1);
								}, ABORT_FORCE_TIMEOUT_MS);
							}
						} catch {
							/* ignore ESRCH */
						}
					}, 5000);
				};
				if (signal.aborted) killProc();
				else signal.addEventListener("abort", killProc, { once: true });
			}

			setupHardTimer();
		});

		currentResult.exitCode = exitCode;
		if (wasAborted) throw new Error("Subagent was aborted");
		return currentResult;
	} finally {
		if (tmpPromptPath)
			try {
				fs.unlinkSync(tmpPromptPath);
			} catch {
				/* ignore */
			}
		if (tmpPromptDir)
			try {
				fs.rmdirSync(tmpPromptDir);
			} catch {
				/* ignore */
			}
	}
}

const ChainItem = Type.Object({
	agent: Type.String({ description: "Name of the agent to invoke" }),
	task: Type.String({ description: "Task with optional {previous} placeholder for prior output" }),
	cwd: Type.Optional(Type.String({ description: "Working directory for the agent process" })),
});

const AgentScopeSchema = StringEnum(["user", "project", "both"] as const, {
	description: 'Which agent directories to use. Default: "user". Use "both" to include project-local agents.',
	default: "user",
});

const SubagentParams = Type.Object({
	agent: Type.Optional(Type.String({ description: "Name of the agent to invoke (for single mode)" })),
	task: Type.Optional(Type.String({ description: "Task to delegate (for single mode)" })),
	chain: Type.Optional(Type.Array(ChainItem, { description: "Array of {agent, task} for sequential execution" })),
	agentScope: Type.Optional(AgentScopeSchema),
	confirmProjectAgents: Type.Optional(
		Type.Boolean({ description: "Prompt before running project-local agents. Default: false.", default: false }),
	),
	cwd: Type.Optional(Type.String({ description: "Working directory for the agent process (single mode)" })),
});

export default function (pi: ExtensionAPI) {
	pi.registerTool({
		name: "subagent",
		label: "Subagent",
		description: [
			"Delegate tasks to specialized subagents with isolated context.",
			"Modes: single (agent + task), chain (sequential with {previous} placeholder).",
			'Default agent scope is "user" (from ~/.pi/agent/agents).',
			'To enable project-local agents in .pi/agents, set agentScope: "both" (or "project").',
		].join(" "),
		parameters: SubagentParams,

		async execute(_toolCallId, params, signal, onUpdate, ctx) {
			const currentDepth = parseInt(process.env.PI_SUBAGENT_DEPTH || "0", 10);
			const canDelegateEnv = process.env.PI_CAN_DELEGATE;
			const canDelegate = canDelegateEnv === "true";
			const MAX_SUBAGENT_DEPTH = 2;

			const globallyBlocked = currentDepth >= MAX_SUBAGENT_DEPTH;
			const locallyBlocked = currentDepth >= 1 && !canDelegate;

			const detectMode = (): "single" | "chain" => {
				if (params.chain && params.chain.length > 0) return "chain";
				return "single";
			};

			if (globallyBlocked || locallyBlocked) {
				const reasons: string[] = [];
				if (globallyBlocked) {
					reasons.push(`global depth limit reached (max: ${MAX_SUBAGENT_DEPTH})`);
				}
				if (currentDepth >= 1 && !canDelegate) {
					const agentName = process.env.PI_CURRENT_AGENT_NAME || "current agent";
					reasons.push(`agent \`${agentName}\` has \`canDelegate: false\` in its frontmatter`);
				}
				return {
					content: [{
						type: "text",
						text: `Subagent delegation is blocked: ${reasons.join(", ")}.`,
					}],
					details: {
						mode: detectMode(),
						agentScope: params.agentScope ?? "user",
						projectAgentsDir: null,
						results: [],
					} as SubagentDetails,
					isError: true,
				};
			}

			const agentScope: AgentScope = params.agentScope ?? "both";
			const discovery = discoverAgents(ctx.cwd, agentScope);
			const agents = discovery.agents;
			const confirmProjectAgents = params.confirmProjectAgents ?? false;

			const hasChain = (params.chain?.length ?? 0) > 0;
			const hasSingle = Boolean(params.agent && params.task);
			const modeCount = Number(hasChain) + Number(hasSingle);

			const makeDetails =
				(mode: "single" | "chain") =>
				(results: SingleResult[]): SubagentDetails => ({
					mode,
					agentScope,
					projectAgentsDir: discovery.projectAgentsDir,
					results,
				});

			if (modeCount !== 1) {
				const available = agents.map((a) => `${a.name} (${a.source})`).join(", ") || "none";
				return {
					content: [
						{
							type: "text",
							text: `Invalid parameters. Provide exactly one mode.\nAvailable agents: ${available}`,
						},
					],
					details: makeDetails("single")([]),
				};
			}

			if ((agentScope === "project" || agentScope === "both") && confirmProjectAgents && ctx.hasUI) {
				const requestedAgentNames = new Set<string>();
				if (params.chain) for (const step of params.chain) requestedAgentNames.add(step.agent);
				if (params.agent) requestedAgentNames.add(params.agent);

				const projectAgentsRequested = Array.from(requestedAgentNames)
					.map((name) => agents.find((a) => a.name === name))
					.filter((a): a is AgentConfig => a?.source === "project");

				if (projectAgentsRequested.length > 0) {
					const names = projectAgentsRequested.map((a) => a.name).join(", ");
					const dir = discovery.projectAgentsDir ?? "(unknown)";
					const ok = await ctx.ui.confirm(
						"Run project-local agents?",
						`Agents: ${names}\nSource: ${dir}\n\nProject agents are repo-controlled. Only continue for trusted repositories.`,
					);
					if (!ok)
						return {
							content: [{ type: "text", text: "Canceled: project-local agents not approved." }],
							details: makeDetails(hasChain ? "chain" : "single")([]),
						};
				}
			}

			if (params.chain && params.chain.length > 0) {
				const results: SingleResult[] = [];
				let previousOutput = "";

				for (let i = 0; i < params.chain.length; i++) {
					const step = params.chain[i];
					const taskWithContext = step.task.replace(/\{previous\}/g, previousOutput);

					const chainUpdate: OnUpdateCallback | undefined = onUpdate
						? (partial) => {
								const currentResult = partial.details?.results[0];
								if (currentResult) {
									const allResults = [...results, currentResult];
									onUpdate({
										content: partial.content,
										details: makeDetails("chain")(allResults),
									});
								}
						  }
						: undefined;

					const result = await runSingleAgent(
						ctx.cwd,
						agents,
						step.agent,
						taskWithContext,
						step.cwd,
						i + 1,
						signal,
						chainUpdate,
						makeDetails("chain"),
					);
					results.push(result);

					const isError =
						result.exitCode !== 0 || result.stopReason === "error" || result.stopReason === "aborted";
					if (isError) {
						const diagnostics = formatSubagentDiagnostics(result);
						return {
							content: [{ type: "text", text: `Chain stopped at step ${i + 1} (${step.agent}):\n\n${diagnostics}` }],
							details: makeDetails("chain")(results),
							isError: true,
						};
					}
					previousOutput = getFinalOutput(result.messages);
				}
				return {
					content: [{ type: "text", text: getFinalOutput(results[results.length - 1].messages) || "(no output)" }],
					details: makeDetails("chain")(results),
				};
			}

			if (params.agent && params.task) {
				const result = await runSingleAgent(
					ctx.cwd,
					agents,
					params.agent,
					params.task,
					params.cwd,
					undefined,
					signal,
					onUpdate,
					makeDetails("single"),
				);
				const isError = result.exitCode !== 0 || result.stopReason === "error" || result.stopReason === "aborted";
				if (isError) {
					const diagnostics = formatSubagentDiagnostics(result);
					return {
						content: [{ type: "text", text: diagnostics }],
						details: makeDetails("single")([result]),
						isError: true,
					};
				}
				return {
					content: [{ type: "text", text: getFinalOutput(result.messages) || "(no output)" }],
					details: makeDetails("single")([result]),
				};
			}

			const available = agents.map((a) => `${a.name} (${a.source})`).join(", ") || "none";
			return {
				content: [{ type: "text", text: `Invalid parameters. Available agents: ${available}` }],
				details: makeDetails("single")([]),
			};
		},

		renderCall(args, theme, _context) {
			const scope: AgentScope = args.agentScope ?? "user";
			if (args.chain && args.chain.length > 0) {
				let text =
					theme.fg("toolTitle", theme.bold("subagent ")) +
					theme.fg("accent", `chain (${args.chain.length} steps)`) +
					theme.fg("muted", ` [${scope}]`);
				for (let i = 0; i < Math.min(args.chain.length, 3); i++) {
					const step = args.chain[i];
					const cleanTask = step.task.replace(/\{previous\}/g, "").trim();
					const preview = cleanTask.length > 40 ? `${cleanTask.slice(0, 40)}...` : cleanTask;
					text +=
						"\n  " +
						theme.fg("muted", `${i + 1}.`) +
						" " +
						theme.fg("accent", step.agent) +
						theme.fg("dim", ` ${preview}`);
				}
				if (args.chain.length > 3) text += `\n  ${theme.fg("muted", `... +${args.chain.length - 3} more`)}`;
				return new Text(text, 0, 0);
			}
			const agentName = args.agent || "...";
			const preview = args.task ? (args.task.length > 60 ? `${args.task.slice(0, 60)}...` : args.task) : "...";
			let text =
				theme.fg("toolTitle", theme.bold("subagent ")) +
				theme.fg("accent", agentName) +
				theme.fg("muted", ` [${scope}]`);
			text += `\n  ${theme.fg("dim", preview)}`;
			return new Text(text, 0, 0);
		},

		renderResult(result, { expanded }, theme, _context) {
			const details = result.details as SubagentDetails | undefined;
			if (!details || details.results.length === 0) {
				const text = result.content[0];
				return new Text(text?.type === "text" ? text.text : "(no output)", 0, 0);
			}

			const mdTheme = getMarkdownTheme();

			const renderDisplayItems = (items: DisplayItem[], limit?: number) => {
				const toShow = limit ? items.slice(-limit) : items;
				const skipped = limit && items.length > limit ? items.length - limit : 0;
				let text = "";
				if (skipped > 0) text += theme.fg("muted", `... ${skipped} earlier items\n`);
				for (const item of toShow) {
					if (item.type === "text") {
						const preview = expanded ? item.text : item.text.split("\n").slice(0, 3).join("\n");
						text += `${theme.fg("toolOutput", preview)}\n`;
					} else {
						text += `${theme.fg("muted", "→ ") + formatToolCall(item.name, item.args, theme.fg.bind(theme))}\n`;
					}
				}
				return text.trimEnd();
			};

			if (details.mode === "single" && details.results.length === 1) {
				const r = details.results[0];
				const isError = r.exitCode !== 0 || r.stopReason === "error" || r.stopReason === "aborted";
				const icon = isError ? theme.fg("error", "✗") : theme.fg("success", "✓");
				const displayItems = getDisplayItems(r.messages);
				const finalOutput = getFinalOutput(r.messages);

				if (expanded) {
					const container = new Container();
					let header = `${icon} ${theme.fg("toolTitle", theme.bold(r.agent))}${theme.fg("muted", ` (${r.agentSource})`)}`;
					if (isError && r.stopReason) header += ` ${theme.fg("error", `[${r.stopReason}]`)}`;
					if (r.phase !== "idle") header += ` ${theme.fg("warning", formatPhase(r.phase))}`;
					container.addChild(new Text(header, 0, 0));
					if (isError && r.errorMessage)
						container.addChild(new Text(theme.fg("error", `Error: ${r.errorMessage}`), 0, 0));
					if (r.thinkingBuffer) {
						container.addChild(new Spacer(1));
						container.addChild(new Text(theme.fg("muted", "─── Thinking ───"), 0, 0));
						const lines = r.thinkingBuffer.trim().split("\n");
						const recent = lines.slice(-5).join("\n");
						container.addChild(new Text(theme.fg("dim", recent), 0, 0));
					}
					container.addChild(new Spacer(1));
					container.addChild(new Text(theme.fg("muted", "─── Task ───"), 0, 0));
					container.addChild(new Text(theme.fg("dim", r.task), 0, 0));
					container.addChild(new Spacer(1));
					container.addChild(new Text(theme.fg("muted", "─── Output ───"), 0, 0));
					if (displayItems.length === 0 && !finalOutput) {
						container.addChild(new Text(theme.fg("muted", "(no output)"), 0, 0));
					} else {
						for (const item of displayItems) {
							if (item.type === "toolCall")
								container.addChild(
									new Text(
										theme.fg("muted", "→ ") + formatToolCall(item.name, item.args, theme.fg.bind(theme)),
										0,
										0,
									),
								);
						}
						if (finalOutput) {
							container.addChild(new Spacer(1));
							container.addChild(new Markdown(finalOutput.trim(), 0, 0, mdTheme));
						}
					}
					const usageStr = formatUsageStats(r.usage, r.model);
					if (usageStr) {
						container.addChild(new Spacer(1));
						container.addChild(new Text(theme.fg("dim", usageStr), 0, 0));
					}
					return container;
				}

				let text = `${icon} ${theme.fg("toolTitle", theme.bold(r.agent))}${theme.fg("muted", ` (${r.agentSource})`)}`;
				if (isError && r.stopReason) text += ` ${theme.fg("error", `[${r.stopReason}]`)}`;
				if (r.phase !== "idle") text += ` ${theme.fg("warning", formatPhase(r.phase))}`;
				if (isError && r.errorMessage) text += `\n${theme.fg("error", `Error: ${r.errorMessage}`)}`;
				if (displayItems.length === 0) {
					if (!isError || !r.errorMessage) text += `\n${theme.fg("muted", "(no output)")}`;
				} else {
					text += `\n${renderDisplayItems(displayItems, COLLAPSED_ITEM_COUNT)}`;
					if (displayItems.length > COLLAPSED_ITEM_COUNT) text += `\n${theme.fg("muted", "(Ctrl+O to expand)")}`;
				}
				const usageStr = formatUsageStats(r.usage, r.model);
				if (usageStr) text += `\n${theme.fg("dim", usageStr)}`;
				return new Text(text, 0, 0);
			}

			const aggregateUsage = (results: SingleResult[]) => {
				const total = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, turns: 0 };
				for (const r of results) {
					total.input += r.usage.input;
					total.output += r.usage.output;
					total.cacheRead += r.usage.cacheRead;
					total.cacheWrite += r.usage.cacheWrite;
					total.cost += r.usage.cost;
					total.turns += r.usage.turns;
				}
				return total;
			};

			if (details.mode === "chain") {
				const successCount = details.results.filter((r) => r.exitCode === 0).length;
				const icon = successCount === details.results.length ? theme.fg("success", "✓") : theme.fg("error", "✗");

				if (expanded) {
					const container = new Container();
					container.addChild(
						new Text(
							icon +
								" " +
								theme.fg("toolTitle", theme.bold("chain ")) +
								theme.fg("accent", `${successCount}/${details.results.length} steps`),
							0,
							0,
						),
					);

					for (const r of details.results) {
						const rIcon = r.exitCode === 0 ? theme.fg("success", "✓") : theme.fg("error", "✗");
						const displayItems = getDisplayItems(r.messages);
						const finalOutput = getFinalOutput(r.messages);

						container.addChild(new Spacer(1));
						container.addChild(
							new Text(
								`${theme.fg("muted", `─── Step ${r.step}: `) + theme.fg("accent", r.agent)} ${rIcon}`,
								0,
								0,
							),
						);
						container.addChild(new Text(theme.fg("muted", "Task: ") + theme.fg("dim", r.task), 0, 0));

						for (const item of displayItems) {
							if (item.type === "toolCall") {
								container.addChild(
									new Text(
										theme.fg("muted", "→ ") + formatToolCall(item.name, item.args, theme.fg.bind(theme)),
										0,
										0,
									),
								);
							}
						}

						if (finalOutput) {
							container.addChild(new Spacer(1));
							container.addChild(new Markdown(finalOutput.trim(), 0, 0, mdTheme));
						}

						const stepUsage = formatUsageStats(r.usage, r.model);
						if (stepUsage) container.addChild(new Text(theme.fg("dim", stepUsage), 0, 0));
					}

					const usageStr = formatUsageStats(aggregateUsage(details.results));
					if (usageStr) {
						container.addChild(new Spacer(1));
						container.addChild(new Text(theme.fg("dim", `Total: ${usageStr}`), 0, 0));
					}
					return container;
				}

				let text =
					icon +
					" " +
					theme.fg("toolTitle", theme.bold("chain ")) +
					theme.fg("accent", `${successCount}/${details.results.length} steps`);
				for (const r of details.results) {
					const rIcon = r.exitCode === 0 ? theme.fg("success", "✓") : theme.fg("error", "✗");
					const displayItems = getDisplayItems(r.messages);
					text += `\n\n${theme.fg("muted", `─── Step ${r.step}: `)}${theme.fg("accent", r.agent)} ${rIcon}`;
					if (displayItems.length === 0) text += `\n${theme.fg("muted", "(no output)")}`;
					else text += `\n${renderDisplayItems(displayItems, 5)}`;
				}
				const usageStr = formatUsageStats(aggregateUsage(details.results));
				if (usageStr) text += `\n\n${theme.fg("dim", `Total: ${usageStr}`)}`;
				text += `\n${theme.fg("muted", "(Ctrl+O to expand)")}`;
				return new Text(text, 0, 0);
			}

			const text = result.content[0];
			return new Text(text?.type === "text" ? text.text : "(no output)", 0, 0);
		},
	});
}
