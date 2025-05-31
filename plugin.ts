import type { PluginOption } from "vite";

// Declare the NAPI functions that exist in ../index.js
declare function analyzeAndTransformCode(code: string, filePath: string, moduleSpecifier?: string): Promise<string>;
declare function analyzeFileChanged(filePath: string, event: string, moduleSpecifier?: string): void;

interface QwikAnalyzerOptions {
  debug?: boolean;
  moduleSpecifier?: string;
}

let isDebugMode = false;
let moduleSpecifier: string | undefined;

export function debug(message: string): void {
  if (isDebugMode) {
    console.log(`[qwik-analyzer] ${message}`);
  }
}

// Lazy loader for NAPI functions to avoid esbuild .node file issues during config loading
class NAPIWrapper {
  private _module: any = null;
  private _loading: Promise<any> | null = null;

  async getModule() {
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

  private async loadModule() {
    try {
      // Use eval to avoid static analysis by bundlers
      const importFn = new Function('specifier', 'return import(specifier)');
      const napiModule = await importFn('../index.js');
      debug('NAPI module loaded successfully');
      return napiModule;
    } catch (error) {
      debug(`Failed to load NAPI module: ${error}`);
      throw error;
    }
  }

  async analyzeAndTransformCode(code: string, filePath: string, moduleSpecifier?: string): Promise<string> {
    const module = await this.getModule();
    debug(`NAPI module available functions: ${Object.keys(module).join(', ')}`);
    
    if (typeof module.analyzeAndTransformCode !== 'function') {
      debug(`analyzeAndTransformCode is not a function, it's a ${typeof module.analyzeAndTransformCode}`);
      throw new Error('analyzeAndTransformCode is not a function');
    }
    
    debug(`Calling analyzeAndTransformCode with file: ${filePath}`);
    return module.analyzeAndTransformCode(code, filePath, moduleSpecifier);
  }

  async analyzeFileChanged(filePath: string, event: string, moduleSpecifier?: string): Promise<void> {
    const module = await this.getModule();
    return module.analyzeFileChanged(filePath, event, moduleSpecifier);
  }
}

const napiWrapper = new NAPIWrapper();

export function qwikAnalyzer(options: QwikAnalyzerOptions = {}): PluginOption {
  isDebugMode = options.debug ?? false;
  moduleSpecifier = options.moduleSpecifier;
  
  if (moduleSpecifier) {
    debug(`Using module specifier filter: ${moduleSpecifier}`);
  }

  return {
    name: "qwik-analyzer",
    enforce: "pre",

    async transform(code: string, id: string) {
      const cleanedId = id.split("?")[0];
      
      // Only transform TypeScript/JSX files, skip node_modules  
      if (
        !cleanedId.endsWith(".tsx") && 
        !cleanedId.endsWith(".ts") ||
        cleanedId.includes("node_modules")
      ) {
        return null;
      }

      debug(`Transforming ${cleanedId}`);

      try {
        // Pass code content to Rust, get transformed code back
        const transformedCode = await napiWrapper.analyzeAndTransformCode(code, cleanedId, moduleSpecifier);
        
        // Only return if code was actually transformed
        if (transformedCode !== code) {
          debug(`Transformed ${cleanedId}`);
          return {
            code: transformedCode,
            map: null // TODO: Generate source map if needed
          };
        }
      } catch (error) {
        debug(`NAPI module not available or error: ${error}`);
        // Gracefully continue without transformation
      }

      return null;
    },

    async handleHotUpdate(ctx) {
      const { file, server } = ctx;
      const change = { event: "update" };
      
      debug(`File update: ${file}`);

      try {
        await napiWrapper.analyzeFileChanged(file, change.event, moduleSpecifier);
      } catch (error) {
        debug(`Error processing file change: ${error}`);
      }
    }
  };
} 