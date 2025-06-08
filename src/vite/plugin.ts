import type { PluginOption } from "vite";

interface QwikAnalyzerOptions {
	debug?: boolean;
}

interface NAPIModule {
	transformWithAnalysis: (code: string, filePath: string) => string;
}

let napiModule: NAPIModule | null = null;

async function getNAPIModule(): Promise<NAPIModule> {
	if (napiModule) return napiModule;

	try {
		// @ts-expect-error - NAPI module has no TypeScript declarations
		napiModule = await import("../index.cjs");
	} catch (error) {
		throw new Error(`Failed to load NAPI module: ${error}`);
	}

	if (!napiModule) {
		throw new Error("Failed to load NAPI module");
	}

	return napiModule;
}

/**
 * Utility function to check if a component is present in the current component tree.
 * This function is analyzed at build time by qwik-analyzer.
 *
 * @param component - The component reference to check for
 * @param injectedValue - Optional boolean value injected by qwik-analyzer at build time
 * @returns boolean indicating if the component is present
 */
export function usePresence<T>(
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
	return {
		name: "qwik-analyzer",
		enforce: "pre",

		async transform(code: string, id: string) {
			const cleanedId = id.split("?")[0];

			if (
				(!cleanedId.endsWith(".tsx") && !cleanedId.endsWith(".jsx")) ||
				cleanedId.includes("node_modules")
			) {
				return null;
			}

			try {
				const napi = await getNAPIModule();

				const transformedCode = napi.transformWithAnalysis(code, cleanedId);

				return transformedCode !== code ? { code: transformedCode } : null;
			} catch (error) {
				if (options.debug) {
					console.log(`[qwik-analyzer] Transform failed: ${error}`);
				}
				return null;
			}
		},
	};
}
