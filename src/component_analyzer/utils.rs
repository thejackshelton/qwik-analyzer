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
    match argument {
        oxc_ast::ast::Argument::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}
