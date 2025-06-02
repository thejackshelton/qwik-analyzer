use oxc_ast::ast::CallExpression;

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
        // Handle member expressions by directly checking the span and extracting from source
        _ => {
            // Try to get member expression
            if let Some(member_expr) = argument.as_member_expression() {
                debug(&format!("Found member expression argument"));
                
                // For member expressions like Checkbox.Description, we want the property name
                // Since we can't easily access the property, we'll use the span to extract from source
                // This requires the source text, which we don't have here
                // For now, we'll have to handle this differently
                None
            } else {
                debug(&format!("Non-identifier, non-member expression argument"));
                None
            }
        }
    }
}
