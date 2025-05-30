use oxc_allocator::Allocator;
use oxc_span::Span;

/// Basic utility functions for AST manipulation
/// This is a simplified version to get the project compiling

/// Creates a boolean literal value
pub fn create_boolean_value(value: bool) -> bool {
    value
}

/// Creates an identifier name string
pub fn create_identifier_name(name: &str) -> String {
    name.to_string()
}

/// Utility function to extract simple JSX element names
pub fn extract_simple_jsx_name(name_str: &str) -> Option<String> {
    if name_str.is_empty() {
        None
    } else {
        Some(name_str.to_string())
    }
}

/// Utility function for simple component name matching
pub fn matches_component_name(actual: &str, expected: &str) -> bool {
    actual == expected
}

/// Helper to check if a string represents a member expression
pub fn is_member_expression(name: &str) -> bool {
    name.contains('.')
}

/// Helper to split member expression into parts
pub fn split_member_expression(name: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}
