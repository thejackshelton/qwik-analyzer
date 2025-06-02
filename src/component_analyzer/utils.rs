use oxc_ast::ast::CallExpression;
use oxc_semantic::Semantic;

#[derive(Debug, Clone)]
pub struct ComponentPresenceCall {
    pub component_name: String,
    pub is_present_in_subtree: bool,
    pub source_file: String,
}

pub fn debug(msg: &str) {
    println!("{}", msg);
}

pub fn extract_function_name(call_expr: &CallExpression) -> Option<String> {
    match &call_expr.callee {
        oxc_ast::ast::Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

pub fn extract_component_name_from_argument(argument: &oxc_ast::ast::Argument) -> Option<String> {
    use oxc_ast::ast::*;
    
    match argument {
        Argument::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

pub fn extract_member_component_name(source_text: &str, span_start: usize, span_end: usize) -> Option<String> {
    let arg_text = &source_text[span_start..span_end];
    if let Some(dot_pos) = arg_text.rfind('.') {
        Some(arg_text[dot_pos + 1..].to_string())
    } else {
        None
    }
}

pub fn component_exists_in_jsx_with_path(semantic: &Semantic, component_name: &str, current_file: &std::path::Path) -> bool {
    use oxc_ast::AstKind;
    
    // Get the target component's symbol ID if it exists
    let target_symbol_id = find_component_symbol_id(semantic, component_name);
    
    for node in semantic.nodes().iter() {
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        // Get the symbol ID for this JSX element
        if let Some(jsx_symbol_id) = get_jsx_element_symbol_id(jsx_opening, semantic) {
            // Compare symbol IDs for exact semantic matching
            if let Some(target_id) = target_symbol_id {
                if jsx_symbol_id == target_id {
                    return true;
                }
            }
        }
        
        // Fallback to name-based matching for direct usage
        if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
            if element_name == component_name {
                return true;
            }
            
            // For simple target components, check if accessible through local namespaces
            if !component_name.contains('.') && element_name.contains('.') {
                debug(&format!("🔍 Checking namespace access: {} in {}", component_name, element_name));
                let parts: Vec<&str> = element_name.split('.').collect();
                if parts.len() == 2 {
                    let namespace = parts[0];
                    let component = parts[1];
                    
                    debug(&format!("🔍 Namespace: {}, Component: {}", namespace, component));
                    if component == component_name {
                        debug(&format!("🔍 Component matches! Checking if {} is local namespace", namespace));
                        // Check if this namespace actually resolves to a local file
                        if can_resolve_namespace_locally(semantic, namespace, current_file) {
                            debug(&format!("✅ Found {} via local namespace {}", component_name, namespace));
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if a namespace can be resolved to a local file by checking if resolution succeeds
fn can_resolve_namespace_locally(semantic: &Semantic, namespace: &str, current_file: &std::path::Path) -> bool {
    use crate::component_analyzer::import_resolver::{find_import_source_for_component, resolve_import_path};
    use crate::component_analyzer::utils::debug;
    use std::path::Path;
    
    if let Some(import_source) = find_import_source_for_component(semantic, namespace) {
        debug(&format!("🔍 Found import for {}: {}", namespace, import_source));
        // If resolve_import_path succeeds, it's a local file. If it fails, it's external.
        let can_resolve = resolve_import_path(&import_source, current_file).is_ok();
        debug(&format!("🔍 Can resolve {} locally: {}", namespace, can_resolve));
        can_resolve
    } else {
        debug(&format!("🔍 No import found for namespace: {}", namespace));
        false
    }
}


fn find_component_symbol_id(semantic: &Semantic, component_name: &str) -> Option<oxc_semantic::SymbolId> {
    // Look for the symbol in the root scope first
    semantic.scoping().get_root_binding(component_name)
}

fn get_jsx_element_symbol_id(jsx_opening: &oxc_ast::ast::JSXOpeningElement, semantic: &Semantic) -> Option<oxc_semantic::SymbolId> {
    use oxc_ast::ast::JSXElementName;
    
    match &jsx_opening.name {
        JSXElementName::Identifier(ident) => {
            // Look up the identifier in the root scope
            semantic.scoping().get_root_binding(&ident.name)
        },
        JSXElementName::MemberExpression(member_expr) => {
            // For member expressions like Checkbox.Description, get the object's symbol
            if let Some(ident) = member_expr.object.get_identifier() {
                semantic.scoping().get_root_binding(&ident.name)
            } else {
                None
            }
        },
        _ => None,
    }
}

fn extract_jsx_element_name(jsx_opening: &oxc_ast::ast::JSXOpeningElement) -> Option<String> {
    use oxc_ast::ast::JSXElementName;
    
    match &jsx_opening.name {
        JSXElementName::Identifier(ident) => Some(ident.name.to_string()),
        JSXElementName::MemberExpression(member_expr) => {
            if let Some(obj) = member_expr.object.get_identifier() {
                Some(format!("{}.{}", obj.name, member_expr.property.name))
            } else {
                None
            }
        },
        _ => None,
    }
}
