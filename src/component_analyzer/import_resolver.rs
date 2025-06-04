use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_parser;
use oxc_resolver::{ResolveOptions, Resolver};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, SourceType, VALID_EXTENSIONS};
use std::fs;
use std::path::Path;

use crate::component_analyzer::jsx_analysis::extract_jsx_element_name;
use crate::component_analyzer::utils::{
  debug, extract_component_name_from_argument, extract_function_name, ComponentPresenceCall,
};
use crate::Result;

pub fn find_import_source_for_component(
  semantic: &Semantic,
  component_name: &str,
) -> Option<String> {
  for node in semantic.nodes().iter() {
    let AstKind::ImportDeclaration(import_decl) = node.kind() else {
      continue;
    };

    let Some(specifiers) = &import_decl.specifiers else {
      continue;
    };

    for specifier in specifiers {
      let Some(specifier_name) = get_specifier_name(specifier) else {
        continue;
      };

      if specifier_name == component_name {
        debug(&format!(
          "📥 Found import for {}: {}",
          component_name, import_decl.source.value
        ));
        return Some(import_decl.source.value.to_string());
      }
    }
  }

  None
}

pub fn resolve_import_path(import_source: &str, current_file: &Path) -> Result<String> {
  let options = ResolveOptions {
    extensions: VALID_EXTENSIONS
      .iter()
      .map(|ext| format!(".{}", ext).into())
      .collect(),
    main_files: vec!["index".into()],
    main_fields: vec!["main".into()],
    ..Default::default()
  };

  let resolver = Resolver::new(options);
  let current_dir = current_file
    .parent()
    .ok_or("Could not get parent directory")?;

  if import_source.starts_with("~/") {
    let mut search_dir = current_dir;
    let mut project_root = None;

    while let Some(parent) = search_dir.parent() {
      if search_dir.join("package.json").exists() {
        project_root = Some(search_dir);
        break;
      }
      search_dir = parent;
    }

    if let Some(root) = project_root {
      let relative_path = &import_source[2..];
      let resolved_path = root.join("src").join(relative_path);
      if resolved_path.exists() {
        return Ok(resolved_path.to_string_lossy().to_string());
      }
      for ext in VALID_EXTENSIONS {
        let path_with_ext = resolved_path.with_extension(ext);
        if path_with_ext.exists() {
          return Ok(path_with_ext.to_string_lossy().to_string());
        }
      }
    }
  }

  match resolver.resolve(current_dir, import_source) {
    Ok(resolution) => {
      let resolved_path = resolution.full_path();
      Ok(resolved_path.to_string_lossy().to_string())
    }
    Err(e) => {
      debug(&format!(
        "❌ Import resolution failed for '{}': {:?}",
        import_source, e
      ));
      Err(format!("Could not resolve import '{}': {:?}", import_source, e).into())
    }
  }
}

