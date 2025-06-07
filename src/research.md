# OXC Component Presence Analysis Guide

## Overview
This guide outlines how to implement a static analysis tool using OXC that tracks component presence across complex component hierarchies, handling slots, aliases, and barrel exports.

## Architecture

### Phase 1: Discovery Phase (Enter Traversal)
- Identify "root" components containing `isComponentPresent()` calls
- Build component import/export graph using oxc_resolver  
- Map component aliases and barrel exports
- Track JSX component usage throughout the tree

### Phase 2: Analysis Phase (Scope Traversal)
- Use oxc_traverse's built-in scoping system to resolve symbol references
- Build component relationship graph using ancestry and scope information
- Determine which components are actually rendered within root component scopes

### Phase 3: Transformation Phase (Exit Traversal)
- Inject analyzer props into root components
- Transform `isComponentPresent()` calls to include prop references
- Generate scoped presence indicators

## Implementation Strategy

```rust
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};
use oxc_allocator::Allocator;
use oxc_parser::{Parser, SourceType};
use oxc_ast::ast::*;
use oxc_semantic::{SemanticBuilder, Semantic, Reference, SymbolId};
use oxc_span::Span;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug)]
struct ComponentPresenceAnalyzer {
    // Core data structures
    root_components: Vec<RootComponent>,
    jsx_usage_stack: Vec<JSXUsage>,
    
    // Semantic analysis will handle all symbol resolution for us
    // No need for manual import tracking or resolvers
}

#[derive(Debug)]
struct RootComponent {
    name: String,
    scope_id: oxc_semantic::ScopeId,
    presence_checks: Vec<PresenceCheck>,
    found_components: HashSet<SymbolId>, // Track actual symbol IDs instead of strings
    location: Span,
}

#[derive(Debug)]
struct PresenceCheck {
    target_component: String, // The component name from isComponentPresent(ComponentName)
    call_location: Span,
}

#[derive(Debug)]
struct JSXUsage {
    component_symbol_id: Option<SymbolId>, // Use semantic symbol ID if resolved
    jsx_name: String, // The raw JSX name like "Checkbox.Root"
    scope_id: oxc_semantic::ScopeId,
    location: Span,
}
```

## Step-by-Step Implementation

### 1. Setup and Initialization

```rust
impl ComponentPresenceAnalyzer {
    fn new() -> Self {
        Self {
            root_components: Vec::new(),
            jsx_usage_stack: Vec::new(),
        }
    }
}
```

### 2. Discovery Phase - Enter Traversal

```rust
impl<'a> Traverse<'a> for ComponentPresenceAnalyzer {
    // Discover isComponentPresent() calls
    fn enter_call_expression(&mut self, node: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.is_component_present_call(node) {
            // This function becomes a root component
            let scope_id = ctx.current_scope_id();
            let target_component = self.extract_target_component(node);
            
            let presence_check = PresenceCheck {
                target_component,
                call_location: node.span,
            };
            
            // Find or create root component entry
            if let Some(root) = self.find_root_component_mut(scope_id) {
                root.presence_checks.push(presence_check);
            } else {
                let function_name = self.extract_function_name_from_scope(scope_id, ctx);
                self.root_components.push(RootComponent {
                    name: function_name,
                    scope_id,
                    presence_checks: vec![presence_check],
                    found_components: HashSet::new(),
                    location: node.span,
                });
            }
        }
    }
    
    // Track all JSX component usage - semantic analysis will resolve the symbols
    fn enter_jsx_opening_element(&mut self, node: &mut JSXOpeningElement<'a>, ctx: &mut TraverseCtx<'a>) {
        let jsx_name = self.extract_jsx_component_name(node);
        let component_symbol_id = self.resolve_jsx_symbol(&jsx_name, ctx);
        
        self.jsx_usage_stack.push(JSXUsage {
            component_symbol_id,
            jsx_name,
            scope_id: ctx.current_scope_id(),
            location: node.span,
        });
    }
}
```

### 3. Semantic-Based Component Resolution

