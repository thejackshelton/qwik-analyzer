# Enhanced oxc API Research for Component Detection

## Overview
Research into leveraging oxc's semantic analysis APIs to improve component detection accuracy, focusing on symbol resolution, import tracing, and JSX analysis.

## Current oxc Version: 0.72.1

### Core APIs Available

#### 1. Semantic Analysis Core
```rust
use oxc_semantic::{Semantic, SemanticBuilder};

// Build semantic information
let semantic_ret = SemanticBuilder::new().build(&program);
let semantic = &semantic_ret.semantic;

// Access symbol table and scope tree
semantic.symbols()  // SymbolTable
semantic.scopes()   // ScopeTree  
semantic.nodes()    // AstNodes
```

#### 2. Symbol Resolution APIs
```rust
// Symbol table access
semantic.symbols().names          // Symbol names
semantic.symbols().declarations   // Symbol declarations
semantic.symbols().references     // Symbol references

// Scope resolution
semantic.scopes().root_unresolved_references()  // Global/external references
semantic.scopes().get_bindings(scope_id)        // Local bindings in scope
```

#### 3. Import Declaration Analysis
```rust
use oxc_ast::AstKind;

for node in semantic.nodes().iter() {
    match node.kind() {
        AstKind::ImportDeclaration(import_decl) => {
            // import_decl.source.value -> import source string
            // import_decl.specifiers -> imported names
        }
        AstKind::ImportSpecifier(spec) => {
            // Access imported/local names
        }
        _ => {}
    }
}
```

#### 4. JSX Element Processing
```rust
AstKind::JSXOpeningElement(jsx_opening) => {
    // jsx_opening.name -> JSX element name
    // Can be JSXElementName::Identifier or JSXElementName::MemberExpression
}

AstKind::JSXMemberExpression(member_expr) => {
    // member_expr.object -> namespace (e.g., "MyTest")
    // member_expr.property -> property (e.g., "Child") 
}
```

## Advanced oxc Research Findings

### 1. Symbol ID Tracking
```rust
use oxc_semantic::SymbolId;

// Find symbol by name and get its ID
fn find_symbol_id(semantic: &Semantic, symbol_name: &str) -> Option<SymbolId> {
    semantic.symbols().names
        .iter_enumerated()
        .find(|(_, name)| name == symbol_name)
        .map(|(symbol_id, _)| symbol_id)
}

// Get all references to a symbol
fn get_symbol_references(semantic: &Semantic, symbol_id: SymbolId) -> Vec<ReferenceId> {
    semantic.symbols().references
        .iter_enumerated()
        .filter_map(|(ref_id, reference)| {
            if reference.symbol_id() == Some(symbol_id) {
                Some(ref_id)
            } else {
                None
            }
        })
        .collect()
}
```

### 2. Scope-Aware Import Resolution
```rust
use oxc_semantic::{ScopeId, ScopeTree};

// Check if identifier is imported vs locally declared
fn is_identifier_imported(semantic: &Semantic, identifier: &str) -> bool {
    // Check if it appears in unresolved references (likely imported)
    semantic.scopes().root_unresolved_references().contains_key(identifier)
}

// Get scope where identifier is declared
fn find_declaration_scope(semantic: &Semantic, identifier: &str) -> Option<ScopeId> {
    for (scope_id, scope) in semantic.scopes().iter_enumerated() {
        if scope.bindings().contains_key(identifier) {
            return Some(scope_id);
        }
    }
    None
}
```

### 3. Advanced JSX Member Expression Analysis
```rust
use oxc_ast::ast::{JSXElementName, JSXMemberExpression};

fn analyze_jsx_member_expression(
    semantic: &Semantic,
    jsx_name: &JSXElementName,
) -> Option<(String, String)> {
    match jsx_name {
        JSXElementName::MemberExpression(member_expr) => {
            // Extract namespace.property (e.g., "MyTest.Child")
            let namespace = extract_namespace_from_object(&member_expr.object)?;
            let property = member_expr.property.name.to_string();
            
            // Use semantic analysis to verify the namespace is imported
            if is_namespace_imported(semantic, &namespace) {
                Some((namespace, property))
            } else {
                None
            }
        }
        JSXElementName::Identifier(ident) => {
            // Simple component like <Description />
            Some((ident.name.to_string(), String::new()))
        }
        _ => None
    }
}

fn is_namespace_imported(semantic: &Semantic, namespace: &str) -> bool {
    // Check if the namespace appears in import declarations
    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            if let Some(specifiers) = &import_decl.specifiers {
                for spec in specifiers {
                    if let Some(local_name) = get_specifier_name(spec) {
                        if local_name == namespace {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
```

### 4. Enhanced Import Source Classification Using oxc
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ImportSourceClassification {
    Local { resolved_path: String },
    External { package_name: String },
    Unresolved { source: String },
}

