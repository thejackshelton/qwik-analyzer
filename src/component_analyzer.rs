use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, JSXOpeningElement};
use oxc_ast::AstKind;
use oxc_parser;
use oxc_resolver::{ResolveOptions, Resolver};
use oxc_semantic::Semantic;
use oxc_span::SourceType;
use std::fs;
use std::path::Path;

use crate::{AnalysisResult, Result, Transformation};

fn debug(msg: &str) {
    println!("{}", msg);
}

#[derive(Debug, Clone)]
struct ComponentPresenceCall {
    component_name: String,
    is_present_in_subtree: bool,
    source_file: String,
}

pub fn analyze_file_with_semantics(file_path: &Path) -> Result<AnalysisResult> {
    let source_text = fs::read_to_string(file_path)?;
    analyze_code_with_semantics(&source_text, file_path)
}

pub fn analyze_code_with_semantics(
    source_text: &str,
    file_path: &Path,
) -> Result<AnalysisResult> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_path).unwrap_or_default();

    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, source_text, source_type).parse();

    if !errors.is_empty() {
        eprintln!("Parser errors: {:?}", errors);
        return Ok(AnalysisResult {
            has_description: false,
            file_path: file_path.to_string_lossy().to_string(),
            dependencies: Vec::new(),
            transformations: Vec::new(),
        });
    }

    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
    let semantic = &semantic_ret.semantic;

    if !semantic_ret.errors.is_empty() {
        eprintln!("Semantic errors: {:?}", semantic_ret.errors);
    }

    let jsx_components = extract_imported_jsx_components(semantic);
    debug(&format!("🔍 Found JSX components: {:?}", jsx_components));

    let mut all_component_calls = Vec::new();
    for jsx_component in jsx_components {
        if let Ok(calls) = find_is_component_present_calls_in_imported_component(semantic, &jsx_component, file_path) {
            all_component_calls.extend(calls);
        }
    }

    for call in &mut all_component_calls {
        call.is_present_in_subtree = is_component_present_in_jsx_subtree(semantic, &call.component_name, file_path)?;
    }

    debug(&format!("📊 Analysis found {} isComponentPresent calls from imported components, {} have target components in current file", 
             all_component_calls.len(), 
             all_component_calls.iter().filter(|c| c.is_present_in_subtree).count()));

    let mut transformations = Vec::new();
    let mut has_any_component = false;

    for call in &all_component_calls {
        if call.is_present_in_subtree {
            has_any_component = true;
        }
    }

    if has_any_component {
        let current_file_transformations = generate_transformations_for_current_file(semantic, &all_component_calls)?;
        transformations.extend(current_file_transformations);
    }

    let current_file_component_transformations = generate_transformations_for_current_file_components(semantic, file_path)?;
    transformations.extend(current_file_component_transformations);

    Ok(AnalysisResult {
        has_description: has_any_component,
        file_path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(),
        transformations,
    })
}