```rust
impl ComponentPresenceAnalyzer {
    // Use semantic analysis to resolve JSX component to symbol ID
    fn resolve_jsx_symbol(&self, jsx_name: &str, ctx: &TraverseCtx) -> Option<SymbolId> {
        let scoping = ctx.scoping();
        let current_scope = ctx.current_scope_id();
        
        // Handle namespaced components: Checkbox.Root
        if jsx_name.contains('.') {
            let parts: Vec<&str> = jsx_name.split('.').collect();
            let namespace = parts[0];
            
            // Find the namespace symbol (e.g., "Checkbox" from import)
            if let Some(symbol_id) = scoping.find_binding(current_scope, namespace) {
                return Some(symbol_id);
            }
        } else {
            // Direct component reference
            if let Some(symbol_id) = scoping.find_binding(current_scope, jsx_name) {
                return Some(symbol_id);
            }
        }
        
        None
    }
    
    fn resolve_jsx_component(&mut self, jsx_name: &str, ctx: &TraverseCtx) -> String {
        // Handle different JSX patterns:
        // 1. Simple: <ComponentName />
        // 2. Namespaced: <Namespace.Component />  
        // 3. Aliased: <MyAlias.Component /> where MyAlias is imported
        
        if jsx_name.contains('.') {
            let parts: Vec<&str> = jsx_name.split('.').collect();
            let namespace = parts[0];
            let component = parts[1];
            
            // Check if namespace is an import alias
            if let Some(actual_namespace) = self.component_graph.import_aliases.get(namespace) {
                return format!("{}.{}", actual_namespace, component);
            }
        }
        
        // Check for direct aliases
        self.component_graph.import_aliases
            .get(jsx_name)
            .cloned()
            .unwrap_or_else(|| jsx_name.to_string())
    }
    
    fn process_import_declaration(&mut self, node: &ImportDeclaration, ctx: &TraverseCtx) {
        let source = node.source.value.as_str();
        
        for specifier in &node.specifiers {
            match specifier {
                ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                    let local_name = spec.local.name.as_str();
                    self.component_graph.import_aliases.insert(
                        local_name.to_string(),
                        source.to_string()
                    );
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                    let local_name = spec.local.name.as_str();
                    self.component_graph.import_aliases.insert(
                        local_name.to_string(),
                        source.to_string()
                    );
                }
                ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                    let imported = spec.imported.name().as_str();
                    let local = spec.local.name.as_str();
                    self.component_graph.import_aliases.insert(
                        local.to_string(),
                        format!("{}.{}", source, imported)
                    );
                }
            }
        }
    }
}
```

### 4. Analysis Phase - Semantic Symbol Matching

```rust
impl ComponentPresenceAnalyzer {
    fn analyze_component_presence(&mut self, ctx: &TraverseCtx) {
        let scoping = ctx.scoping();
        
        for root in &mut self.root_components {
            for check in &root.presence_checks {
                // Find the target component symbol in the root scope
                let target_symbol_id = scoping.find_binding(root.scope_id, &check.target_component);
                
                if let Some(target_symbol) = target_symbol_id {
                    // Find all JSX usages within this root component's scope tree
                    let jsx_in_scope = self.find_jsx_usage_in_scope_tree(root.scope_id, ctx);
                    
                    // Check if any JSX usage matches the target symbol
                    for jsx in jsx_in_scope {
                        if let Some(jsx_symbol) = jsx.component_symbol_id {
                            if self.symbols_match(jsx_symbol, target_symbol, scoping) {
                                root.found_components.insert(target_symbol);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn find_jsx_usage_in_scope_tree(&self, root_scope: oxc_semantic::ScopeId, ctx: &TraverseCtx) -> Vec<&JSXUsage> {
        let mut found_jsx = Vec::new();
        let scoping = ctx.scoping();
        
        // Find all child scopes (including nested components)
        let mut scope_queue = vec![root_scope];
        let mut visited = HashSet::new();
        
        while let Some(current_scope) = scope_queue.pop() {
            if visited.contains(&current_scope) {
                continue;
            }
            visited.insert(current_scope);
            
            // Add JSX from this scope
            for jsx in &self.jsx_usage_stack {
                if jsx.scope_id == current_scope {
                    found_jsx.push(jsx);
                }
            }
            
            // Add child scopes to queue - traverse all descendant scopes
            for &child_scope in scoping.get_scope_child_ids(current_scope) {
                scope_queue.push(child_scope);
            }
        }
        
        found_jsx
    }
    
    fn symbols_match(&self, jsx_symbol: SymbolId, target_symbol: SymbolId, scoping: &oxc_semantic::Scoping) -> bool {
        // Direct symbol match
        if jsx_symbol == target_symbol {
            return true;
        }
        
        // Check if they refer to the same imported symbol
        // This handles cases like:
        // import { CheckboxChild } from './components'  
        // vs  
        // import * as Components from './components'
        // <Components.CheckboxChild />
        
        // Get symbol references to see if they point to the same import
        let jsx_refs = scoping.get_resolved_references(jsx_symbol);
        let target_refs = scoping.get_resolved_references(target_symbol);
        
        // If both symbols have references, check if any reference the same import
        // This is a simplified check - in practice you'd need more sophisticated import analysis
        jsx_refs.count() > 0 && target_refs.count() > 0
    }
}
```