pub fn resolve_component_from_index(index_file_path: &str, component_name: &str) -> Result<String> {
  debug(&format!(
    "🔍 Using oxc to resolve {} from index file: {}",
    component_name, index_file_path
  ));

  let source_text = fs::read_to_string(index_file_path)?;
  let allocator = Allocator::default();
  let source_type = SourceType::from_path(Path::new(index_file_path)).unwrap_or_default();

  let oxc_parser::ParserReturn {
    program, errors, ..
  } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

  if !errors.is_empty() {
    return Err("Failed to parse index file".into());
  }

  let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
  let semantic = &semantic_ret.semantic;

  // Look for object export pattern: export const MyTest = { Root: MyTestRoot, Child: MyTestChild }
  for node in semantic.nodes().iter() {
    match node.kind() {
      // Handle re-export pattern: export { MyTestRoot as Root } from "./my-test-root"
      AstKind::ExportNamedDeclaration(export_decl) => {
        for specifier in &export_decl.specifiers {
          let exported_name = &specifier.exported.name();
          if exported_name == component_name {
            if let Some(source) = &export_decl.source {
              debug(&format!(
                "📂 Found re-export {} from source: {}",
                component_name, source.value
              ));
              
              let index_file = Path::new(index_file_path);
              return resolve_import_path(&source.value, index_file);
            }
          }
        }
      }
      
      // Handle object export pattern: export const MyTest = { Root: MyTestRoot, Child: MyTestChild }
      AstKind::VariableDeclarator(declarator) => {
        // Check if this variable has an object expression as init
        if let Some(oxc_ast::ast::Expression::ObjectExpression(obj_expr)) = &declarator.init {
          // Look through object properties for our component
          for prop in &obj_expr.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
              if let oxc_ast::ast::PropertyKey::StaticIdentifier(key) = &obj_prop.key {
                if key.name == component_name {
                  // Found the property! Get the value which should be an identifier
                  if let oxc_ast::ast::Expression::Identifier(value_ident) = &obj_prop.value {
                    let import_name = &value_ident.name;
                    debug(&format!(
                      "📂 Found object property {} maps to identifier: {}",
                      component_name, import_name
                    ));
                    
                    // Find the import for this identifier in the same file
                    if let Some(import_source) = find_import_source_for_component(semantic, import_name) {
                      debug(&format!(
                        "📂 Found import source for {}: {}",
                        import_name, import_source
                      ));
                      
                      let index_file = Path::new(index_file_path);
                      return resolve_import_path(&import_source, index_file);
                    }
                  }
                }
              }
            }
          }
        }
      }
      _ => {}
    }
  }

  Err(format!("Could not find component {} in index file", component_name).into())
}


pub fn find_calls_in_file(file_path: &str) -> Result<Vec<ComponentPresenceCall>> {
  let source_text = fs::read_to_string(file_path)?;
  let allocator = Allocator::default();
  let source_type = SourceType::from_path(Path::new(file_path)).unwrap_or_default();

  let oxc_parser::ParserReturn {
    program, errors, ..
  } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

  if !errors.is_empty() {
    return Ok(Vec::new());
  }

  let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
  let semantic = &semantic_ret.semantic;

  let mut calls = Vec::new();

  for node in semantic.nodes().iter() {
    let AstKind::CallExpression(call_expr) = node.kind() else {
      continue;
    };

    let Some(function_name) = extract_function_name(call_expr) else {
      continue;
    };

    if function_name != "isComponentPresent" {
      continue;
    }

    let Some(first_arg) = call_expr.arguments.first() else {
      continue;
    };

    let component_name = if let Some(name) = extract_component_name_from_argument(first_arg) {
      name
    } else {
      let arg_span = first_arg.span();
      let full_text = &source_text[arg_span.start as usize..arg_span.end as usize];
      if full_text.contains('.') {
        debug(&format!(
          "Extracted full member expression from find_calls_in_file: {}",
          full_text
        ));
        full_text.to_string()
      } else {
        debug(&format!(
          "Could not extract component name from argument in find_calls_in_file"
        ));
        continue;
      }
    };

    debug(&format!(
      "🔍 Found isComponentPresent({}) call in {}",
      component_name, file_path
    ));

    calls.push(ComponentPresenceCall {
      component_name,
      is_present_in_subtree: false,
      source_file: file_path.to_string(),
    });
  }

  Ok(calls)
}

pub fn file_has_component(file_path: &str, target_component: &str) -> Result<bool> {
  let source_text = fs::read_to_string(file_path)?;
  let allocator = Allocator::default();
  let source_type = SourceType::from_path(Path::new(file_path)).unwrap_or_default();

  let oxc_parser::ParserReturn {
    program, errors, ..
  } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

  if !errors.is_empty() {
    return Ok(false);
  }

  let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
  let semantic = &semantic_ret.semantic;

  for node in semantic.nodes().iter() {
    let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
      continue;
    };

    let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
      continue;
    };

    if element_name == target_component || element_name.ends_with(&format!(".{}", target_component))
    {
      debug(&format!("✅ Found {} in {}", target_component, file_path));
      return Ok(true);
    }
  }

  Ok(false)
}

fn get_specifier_name<'a>(
  specifier: &'a oxc_ast::ast::ImportDeclarationSpecifier,
) -> Option<&'a str> {
  match specifier {
    oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => Some(&spec.local.name),
    oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
      Some(&spec.local.name)
    }
    oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
      Some(&spec.local.name)
    }
  }
}
