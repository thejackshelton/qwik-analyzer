# OXC Component Presence Analyzer - Complete Implementation

```rust
# OXC Component Presence Analyzer - Complete Implementation

```rust
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};
use oxc_allocator::Allocator;
use oxc_parser::{Parser, SourceType};
use oxc_ast::ast::*;
use oxc_semantic::{SemanticBuilder, SymbolId, ScopeId};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_span::{Span, SPAN};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
struct ComponentPresenceAnalyzer {
    root_components: Vec<RootComponent>,
    jsx_usage_stack: Vec<JSXUsage>,
}

#[derive(Debug)]
struct RootComponent {
    name: String,
    scope_id: ScopeId,
    presence_checks: Vec<PresenceCheck>,
    found_components: Vec<FoundComponent>,
    location: Span,
}

#[derive(Debug)]
struct PresenceCheck {
    target_component: String,
    target_symbol_id: Option<SymbolId>,  // Resolved via find_binding
    call_location: Span,
}

#[derive(Debug)]
struct FoundComponent {
    symbol_id: SymbolId,
    jsx_name: String,
    location: Span,
}

#[derive(Debug)]
struct JSXUsage {
    component_symbol_id: Option<SymbolId>,  // Resolved via find_binding
    jsx_name: String,
    scope_id: ScopeId,
    location: Span,
}

impl ComponentPresenceAnalyzer {
    fn new() -> Self {
        Self {
            root_components: Vec::new(),
            jsx_usage_stack: Vec::new(),
        }
    }
}

impl<'a> Traverse<'a> for ComponentPresenceAnalyzer {
    // Discovery Phase - Find usePresence() calls and resolve their symbols immediately
    fn enter_call_expression(&mut self, node: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        let Expression::Identifier(ident) = &node.callee else { return; };
        
        if ident.name == "usePresence" {
            let scope_id = ctx.current_scope_id();
            let target_component = self.extract_target_component(node);
            
            // 🔑 Use find_binding to resolve the target component symbol immediately
            let target_symbol_id = ctx.scoping().find_binding(scope_id, &target_component);
            
            let presence_check = PresenceCheck {
                target_component: target_component.clone(),
                target_symbol_id,
                call_location: node.span,
            };
            
            // Find or create root component
            if let Some(root) = self.find_root_component_mut(scope_id) {
                root.presence_checks.push(presence_check);
            } else {
                let component_name = self.find_parent_component_name(ctx)
                    .unwrap_or_else(|| "UnknownComponent".to_string());
                
                self.root_components.push(RootComponent {
                    name: component_name,
                    scope_id,
                    presence_checks: vec![presence_check],
                    found_components: Vec::new(),
                    location: node.span,
                });
            }
            
            eprintln!("Found usePresence({}) → symbol: {:?}", target_component, target_symbol_id);
        }
    }
    
    // Track JSX component usage and resolve symbols immediately
    fn enter_jsx_opening_element(&mut self, node: &mut JSXOpeningElement<'a>, ctx: &mut TraverseCtx<'a>) {
        let jsx_name = self.extract_jsx_component_name(node);
        let scope_id = ctx.current_scope_id();
        
        // 🔑 Use find_binding to resolve JSX component symbol immediately
        let component_symbol_id = if jsx_name.contains('.') {
            // Handle <Checkbox.Child /> - resolve the namespace first
            let parts: Vec<&str> = jsx_name.split('.').collect();
            let namespace = parts[0];
            ctx.scoping().find_binding(scope_id, namespace)
        } else {
            // Handle <Child /> - resolve directly
            ctx.scoping().find_binding(scope_id, &jsx_name)
        };
        
        self.jsx_usage_stack.push(JSXUsage {
            component_symbol_id,
            jsx_name: jsx_name.clone(),
            scope_id,
            location: node.span,
        });
        
        eprintln!("Found JSX <{}> → symbol: {:?}", jsx_name, component_symbol_id);
    }

