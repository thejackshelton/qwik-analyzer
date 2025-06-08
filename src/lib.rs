use napi_derive::napi;
use oxc_ast::ast::*;
use oxc_ast::ast;
use oxc_traverse::{traverse_mut, Traverse, TraverseCtx};
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use oxc_parser::{ Parser };
use oxc_semantic::{ SemanticBuilder, SemanticBuilderReturn };

struct RootComponent {
  name: String,
  presence_checks: Vec<String>,
  found_components: Vec<String>
}


struct QwikAnalyzer {
  root_components: Vec<RootComponent>,
}

impl QwikAnalyzer {
  fn find_root_from_presence_check() {
    
  }
}

impl<'a> Traverse<'a> for QwikAnalyzer {
  fn enter_call_expression(&mut self, node: &mut ast::CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
      let Expression::Identifier(ident) = &node.callee else {
        return;
      };

      if ident.name == "usePresence" {
        println!("I would pass the check! {:?}", &node)
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
    root_components: Vec::new()
  };

  let scoping = semantic.into_scoping();

  traverse_mut(&mut analyzer, &allocator, &mut program, scoping);



  println!("Transforming: {}", file_path);

  Ok(code)
}