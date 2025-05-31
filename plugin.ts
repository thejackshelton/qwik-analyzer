import type { PluginOption } from "vite";

interface QwikAnalyzerOptions {
  debug?: boolean;
}

let isDebugMode = false;

export function debug(message: string): void {
  if (isDebugMode) {
    console.log(`[qwik-analyzer] ${message}`);
  }
}

export function qwikAnalyzer(options: QwikAnalyzerOptions = {}): PluginOption {
  isDebugMode = options.debug ?? false;

  return {
    name: "qwik-analyzer",
    enforce: "pre",

    async watchChange(id: string, change: { event: string }) {
      const cleanedId = id.split("?")[0];
      
      // Only analyze TypeScript/JSX files, skip node_modules
      if (
        (cleanedId.endsWith(".tsx") || cleanedId.endsWith(".ts")) &&
        !cleanedId.includes("node_modules")
      ) {
        debug(`File ${change.event}: ${cleanedId}`);
        
        try {
          // Import from parent directory since plugin.js is in dist/
          const analyzerPath = new URL("../index.js", import.meta.url).pathname;
          const { analyzeFileChanged } = await import(analyzerPath);
          analyzeFileChanged(cleanedId, change.event);
        } catch (error) {
          console.error(`[qwik-analyzer] Error processing change for ${cleanedId}:`, error);
        }
      }
    },
  };
} 