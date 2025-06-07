use napi_derive::napi;
use oxc_ast::ast::*;
use oxc_ast::ast;
use oxc_traverse::{traverse_mut, Ancestor, Traverse, TraverseCtx};
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use oxc_parser::{ Parser, ParserReturn };
use oxc_semantic::{ Semantic, SemanticBuilder, SemanticBuilderReturn };

struct RootComponent {
  name: String,
  presence_checks: Vec<String>,
  found_components: Vec<String>
}


struct QwikAnalyzer {
  root_components: Vec<RootComponent>,
}

impl<'a> Traverse<'a> for QwikAnalyzer {
  fn enter_call_expression(&mut self, node: &mut ast::CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
      if let Expression::Identifier(ident) = &node.callee {
        if ident.name == "isComponentPresent" {
          println!("Component present!: ");
        }
      }
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

  let mut analyzer = QwikAnalyzer {
    root_components: Vec::new()
  };

  let scoping = semantic.into_scoping();

  traverse_mut(&mut analyzer, &allocator, &mut program, scoping);



  println!("Transforming: {}", file_path);

  Ok(code)
}