fn extract_imported_jsx_components(semantic: &Semantic) -> Vec<String> {
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
                }


                else if element_name.chars().next().map_or(false, |c| c.is_ascii_uppercase()) &&
                        !is_html_element(&element_name) {
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

fn find_is_component_present_calls_in_imported_component(
    semantic: &Semantic,
    jsx_component: &str,
    current_file: &Path,
) -> Result<Vec<ComponentPresenceCall>> {
    debug(&format!("🔍 Analyzing imported component: {}", jsx_component));
    
    if jsx_component.contains('.') {
        let parts: Vec<&str> = jsx_component.split('.').collect();
        if parts.len() == 2 {
            let module_name = parts[0];
            let component_name = parts[1];
            
            let import_source = find_import_source_for_component(semantic, module_name);
            if let Some(source) = import_source {
                if source.starts_with('.') {
                    if let Ok(module_dir) = resolve_import_with_oxc(&source, current_file) {
                        debug(&format!("📂 Resolved module {} to: {}", module_name, module_dir));
                        
                        let component_file = find_component_file_in_module(&module_dir, component_name)?;
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
                    debug(&format!("📂 Resolved component {} to: {}", jsx_component, resolved_path));
                    return analyze_file_for_is_component_present_calls(&resolved_path);
                }
            }
        }
    }
    
    Ok(Vec::new())
}

fn find_component_file_in_module(module_dir: &str, component_name: &str) -> Result<String> {
    let module_path = Path::new(module_dir);
    
    let actual_module_dir = if module_dir.ends_with("index.ts") || module_dir.ends_with("index.tsx") || 
                               module_dir.ends_with("index.js") || module_dir.ends_with("index.jsx") {
        module_path.parent().ok_or("Could not get module parent directory")?
    } else {
        module_path
    };
    
    let component_file_name = component_name.to_lowercase();
    
    for ext in &[".tsx", ".ts", ".jsx", ".js"] {
        let component_file = actual_module_dir.join(format!("{}{}", component_file_name, ext));
        if component_file.exists() {
            debug(&format!("📂 Found component file: {}", component_file.display()));
            return Ok(component_file.to_string_lossy().to_string());
        }
    }
    
    Err(format!("Could not find component file for {} in {}", component_name, actual_module_dir.display()).into())
}

fn analyze_file_for_is_component_present_calls(file_path: &str) -> Result<Vec<ComponentPresenceCall>> {
    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(file_path)).unwrap_or_default();

    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        return Ok(Vec::new());
    }

    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
    let semantic = semantic_ret.semantic;

    let mut calls = Vec::new();

    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "isComponentPresent" {
                    if let Some(first_arg) = call_expr.arguments.first() {
                        if let Some(component_name) = extract_component_name_from_argument(first_arg) {
                            debug(&format!("🔍 Found isComponentPresent({}) call in {}", component_name, file_path));
                            
                            calls.push(ComponentPresenceCall {
                                component_name,
                                is_present_in_subtree: false,
                                source_file: file_path.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(calls)
}

fn is_component_present_in_jsx_subtree(
    semantic: &Semantic,
    component_name: &str,
    current_file: &Path,
) -> Result<bool> {
    debug(&format!("🔍 Checking if {} is present in JSX subtree", component_name));
    
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
                if element_name == component_name {
                    debug(&format!("✅ Found direct usage of {} in JSX", component_name));
                    return Ok(true);
                }
                
                if element_name.ends_with(&format!(".{}", component_name)) {
                    debug(&format!("✅ Found member expression usage of {} in JSX: {}", component_name, element_name));
                    return Ok(true);
                }
            }
        }
    }

    debug(&format!("🔍 Checking imported components for {} usage...", component_name));
    
    let jsx_components = extract_imported_jsx_components(semantic);
    
    for jsx_component in jsx_components {
        if jsx_component.ends_with(&format!(".{}", component_name)) {
            continue;
        }
        
        if let Ok(contains_target) = analyze_imported_component_for_target(
            semantic, 
            &jsx_component, 
            component_name, 
            current_file
        ) {
            if contains_target {
                debug(&format!("✅ Found {} in imported component {}", component_name, jsx_component));
                return Ok(true);
            }
        }
    }

    debug(&format!("❌ Component {} not found in JSX subtree", component_name));
    Ok(false)
}

fn is_html_element(name: &str) -> bool {
    matches!(name.to_lowercase().as_str(), 
        "div" | "span" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | 
        "a" | "img" | "input" | "button" | "form" | "ul" | "ol" | "li" |
        "table" | "tr" | "td" | "th" | "thead" | "tbody" | "nav" | "header" |
        "footer" | "main" | "section" | "article" | "aside" | "details" |
        "summary" | "dialog" | "canvas" | "svg" | "video" | "audio"
    )
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
                debug(&format!("📂 Analyzing {} (from {}) for {}", jsx_component, resolved_path, target_component));
                
                return analyze_file_for_component_usage(&resolved_path, target_component);
            }
        }
    }
    
    Ok(false)
}

