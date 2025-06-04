use oxc_semantic::Semantic;
use std::path::Path;
use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_parser;
use oxc_span::SourceType;

use crate::component_analyzer::import_resolver::{
  file_has_component, find_calls_in_file, resolve_component_from_index,
  find_import_source_for_component, resolve_import_path,
};
use crate::component_analyzer::jsx_analysis::extract_imported_jsx_components;
use crate::component_analyzer::utils::{
  component_exists_in_jsx_with_path, debug, ComponentPresenceCall,
};
use crate::Result;

fn is_external_import(import_source: &str, current_file: &Path) -> bool {
  // Use oxc_resolver to get the actual resolved path
  match resolve_import_path(import_source, current_file) {
    Ok(resolved_path) => {
      // Check if the resolved path contains node_modules
      resolved_path.contains("node_modules")
    }
    Err(_) => {
      // If resolution fails, assume it's external (safer default)
      true
    }
  }
}

pub fn find_presence_calls(
  semantic: &Semantic,
  jsx_component: &str,
  current_file: &Path,
) -> Result<Vec<ComponentPresenceCall>> {
  debug(&format!(
    "🔍 Analyzing imported component: {}",
    jsx_component
  ));

  if jsx_component.contains('.') {
    let parts: Vec<&str> = jsx_component.split('.').collect();
    if parts.len() != 2 {
      return Ok(Vec::new());
    }

    let module_name = parts[0];
    let component_name = parts[1];

    let Some(import_source) = find_import_source_for_component(semantic, module_name) else {
      return Ok(Vec::new());
    };

    let Ok(module_dir) = resolve_import_path(&import_source, current_file) else {
      return Ok(Vec::new());
    };

    debug(&format!(
      "📂 Resolved module {} to: {}",
      module_name, module_dir
    ));

    debug(&format!("📂 About to scan module {} for component {}", module_dir, component_name));
    
    // Try to find index file in the module directory
    let module_path = Path::new(&module_dir);
    let index_file = if module_path.is_file() {
      module_dir.clone()
    } else {
      // Look for index.ts or index.tsx in the directory
      let index_ts = module_path.join("index.ts");
      let index_tsx = module_path.join("index.tsx");
      if index_ts.exists() {
        index_ts.to_string_lossy().to_string()
      } else if index_tsx.exists() {
        index_tsx.to_string_lossy().to_string()
      } else {
        module_dir.clone() // Fallback to original behavior
      }
    };
    
    debug(&format!("🔍 Trying resolve_component_from_index for {} in index file {}", component_name, index_file));
    if let Ok(component_file) = resolve_component_from_index(&index_file, component_name) {
      debug(&format!("📂 Found component file: {}", component_file));
      return find_calls_in_file(&component_file);
    } else {
      debug(&format!("📂 No direct component file found, scanning entire module: {}", module_dir));
      return find_calls_in_module(&module_dir);
    }
  }

  let Some(import_source) = find_import_source_for_component(semantic, jsx_component) else {
    return Ok(Vec::new());
  };

  let Ok(resolved_path) = resolve_import_path(&import_source, current_file) else {
    return Ok(Vec::new());
  };

  debug(&format!(
    "📂 Resolved component {} to: {}",
    jsx_component, resolved_path
  ));
  find_calls_in_file(&resolved_path)
}

pub fn has_component(
  semantic: &Semantic,
  component_name: &str,
  current_file: &Path,
) -> Result<bool> {
  debug(&format!(
    "🔍 Checking if {} is present in JSX subtree",
    component_name
  ));

  if component_exists_in_jsx_with_path(semantic, component_name, current_file) {
    debug(&format!(
      "✅ Found direct usage of {} in JSX",
      component_name
    ));
    return Ok(true);
  }

  debug(&format!(
    "🔍 Checking imported components for {} usage...",
    component_name
  ));

  let jsx_components = extract_imported_jsx_components(semantic);

  for jsx_component in jsx_components {
    debug(&format!("🔍 Processing JSX component: {} looking for {}", jsx_component, component_name));
    
    // Check if jsx_component resolves to the component_name we're looking for
    // e.g., MyTest.Child resolves to MyTestChild
    if jsx_component.contains('.') && !component_name.contains('.') {
      if let Ok(component_file) = resolve_component_from_jsx_to_file(&jsx_component, current_file) {
        // Check if the component file defines the component we're looking for
        if component_file_defines_component(&component_file, component_name)? {
          debug(&format!(
            "✅ Found {} via JSX component {} which resolves to the same file",
            component_name, jsx_component
          ));
          return Ok(true);
        }
      }
    }
    
    // For member expressions, only match if they're exactly the same
    if jsx_component.contains('.') && component_name.contains('.') {
      if jsx_component == component_name {
        // Check if this is from an external package before considering it a match
        let module_name = jsx_component.split('.').next().unwrap_or("");
        if let Some(import_source) = find_import_source_for_component(semantic, module_name) {
          if is_external_import(&import_source, current_file) {
            debug(&format!("❌ Skipping external component: {} from {}", jsx_component, import_source));
            continue;
          }
        }
        debug(&format!("✅ Found exact match: {} == {}", jsx_component, component_name));
        return Ok(true);
      }
      continue; // Skip if both have dots but don't match exactly
    }

    let Some(import_source) = find_import_source_for_component(semantic, &jsx_component) else {
      continue;
    };

    // Skip external components early
    if is_external_import(&import_source, current_file) {
      debug(&format!("❌ Skipping external import: {} from {}", jsx_component, import_source));
      continue;
    }

    let Ok(resolved_path) = resolve_import_path(&import_source, current_file) else {
      continue;
    };

    debug(&format!(
      "📂 Analyzing {} (from {}) for {}",
      jsx_component, resolved_path, component_name
    ));

    let presence_calls = find_calls_in_file(&resolved_path)?;
    for call in &presence_calls {
      if call.component_name == component_name {
        debug(&format!(
          "✅ Found {} via imported component {}",
          component_name, jsx_component
        ));
        return Ok(true);
      }
    }

    if presence_calls.is_empty() {
      if !component_name.contains('.') && file_has_component(&resolved_path, component_name)? {
        debug(&format!(
          "✅ Found {} in imported component {}",
          component_name, jsx_component
        ));
        return Ok(true);
      }
    }
  }

  debug(&format!(
    "❌ Component {} not found in JSX subtree",
    component_name
  ));
  Ok(false)
}

