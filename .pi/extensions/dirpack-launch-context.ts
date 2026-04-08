import type { ExtensionAPI, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, realpathSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const COMMAND_NAME = "dirpack";
const CUSTOM_MESSAGE_TYPE = "dirpack-launch-context";
const STATE_ENTRY_TYPE = "dirpack-launch-config";
const EVENT_ENTRY_TYPE = "dirpack-launch-event";
const STATUS_KEY = "dirpack-launch-context";
const DEFAULT_BUDGET_TOKENS = 2000;
const PACK_TIMEOUT_MS = 30_000;
const OUTPUT_FORMAT = "pipe";

type Config = {
	enabled: boolean;
	budgetTokens: number;
};

type PackCandidate = {
	command: string;
	prefixArgs: string[];
	label: string;
};

type PackSuccess = {
	kind: "ok";
	output: string;
	commandLine: string;
	elapsedMs: number;
};

type PackFailure = {
	kind: "error";
	message: string;
	retryable: boolean;
};

type PackOutcome = PackSuccess | PackFailure;

type LastPack = {
	source: string;
	budgetTokens: number;
	generatedAt: string;
	sha256: string;
	commandLine: string;
	elapsedMs: number;
	outputLength: number;
};

function getRepoRoot(): string {
	const sourceFile = realpathSync(__filename);
	return resolve(dirname(sourceFile), "..", "..");
}

const REPO_ROOT = getRepoRoot();

function normalizeBudget(value: unknown): number {
	const parsed = typeof value === "number" ? value : Number(value);
	if (!Number.isFinite(parsed)) {
		return DEFAULT_BUDGET_TOKENS;
	}
	const normalized = Math.trunc(parsed);
	return normalized > 0 ? normalized : DEFAULT_BUDGET_TOKENS;
}

function hashText(text: string): string {
	return createHash("sha256").update(text).digest("hex");
}

function commandCandidates(): PackCandidate[] {
	const candidates: PackCandidate[] = [];
	const seen = new Set<string>();

	const add = (command: string, prefixArgs: string[] = [], label = command) => {
		const key = JSON.stringify([command, ...prefixArgs]);
		if (!command || seen.has(key)) {
			return;
		}
		seen.add(key);
		candidates.push({ command, prefixArgs, label });
	};

	const envCommand = process.env.PI_DIRPACK_LAUNCH_CONTEXT_COMMAND?.trim();
	if (envCommand) {
		add(envCommand, [], envCommand);
	}

	for (const binaryPath of [
		join(REPO_ROOT, "target", "debug", "dirpack"),
		join(REPO_ROOT, "target", "release", "dirpack"),
	]) {
		if (existsSync(binaryPath)) {
			add(binaryPath, [], binaryPath);
		}
	}

	add("dirpack", [], "dirpack");
	return candidates;
}

function runDirpack(candidate: PackCandidate, cwd: string, budgetTokens: number): Promise<PackOutcome> {
	return new Promise((resolvePromise) => {
		const args = [
			...candidate.prefixArgs,
			"pack",
			".",
			"-t",
			String(budgetTokens),
			"-f",
			OUTPUT_FORMAT,
			"--root-label",
			".",
		];
		const startedAt = Date.now();
		let stdout = "";
		let stderr = "";
		let finished = false;
		let timedOut = false;

		const finish = (outcome: PackOutcome) => {
			if (finished) {
				return;
			}
			finished = true;
			clearTimeout(timer);
			resolvePromise(outcome);
		};

		const child = spawn(candidate.command, args, {
			cwd,
			env: process.env,
			stdio: ["ignore", "pipe", "pipe"],
		});

		const timer = setTimeout(() => {
			timedOut = true;
			child.kill("SIGTERM");
			setTimeout(() => child.kill("SIGKILL"), 250).unref();
		}, PACK_TIMEOUT_MS);

		child.on("error", (error) => {
			finish({
				kind: "error",
				message: `${candidate.label}: ${error.message}`,
				retryable: error.message.includes("ENOENT"),
			});
		});

		child.stdout.setEncoding("utf8");
		child.stderr.setEncoding("utf8");
		child.stdout.on("data", (chunk) => {
			stdout += chunk;
		});
		child.stderr.on("data", (chunk) => {
			stderr += chunk;
		});

		child.on("close", (code) => {
			if (timedOut) {
				finish({
					kind: "error",
					message: `${candidate.label} timed out after ${PACK_TIMEOUT_MS}ms`,
					retryable: false,
				});
				return;
			}

			const output = stdout.trim();
			if (code === 0 && output) {
				finish({
					kind: "ok",
					output,
					commandLine: [candidate.command, ...args].join(" "),
					elapsedMs: Date.now() - startedAt,
				});
				return;
			}

			const stderrText = stderr.trim();
			const message = stderrText
				? `${candidate.label} exited ${code}: ${stderrText}`
				: `${candidate.label} exited ${code} without output`;
			finish({ kind: "error", message, retryable: false });
		});
	});
}

async function generateDirpack(cwd: string, budgetTokens: number): Promise<PackOutcome> {
	let lastFailure: PackFailure = {
		kind: "error",
		message: "No dirpack command candidates found",
		retryable: false,
	};

	for (const candidate of commandCandidates()) {
		const outcome = await runDirpack(candidate, cwd, budgetTokens);
		if (outcome.kind === "ok") {
			return outcome;
		}
		lastFailure = outcome;
		if (!outcome.retryable) {
			break;
		}
	}

	return lastFailure;
}

function formatContextMessage(budgetTokens: number, source: string, output: string): string {
	return [
		"# Dirpack Launch Context",
		`source: ${source}`,
		`budget_tokens: ${budgetTokens}`,
		`format: ${OUTPUT_FORMAT}`,
		"root_label: .",
		"",
		"This hidden context was generated automatically from the current working directory.",
		"",
		"<dirpack>",
		output,
		"</dirpack>",
	].join("\n");
}

function restoreConfig(ctx: ExtensionContext): Config {
	const restored: Config = {
		enabled: true,
		budgetTokens: DEFAULT_BUDGET_TOKENS,
	};

	for (const entry of ctx.sessionManager.getBranch()) {
		if (entry.type !== "custom" || entry.customType !== STATE_ENTRY_TYPE) {
			continue;
		}
		const data = entry.data as Partial<Config> | undefined;
		if (typeof data?.enabled === "boolean") {
			restored.enabled = data.enabled;
		}
		if (data?.budgetTokens !== undefined) {
			restored.budgetTokens = normalizeBudget(data.budgetTokens);
		}
	}

	return restored;
}

function persistConfig(pi: ExtensionAPI, config: Config): void {
	pi.appendEntry(STATE_ENTRY_TYPE, {
		enabled: config.enabled,
		budgetTokens: config.budgetTokens,
	});
}

function statusText(config: Config, lastPack: LastPack | undefined): string {
	const lines = [
		`dirpack launch context: ${config.enabled ? "on" : "off"}`,
		`budget tokens: ${config.budgetTokens}`,
	];

	if (lastPack) {
		lines.push(
			`last generated: ${lastPack.generatedAt}`,
			`last source: ${lastPack.source}`,
			`last sha256: ${lastPack.sha256}`,
			`last elapsed ms: ${lastPack.elapsedMs}`,
		);
	} else {
		lines.push("last generated: not yet generated in this runtime");
	}

	return lines.join("\n");
}

function setStatus(ctx: ExtensionContext, config: Config, lastPack: LastPack | undefined, loading = false): void {
	if (!ctx.hasUI) {
		return;
	}

	const theme = ctx.ui.theme;
	if (!config.enabled) {
		ctx.ui.setStatus(STATUS_KEY, theme.fg("dim", "dirpack off"));
		return;
	}

	if (loading) {
		ctx.ui.setStatus(
			STATUS_KEY,
			theme.fg("accent", "dirpack") + theme.fg("dim", ` ${config.budgetTokens}t packing...`),
		);
		return;
	}

	const suffix = lastPack ? ` ${config.budgetTokens}t ready` : ` ${config.budgetTokens}t`;
	ctx.ui.setStatus(STATUS_KEY, theme.fg("accent", "dirpack") + theme.fg("dim", suffix));
}

function parseCommand(args: string):
	| { type: "status" }
	| { type: "on" }
	| { type: "off" }
	| { type: "refresh" }
	| { type: "budget"; budgetTokens: number }
	| { type: "error"; message: string } {
	const trimmed = args.trim();
	if (!trimmed || trimmed === "status") {
		return { type: "status" };
	}
	if (trimmed === "on") {
		return { type: "on" };
	}
	if (trimmed === "off") {
		return { type: "off" };
	}
	if (trimmed === "refresh") {
		return { type: "refresh" };
	}

	const budgetMatch = trimmed.match(/^budget(?:\s+|=)(\d+)$/);
	if (budgetMatch) {
		return { type: "budget", budgetTokens: normalizeBudget(Number(budgetMatch[1])) };
	}

	return {
		type: "error",
		message: `Usage: /${COMMAND_NAME} [status|on|off|refresh|budget <tokens>]`,
	};
}

function getArgumentCompletions(prefix: string) {
	const options = ["status", "on", "off", "refresh", "budget 1000", "budget 2000", "budget 4000"];
	const trimmed = prefix.trimStart();
	const filtered = options.filter((option) => option.startsWith(trimmed));
	return filtered.length > 0 ? filtered.map((value) => ({ value, label: value })) : null;
}

export default function dirpackLaunchContextExtension(pi: ExtensionAPI) {
	let config: Config = {
		enabled: true,
		budgetTokens: DEFAULT_BUDGET_TOKENS,
	};
	let lastPack: LastPack | undefined;

	async function injectFreshDirpack(ctx: ExtensionContext, source: string): Promise<boolean> {
		if (!config.enabled) {
			setStatus(ctx, config, lastPack, false);
			return false;
		}

		setStatus(ctx, config, lastPack, true);
		const outcome = await generateDirpack(ctx.cwd, config.budgetTokens);
		if (outcome.kind !== "ok") {
			lastPack = undefined;
			setStatus(ctx, config, lastPack, false);
			if (ctx.hasUI) {
				ctx.ui.notify(`dirpack launch context failed: ${outcome.message}`, "warning");
			}
			return false;
		}

		const generatedAt = new Date().toISOString();
		const content = formatContextMessage(config.budgetTokens, source, outcome.output);
		const sha256 = hashText(content);
		lastPack = {
			source,
			budgetTokens: config.budgetTokens,
			generatedAt,
			sha256,
			commandLine: outcome.commandLine,
			elapsedMs: outcome.elapsedMs,
			outputLength: outcome.output.length,
		};

		const details = {
			source,
			cwd: ctx.cwd,
			budgetTokens: config.budgetTokens,
			format: OUTPUT_FORMAT,
			rootLabel: ".",
			generatedAt,
			sha256,
			commandLine: outcome.commandLine,
			elapsedMs: outcome.elapsedMs,
			outputLength: outcome.output.length,
			outputLines: outcome.output.split(/\r?\n/).length,
			repoRoot: REPO_ROOT,
		};

		pi.appendEntry(EVENT_ENTRY_TYPE, details);
		pi.sendMessage({
			customType: CUSTOM_MESSAGE_TYPE,
			content,
			display: false,
			details,
		});
		setStatus(ctx, config, lastPack, false);
		return true;
	}

	function reloadConfig(ctx: ExtensionContext): void {
		config = restoreConfig(ctx);
		lastPack = undefined;
		setStatus(ctx, config, lastPack, false);
	}

	pi.registerCommand(COMMAND_NAME, {
		description: "Manage automatic dirpack launch context injection",
		getArgumentCompletions,
		handler: async (args, ctx) => {
			const action = parseCommand(args);
			if (action.type === "error") {
				ctx.ui.notify(action.message, "warning");
				return;
			}

			if (action.type === "status") {
				ctx.ui.notify(statusText(config, lastPack), "info");
				return;
			}

			if (action.type === "off") {
				config.enabled = false;
				persistConfig(pi, config);
				setStatus(ctx, config, lastPack, false);
				ctx.ui.notify("dirpack launch context disabled for this session", "info");
				return;
			}

			if (action.type === "budget") {
				config.budgetTokens = action.budgetTokens;
				persistConfig(pi, config);
				if (!config.enabled) {
					setStatus(ctx, config, lastPack, false);
					ctx.ui.notify(
						`dirpack budget saved as ${config.budgetTokens} tokens (currently off)`,
						"info",
					);
					return;
				}

				const injected = await injectFreshDirpack(ctx, `command:budget:${config.budgetTokens}`);
				ctx.ui.notify(
					injected
						? `dirpack budget set to ${config.budgetTokens} tokens and refreshed`
						: `dirpack budget set to ${config.budgetTokens} tokens`,
					"info",
				);
				return;
			}

			if (action.type === "on") {
				config.enabled = true;
				persistConfig(pi, config);
				const injected = await injectFreshDirpack(ctx, "command:on");
				ctx.ui.notify(
					injected ? "dirpack launch context enabled and refreshed" : "dirpack launch context enabled",
					"info",
				);
				return;
			}

			const injected = await injectFreshDirpack(ctx, "command:refresh");
			ctx.ui.notify(
				injected ? "dirpack launch context refreshed" : "dirpack launch context is off",
				"info",
			);
		},
	});

	pi.on("session_start", async (event, ctx) => {
		reloadConfig(ctx);
		if (config.enabled) {
			await injectFreshDirpack(ctx, `session_start:${event.reason}`);
		}
	});

	pi.on("session_tree", async (_event, ctx) => {
		reloadConfig(ctx);
	});

	pi.on("session_shutdown", async (_event, ctx) => {
		if (ctx.hasUI) {
			ctx.ui.setStatus(STATUS_KEY, undefined);
		}
	});
}
