import type { PluginOption } from "vite";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { createRequire } from "node:module";

interface QwikAnalyzerOptions {
	debug?: boolean;
}

interface NAPIModule {
	analyzeAndTransformCode: (code: string, filePath: string) => string;
	analyzeFileChanged: (filePath: string, event: string) => void;
}

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

let isDebugMode = false;

export function debug(message: string): void {
	if (isDebugMode) {
		console.log(`[qwik-analyzer] ${message}`);
	}
}

class NAPIWrapper {
	private _module: NAPIModule | null = null;
	private _loading: Promise<NAPIModule> | null = null;

	async getModule(): Promise<NAPIModule> {
		if (this._module) {
			return this._module;
		}

		if (this._loading) {
			return this._loading;
		}

		this._loading = this.loadModule();
		this._module = await this._loading;
		return this._module;
	}

	private async loadModule(): Promise<NAPIModule> {
		try {
			// Resolve path to index.js relative to this plugin file
			const indexPath = resolve(__dirname, "../../index.js");
			const napiModule = require(indexPath);
			debug("NAPI module loaded successfully");
			return napiModule;
		} catch (error) {
			debug(`Failed to load NAPI module: ${error}`);
			throw error;
		}
	}

	async analyzeAndTransformCode(
		code: string,
		filePath: string,
	): Promise<string> {
		const module = await this.getModule();
		debug(`NAPI module available functions: ${Object.keys(module).join(", ")}`);

		if (typeof module.analyzeAndTransformCode !== "function") {
			debug(
				`analyzeAndTransformCode is not a function, it's a ${typeof module.analyzeAndTransformCode}`,
			);
			throw new Error("analyzeAndTransformCode is not a function");
		}

		debug(`Calling analyzeAndTransformCode with file: ${filePath}`);
		return module.analyzeAndTransformCode(code, filePath);
	}

	async analyzeFileChanged(filePath: string, event: string): Promise<void> {
		const module = await this.getModule();
		return module.analyzeFileChanged(filePath, event);
	}
}

const napiWrapper = new NAPIWrapper();

/**
 * Utility function to check if a component is present in the current component tree.
 * This function is analyzed at build time by qwik-analyzer.
 *
 * @param component - The component reference to check for
 * @param injectedValue - Optional boolean value injected by qwik-analyzer at build time
 * @returns boolean indicating if the component is present
 */
export function isComponentPresent<T>(
	component: unknown,
	injectedValue?: boolean,
): boolean {
	if (injectedValue !== undefined) {
		return injectedValue;
	}

	return false;
}

export default function qwikAnalyzer(
	options: QwikAnalyzerOptions = {},
): PluginOption {
	isDebugMode = options.debug ?? false;

	return {
		name: "qwik-analyzer",
		enforce: "pre",

		async transform(code: string, id: string) {
			const cleanedId = id.split("?")[0];

			if (
				(!cleanedId.endsWith(".tsx") && !cleanedId.endsWith(".ts")) ||
				cleanedId.includes("node_modules")
			) {
				return null;
			}

			debug(`Transforming ${cleanedId}`);

			try {
				console.log("Analyzing and transforming code");
				const transformedCode = await napiWrapper.analyzeAndTransformCode(
					code,
					cleanedId,
				);

				if (transformedCode !== code) {
					debug(`Transformed ${cleanedId}`);
					return {
						code: transformedCode,
						map: null,
					};
				}
			} catch (error) {
				debug(`NAPI module not available or error: ${error}`);
			}

			return null;
		},

		watchChange(id) {
			debug(`File changed: ${id}`);

			try {
				napiWrapper.analyzeFileChanged(id, "change");
			} catch (error) {
				debug(`Error processing file change: ${error}`);
			}
		},

		handleHotUpdate(ctx) {
			const { file, server } = ctx;

			const module = server.moduleGraph.getModuleById(file);
			if (module) {
				for (const importer of module.importers) {
					server.moduleGraph.invalidateModule(importer);
					debug(`Invalidated importer: ${importer.id}`);
				}
			}
		},
	};
}