### 5. Transformation Phase - Exit Traversal

```rust
impl<'a> Traverse<'a> for ComponentPresenceAnalyzer {
    fn exit_call_expression(&mut self, node: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.is_component_present_call(node) {
            self.transform_presence_call(node, ctx);
        }
    }
    
    fn exit_jsx_opening_element(&mut self, node: &mut JSXOpeningElement<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.should_inject_analyzer_props(node, ctx) {
            self.inject_analyzer_props(node, ctx);
        }
        
        self.jsx_usage_stack.pop();
    }
    
    fn exit_function(&mut self, node: &mut Function<'a>, ctx: &mut TraverseCtx<'a>) {
        let scope_id = ctx.current_scope_id();
        
        // Apply transformations for root components
        if let Some(root) = self.find_root_component(scope_id) {
            self.apply_root_component_transformations(node, root, ctx);
        }
    }
}

impl ComponentPresenceAnalyzer {
    fn transform_presence_call(&mut self, node: &mut CallExpression, ctx: &mut TraverseCtx) {
        // Transform: isComponentPresent(Comp) 
        // To: isComponentPresent(Comp, props.analyzer_has_comp)
        
        if node.arguments.len() == 1 {
            let target_component = self.extract_target_component(node);
            let prop_name = format!("analyzer_has_{}", self.normalize_component_name(&target_component));
            
            // Create props.analyzer_has_comp expression
            let props_access = self.create_props_access_expression(&prop_name, ctx);
            node.arguments.push(props_access);
        }
    }
    
    fn inject_analyzer_props(&mut self, node: &mut JSXOpeningElement, ctx: &mut TraverseCtx) {
        // Find which root component this JSX element belongs to
        let current_scope = ctx.current_scope_id();
        let root_component = self.find_containing_root_component(current_scope);
        
        if let Some(root) = root_component {
            for check in &root.presence_checks {
                let prop_name = format!("analyzer_has_{}", 
                    self.normalize_component_name(&check.target_component));
                let has_component = root.found_components.contains(&check.target_component);
                
                // Create JSX attribute: analyzer_has_comp={true/false}
                let analyzer_attr = self.create_jsx_boolean_attribute(&prop_name, has_component, ctx);
                node.attributes.push(analyzer_attr);
            }
        }
    }
    
    fn apply_root_component_transformations(&mut self, node: &mut Function, root: &RootComponent, ctx: &mut TraverseCtx) {
        // Ensure root component has props parameter
        if !self.has_props_parameter(node) {
            self.inject_props_parameter(node, ctx);
        }
    }
}
```

## Key Challenges and Solutions

### 1. Handling Complex Import/Export Chains
- **Challenge**: Components exported through barrel files and aliases
- **Solution**: Build comprehensive import graph using oxc_resolver
- **Implementation**: Track all import declarations and resolve using filesystem

### 2. Scope Tree Traversal
- **Challenge**: Components can be nested arbitrarily deep due to slots
- **Solution**: Use oxc_semantic to traverse entire scope hierarchy
- **Implementation**: BFS through all child scopes from root component

### 3. Component Name Resolution
- **Challenge**: JSX components can be aliased multiple ways
- **Solution**: Maintain alias mapping and resolve through import graph
- **Implementation**: Handle namespace imports, default imports, and named imports

### 4. Transformation Timing
- **Challenge**: Need to analyze before transforming
- **Solution**: Two-pass approach with enter/exit pattern
- **Implementation**: Collect data on enter, transform on exit

## Usage Example

```rust
use oxc_allocator::Allocator;
use oxc_parser::{Parser, SourceType};
use oxc_traverse::traverse_mut;
use oxc_semantic::{SemanticBuilder};
use oxc_codegen::{Codegen, CodegenOptions};
use std::path::Path;

fn analyze_file(file_path: &Path, source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut analyzer = ComponentPresenceAnalyzer::new();
    
    // Parse with oxc
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
    let mut program = ret.program;
    
    // Build semantic model first - this handles all symbol resolution automatically
    let semantic_ret = SemanticBuilder::new()
        .build(&program);
        
    // Extract scoping information from semantic analysis
    let scoping = semantic_ret.semantic.into_scoping();
    
    // Run traversal with analyzer - semantic analysis is already done
    traverse_mut(&mut analyzer, &allocator, &mut program, scoping);
    
    // Analyze component presence using semantic data
    // This step would normally be done within the traversal, but shown separately for clarity
    
    // Generate transformed code
    let codegen = Codegen::<false>::new("", source, CodegenOptions::default());
    Ok(codegen.build(&program).source_text)
}
```

