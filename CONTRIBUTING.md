# Contributing to qwik-analyzer

A Rust-based build-time analyzer for Qwik applications using NAPI-RS and OXC.

## Architecture

```
Vite Plugin (TS) → NAPI-RS → Rust Core → OXC Parser
```

### Vite Plugin (`src/vite/plugin.ts`)
Standard Vite plugin that calls Rust functions via NAPI-RS for heavy analysis work.

### Rust Core (`src/`)

#### `lib.rs` - NAPI Bridge
Exports functions to JavaScript:
```rust
#[napi]
pub fn analyze_and_transform_code(code: String, file_path: String) -> napi::Result<String>
```

#### `component_analyzer/` - Analysis Engine
- `mod.rs` - Main coordinator
- `jsx_analysis.rs` - Parses JSX/TSX, tracks component usage
- `import_resolver.rs` - Resolves imports, builds dependency graph
- `component_presence.rs` - Determines if components are used
- `transformations.rs` - Replaces `isComponentPresent()` calls with booleans
- `utils.rs` - Shared utilities

## Tech Stack

- **[OXC](https://oxc.rs/)** - JavaScript parser (3x faster than SWC)
- **[NAPI-RS](https://napi.rs/)** - Rust ↔ Node.js bridge
- **[Vite](https://vite.dev/)** - Build tool integration

## Development

```bash
# Setup
pnpm install

# Build Rust module
pnpm build          # Release
pnpm build:debug    # Development

# Test
pnpm test
pnpm dev # Run the example Qwik app
```

## How It Works

1. OXC parses TypeScript/JSX files
2. Find `isComponentPresent(Component)` calls
3. Scan codebase for actual component usage
4. Replace calls with `true`/`false` literals
5. Dead code elimination removes unused branches

## Adding Features

1. Add analysis logic in `component_analyzer/` (create a new folder for a major feature)
2. Export via `lib.rs` with `#[napi]`
3. Update Vite plugin to call new function
4. Add tests