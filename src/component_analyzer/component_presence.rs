use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use std::path::Path;

use crate::component_analyzer::import_resolver::{
    file_has_component, find_calls_in_file, find_component_file_in_module,
    find_import_source_for_component, resolve_import_path,
};
use crate::component_analyzer::jsx_analysis::{
    extract_imported_jsx_components, extract_jsx_element_name,
};
use crate::component_analyzer::utils::{debug, ComponentPresenceCall};
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

        if !import_source.starts_with('.') {
            return Ok(Vec::new());
        }

        let Ok(module_dir) = resolve_import_path(&import_source, current_file) else {
            return Ok(Vec::new());
        };

        debug(&format!(
            "📂 Resolved module {} to: {}",
            module_name, module_dir
        ));

        let component_file = find_component_file_in_module(&module_dir, component_name)?;
        debug(&format!("📂 Found component file: {}", component_file));

        return find_calls_in_file(&component_file);
    }

    let Some(import_source) = find_import_source_for_component(semantic, jsx_component) else {
        return Ok(Vec::new());
    };

    if !import_source.starts_with('.') {
        return Ok(Vec::new());
    }

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

    // Check direct usage first
    for node in semantic.nodes().iter() {
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
            continue;
        };

        if element_name == component_name {
            debug(&format!(
                "✅ Found direct usage of {} in JSX",
                component_name
            ));
            return Ok(true);
        }

        if element_name.ends_with(&format!(".{}", component_name)) {
            debug(&format!(
                "✅ Found member expression usage of {} in JSX: {}",
                component_name, element_name
            ));
            return Ok(true);
        }
    }

    debug(&format!(
        "🔍 Checking imported components for {} usage...",
        component_name
    ));

    // Check imported components
    let jsx_components = extract_imported_jsx_components(semantic);

    for jsx_component in jsx_components {
        if jsx_component.ends_with(&format!(".{}", component_name)) {
            continue;
        }

        let Some(import_source) = find_import_source_for_component(semantic, &jsx_component) else {
            continue;
        };

        if !import_source.starts_with('.') {
            continue;
        }

        let Ok(resolved_path) = resolve_import_path(&import_source, current_file) else {
            continue;
        };

        debug(&format!(
            "📂 Analyzing {} (from {}) for {}",
            jsx_component, resolved_path, component_name
        ));

        if file_has_component(&resolved_path, component_name)? {
            debug(&format!(
                "✅ Found {} in imported component {}",
                component_name, jsx_component
            ));
            return Ok(true);
        }
    }

    debug(&format!(
        "❌ Component {} not found in JSX subtree",
        component_name
    ));
    Ok(false)
}