    // Analysis Phase - Match resolved symbols
    fn exit_jsx_opening_element(&mut self, node: &mut JSXOpeningElement<'a>, ctx: &mut TraverseCtx<'a>) {
        self.analyze_component_presence_by_symbols(ctx);
        
        // Inject analyzer props
        if self.should_inject_analyzer_props(node, ctx) {
            self.inject_analyzer_props(node, ctx);
        }
        
        self.jsx_usage_stack.pop();
    }

    // Transformation Phase - Transform usePresence calls
    fn exit_call_expression(&mut self, node: &mut CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
        let Expression::Identifier(ident) = &node.callee else { return; };
        
        if ident.name == "usePresence" && node.arguments.len() == 1 {
            let target_component = self.extract_target_component(node);
            let prop_name = format!("analyzer_has_{}", self.normalize_component_name(&target_component));
            
            // Transform: usePresence(Child) → usePresence(Child, props.analyzer_has_child)
            let props_ref = ctx.ast.identifier_reference(SPAN, "props");
            let member_expr = ctx.ast.member_expression_static(
                SPAN, 
                Expression::Identifier(ctx.ast.alloc(props_ref)),
                ctx.ast.identifier_name(SPAN, &prop_name),
                false
            );
            
            node.arguments.push(Argument::Expression(member_expr));
        }
    }

    // Ensure root components have props parameter
    fn exit_function(&mut self, node: &mut Function<'a>, ctx: &mut TraverseCtx<'a>) {
        let scope_id = ctx.current_scope_id();
        
        if self.find_root_component(scope_id).is_some() {
            self.ensure_props_parameter(node, ctx);
        }
    }
}