fn classify_import_with_oxc(
    semantic: &Semantic,
    import_source: &str,
    current_file: &Path,
) -> ImportSourceClassification {
    // First, try oxc resolver
    match resolve_import_path(import_source, current_file) {
        Ok(resolved_path) => {
            if resolved_path.contains("node_modules") {
                // Extract package name from resolved path
                let package_name = extract_package_name_from_path(&resolved_path);
                ImportSourceClassification::External { package_name }
            } else {
                ImportSourceClassification::Local { resolved_path }
            }
        }
        Err(_) => {
            // Fallback to pattern matching
            if import_source.starts_with('.') || import_source.starts_with('/') {
                ImportSourceClassification::Local { 
                    resolved_path: format!("unresolved: {}", import_source) 
                }
            } else {
                ImportSourceClassification::External { 
                    package_name: import_source.to_string() 
                }
            }
        }
    }
}
```

### 5. Component Presence Detection with Full Semantic Awareness
```rust
fn has_component_semantic_enhanced(
    semantic: &Semantic,
    target_component: &str,
    current_file: &Path,
) -> Result<bool> {
    // 1. Direct JSX analysis with import classification
    if jsx_contains_local_component_semantic(semantic, target_component, current_file)? {
        return Ok(true);
    }
    
    // 2. Analyze imported components with semantic filtering
    let jsx_components = extract_jsx_components_with_semantic_info(semantic, current_file)?;
    
    for jsx_info in jsx_components {
        // Skip external components early using semantic analysis
        if jsx_info.classification == ImportSourceClassification::External { .. } {
            debug(&format!("❌ Skipping external JSX component: {}", jsx_info.name));
            continue;
        }
        
        // For local components, check for presence calls
        if let ImportSourceClassification::Local { resolved_path } = &jsx_info.classification {
            // Recursive analysis of component file
            if component_file_contains_target_semantic(resolved_path, target_component, current_file)? {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}

#[derive(Debug)]
struct SemanticJSXComponentInfo {
    name: String,                           // "MyTest.Child"
    namespace: Option<String>,              // "MyTest" 
    property: Option<String>,               // "Child"
    classification: ImportSourceClassification,
    symbol_id: Option<SymbolId>,           // oxc symbol tracking
}

fn extract_jsx_components_with_semantic_info(
    semantic: &Semantic,
    current_file: &Path,
) -> Result<Vec<SemanticJSXComponentInfo>> {
    let mut components = Vec::new();
    
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(jsx_info) = analyze_jsx_with_semantic_context(semantic, jsx_opening, current_file)? {
                components.push(jsx_info);
            }
        }
    }
    
    Ok(components)
}
```

### 6. Recursive Component Analysis with oxc
```rust
fn component_file_contains_target_semantic(
    component_file: &str,
    target_component: &str,
    original_file: &Path,
) -> Result<bool> {
    // Parse the component file using oxc
    let source_text = std::fs::read_to_string(component_file)?;
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(component_file)).unwrap_or_default();
    
    let oxc_parser::ParserReturn { program, errors, .. } = 
        oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();
    
    if !errors.is_empty() {
        return Ok(false);
    }
    
    let semantic_ret = SemanticBuilder::new().build(&program);
    let semantic = &semantic_ret.semantic;
    
    // 1. Check for direct isComponentPresent calls
    if has_presence_call_for_component(semantic, target_component) {
        return Ok(true);
    }
    
    // 2. Check JSX content recursively
    if jsx_contains_target_component_recursive(semantic, target_component, Path::new(component_file))? {
        return Ok(true);
    }
    
    Ok(false)
}

fn jsx_contains_target_component_recursive(
    semantic: &Semantic,
    target_component: &str,
    current_file: &Path,
) -> Result<bool> {
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            // Resolve JSX element to actual component
            if let Some(resolved_component) = resolve_jsx_to_component_semantic(
                semantic, 
                jsx_opening, 
                current_file
            )? {
                if resolved_component == target_component {
                    return Ok(true);
                }
                
                // For member expressions like MyTest.Child -> MyTestChild
                if target_component.contains('.') {
                    if jsx_member_resolves_to_target(&jsx_opening.name, target_component, semantic)? {
                        return Ok(true);
                    }
                } else {
                    // Simple name matching with namespace resolution
                    if resolve_member_to_simple_name(&jsx_opening.name, semantic)? == target_component {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
```

## Key Benefits of Enhanced oxc Integration

### 1. Accurate Symbol Resolution
- Uses oxc's actual symbol table instead of string matching
- Tracks imports through their symbol IDs
- Distinguishes between local declarations and imports

### 2. Scope-Aware Analysis  
- Leverages oxc's scope tree for proper identifier resolution
- Handles nested scopes and shadowing correctly
- Identifies unresolved references (likely external)

### 3. Semantic Import Classification
- Combines oxc resolver with semantic analysis
- Handles complex import scenarios (re-exports, barrel files)
- Robust fallback when resolution fails

### 4. Recursive Component Detection
- Uses semantic analysis at each level of recursion
- Properly resolves JSX member expressions through symbol table
- Maintains context across file boundaries

## Implementation Strategy

### Phase 1: Enhanced Symbol Tracking
- Implement `SemanticJSXComponentInfo` structure
- Add symbol ID tracking for imports
- Create semantic-aware import classification

### Phase 2: Recursive Analysis with oxc
- Add `component_file_contains_target_semantic()`
- Implement JSX member expression resolution
- Add recursive JSX analysis

### Phase 3: Integration and Optimization
- Integrate with existing `has_component()` function
- Add caching for semantic analysis results
- Optimize for performance with large codebases

This approach maximizes the use of oxc's semantic analysis capabilities while providing robust component detection for complex scenarios.