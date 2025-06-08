use std::collections::{HashMap, HashSet};

use napi_derive::napi;
use oxc_ast::ast::*;
use oxc_ast::ast;
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use oxc_parser::{ Parser };
use oxc_semantic::{ ScopeId, SemanticBuilder, SemanticBuilderReturn };

// struct RootComponent {
//   name: String,
//   presence_checks: Vec<String>,
//   found_components: Vec<String>
// }


struct QwikAnalyzer {
  component_scopes: HashSet<ScopeId>,
  root_components: HashMap<ScopeId, String>,
}

impl<'a> Traverse<'a> for QwikAnalyzer {
  fn enter_call_expression(&mut self, node: &mut ast::CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
      let Expression::Identifier(ident) = &node.callee else {
        return;
      };

      if ident.name == "component$" {
        self.component_scopes.insert(ctx.current_scope_id());
      } else if ident.name == "usePresence" {
        if let Some(ast::Argument::Identifier(target)) = node.arguments.first() {
          let target_name = target.name.to_string();

          for ancestor_scope in ctx.ancestor_scopes() {
            if self.component_scopes.contains(&ancestor_scope) {
              println!("Root component in scope {:?} looks for: {}", ancestor_scope, target_name);
              self.root_components.insert(ancestor_scope, target_name);
              break;
            }
          }

        }
      };
  }  

}

#[napi]
fn transform_with_analysis(code: String, file_path: String) -> napi::Result<String> {
  let allocator = Allocator::new();
  let source_type = SourceType::from_path(&file_path).unwrap_or_default();
  let parse_return = Parser::new(&allocator, &code, source_type).parse();
  let mut program = parse_return.program;

  let SemanticBuilderReturn {
    semantic, errors: semantic_errors
  } = SemanticBuilder::new().build(&program);

  if !semantic_errors.is_empty() {
    eprintln!("Qwik Analyzer: Semantic errors found in: {}: {:?}", file_path, semantic_errors);
  }

  let mut analyzer = QwikAnalyzer {
    component_scopes: HashSet::new(),
    root_components: HashMap::new()
  };

  let scoping = semantic.into_scoping();

  traverse_mut(&mut analyzer, &allocator, &mut program, scoping);



  println!("Transforming: {}", file_path);

  Ok(code)
}