use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use std::path::Path;

use crate::component_analyzer::jsx_analysis::extract_jsx_element_name;
use crate::component_analyzer::import_resolver::{find_import_source_for_component, resolve_import_path, resolve_component_from_index};
use crate::component_analyzer::utils::{
  debug, extract_component_name_from_argument, extract_function_name, ComponentPresenceCall,
};
use crate::{Result, Transformation};

pub fn transform_file(
  semantic: &Semantic,
  component_calls: &Vec<ComponentPresenceCall>,
  current_file: &Path,
) -> Result<Vec<Transformation>> {
  let mut transformations = Vec::new();

  for call in component_calls {
    // Generate JSX props for all calls, not just the ones that are present
    let current_file_transformations = generate_jsx_prop_transformations(semantic, &call, current_file)?;
    transformations.extend(current_file_transformations);
  }

  Ok(transformations)
}

fn generate_jsx_prop_transformations(
  semantic: &Semantic,
  call: &ComponentPresenceCall,
  current_file: &Path,
) -> Result<Vec<Transformation>> {
  let mut transformations = Vec::new();

  debug(&format!(
    "🔍 Looking for JSX component corresponding to source file: {}",
    call.source_file
  ));

  for node in semantic.nodes().iter() {
    let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
      continue;
    };

    let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
      continue;
    };

    debug(&format!(
      "🔍 Found JSX element: {} - checking if it resolves to component with isComponentPresent call in {}",
      element_name, call.source_file
    ));

    if !jsx_element_resolves_to_source_file(semantic, &element_name, &call.source_file, current_file)? {
      debug(&format!(
        "❌ JSX element {} does NOT resolve to source file {}",
        element_name, call.source_file
      ));
      continue;
    }

    debug(&format!(
      "✅ JSX element {} SHOULD receive props for source file {}",
      element_name, call.source_file
    ));

    debug(&format!(
      "🔧 Adding prop to JSX component: {}",
      element_name
    ));

    let prop_name = format!(
      "__qwik_analyzer_has_{}",
      call.component_name.replace(".", "_")
    );
    let prop_value = call.is_present_in_subtree;
    let new_prop = format!(" {}={{{}}}", prop_name, prop_value);
    let insert_pos = jsx_opening.span.end - 1;

    transformations.push(Transformation {
      start: insert_pos,
      end: insert_pos,
      replacement: new_prop,
    });
  }

  Ok(transformations)
}

fn jsx_element_resolves_to_source_file(
  semantic: &Semantic,
  element_name: &str,
  target_source_file: &str,
  current_file: &Path,
) -> Result<bool> {
  // For namespaced components like MyTest.Root, resolve the actual component file
  if element_name.contains('.') {
    let parts: Vec<&str> = element_name.split('.').collect();
    if parts.len() != 2 {
      return Ok(false);
    }

    let module_name = parts[0];
    let component_name = parts[1];

    // Find the import source for the module
    let Some(import_source) = find_import_source_for_component(semantic, module_name) else {
      return Ok(false);
    };

    // Use the passed current_file for import resolution

    // Resolve the import path to get the module directory
    let module_path = match resolve_import_path(&import_source, current_file) {
      Ok(path) => path,
      Err(_) => {
        debug(&format!(
          "❌ Could not resolve import path for {} - skipping external package",
          import_source
        ));
        return Ok(false);
      }
    };

    // Find the index file in the module directory
    let module_path_obj = Path::new(&module_path);
    let index_file = if module_path_obj.is_file() {
      module_path.clone()
    } else {
      // Look for index.ts or index.tsx in the directory
      let index_ts = module_path_obj.join("index.ts");
      let index_tsx = module_path_obj.join("index.tsx");
      if index_ts.exists() {
        index_ts.to_string_lossy().to_string()
      } else if index_tsx.exists() {
        index_tsx.to_string_lossy().to_string()
      } else {
        module_path.clone() // Fallback to original behavior
      }
    };

    // Use oxc semantic to analyze the index file and find the export for this component
    if let Ok(component_file) = resolve_component_from_index(&index_file, component_name) {
      debug(&format!(
        "🔍 Resolved JSX component {} to file: {}",
        element_name, component_file
      ));
      debug(&format!(
        "🔍 Comparing with target source file: {}",
        target_source_file
      ));
      
      // Compare the resolved component file with the target source file
      let component_path = Path::new(&component_file);
      let target_path = Path::new(target_source_file);
      
      // Use canonical paths if possible, otherwise compare as strings
      match (component_path.canonicalize(), target_path.canonicalize()) {
        (Ok(comp_canonical), Ok(target_canonical)) => {
          let matches = comp_canonical == target_canonical;
          debug(&format!(
            "🔍 Canonical path comparison: {} == {} -> {}",
            comp_canonical.display(), target_canonical.display(), matches
          ));
          return Ok(matches);
        }
        _ => {
          // Fallback to string comparison
          let matches = component_file == target_source_file;
          debug(&format!(
            "🔍 String comparison fallback: {} == {} -> {}",
            component_file, target_source_file, matches
          ));
          return Ok(matches);
        }
      }
    } else {
      debug(&format!(
        "🔍 Could not find component file for {} in module {}",
        component_name, module_path
      ));
    }
  } else {
    // For simple components, try to resolve directly
    let Some(import_source) = find_import_source_for_component(semantic, element_name) else {
      return Ok(false);
    };

    // Use the passed current_file for import resolution

    if let Ok(resolved_path) = resolve_import_path(&import_source, current_file) {
      debug(&format!(
        "🔍 Resolved JSX component {} to file: {}",
        element_name, resolved_path
      ));
      
      let resolved_path_obj = Path::new(&resolved_path);
      let target_path = Path::new(target_source_file);
      
      match (resolved_path_obj.canonicalize(), target_path.canonicalize()) {
        (Ok(resolved_canonical), Ok(target_canonical)) => {
          let matches = resolved_canonical == target_canonical;
          debug(&format!(
            "🔍 Path comparison: {} == {} -> {}",
            resolved_canonical.display(), target_canonical.display(), matches
          ));
          return Ok(matches);
        }
        _ => {
          let matches = resolved_path == target_source_file;
          debug(&format!(
            "🔍 String comparison: {} == {} -> {}",
            resolved_path, target_source_file, matches
          ));
          return Ok(matches);
        }
      }
    }
  }

  Ok(false)
}

