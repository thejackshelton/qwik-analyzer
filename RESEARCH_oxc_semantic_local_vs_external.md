# Research: Using oxc_semantic to Determine Local vs External Imports

## Problem Statement
We need to determine if JSX elements like `<Checkbox.Description />` come from external packages (node_modules) vs local files, where `Checkbox` is imported from "@kunai-consulting/qwik". Currently, we're using string pattern matching which is unreliable.

## Current Approach (Problematic)
```rust
// In jsx_analysis.rs - this is what we want to avoid
if element_name.contains('.') {
  if let Some(full_component) = parse_member_component(&element_name) {
    // String pattern matching on import sources - unreliable!
  }
}
```

## Recommended oxc_semantic API Solutions

### 1. Use oxc_resolver Result Analysis
The `resolve_import_path` function already returns a resolved path. We can analyze this path:

```rust
use std::path::Path;

pub fn is_external_import(resolved_path: &str) -> bool {
    let path = Path::new(resolved_path);
    
    // Check if path contains node_modules directory
    path.components().any(|component| {
        component.as_os_str() == "node_modules"
    })
}

// Usage after resolve_import_path:
let resolved = resolve_import_path(import_source, current_file)?;
let is_external = is_external_import(&resolved);
```

### 2. Enhanced Import Analysis with Symbol Tracing
Use oxc_semantic's symbol table to trace imports to their sources:

```rust
use oxc_semantic::Semantic;
use oxc_ast::AstKind;

pub fn trace_import_to_source(semantic: &Semantic, component_name: &str, current_file: &Path) -> Option<ImportSourceInfo> {
    // 1. Find the import declaration for the component
    for node in semantic.nodes().iter() {
        let AstKind::ImportDeclaration(import_decl) = node.kind() else {
            continue;
        };

        if let Some(specifiers) = &import_decl.specifiers {
            for specifier in specifiers {
                if let Some(local_name) = get_specifier_name(specifier) {
                    if local_name == component_name.split('.').next().unwrap_or(component_name) {
                        let import_source = import_decl.source.value.to_string();
                        
                        // 2. Resolve the import path
                        if let Ok(resolved_path) = resolve_import_path(&import_source, current_file) {
                            return Some(ImportSourceInfo {
                                import_source,
                                resolved_path: resolved_path.clone(),
                                is_external: is_external_import(&resolved_path),
                                is_relative: import_source.starts_with("./") || import_source.starts_with("../"),
                                is_absolute: import_source.starts_with("/"),
                                is_bare_specifier: !import_source.starts_with(".") && !import_source.starts_with("/")
                            });
                        }
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct ImportSourceInfo {
    pub import_source: String,     // Original import: "@kunai-consulting/qwik"
    pub resolved_path: String,     // Resolved path: "/path/to/node_modules/@kunai-consulting/qwik/index.js"
    pub is_external: bool,         // true if from node_modules
    pub is_relative: bool,         // true if starts with ./ or ../
    pub is_absolute: bool,         // true if starts with /
    pub is_bare_specifier: bool,   // true if bare specifier (likely external)
}
```

### 3. Use oxc_semantic Symbol References (Advanced)
For more complex scenarios, use the symbol table to trace references:

```rust
use oxc_semantic::{Semantic, SymbolId};

pub fn trace_symbol_to_declaration(semantic: &Semantic, symbol_name: &str) -> Option<SymbolId> {
    // Find symbol by name in the symbol table
    for (symbol_id, name) in semantic.symbols().names.iter_enumerated() {
        if name == symbol_name {
            return Some(symbol_id);
        }
    }
    None
}

pub fn get_symbol_import_info(semantic: &Semantic, symbol_id: SymbolId) -> Option<String> {
    // Get the declaration node for this symbol
    let declaration_node_id = semantic.symbols().declarations[symbol_id];
    let declaration_node = semantic.nodes().get_node(declaration_node_id);
    
    // Check if this declaration is part of an import
    match declaration_node.kind() {
        AstKind::ImportSpecifier(spec) => {
            // Find the parent import declaration
            // This would require walking up the AST to find the ImportDeclaration
            // and extracting the source
            // Implementation depends on AST structure
        }
        _ => None
    }
}
```

### 4. Comprehensive Import Classification
Combine all approaches for robust detection:

```rust
pub fn classify_jsx_component_import(
    semantic: &Semantic, 
    element_name: &str, 
    current_file: &Path
) -> ComponentImportClassification {
    // Extract base component name (before any dots)
    let base_component = element_name.split('.').next().unwrap_or(element_name);
    
    // 1. Try to find import source
    if let Some(import_info) = trace_import_to_source(semantic, base_component, current_file) {
        return ComponentImportClassification {
            element_name: element_name.to_string(),
            base_component: base_component.to_string(),
            import_info: Some(import_info.clone()),
            classification: if import_info.is_external {
                ImportType::External
            } else {
                ImportType::Local
            }
        };
    }
    
    // 2. Fallback: Check if it looks like a component but has no import
    if is_component_name(base_component) {
        ComponentImportClassification {
            element_name: element_name.to_string(),
            base_component: base_component.to_string(),
            import_info: None,
            classification: ImportType::LocalDefined, // Defined in same file
        }
    } else {
        ComponentImportClassification {
            element_name: element_name.to_string(),
            base_component: base_component.to_string(),
            import_info: None,
            classification: ImportType::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentImportClassification {
    pub element_name: String,
    pub base_component: String,
    pub import_info: Option<ImportSourceInfo>,
    pub classification: ImportType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportType {
    External,     // From node_modules
    Local,        // From local files (relative/absolute paths)
    LocalDefined, // Defined in the same file
    Unknown,      // Could not determine
}
```

## Key Benefits of This Approach

1. **Actual Resolution**: Uses oxc's actual resolver instead of string patterns
2. **Robust Detection**: Handles complex import scenarios (re-exports, barrel files, etc.)
3. **Path Analysis**: Analyzes resolved paths to definitively determine node_modules vs local
4. **Semantic Awareness**: Leverages oxc's symbol table for accurate import tracing

## Implementation Strategy

1. **Start Simple**: Implement path analysis on existing `resolve_import_path` results
2. **Enhance Gradually**: Add symbol table tracing for complex cases
3. **Filter Early**: Only process external imports for component presence analysis
4. **Cache Results**: Store classification results to avoid repeated resolution

## Integration Points

Update these functions to use the new classification:
- `extract_imported_jsx_components()` - Filter to only local components
- `find_presence_calls()` - Only analyze calls for local components  
- `has_component()` - Only check local component presence

This approach eliminates unreliable string pattern matching and uses oxc's actual semantic analysis and resolution capabilities.