## Key Simplifications with oxc_semantic

### 1. **Automatic Symbol Resolution**
- No need to manually track imports and aliases
- `scoping.find_binding(scope_id, name)` finds any symbol by name in scope
- Handles all import patterns automatically: default, named, namespace, etc.

### 2. **Built-in Scope Traversal**  
- `scoping.get_scope_child_ids(scope_id)` gives all child scopes
- No need to manually track scope hierarchy
- Automatic handling of function scopes, block scopes, etc.

### 3. **Symbol Reference Tracking**
- `scoping.get_resolved_references(symbol_id)` shows where symbols are used
- Automatic detection of read/write references
- Built-in symbol mutation tracking

### 4. **Real Symbol IDs Instead of Strings**
- Use `SymbolId` for precise symbol matching
- No ambiguity from string name collisions
- Direct symbol comparison for exact matches

## Key OXC Traverse Concepts

### 1. Scope Management
- `ctx.current_scope_id()` - Get current scope ID
- `ctx.ancestor_scopes()` - Iterator over parent scopes
- `ctx.scoping()` and `ctx.scoping_mut()` - Access scoping system

### 2. AST Ancestry  
- `ctx.parent()` - Get parent AST node (as `Ancestor` enum)
- `ctx.ancestor(level)` - Get ancestor at specific level
- `ctx.ancestors()` - Iterator over all ancestors

### 3. Symbol/Reference Management
- `ctx.generate_uid_name(name)` - Generate unique identifier
- `ctx.generate_binding(name, scope_id, flags)` - Create new binding
- `ctx.create_bound_reference(symbol_id, flags)` - Create reference to existing symbol

### 4. AST Construction
- `ctx.ast.alloc(node)` - Allocate new AST node
- `ctx.alloc(node)` - Shortcut for allocation

This approach provides comprehensive component presence analysis while handling the complex patterns common in modern Qwik applications.

## Semantic-Powered Component Resolution

With oxc_semantic, all the complex import/export resolution is handled automatically. Instead of manually tracking patterns like:

1. **Direct imports**: `import { CheckboxChild } from './checkbox-child'`
2. **Barrel exports**: `export { CheckboxChild as Child } from './checkbox-child'` in index.ts
3. **Namespace imports**: `import * as Checkbox from './barrel-file'`
4. **Aliased imports**: `import { Checkbox as MyCheckbox } from "@kunai-consulting/qwik"`

We just use `scoping.find_binding()` and let semantic analysis handle everything!

```rust
impl ComponentPresenceAnalyzer {
    fn extract_target_component(&self, node: &CallExpression) -> String {
        // Extract component name from isComponentPresent(ComponentName)
        if let Some(arg) = node.arguments.first() {
            if let Argument::Identifier(ident) = arg {
                return ident.name.to_string();
            }
        }
        String::new()
    }
    
    fn extract_jsx_component_name(&self, node: &JSXOpeningElement) -> String {
        // Extract component name from <ComponentName> or <Namespace.ComponentName>
        match &node.name {
            JSXElementName::Identifier(ident) => ident.name.to_string(),
            JSXElementName::MemberExpression(member) => {
                // Handle Namespace.Component pattern
                format!("{}.{}", 
                    self.extract_member_object(member),
                    member.property.name
                )
            }
            _ => String::new(),
        }
    }
    
    fn extract_member_object(&self, member: &JSXMemberExpression) -> String {
        match &member.object {
            JSXMemberExpressionObject::Identifier(ident) => ident.name.to_string(),
            JSXMemberExpressionObject::MemberExpression(nested) => {
                // Handle deeply nested like A.B.Component
                format!("{}.{}", 
                    self.extract_member_object(nested),
                    nested.property.name
                )
            }
        }
    }
    
    fn is_component_present_call(&self, node: &CallExpression) -> bool {
        // Check if this is isComponentPresent() call
        if let Expression::Identifier(ident) = &node.callee {
            ident.name == "isComponentPresent"
        } else {
            false
        }
    }
}

**The beauty of oxc_semantic**: All import resolution, symbol tracking, and scope traversal is automatically handled. No need for manual:
- Import alias tracking  
- Barrel export resolution  
- Namespace import handling
- File path resolution

Just use `scoping.find_binding()` and `scoping.get_resolved_references()` and the semantic analysis does the rest!