pub fn transform_components(semantic: &Semantic, file_path: &Path) -> Result<Vec<Transformation>> {
  if !has_component_present_calls(semantic) {
    return Ok(Vec::new());
  }

  let source_text = std::fs::read_to_string(file_path)?;
  let mut transformations = Vec::new();

  if let Some(transformation) =
    create_props_parameter_transformation(semantic, &source_text, file_path)?
  {
    transformations.push(transformation);
  }

  transformations.extend(create_component_present_call_transformations(
    semantic, file_path,
  )?);

  Ok(transformations)
}

fn has_component_present_calls(semantic: &Semantic) -> bool {
  for node in semantic.nodes().iter() {
    let AstKind::CallExpression(call_expr) = node.kind() else {
      continue;
    };

    let Some(function_name) = extract_function_name(call_expr) else {
      continue;
    };

    if function_name == "isComponentPresent" {
      return true;
    }
  }

  false
}

fn create_props_parameter_transformation(
  semantic: &Semantic,
  source_text: &str,
  file_path: &Path,
) -> Result<Option<Transformation>> {
  let Some((component_start, component_has_props)) = find_component_info(semantic) else {
    return Ok(None);
  };

  if component_has_props {
    return Ok(None);
  }

  let component_text = &source_text[component_start as usize..];
  let Some(paren_pos) = component_text.find('(') else {
    return Ok(None);
  };

  let insert_pos = component_start + paren_pos as u32 + 1;
  debug(&format!(
    "🔧 Adding props parameter at position {} in {}",
    insert_pos,
    file_path.display()
  ));

  Ok(Some(Transformation {
    start: insert_pos,
    end: insert_pos,
    replacement: "props: any".to_string(),
  }))
}

fn find_component_info(semantic: &Semantic) -> Option<(u32, bool)> {
  use oxc_ast::ast::*;

  semantic
    .nodes()
    .iter()
    .filter_map(|node| {
      if let AstKind::CallExpression(call_expr) = node.kind() {
        if extract_function_name(call_expr)? == "component$" {
          if let Some(Argument::ArrowFunctionExpression(arrow_fn)) = call_expr.arguments.first() {
            return Some((arrow_fn.span.start, !arrow_fn.params.items.is_empty()));
          }
        }
      }
      None
    })
    .next()
}

fn create_component_present_call_transformations(
  semantic: &Semantic,
  file_path: &Path,
) -> Result<Vec<Transformation>> {
  let mut transformations = Vec::new();
  let source_text = std::fs::read_to_string(file_path)?;

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

    let arg_span = first_arg.span();
    let arg_text = &source_text[arg_span.start as usize..arg_span.end as usize];

    let component_name = if let Some(name) = extract_component_name_from_argument(first_arg) {
      name
    } else if arg_text.contains('.') {
      arg_text.to_string()
    } else {
      continue;
    };

    let prop_name = format!("__qwik_analyzer_has_{}", component_name.replace(".", "_"));
    let new_call = format!("isComponentPresent({}, props.{})", arg_text, prop_name);

    transformations.push(Transformation {
      start: call_expr.span.start,
      end: call_expr.span.end,
      replacement: new_call,
    });

    debug(&format!(
      "🔧 Transforming isComponentPresent({}) call in {}",
      component_name,
      file_path.display()
    ));
  }

  Ok(transformations)
}
