use napi_derive::napi;
use oxc_ast::*;
use oxc_traverse::{Ancestor, Traverse, TraverseCtx};

#[derive(Debug)]
#[napi(object)]
struct ComponentInfo {
  pub name: String,
  pub found: bool,
}

struct QwikAnalyzer {
  found_components: Vec<ComponentInfo>,
}

impl<'a> Traverse<'a> for QwikAnalyzer {
  fn enter_call_expression(&mut self, node: &mut ast::CallExpression<'a>, ctx: &mut TraverseCtx<'a>) {
      println!("Inside a call expression! {:?}", &node)
  }  
}

fn main() {


  println!("HEYYYY {:?}", &analzyer);
}

#[napi]
fn analyze_file(file_path: string) -> napi::Result<ComponentInfo> {
  
}