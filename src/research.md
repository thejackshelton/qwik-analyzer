# OXC Component Presence Analysis Guide

## Overview
This guide outlines how to implement a static analysis tool using OXC that tracks component presence across complex component hierarchies, handling slots, aliases, and barrel exports.

## Architecture

### Phase 1: Discovery Phase (Enter Traversal)
- Identify "root" components containing `isComponentPresent()` calls
- Build component import/export graph using oxc_resolver  
- Map component aliases and barrel exports
- Track JSX component usage throughout the tree

### Phase 2: Analysis Phase (Semantic Analysis)
- Use oxc_semantic to resolve symbol references
- Build component relationship graph
- Determine which components are actually rendered within root component scopes

### Phase 3: Transformation Phase (Exit Traversal)
- Inject analyzer props into root components
- Transform `isComponentPresent()` calls to include prop references
- Generate scoped presence indicators

## Implementation Strategy

```rust
use oxc_traverse::{Traverse, TraverseCtx};
use oxc_semantic::{SemanticBuilder, AstNode};
use oxc_resolver::{Resolver, ResolveOptions};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct ComponentPresenceAnalyzer {
    // Core data structures
    root_components: Vec<RootComponent>,
    component_graph: ComponentGraph,
    resolver: Resolver,
    
    // Analysis state
    current_scope_stack: Vec<ScopeId>,
    jsx_usage_stack: Vec<JSXUsage>,
    pending_transformations: Vec<Transformation>,
}

#[derive(Debug)]
struct RootComponent {
    name: String,
    scope_id: ScopeId,
    presence_checks: Vec<PresenceCheck>,
    found_components: HashSet<String>,
    location: Span,
}

#[derive(Debug)]
struct PresenceCheck {
    target_component: String,
    resolved_imports: Vec<ImportPath>,
    call_location: Span,
}

#[derive(Debug)]
struct ComponentGraph {
    // Maps component names to their resolved file paths
    component_files: HashMap<String, PathBuf>,
    // Maps import specifiers to actual component names
    import_aliases: HashMap<String, String>,
    // Barrel export mappings
    barrel_exports: HashMap<String, Vec<String>>,
}
```

## Step-by-Step Implementation

### 1. Setup and Initialization

```rust
impl ComponentPresenceAnalyzer {
    fn new(project_root: PathBuf) -> Self {
        let resolver = Resolver::new(ResolveOptions {
            extensions: vec![".tsx".into(), ".ts".into(), ".jsx".into(), ".js".into()],
            condition_names: vec!["import".into(), "module".into(), "default".into()],
            ..Default::default()
        });
        
        Self {
            root_components: Vec::new(),
            component_graph: ComponentGraph::new(),
            resolver,
            current_scope_stack: Vec::new(),
            jsx_usage_stack: Vec::new(),
            pending_transformations: Vec::new(),
        }
    }
}
```

### 2. Discovery Phase - Enter Traversal

```rust
impl<'a> Traverse<'a> for ComponentPresenceAnalyzer {
    // Track function/component scope entry
    fn enter_function(&mut self, node: &mut Function<'a>, ctx: &mut TraverseCtx<'a>) {
        let scope_id = ctx.current_scope_id();
        self.current_scope_stack.push(scope_id);
        
        // Check if this function will become a root component
        // (we'll determine this when we find isComponentPresent calls)
    }
    
    // Discover isComponentPresent() calls
    fn enter_call_expression(&mut self, node: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        if self.is_component_present_call(node) {
            // This function becomes a root component
            let scope_id = *self.current_scope_stack.last().unwrap();
            let target_component = self.extract_target_component(node);
            
            // Resolve the target component through imports/exports
            let resolved_imports = self.resolve_component_imports(&target_component, ctx);
            
            let presence_check = PresenceCheck {
                target_component,
                resolved_imports,
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
    
    // Track all JSX component usage
    fn enter_jsx_opening_element(&mut self, node: &mut JSXOpeningElement<'a>, ctx: &mut TraverseCtx<'a>) {
        let jsx_name = self.extract_jsx_component_name(node);
        let resolved_component = self.resolve_jsx_component(&jsx_name, ctx);
        
        self.jsx_usage_stack.push(JSXUsage {
            component_name: jsx_name,
            resolved_name: resolved_component,
            scope_id: *self.current_scope_stack.last().unwrap(),
            location: node.span,
        });
    }
    
    // Track import declarations for component resolution
    fn enter_import_declaration(&mut self, node: &mut ImportDeclaration<'a>, ctx: &mut TraverseCtx<'a>) {
        self.process_import_declaration(node, ctx);
    }
}
```

