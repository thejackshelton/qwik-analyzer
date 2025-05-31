use oxc_ast::ast::JSXOpeningElement;
use oxc_ast::AstKind;
use oxc_semantic::Semantic;

use crate::component_analyzer::utils::debug;

pub fn extract_imported_jsx_components(semantic: &Semantic) -> Vec<String> {
    let mut components = Vec::new();

    for node in semantic.nodes().iter() {
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
            continue;
        };

        // Handle module.Component syntax (e.g., Icons.Home)
        if element_name.contains('.') {
            if let Some(full_component) = parse_member_component(&element_name) {
                if !components.contains(&full_component) {
                    debug(&format!("🏷️ Found imported component: {}", full_component));
                    components.push(full_component);
                }
            }
            continue;
        }

        // Handle direct component imports (must start with uppercase and not be HTML)
        if is_component_name(&element_name) && !components.contains(&element_name) {
            components.push(element_name.clone());
            debug(&format!("🏷️ Found imported component: {}", element_name));
        }
    }

    components
}

fn parse_member_component(element_name: &str) -> Option<String> {
    let parts: Vec<&str> = element_name.split('.').collect();
    if parts.len() == 2 {
        Some(format!("{}.{}", parts[0], parts[1]))
    } else {
        None
    }
}

fn is_component_name(name: &str) -> bool {
    name.chars()
        .next()
        .map_or(false, |c| c.is_ascii_uppercase())
        && !is_html_element(name)
}

pub fn extract_jsx_element_name(jsx_opening: &JSXOpeningElement) -> Option<String> {
    match &jsx_opening.name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => Some(identifier.name.to_string()),
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXElementName::MemberExpression(member_expr) => {
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        _ => None,
    }
}

fn extract_jsx_member_object_name(
    object: &oxc_ast::ast::JSXMemberExpressionObject,
) -> Option<String> {
    match object {
        oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(member_expr) => {
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        _ => None,
    }
}

fn is_html_element(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "div"
            | "span"
            | "p"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "a"
            | "img"
            | "input"
            | "button"
            | "form"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "thead"
            | "tbody"
            | "nav"
            | "header"
            | "footer"
            | "main"
            | "section"
            | "article"
            | "aside"
            | "details"
            | "summary"
            | "dialog"
            | "canvas"
            | "svg"
            | "video"
            | "audio"
    )
}
