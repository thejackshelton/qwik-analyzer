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

pub fn component_exists_in_jsx_with_path(semantic: &Semantic, component_name: &str, current_file: &std::path::Path) -> bool {
    use oxc_ast::AstKind;
    
    for node in semantic.nodes().iter() {
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
            if element_name == component_name {
                return true;
            }
            
            if !component_name.contains('.') && element_name.contains('.') {
                let parts: Vec<&str> = element_name.split('.').collect();
                if parts.len() == 2 {
                    let namespace = parts[0];
                    let component = parts[1];
                    
                    if component == component_name && can_resolve_namespace_locally(semantic, namespace, current_file) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn can_resolve_namespace_locally(semantic: &Semantic, namespace: &str, current_file: &std::path::Path) -> bool {
    use crate::component_analyzer::import_resolver::{find_import_source_for_component, resolve_import_path};
    
    if let Some(import_source) = find_import_source_for_component(semantic, namespace) {
        resolve_import_path(&import_source, current_file).is_ok()
    } else {
        false
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
