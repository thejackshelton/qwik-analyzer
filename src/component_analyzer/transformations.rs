use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use std::path::Path;

use crate::component_analyzer::jsx_analysis::extract_jsx_element_name;
use crate::component_analyzer::utils::{
    debug, extract_component_name_from_argument, extract_function_name, ComponentPresenceCall,
};
use crate::{Result, Transformation};

pub fn transform_file(
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

fn generate_jsx_prop_transformations(
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
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
            continue;
        };

        if !should_add_prop_to_component(&element_name, source_file_name) {
            continue;
        }

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

    Ok(transformations)
}

fn should_add_prop_to_component(element_name: &str, source_file_name: &str) -> bool {
    if element_name.contains('.') {
        let parts: Vec<&str> = element_name.split('.').collect();
        if parts.len() == 2 {
            let component_name = parts[1].to_lowercase();
            return component_name == source_file_name;
        }
        return false;
    }

    element_name.to_lowercase() == source_file_name
}

pub fn transform_components(semantic: &Semantic, file_path: &Path) -> Result<Vec<Transformation>> {
    if !has_component_present_calls(semantic) {
        return Ok(Vec::new());
    }

    let source_text = std::fs::read_to_string(file_path)?;
    let mut transformations = Vec::new();

    // Add props parameter if component doesn't have one
    if let Some(transformation) =
        create_props_parameter_transformation(semantic, &source_text, file_path)?
    {
        transformations.push(transformation);
    }

    // Transform isComponentPresent calls
    let call_transformations = create_component_present_call_transformations(semantic, file_path)?;
    transformations.extend(call_transformations);

    Ok(transformations)
}

fn has_component_present_calls(semantic: &Semantic) -> bool {
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call_expr) = node.kind() else {
            continue;
        };

        let Some(function_name) = extract_function_name(call_expr) else {
            continue;
        };

        if function_name == "isComponentPresent" {
            return true;
        }
    }

    false
}

fn create_props_parameter_transformation(
    semantic: &Semantic,
    source_text: &str,
    file_path: &Path,
) -> Result<Option<Transformation>> {
    let Some((component_start, component_has_props)) = find_component_info(semantic) else {
        return Ok(None);
    };

    if component_has_props {
        return Ok(None);
    }

    let component_text = &source_text[component_start as usize..];
    let Some(paren_pos) = component_text.find('(') else {
        return Ok(None);
    };

    let insert_pos = component_start + paren_pos as u32 + 1;
    debug(&format!(
        "🔧 Adding props parameter at position {} in {}",
        insert_pos,
        file_path.display()
    ));

    Ok(Some(Transformation {
        start: insert_pos,
        end: insert_pos,
        replacement: "props: any".to_string(),
    }))
}

fn find_component_info(semantic: &Semantic) -> Option<(u32, bool)> {
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call_expr) = node.kind() else {
            continue;
        };

        let Some(function_name) = extract_function_name(call_expr) else {
            continue;
        };

        if function_name != "component$" {
            continue;
        }

        let Some(first_arg) = call_expr.arguments.first() else {
            continue;
        };

        let oxc_ast::ast::Argument::ArrowFunctionExpression(arrow_fn) = first_arg else {
            continue;
        };

        let component_start = arrow_fn.span.start;
        let component_has_props = !arrow_fn.params.items.is_empty();
        return Some((component_start, component_has_props));
    }

    None
}

fn create_component_present_call_transformations(
    semantic: &Semantic,
    file_path: &Path,
) -> Result<Vec<Transformation>> {
    let mut transformations = Vec::new();

    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call_expr) = node.kind() else {
            continue;
        };

        let Some(function_name) = extract_function_name(call_expr) else {
            continue;
        };

        if function_name != "isComponentPresent" {
            continue;
        }

        let Some(first_arg) = call_expr.arguments.first() else {
            continue;
        };

        let Some(component_name) = extract_component_name_from_argument(first_arg) else {
            continue;
        };

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

    Ok(transformations)
}
