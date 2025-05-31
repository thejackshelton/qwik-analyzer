use oxc_ast::ast::JSXOpeningElement;
use oxc_ast::AstKind;
use oxc_semantic::Semantic;

use crate::component_analyzer::utils::debug;

pub fn extract_imported_jsx_components(semantic: &Semantic) -> Vec<String> {
    let mut components = Vec::new();

    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
                if element_name.contains('.') {
                    let parts: Vec<&str> = element_name.split('.').collect();
                    if parts.len() == 2 {
                        let component_module = parts[0];
                        let component_name = parts[1];
                        let full_component = format!("{}.{}", component_module, component_name);
                        if !components.contains(&full_component) {
                            debug(&format!("🏷️  Found imported component: {}", full_component));
                            components.push(full_component);
                        }
                    }
                } else if element_name
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_uppercase())
                    && !is_html_element(&element_name)
                {
                    if !components.contains(&element_name) {
                        components.push(element_name.clone());
                        debug(&format!("🏷️  Found imported component: {}", element_name));
                    }
                }
            }
        }
    }

    components
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

pub fn extract_jsx_member_object_name(
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

pub fn is_html_element(name: &str) -> bool {
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
