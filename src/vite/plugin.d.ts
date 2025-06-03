import type { PluginOption } from "vite";
interface QwikAnalyzerOptions {
	debug?: boolean;
}
export declare function debug(message: string): void;
/**
 * Utility function to check if a component is present in the current component tree.
 * This function is analyzed at build time by qwik-analyzer.
 *
 * @param component - The component reference to check for
 * @param injectedValue - Optional boolean value injected by qwik-analyzer at build time
 * @returns boolean indicating if the component is present
 */
export declare function isComponentPresent<T>(
	component: unknown,
	injectedValue?: boolean,
): boolean;
export default function qwikAnalyzer(
	options?: QwikAnalyzerOptions,
): PluginOption;
export {};
