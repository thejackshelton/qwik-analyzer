# qwik-analyzer

A high-performance Rust-based build-time analyzer for Qwik applications. This tool provides compile-time analysis and code transformation capabilities, enabling advanced optimizations and build-time checks for your Qwik projects.

## Features

- 🚀 **High Performance**: Built with Rust using NAPI-RS for maximum speed
- 🔍 **Component Analysis**: Analyze component usage patterns at build time
- 🛠️ **Code Transformation**: Transform code based on analysis results
- 📦 **Vite Integration**: Seamless integration with Vite build pipeline
- 🎯 **Type Safe**: Full TypeScript support with proper type definitions

## Installation

```bash
# npm
npm install @jackshelton/qwik-analyzer

# pnpm
pnpm add @jackshelton/qwik-analyzer

# yarn
yarn add @jackshelton/qwik-analyzer
```

> The @jackshelton scope is needed because napi-rs makes scopes required for publishing.

## Usage

### Vite Plugin

Add the qwik-analyzer plugin to your Vite configuration:

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import qwikAnalyzer from '@jackshelton/qwik-analyzer';

export default defineConfig({
  plugins: [
    qwikAnalyzer({
      debug: true, // Optional: Enable debug logging
    }),
    // ... other plugins
  ],
});
```

### Component Presence Detection

Use the `isComponentPresent` function to conditionally render or execute code based on whether specific components are used in your application:

```tsx
import { component$ } from '@builder.io/qwik';
import { isComponentPresent } from '@jackshelton/qwik-analyzer';
import { MyComponent } from './my-component';
import { AnotherComponent } from './another-component';

export const LibraryComponent = component$(() => {
  const hasMyComponent = isComponentPresent(MyComponent);
  const hasAnotherComponent = isComponentPresent(AnotherComponent);

  return (
    <div>
      {hasMyComponent && (
        <p>MyComponent is used somewhere in this subtree</p>
      )}
      {!hasAnotherComponent && (
        <p>AnotherComponent is not used in this subtree</p>
      )}
    </div>
  );
});
```

## API Reference

### Default Export: Vite Plugin

```typescript
function qwikAnalyzer(options?: QwikAnalyzerOptions): PluginOption
```

#### Options

- `debug?: boolean` - Enable debug logging (default: `false`)

### `isComponentPresent<T>(component: unknown, injectedValue?: boolean): boolean`

Checks if a component is present in the current component tree. This function is analyzed and transformed at build time.

#### Parameters

- `component` - The component reference to check for
- `injectedValue?` - Optional boolean value injected by qwik-analyzer at build time

#### Returns

- `boolean` - `true` if the component is present in the application, `false` otherwise

#### How it works

1. **Build Time**: The analyzer scans your entire codebase to determine component usage
2. **Transform**: Calls to `isComponentPresent` are replaced with the actual boolean values
3. **Runtime**: Your code receives the pre-computed boolean values, enabling dead code elimination

### Bundle Size Optimization

```typescript
// utils/feature-flags.ts
import { isComponentPresent } from '@jackshelton/qwik-analyzer';
import { AdvancedEditor } from '../components/advanced-editor';
import { SimpleEditor } from '../components/simple-editor';

export const loadEditor = () => {
  const hasAdvancedEditor = isComponentPresent(AdvancedEditor);
  
  if (hasAdvancedEditor) {
    // Only load the heavy editor library if the advanced editor is used
    return import('heavy-editor-library');
  } else {
    // Load lightweight alternative
    return import('simple-editor-library');
  }
};
```

## How It Works

The qwik-analyzer performs static analysis of your codebase during the build process:

1. **File Scanning**: Analyzes all TypeScript/JSX files in your project
2. **Component Tracking**: Builds a dependency graph of component usage
3. **Code Transformation**: Replaces `isComponentPresent` calls with boolean literals
4. **Hot Module Replacement**: Updates analysis when files change during development

## Development

This project uses Rust with NAPI-RS for the core analysis engine and TypeScript for the Vite plugin interface.

### Prerequisites

- Node.js 18+ 
- Rust toolchain
- pnpm (recommended)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/your-username/qwik-analyzer.git
cd qwik-analyzer

# Install dependencies
pnpm install

# Build the native module
pnpm build

# Run tests
pnpm test

# Test with the example app
pnpm -C qwik-app dev
```

## Platform Support

The analyzer supports all major platforms:

- ✅ Windows (x64, x86, ARM64)
- ✅ macOS (x64, ARM64)
- ✅ Linux (x64, ARM64, ARM)
- ✅ FreeBSD

Pre-built binaries are available for all supported platforms.

## Performance

Built with Rust for maximum performance:

- 🔥 **Fast Analysis**: Typical projects analyzed in milliseconds
- 🔄 **Incremental Updates**: Only re-analyzes changed files
- 💾 **Low Memory Usage**: Efficient memory management with Rust
- ⚡ **Native Speed**: No JavaScript overhead for core analysis

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

MIT © Jack Shelton

## Related Projects

- [Qwik](https://qwik.builder.io/) - The framework this analyzer is built for
- [NAPI-RS](https://napi.rs/) - Rust bindings for Node.js used in this project
- [Vite](https://vitejs.dev/) - The build tool for the web