fn find_import_source_for_component(semantic: &Semantic, component_name: &str) -> Option<String> {
    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            let module_source = import_decl.source.value.to_string();
            
            if let Some(specifiers) = &import_decl.specifiers {
                for spec in specifiers {
                    let local_name = match spec {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                            spec.local.name.to_string()
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                            spec.local.name.to_string()
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(spec) => {
                            spec.local.name.to_string()
                        }
                    };
                    
                    if local_name == component_name {
                        return Some(module_source);
                    }
                }
            }
        }
    }
    
    None
}

/// Analyze a file to see if it contains usage of the target component
fn analyze_file_for_component_usage(file_path: &str, target_component: &str) -> Result<bool> {
    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(file_path)).unwrap_or_default();

    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        return Ok(false);
    }

    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
    let semantic = semantic_ret.semantic;

    // Check for target component usage
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_opening) {
                if element_name == target_component || 
                   element_name.ends_with(&format!(".{}", target_component)) {
                    debug(&format!("✅ Found {} in {}", target_component, file_path));
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

fn resolve_import_with_oxc(import_source: &str, current_file: &Path) -> Result<String> {
    let options = ResolveOptions {
        extensions: vec![".tsx".into(), ".ts".into(), ".jsx".into(), ".js".into()],
        ..Default::default()
    };

    let resolver = Resolver::new(options);
    let current_dir = current_file
        .parent()
        .ok_or("Could not get parent directory")?;

    match resolver.resolve(current_dir, import_source) {
        Ok(resolution) => {
            let resolved_path = resolution.full_path();
            Ok(resolved_path.to_string_lossy().to_string())
        }
        Err(e) => {
            debug(&format!("❌ OXC resolution failed for '{}': {:?}", import_source, e));
            Err(format!("Could not resolve import '{}': {:?}", import_source, e).into())
        }
    }
}

fn generate_transformations_for_current_file(
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
    
    let source_file_name = Path::new(&call.source_file).file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    debug(&format!("🔍 Looking for JSX component corresponding to source file: {} ({})", call.source_file, source_file_name));
    
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
                    debug(&format!("🔧 Adding prop to JSX component: {}", element_name));
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

fn extract_function_name(call_expr: &CallExpression) -> Option<String> {
    match &call_expr.callee {
        oxc_ast::ast::Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

fn extract_component_name_from_argument(argument: &oxc_ast::ast::Argument) -> Option<String> {
    match argument {
        oxc_ast::ast::Argument::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

fn extract_jsx_element_name(jsx_opening: &JSXOpeningElement) -> Option<String> {
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

fn generate_transformations_for_current_file_components(
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
                            if let oxc_ast::ast::Argument::ArrowFunctionExpression(arrow_fn) = first_arg {
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
                    debug(&format!("🔧 Adding props parameter at position {} in {}", insert_pos, file_path.display()));
                }
            }
        }
 
        for node in semantic.nodes().iter() {
            if let AstKind::CallExpression(call_expr) = node.kind() {
                if let Some(function_name) = extract_function_name(call_expr) {
                    if function_name == "isComponentPresent" {
                        if let Some(first_arg) = call_expr.arguments.first() {
                            if let Some(component_name) = extract_component_name_from_argument(first_arg) {
                                let prop_name = format!("__qwik_analyzer_has_{}", component_name);
                                let new_call = format!("isComponentPresent({}, props.{})", component_name, prop_name);
                                
                                transformations.push(Transformation {
                                    start: call_expr.span.start,
                                    end: call_expr.span.end,
                                    replacement: new_call,
                                });
                                debug(&format!("🔧 Transforming isComponentPresent({}) call in {}", component_name, file_path.display()));
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(transformations)
}
