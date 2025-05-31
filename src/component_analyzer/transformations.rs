use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use std::path::Path;

use crate::component_analyzer::jsx_analysis::extract_jsx_element_name;
use crate::component_analyzer::utils::{
    debug, extract_component_name_from_argument, extract_function_name, ComponentPresenceCall,
};
use crate::{Result, Transformation};

pub fn generate_transformations_for_current_file(
    semantic: &Semantic,
    component_calls: &Vec<ComponentPresenceCall>,
) -> Result<Vec<Transformation>> {
    let mut transformations = Vec::new();

    for call in component_calls {
        if call.is_present_in_subtree {
            let current_file_transformations = generate_jsx_prop_transformations(semantic, &call)?;
            transformations.extend(current_file_transformations);
        }
    }

    Ok(transformations)
}

pub fn generate_jsx_prop_transformations(
    semantic: &Semantic,
    call: &ComponentPresenceCall,
) -> Result<Vec<Transformation>> {
    let mut transformations = Vec::new();

    let source_file_name = Path::new(&call.source_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    debug(&format!(
        "🔍 Looking for JSX component corresponding to source file: {} ({})",
        call.source_file, source_file_name
    ));

    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
                let should_add_prop = if element_name.contains('.') {
                    let parts: Vec<&str> = element_name.split('.').collect();
                    if parts.len() == 2 {
                        let component_name = parts[1].to_lowercase();
                        component_name == source_file_name
                    } else {
                        false
                    }
                } else {
                    element_name.to_lowercase() == source_file_name
                };

                if should_add_prop {
                    debug(&format!(
                        "🔧 Adding prop to JSX component: {}",
                        element_name
                    ));
                    let prop_name = format!("__qwik_analyzer_has_{}", call.component_name);
                    let prop_value = call.is_present_in_subtree;
                    let new_prop = format!(" {}={{{}}}", prop_name, prop_value);

                    let insert_pos = jsx_opening.span.end - 1;

                    transformations.push(Transformation {
                        start: insert_pos,
                        end: insert_pos,
                        replacement: new_prop,
                    });
                }
            }
        }
    }

    Ok(transformations)
}

pub fn generate_transformations_for_current_file_components(
    semantic: &Semantic,
    file_path: &Path,
) -> Result<Vec<Transformation>> {
    let mut transformations = Vec::new();

    let mut has_is_component_present_calls = false;
    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "isComponentPresent" {
                    has_is_component_present_calls = true;
                    break;
                }
            }
        }
    }

    if has_is_component_present_calls {
        let source_text = std::fs::read_to_string(file_path)?;

        let mut component_has_props = false;
        let mut component_span: Option<(u32, u32)> = None;

        for node in semantic.nodes().iter() {
            if let AstKind::CallExpression(call_expr) = node.kind() {
                if let Some(function_name) = extract_function_name(call_expr) {
                    if function_name == "component$" {
                        if let Some(first_arg) = call_expr.arguments.first() {
                            if let oxc_ast::ast::Argument::ArrowFunctionExpression(arrow_fn) =
                                first_arg
                            {
                                component_span = Some((arrow_fn.span.start, arrow_fn.span.end));
                                component_has_props = !arrow_fn.params.items.is_empty();
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some((component_start, _)) = component_span {
            if !component_has_props {
                let component_text = &source_text[component_start as usize..];
                if let Some(paren_pos) = component_text.find('(') {
                    let insert_pos = component_start + paren_pos as u32 + 1;
                    transformations.push(Transformation {
                        start: insert_pos,
                        end: insert_pos,
                        replacement: "props: any".to_string(),
                    });
                    debug(&format!(
                        "🔧 Adding props parameter at position {} in {}",
                        insert_pos,
                        file_path.display()
                    ));
                }
            }
        }

        for node in semantic.nodes().iter() {
            if let AstKind::CallExpression(call_expr) = node.kind() {
                if let Some(function_name) = extract_function_name(call_expr) {
                    if function_name == "isComponentPresent" {
                        if let Some(first_arg) = call_expr.arguments.first() {
                            if let Some(component_name) =
                                extract_component_name_from_argument(first_arg)
                            {
                                let prop_name = format!("__qwik_analyzer_has_{}", component_name);
                                let new_call = format!(
                                    "isComponentPresent({}, props.{})",
                                    component_name, prop_name
                                );

                                transformations.push(Transformation {
                                    start: call_expr.span.start,
                                    end: call_expr.span.end,
                                    replacement: new_call,
                                });
                                debug(&format!(
                                    "🔧 Transforming isComponentPresent({}) call in {}",
                                    component_name,
                                    file_path.display()
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(transformations)
}