fn find_calls_in_module(module_path: &str) -> Result<Vec<ComponentPresenceCall>> {
  use std::fs;
  use oxc_span::VALID_EXTENSIONS;
  
  let mut all_calls = Vec::new();
  
  let module_dir = if module_path.ends_with(".ts") || module_path.ends_with(".tsx") || 
                     module_path.ends_with(".js") || module_path.ends_with(".jsx") {
    Path::new(module_path).parent().ok_or("Could not get module directory")?
  } else {
    Path::new(module_path)
  };
  
  debug(&format!("🔍 Scanning directory: {}", module_dir.display()));
  
  if let Ok(entries) = fs::read_dir(module_dir) {
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_file() {
        if let Some(extension) = path.extension() {
          if VALID_EXTENSIONS.iter().any(|&ext| ext == extension.to_str().unwrap_or("")) {
            let file_path = path.to_string_lossy().to_string();
            debug(&format!("📄 Checking file: {}", file_path));
            if let Ok(calls) = find_calls_in_file(&file_path) {
              all_calls.extend(calls);
            }
          }
        }
      }
    }
  }
  
  Ok(all_calls)
}

fn resolve_component_from_jsx_to_file(jsx_component: &str, current_file: &Path) -> Result<String> {
  // Handle JSX components like MyTest.Child -> resolve to MyTestChild file
  if !jsx_component.contains('.') {
    return Err("Not a namespaced component".into());
  }
  
  let parts: Vec<&str> = jsx_component.split('.').collect();
  if parts.len() != 2 {
    return Err("Invalid component format".into());
  }
  
  let module_name = parts[0];
  let component_name = parts[1];
  
  debug(&format!("🔍 Resolving JSX component {} to file", jsx_component));
  
  // Find the import source for the module
  let allocator = Allocator::default();
  let source_text = std::fs::read_to_string(current_file)?;
  let source_type = SourceType::from_path(current_file).unwrap_or_default();
  
  let oxc_parser::ParserReturn { program, errors, .. } = 
    oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();
  
  if !errors.is_empty() {
    return Err("Failed to parse current file".into());
  }
  
  let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
  let semantic = &semantic_ret.semantic;
  
  let Some(import_source) = find_import_source_for_component(semantic, module_name) else {
    return Err(format!("Could not find import for module {}", module_name).into());
  };
  
  let module_path = resolve_import_path(&import_source, current_file)?;
  
  // Try to resolve the component through the index file
  let module_dir = std::path::Path::new(&module_path);
  let index_file = if module_dir.is_file() {
    module_path.clone()
  } else {
    let index_ts = module_dir.join("index.ts");
    let index_tsx = module_dir.join("index.tsx");
    if index_ts.exists() {
      index_ts.to_string_lossy().to_string()
    } else if index_tsx.exists() {
      index_tsx.to_string_lossy().to_string()
    } else {
      return Err("Could not find index file".into());
    }
  };
  
  resolve_component_from_index(&index_file, component_name)
}

fn component_file_defines_component(component_file: &str, component_name: &str) -> Result<bool> {
  debug(&format!("🔍 Checking if {} defines component {}", component_file, component_name));
  
  let source_text = std::fs::read_to_string(component_file)?;
  let allocator = Allocator::default();
  let source_type = SourceType::from_path(std::path::Path::new(component_file)).unwrap_or_default();
  
  let oxc_parser::ParserReturn { program, errors, .. } = 
    oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();
  
  if !errors.is_empty() {
    return Ok(false);
  }
  
  let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
  let semantic = &semantic_ret.semantic;
  
  // Look for export declarations that match the component name
  for node in semantic.nodes().iter() {
    match node.kind() {
      AstKind::ExportNamedDeclaration(export_decl) => {
        // Handle export { MyTestChild }
        for specifier in &export_decl.specifiers {
          let exported_name = &specifier.exported.name();
          if exported_name == component_name {
            debug(&format!("✅ Found export specifier for {}", component_name));
            return Ok(true);
          }
        }
      }
      AstKind::VariableDeclarator(declarator) => {
        // Handle export const MyTestChild = component$(() => ...)
        if let Some(binding) = declarator.id.get_binding_identifier() {
          if binding.name == component_name {
            debug(&format!("✅ Found variable declarator for {}", component_name));
            return Ok(true);
          }
        }
      }
      _ => {}
    }
  }
  
  debug(&format!("❌ Component {} not found in {}", component_name, component_file));
  Ok(false)
}
