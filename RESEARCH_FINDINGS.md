# oxc Semantic Analysis Research Findings

## Problem Statement
The qwik-analyzer needs to distinguish between local components and external package components when analyzing JSX elements and `isComponentPresent()` calls.

**Current Issue:**
- `isComponentPresent(Checkbox.Description)` in root.tsx should NOT trigger for `<Checkbox.Description />` JSX
- `Checkbox` is imported from "@kunai-consulting/qwik" (external package)
- But `isComponentPresent(Description)` SHOULD trigger for `<Description />` JSX (local component)

## oxc_semantic API Research

### Key APIs Available in oxc_semantic v0.72.1:

1. **Symbol Resolution:**
   ```rust
   semantic.symbols() // Access to SymbolTable
   semantic.scopes()  // Access to ScopeTree
   ```

2. **Import/Reference Analysis:**
   ```rust
   // Check if a reference is unresolved (global/external)
   semantic.scopes().root_unresolved_references()
   
   // For each node, can check if it's an import declaration
   AstKind::ImportDeclaration(import_decl)
   ```

3. **JSX Element Analysis:**
   ```rust
   // Already used in current code
   AstKind::JSXOpeningElement(jsx_opening)
   ```

## Recommended Solution

### 1. Create a Semantic-Aware Component Analyzer

The key insight is to use oxc's symbol resolution to determine if a JSX element refers to:
- **Local symbol**: Component imported from relative paths or declared locally
- **External symbol**: Component imported from node_modules

### 2. Enhanced Import Source Classification

```rust
pub fn classify_import_source(import_source: &str) -> ImportSourceType {
    if import_source.starts_with('.') || import_source.starts_with('/') {
        ImportSourceType::Local
    } else if import_source.starts_with('~') {
        ImportSourceType::LocalAlias  // Project-relative
    } else {
        ImportSourceType::External    // node_modules
    }
}

enum ImportSourceType {
    Local,      // ./component or ../component
    LocalAlias, // ~/components/component  
    External,   // @package/component or package
}
```

### 3. Semantic-Aware JSX Analysis

Instead of just string matching, use semantic analysis:

```rust
pub fn is_jsx_element_local_component(
    semantic: &Semantic,
    jsx_element_name: &str,
    current_file: &Path,
) -> bool {
    // For member expressions like "Checkbox.Description"
    if jsx_element_name.contains('.') {
        let parts: Vec<&str> = jsx_element_name.split('.').collect();
        let namespace = parts[0];
        
        // Check if namespace is imported from external source
        if let Some(import_source) = find_import_source_for_component(semantic, namespace) {
            match classify_import_source(&import_source) {
                ImportSourceType::External => return false, // External component
                ImportSourceType::Local | ImportSourceType::LocalAlias => {
                    // Local component - check if it resolves properly
                    return resolve_import_path(&import_source, current_file).is_ok();
                }
            }
        }
        return false; // Can't resolve namespace
    }
    
    // For simple components like "Description"
    // Check if it's imported from local source or declared locally
    if let Some(import_source) = find_import_source_for_component(semantic, jsx_element_name) {
        match classify_import_source(&import_source) {
            ImportSourceType::External => false,
            ImportSourceType::Local | ImportSourceType::LocalAlias => true,
        }
    } else {
        // Not imported - could be locally declared or global
        // Use oxc semantic analysis to check if it's a local symbol
        is_identifier_locally_declared(semantic, jsx_element_name)
    }
}

fn is_identifier_locally_declared(semantic: &Semantic, identifier: &str) -> bool {
    // Check if the identifier is in unresolved references (external/global)
    !semantic.scopes().root_unresolved_references().contains_key(identifier)
}
```

### 4. Updated Component Presence Logic

```rust
pub fn has_component_semantic_aware(
    semantic: &Semantic,
    component_name: &str,
    current_file: &Path,
) -> Result<bool> {
    // First check direct JSX usage with semantic awareness
    if jsx_contains_local_component(semantic, component_name, current_file) {
        return Ok(true);
    }

    // Then check imported components, but only local ones
    let jsx_components = extract_local_jsx_components(semantic, current_file);
    
    for jsx_component in jsx_components {
        // Only analyze local components for presence calls
        let presence_calls = find_calls_in_file_for_local_component(&jsx_component, current_file)?;
        for call in &presence_calls {
            if call.component_name == component_name {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}
```

## Implementation Strategy

### Phase 1: Add Import Classification
1. Create `ImportSourceType` enum
2. Implement `classify_import_source()` function
3. Update import resolution to be source-type aware

### Phase 2: Semantic-Aware JSX Analysis  
1. Create `is_jsx_element_local_component()` function
2. Update `component_exists_in_jsx_with_path()` to use semantic analysis
3. Filter out external component JSX elements

### Phase 3: Enhanced Component Presence Detection
1. Create `extract_local_jsx_components()` that only returns local components
2. Update `has_component()` to use semantic-aware filtering
3. Ensure external component JSX doesn't trigger local presence calls

## Benefits

1. **Accurate Component Distinction**: External components like `Checkbox.Description` won't trigger local presence calls
2. **Performance**: Avoid analyzing external packages unnecessarily  
3. **Maintainability**: Clear separation between local and external component logic
4. **Semantic Correctness**: Uses proper symbol resolution instead of string matching

## Files to Modify

1. `src/component_analyzer/import_resolver.rs` - Add import classification
2. `src/component_analyzer/jsx_analysis.rs` - Add semantic-aware JSX filtering
3. `src/component_analyzer/component_presence.rs` - Update presence detection logic
4. `src/component_analyzer/utils.rs` - Add semantic helper functions

This approach leverages oxc's semantic analysis capabilities to provide accurate component distinction while maintaining the existing API compatibility.