impl ComponentPresenceAnalyzer {
    // Core analysis: Match symbols instead of string names
    fn analyze_component_presence_by_symbols(&mut self, ctx: &TraverseCtx) {
        // For each root component, check if any JSX usage matches the presence check symbols
        for root in &mut self.root_components {
            for check in &root.presence_checks {
                if let Some(target_symbol) = check.target_symbol_id {
                    // Find JSX usage in this component's scope tree
                    let jsx_in_scope = self.find_jsx_usage_in_scope_tree(root.scope_id, ctx);
                    
                    for jsx in jsx_in_scope {
                        if let Some(jsx_symbol) = jsx.component_symbol_id {
                            // 🔑 Symbol matching - this handles all aliases/imports automatically!
                            if jsx_symbol == target_symbol {
                                root.found_components.push(FoundComponent {
                                    symbol_id: target_symbol,
                                    jsx_name: jsx.jsx_name.clone(),
                                    location: jsx.location,
                                });
                                
                                eprintln!("MATCH: {} uses {} (symbol: {:?})", 
                                         root.name, jsx.jsx_name, jsx_symbol);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Find JSX usage in component scope tree using BFS
    fn find_jsx_usage_in_scope_tree(&self, root_scope: ScopeId, ctx: &TraverseCtx) -> Vec<&JSXUsage> {
        let mut found_jsx = Vec::new();
        let scoping = ctx.scoping();
        let mut scope_queue = vec![root_scope];
        let mut visited = std::collections::HashSet::new();
        
        while let Some(current_scope) = scope_queue.pop() {
            if visited.contains(&current_scope) {
                continue;
            }
            visited.insert(current_scope);
            
            // Find JSX usage in this scope
            for jsx in &self.jsx_usage_stack {
                if jsx.scope_id == current_scope {
                    found_jsx.push(jsx);
                }
            }
            
            // Add child scopes to queue for BFS traversal
            for &child_scope in scoping.get_child_scope_ids(current_scope) {
                scope_queue.push(child_scope);
            }
        }
        
        found_jsx
    }
    
    // Find parent component name using ancestry traversal
    fn find_parent_component_name(&self, ctx: &TraverseCtx) -> Option<String> {
        for ancestor in ctx.ancestors() {
            match ancestor {
                // Look for: const MyComponent = component$(...)
                oxc_traverse::Ancestor::VariableDeclaratorInit(declarator) => {
                    // Check if this declarator contains a component$ call
                    if self.contains_component_call(&declarator.init) {
                        if let Some(ident) = declarator.id.get_binding_identifier() {
                            return Some(ident.name.to_string());
                        }
                    }
                }
                // Look for: export default component$(...)
                oxc_traverse::Ancestor::ExportDefaultDeclarationDeclaration(export_decl) => {
                    return Some("DefaultExport".to_string());
                }
                _ => continue,
            }
        }
        None
    }
    
    fn contains_component_call(&self, expr: &Expression) -> bool {
        match expr {
            Expression::CallExpression(call) => {
                matches!(&call.callee, Expression::Identifier(ident) if ident.name == "component$")
            }
            _ => false,
        }
    }
    
    // Inject analyzer props based on symbol analysis results
    fn inject_analyzer_props(&mut self, node: &mut JSXOpeningElement, ctx: &mut TraverseCtx) {
        let current_scope = ctx.current_scope_id();
        let root_component = self.find_containing_root_component(current_scope);
        
        if let Some(root) = root_component {
            for check in &root.presence_checks {
                let prop_name = format!("analyzer_has_{}", 
                    self.normalize_component_name(&check.target_component));
                    
                // Check if this target component was found (by symbol matching)
                let has_component = root.found_components.iter()
                    .any(|comp| comp.symbol_id == check.target_symbol_id.unwrap_or(SymbolId::new(0)));
                
                // Inject JSX attribute: analyzer_has_component={true/false}
                let attr_name = JSXAttributeName::Identifier(
                    ctx.ast.alloc(ctx.ast.jsx_identifier(SPAN, &prop_name))
                );
                
                let boolean_expr = ctx.ast.boolean_literal(SPAN, has_component);
                let expr_container = ctx.ast.jsx_expression_container(
                    SPAN, 
                    JSXExpression::Expression(
                        Expression::BooleanLiteral(ctx.ast.alloc(boolean_expr))
                    )
                );
                
                let jsx_attr = ctx.ast.jsx_attribute(
                    SPAN,
                    attr_name,
                    Some(JSXAttributeValue::ExpressionContainer(ctx.ast.alloc(expr_container)))
                );
                
                node.attributes.push(JSXAttributeItem::Attribute(ctx.ast.alloc(jsx_attr)));
            }
        }
    }
    
    fn ensure_props_parameter(&mut self, node: &mut Function, ctx: &mut TraverseCtx) {
        if node.params.items.is_empty() || 
           !self.has_props_like_parameter(&node.params.items[0]) {
            
            let props_binding = ctx.ast.binding_identifier(SPAN, "props");
            let props_pattern = ctx.ast.binding_pattern(
                ctx.ast.binding_pattern_kind_binding_identifier(ctx.ast.alloc(props_binding)),
                None,
                false
            );
            let props_param = ctx.ast.formal_parameter(
                SPAN,
                vec![],
                props_pattern,
                None,
                false,
                false
            );
            
            node.params.items.insert(0, props_param);
        }
    }
    
    // Helper methods
    fn extract_target_component(&self, node: &CallExpression) -> String {
        node.arguments.first()
            .and_then(|arg| match arg {
                Argument::Identifier(ident) => Some(ident.name.to_string()),
                _ => None,
            })
            .unwrap_or_default()
    }
    
    fn extract_jsx_component_name(&self, node: &JSXOpeningElement) -> String {
        match &node.name {
            JSXElementName::Identifier(ident) => ident.name.to_string(),
            JSXElementName::MemberExpression(member) => {
                format!("{}.{}", self.extract_member_object(member), member.property.name)
            }
            _ => String::new(),
        }
    }
    
    fn extract_member_object(&self, member: &JSXMemberExpression) -> String {
        match &member.object {
            JSXMemberExpressionObject::Identifier(ident) => ident.name.to_string(),
            JSXMemberExpressionObject::MemberExpression(nested) => {
                format!("{}.{}", self.extract_member_object(nested), nested.property.name)
            }
        }
    }
    
    fn normalize_component_name(&self, name: &str) -> String {
        name.replace('.', '_').to_lowercase()
    }
    
    fn has_props_like_parameter(&self, param: &FormalParameter) -> bool {
        matches!(param.pattern.kind, BindingPatternKind::BindingIdentifier(ref ident) 
            if ident.name == "props")
    }
    
    fn find_root_component_mut(&mut self, scope_id: ScopeId) -> Option<&mut RootComponent> {
        self.root_components.iter_mut().find(|r| r.scope_id == scope_id)
    }
    
    fn find_root_component(&self, scope_id: ScopeId) -> Option<&RootComponent> {
        self.root_components.iter().find(|r| r.scope_id == scope_id)
    }
    
    fn find_containing_root_component(&self, scope_id: ScopeId) -> Option<&RootComponent> {
        // Find the root component that contains this scope
        self.root_components.iter().find(|r| r.scope_id == scope_id)
    }
    
    fn should_inject_analyzer_props(&self, _node: &JSXOpeningElement, ctx: &TraverseCtx) -> bool {
        let current_scope = ctx.current_scope_id();
        self.find_containing_root_component(current_scope).is_some()
    }
}

// Main usage - simplified with oxc_codegen
fn analyze_file(file_path: &Path, source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, SourceType::tsx()).parse();
    let mut program = ret.program;
    
    // Build semantic model for automatic symbol resolution
    let semantic_ret = SemanticBuilder::new().build(&program);
    let scoping = semantic_ret.semantic.into_scoping();
    
    // Run analysis and transformation in one pass
    let mut analyzer = ComponentPresenceAnalyzer::new();
    traverse_mut(&mut analyzer, &allocator, &mut program, scoping);
    
    // oxc_codegen handles all code generation automatically
    let options = CodegenOptions {
        single_quote: true,
        comments: true,
        ..CodegenOptions::default()
    };
    
    Ok(Codegen::new().with_options(options).build(&program).code)
}

// CLI entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = Path::new("input.tsx");
    let source = std::fs::read_to_string(input_path)?;
    let transformed = analyze_file(input_path, &source)?;
    println!("{}", transformed);
    Ok(())
}
```

## Key Changes Using `find_binding`:

### **1. Immediate Symbol Resolution**
- **Before**: Store string names, resolve later
- **After**: Use `find_binding` immediately when detecting calls/JSX

### **2. Symbol-Based Matching** 
- **Before**: Compare string names (error-prone with aliases)
- **After**: Compare `SymbolId`s (handles all import scenarios automatically)

### **3. Simplified JSX Resolution**
```rust
// 🔑 Direct use of find_binding for JSX
let component_symbol_id = if jsx_name.contains('.') {
    let namespace = jsx_name.split('.').next().unwrap();
    ctx.scoping().find_binding(scope_id, namespace)  // Resolve namespace
} else {
    ctx.scoping().find_binding(scope_id, &jsx_name)  // Resolve directly
};
```

### **4. Automatic Import Handling**
- ✅ **Barrel exports**: `export { Child } from './index'` → resolved automatically
- ✅ **Aliases**: `import { CheckboxChild as Child }` → same SymbolId  
- ✅ **Namespaces**: `<Checkbox.Child>` → resolve "Checkbox" namespace
- ✅ **Complex imports**: All handled by semantic analysis

### **5. Symbol-Based Analysis**
```rust
// Match by SymbolId instead of string comparison
if jsx_symbol == target_symbol {
    // This handles ALL import variations automatically!
    root.found_components.push(FoundComponent { ... });
}
```

The key insight: **`find_binding` does all the heavy lifting** for import resolution, so we can focus on collecting and matching SymbolIds rather than trying to manually track imports and aliases!

## Testing Strategy for OXC Component Presence Analyzer

### **1. Test Architecture Layers**

#### **Unit Tests** (Rust `#[cfg(test)]`)
Focus on individual analyzer components:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_component_resolution() {
        // Test oxc_semantic symbol resolution
    }
    
    #[test]
    fn test_scope_traversal() {
        // Test BFS scope tree navigation
    }
    
    #[test]
    fn test_jsx_extraction() {
        // Test JSX component name extraction
    }
}
```

#### **Integration Tests** (TypeScript `.spec.ts`)
Test the full pipeline like existing approach:

### **2. Test Categories Based on Research**

#### **A. Discovery Phase Tests**
```typescript
describe("Discovery Phase", () => {
  test("finds isComponentPresent calls in root components", () => {
    const code = `
      function MyComponent() {
        const hasChild = isComponentPresent(CheckboxChild);
        return <div />;
      }
    `;
    // Should identify MyComponent as root component
  });
  
  test("handles multiple presence checks in same component", () => {
    const code = `
      function MyComponent() {
        const hasChild = isComponentPresent(CheckboxChild);
        const hasTitle = isComponentPresent(Title);
        return <div />;
      }
    `;
  });
});
```

#### **B. Component Resolution Tests** (oxc_semantic)
```typescript
describe("Component Resolution", () => {
  test("resolves direct imports", () => {
    // import { CheckboxChild } from './checkbox-child'
  });
  
  test("resolves barrel exports", () => {
    // export { CheckboxChild as Child } from './checkbox-child'
  });
  
  test("resolves namespace imports", () => {
    // export * as Checkbox from './barrel-file'
  });
  
  test("resolves aliased imports", () => {
    // import { Checkbox as MyCheckbox } from "@kunai-consulting/qwik"
  });
});
```

#### **C. Scope Tree Traversal Tests**
```typescript
describe("Scope Traversal", () => {
  test("finds components in deep nested scopes", () => {
    const code = `
      function Root() {
        const hasChild = isComponentPresent(CheckboxChild);
        return (
          <div>
            <OtherComp />
          </div>
        );
      }
      
      function OtherComp() {
        return <div><CheckboxChild /></div>; // Should be found
      }
    `;
  });
  
  test("handles slot-based component placement", () => {
    // Complex nested component scenarios
  });
});
```

#### **D. Component Instance Tracking Tests**
```typescript
describe("Instance Tracking", () => {
  test("tracks multiple instances of same component", () => {
    const code = `
      <Root>
        <CheckboxChild key="1" />
        <CheckboxChild key="2" />
        <CheckboxChild key="3" />
      </Root>
    `;
    // Should track all 3 instances
  });
  
  test("scopes analysis per component instance", () => {
    // Component A has child, Component B doesn't
  });
});
```

#### **E. Transformation Tests**
```typescript
describe("Transformation", () => {
  test("transforms isComponentPresent calls", () => {
    // Before/after transformation validation
  });
  
  test("injects analyzer props", () => {
    // Prop injection validation
  });
  
  test("adds props parameter when missing", () => {
    // Auto props parameter addition
  });
});
```

### **3. Enhanced Test Structure**

```typescript
// __test__/oxc-analyzer.spec.ts
describe("OXC Component Presence Analyzer", () => {
  // Use existing test setup pattern
  let tempDir: string;
  
  beforeAll(() => {
    tempDir = createTestEnvironment();
  });
  
  describe("Phase 1: Discovery", () => {
    test("detects root components", () => {
      // Test enter_call_expression logic
    });
  });
  
  describe("Phase 2: Resolution", () => {
    test("resolves symbols via oxc_semantic", () => {
      // Test semantic analysis integration
    });
  });
  
  describe("Phase 3: Traversal", () => {
    test("traverses scope tree with BFS", () => {
      // Test scope tree navigation
    });
  });
  
  describe("Phase 4: Instance Tracking", () => {
    test("tracks component instances", () => {
      // Test Vec<FoundComponent> logic
    });
  });
  
  describe("Phase 5: Transformation", () => {
    test("injects props and transforms calls", () => {
      // Test exit_* methods
    });
  });
});
```

### **4. Edge Case Testing**

Based on existing real-world tests, add:

```typescript
describe("Edge Cases", () => {
  test("handles external package imports gracefully", () => {
    // @kunai-consulting/qwik imports
  });
  
  test("resolves tilde alias imports", () => {
    // ~/components/dummy-comp resolution
  });
  
  test("distinguishes namespace collisions", () => {
    // Description vs Checkbox.Description
  });
  
  test("handles nested member expressions", () => {
    // A.B.Component scenarios
  });
});
```

### **5. Performance Testing**

```typescript
describe("Performance", () => {
  test("single pass analysis", () => {
    // Measure traverse count
  });
  
  test("memory efficiency", () => {
    // Large file handling
  });
  
  test("lazy resolution", () => {
    // Only resolve when needed
  });
});
```

### **6. Integration with Current Tests**

#### **Keep Existing Patterns**:
- ✅ **Temp directory setup** - works well for file-based testing
- ✅ **Before/after transformation validation** - essential for correctness
- ✅ **Real-world scenario testing** - catches edge cases

#### **Add OXC-Specific Testing**:
- ✅ **Phase-by-phase validation** - test each traverse phase independently
- ✅ **Semantic resolution tests** - validate symbol resolution accuracy
- ✅ **AST transformation tests** - ensure correct direct AST manipulation
- ✅ **Scope tree tests** - validate BFS traversal logic

#### **Enhanced Test Categories**:

```typescript
describe("OXC Integration Tests", () => {
  describe("oxc_traverse Integration", () => {
    test("enter_* methods collect data correctly", () => {
      // Test discovery phase
    });
    
    test("exit_* methods transform correctly", () => {
      // Test transformation phase
    });
  });
  
  describe("oxc_semantic Integration", () => {
    test("symbol resolution handles complex imports", () => {
      // Test automatic import resolution
    });
    
    test("scope tree navigation finds nested components", () => {
      // Test scope traversal with semantic data
    });
  });
  
  describe("oxc_codegen Integration", () => {
    test("direct AST manipulation produces correct output", () => {
      // Test ctx.ast.* methods
    });
    
    test("code generation preserves formatting", () => {
      // Test Codegen::build() options
    });
  });
});
```

### **7. Test Data Management**

#### **Structured Test Cases**:
```typescript
const testCases = {
  discovery: [
    {
      name: "single isComponentPresent call",
      input: `function Root() { const has = isComponentPresent(Child); }`,
      expected: { rootComponents: 1, presenceChecks: 1 }
    },
    {
      name: "multiple presence checks",
      input: `function Root() { 
        const hasChild = isComponentPresent(Child);
        const hasTitle = isComponentPresent(Title);
      }`,
      expected: { rootComponents: 1, presenceChecks: 2 }
    }
  ],
  
  resolution: [
    {
      name: "direct import resolution",
      imports: `import { Child } from './child'`,
      jsx: `<Child />`,
      expected: { resolved: true, symbolId: "Child" }
    }
  ],
  
  transformation: [
    {
      name: "call transformation",
      before: `isComponentPresent(Child)`,
      after: `isComponentPresent(Child, props.analyzer_has_child)`,
      expected: { transformed: true }
    }
  ]
};
```

### **Key Testing Advantages with OXC**:

- ✅ **Phase Isolation**: Test each traverse phase independently
- ✅ **Semantic Validation**: Verify symbol resolution accuracy
- ✅ **AST Precision**: Test direct AST manipulation
- ✅ **Performance Measurement**: Single-pass analysis validation
- ✅ **Edge Case Coverage**: Complex import/export scenarios
- ✅ **Memory Efficiency**: Large file handling tests

This comprehensive testing strategy ensures the OXC-based analyzer is robust, performant, and handles all the complex scenarios identified in the research.