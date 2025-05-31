use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use std::path::Path;

use crate::component_analyzer::import_resolver::{
    analyze_file_for_component_usage, analyze_file_for_is_component_present_calls,
    find_component_file_in_module, find_import_source_for_component, resolve_import_with_oxc,
};
use crate::component_analyzer::jsx_analysis::{
    extract_imported_jsx_components, extract_jsx_element_name,
};
use crate::component_analyzer::utils::{debug, ComponentPresenceCall};
use crate::Result;

pub fn find_is_component_present_calls_in_imported_component(
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
        if parts.len() == 2 {
            let module_name = parts[0];
            let component_name = parts[1];

            let import_source = find_import_source_for_component(semantic, module_name);
            if let Some(source) = import_source {
                if source.starts_with('.') {
                    if let Ok(module_dir) = resolve_import_with_oxc(&source, current_file) {
                        debug(&format!(
                            "📂 Resolved module {} to: {}",
                            module_name, module_dir
                        ));

                        let component_file =
                            find_component_file_in_module(&module_dir, component_name)?;
                        debug(&format!("📂 Found component file: {}", component_file));

                        return analyze_file_for_is_component_present_calls(&component_file);
                    }
                }
            }
        }
    } else {
        let import_source = find_import_source_for_component(semantic, jsx_component);
        if let Some(source) = import_source {
            if source.starts_with('.') {
                if let Ok(resolved_path) = resolve_import_with_oxc(&source, current_file) {
                    debug(&format!(
                        "📂 Resolved component {} to: {}",
                        jsx_component, resolved_path
                    ));
                    return analyze_file_for_is_component_present_calls(&resolved_path);
                }
            }
        }
    }

    Ok(Vec::new())
}

pub fn is_component_present_in_jsx_subtree(
    semantic: &Semantic,
    component_name: &str,
    current_file: &Path,
) -> Result<bool> {
    debug(&format!(
        "🔍 Checking if {} is present in JSX subtree",
        component_name
    ));

    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
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
        }
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

        if let Ok(contains_target) = analyze_imported_component_for_target(
            semantic,
            &jsx_component,
            component_name,
            current_file,
        ) {
            if contains_target {
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

fn analyze_imported_component_for_target(
    semantic: &Semantic,
    jsx_component: &str,
    target_component: &str,
    current_file: &Path,
) -> Result<bool> {
    let import_source = find_import_source_for_component(semantic, jsx_component);

    if let Some(source) = import_source {
        if source.starts_with('.') {
            if let Ok(resolved_path) = resolve_import_with_oxc(&source, current_file) {
                debug(&format!(
                    "📂 Analyzing {} (from {}) for {}",
                    jsx_component, resolved_path, target_component
                ));

                return analyze_file_for_component_usage(&resolved_path, target_component);
            }
        }
    }

    Ok(false)
}
