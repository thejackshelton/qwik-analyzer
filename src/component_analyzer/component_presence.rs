use oxc_semantic::Semantic;
use std::path::Path;

use crate::component_analyzer::import_resolver::{
  file_has_component, find_calls_in_file, resolve_component_from_index,
  find_import_source_for_component, resolve_import_path,
};
use crate::component_analyzer::jsx_analysis::extract_imported_jsx_components;
use crate::component_analyzer::utils::{
  component_exists_in_jsx_with_path, debug, ComponentPresenceCall,
};
use crate::Result;

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
    
    debug(&format!("🔍 Trying resolve_component_from_index for {} in {}", component_name, module_dir));
    if let Ok(component_file) = resolve_component_from_index(&module_dir, component_name) {
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
    if jsx_component.ends_with(&format!(".{}", component_name)) {
      continue;
    }
    
    if jsx_component.contains('.') && component_name.contains(&jsx_component.split('.').last().unwrap_or("")) {
      debug(&format!("✅ Found potential match: {} contains {}", jsx_component, component_name));
      return Ok(true);
    }

    let Some(import_source) = find_import_source_for_component(semantic, &jsx_component) else {
      continue;
    };

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
