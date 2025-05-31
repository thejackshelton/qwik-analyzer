use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_parser;
use oxc_resolver::{ResolveOptions, Resolver};
use oxc_semantic::Semantic;
use oxc_span::{SourceType, VALID_EXTENSIONS};
use std::fs;
use std::path::Path;

use crate::component_analyzer::jsx_analysis::extract_jsx_element_name;
use crate::component_analyzer::utils::{
    debug, extract_component_name_from_argument, extract_function_name, ComponentPresenceCall,
};
use crate::Result;

pub fn find_import_source_for_component(
    semantic: &Semantic,
    component_name: &str,
) -> Option<String> {
    for node in semantic.nodes().iter() {
        let AstKind::ImportDeclaration(import_decl) = node.kind() else {
            continue;
        };

        let module_source = import_decl.source.value.to_string();
        let Some(specifiers) = &import_decl.specifiers else {
            continue;
        };

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

    None
}

pub fn resolve_import_path(import_source: &str, current_file: &Path) -> Result<String> {
    let options = ResolveOptions {
        extensions: VALID_EXTENSIONS
            .iter()
            .map(|ext| format!(".{}", ext).into())
            .collect(),
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
            debug(&format!(
                "❌ Import resolution failed for '{}': {:?}",
                import_source, e
            ));
            Err(format!("Could not resolve import '{}': {:?}", import_source, e).into())
        }
    }
}

pub fn find_component_file_in_module(module_dir: &str, component_name: &str) -> Result<String> {
    let module_path = Path::new(module_dir);

    let actual_module_dir = if is_index_file(module_dir) {
        module_path
            .parent()
            .ok_or("Could not get module parent directory")?
    } else {
        module_path
    };

    let component_file_name = component_name.to_lowercase();

    for ext in VALID_EXTENSIONS {
        let component_file = actual_module_dir.join(format!("{}.{}", component_file_name, ext));
        if component_file.exists() {
            debug(&format!(
                "📂 Found component file: {}",
                component_file.display()
            ));
            return Ok(component_file.to_string_lossy().to_string());
        }
    }

    Err(format!(
        "Could not find component file for {} in {}",
        component_name,
        actual_module_dir.display()
    )
    .into())
}

fn is_index_file(path: &str) -> bool {
    VALID_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(&format!("index.{}", ext)))
}

pub fn find_calls_in_file(file_path: &str) -> Result<Vec<ComponentPresenceCall>> {
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

        debug(&format!(
            "🔍 Found isComponentPresent({}) call in {}",
            component_name, file_path
        ));

        calls.push(ComponentPresenceCall {
            component_name,
            is_present_in_subtree: false,
            source_file: file_path.to_string(),
        });
    }

    Ok(calls)
}

pub fn file_has_component(file_path: &str, target_component: &str) -> Result<bool> {
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
        let AstKind::JSXOpeningElement(jsx_opening) = node.kind() else {
            continue;
        };

        let Some(element_name) = extract_jsx_element_name(jsx_opening) else {
            continue;
        };

        if element_name == target_component
            || element_name.ends_with(&format!(".{}", target_component))
        {
            debug(&format!("✅ Found {} in {}", target_component, file_path));
            return Ok(true);
        }
    }

    Ok(false)
}