### 3. Component Resolution System

```rust
impl ComponentPresenceAnalyzer {
    fn resolve_component_imports(&mut self, component_name: &str, ctx: &TraverseCtx) -> Vec<ImportPath> {
        let mut resolved_paths = Vec::new();
        
        // 1. Check direct imports in current file
        if let Some(import_path) = self.find_direct_import(component_name, ctx) {
            resolved_paths.push(import_path);
        }
        
        // 2. Check barrel exports (index.ts files)
        if let Some(barrel_paths) = self.component_graph.barrel_exports.get(component_name) {
            resolved_paths.extend(barrel_paths.clone());
        }
        
        // 3. Use oxc_resolver to resolve file paths
        for path in &mut resolved_paths {
            if let Ok(resolution) = self.resolver.resolve(&ctx.file_path, &path.specifier) {
                path.resolved_file = Some(resolution.full_path().to_path_buf());
            }
        }
        
        resolved_paths
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

### 4. Analysis Phase - Component Matching

```rust
impl ComponentPresenceAnalyzer {
    fn analyze_component_presence(&mut self, ctx: &TraverseCtx) {
        for root in &mut self.root_components {
            for check in &root.presence_checks {
                // Find all JSX usages within this root component's scope
                let jsx_in_scope = self.find_jsx_usage_in_scope_tree(root.scope_id, ctx);
                
                // Check if any JSX usage matches the target component
                for jsx in jsx_in_scope {
                    if self.component_matches_target(&jsx.resolved_name, &check.target_component) {
                        root.found_components.insert(check.target_component.clone());
                        break;
                    }
                }
            }
        }
    }
    
    fn find_jsx_usage_in_scope_tree(&self, root_scope: ScopeId, ctx: &TraverseCtx) -> Vec<&JSXUsage> {
        let mut found_jsx = Vec::new();
        
        // Use oxc_semantic to traverse scope hierarchy
        let semantic = ctx.semantic();
        let scopes = semantic.scopes();
        
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
            
            // Add child scopes to queue
            for child_scope in scopes.get_child_ids(current_scope) {
                scope_queue.push(child_scope);
            }
        }
        
        found_jsx
    }
    
    fn component_matches_target(&self, jsx_component: &str, target: &str) -> bool {
        // Direct match
        if jsx_component == target {
            return true;
        }
        
        // Check through import aliases
        if let Some(aliased) = self.component_graph.import_aliases.get(jsx_component) {
            if aliased == target {
                return true;
            }
        }
        
        // Check barrel exports
        if let Some(exports) = self.component_graph.barrel_exports.get(target) {
            if exports.contains(&jsx_component.to_string()) {
                return true;
            }
        }
        
        false
    }
}
```

### 5. Transformation Phase - Exit Traversal

```rust
impl<'a> Traverse<'a> for ComponentPresenceAnalyzer {
    fn exit_function(&mut self, node: &mut Function<'a>, ctx: &mut TraverseCtx<'a>) {
        let scope_id = self.current_scope_stack.pop().unwrap();
        
        // Apply transformations for root components
        if let Some(root) = self.find_root_component(scope_id) {
            self.apply_root_component_transformations(node, root, ctx);
        }
    }
    
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
        let current_scope = *self.current_scope_stack.last().unwrap();
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
fn analyze_file(file_path: &Path, source: &str) -> Result<String, Error> {
    let mut analyzer = ComponentPresenceAnalyzer::new(file_path.parent().unwrap().to_path_buf());
    
    // Parse with oxc
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
    let mut program = ret.program;
    
    // Build semantic model
    let semantic = SemanticBuilder::new(source, SourceType::tsx())
        .build(&program)
        .semantic;
    
    // Create traverse context
    let mut ctx = TraverseCtx::new(ScopeTree::default(), SymbolTable::default(), &mut semantic);
        
    // Run analysis
    analyzer.traverse(&mut program, &mut ctx);
    
    // Generate transformed code
    let codegen = Codegen::<false>::new("", source, CodegenOptions::default());
    Ok(codegen.build(&program).source_text)
}
```

This approach provides comprehensive component presence analysis while handling the complex patterns common in modern Qwik applications.
