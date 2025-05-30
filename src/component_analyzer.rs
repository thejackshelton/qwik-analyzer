use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclaration, JSXElement};
use oxc_ast::AstKind;
use oxc_parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span;
use std::fs;
use std::path::Path;

use crate::{parse_file_with_semantic, AnalysisResult, CandidateComponent, Result};

/// Checks if the code contains imports from a specific package using semantic analysis.
pub fn check_imports_from_package_semantic(semantic: &Semantic, package_name: &str) -> bool {
    println!("🔍 Checking for imports from package: '{}'", package_name);

    // Walk through all nodes to find import declarations using correct iterator
    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            println!("📦 Found import: '{}'", import_decl.source.value);
            if import_decl.source.value == package_name {
                println!("✅ Found target package import!");
                return true;
            }
        }
    }

    println!("❌ No imports found from target package");
    false
}

/// Finds JSX components within parent components using semantic analysis.
pub fn find_component_within_parent_semantic(
    semantic: &Semantic,
    parent_component: &str,
    child_component: &str,
) -> AnalysisResult {
    let mut found_directly = false;
    let mut candidate_components = Vec::new();
    let mut in_parent_component = false;
    let mut nesting_level = 0;

    println!(
        "🔍 Looking for '{}' inside '{}'",
        child_component, parent_component
    );

    // Walk through all AST nodes using correct iterator
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::JSXElement(jsx_element) => {
                if let Some(element_name) = extract_jsx_element_name(jsx_element) {
                    println!("🏷️  Found JSX element: '{}'", element_name);

                    // Check if this is the parent component
                    if element_name == parent_component {
                        println!("📦 Entering parent component: {}", parent_component);
                        in_parent_component = true;
                        nesting_level += 1;
                    }
                    // Check if we're inside a parent component and found the child
                    else if in_parent_component && element_name.contains(child_component) {
                        println!("🎯 Found child component '{}' inside parent!", element_name);
                        found_directly = true;
                    }
                    // Collect other components inside parent for indirect analysis
                    else if in_parent_component && nesting_level > 0 {
                        // Don't include nested parent components or obvious child components
                        if !element_name.starts_with(
                            &parent_component.split('.').next().unwrap_or("").to_string(),
                        ) {
                            println!("📋 Adding candidate component: {}", element_name);
                            candidate_components.push(CandidateComponent {
                                component_name: element_name,
                                import_source: None,
                                resolved_path: None,
                                provides_description: false,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    println!(
        "📊 Analysis complete - found directly: {}, candidates: {}",
        found_directly,
        candidate_components.len()
    );

    AnalysisResult {
        has_description: found_directly,
        found_directly,
        candidate_components,
    }
}

/// Extract JSX element name from JSX element node
fn extract_jsx_element_name(jsx_element: &JSXElement) -> Option<String> {
    match &jsx_element.opening_element.name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => Some(identifier.name.to_string()),
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXElementName::MemberExpression(member_expr) => {
            // Handle member expressions like Checkbox.Description
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        oxc_ast::ast::JSXElementName::NamespacedName(namespaced) => {
            Some(format!("{}:{}", namespaced.namespace.name, namespaced.name))
        }
        oxc_ast::ast::JSXElementName::ThisExpression(_) => {
            // Skip 'this' expressions as they're not standard component names
            None
        }
    }
}

/// Extract object name from JSX member expression object
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
        oxc_ast::ast::JSXMemberExpressionObject::ThisExpression(_) => {
            // Skip 'this' expressions as they're not standard component names
            None
        }
    }
}

/// Analyze a file with proper semantic analysis
pub fn analyze_file_with_semantics(file_path: &Path) -> Result<AnalysisResult> {
    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(file_path).unwrap_or_default();

    // Parse the file
    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        eprintln!("Parser errors in {}: {:?}", file_path.display(), errors);
    }

    // Build semantic information
    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);

    if !semantic_ret.errors.is_empty() {
        eprintln!(
            "Semantic errors in {}: {:?}",
            file_path.display(),
            semantic_ret.errors
        );
    }

    let semantic = semantic_ret.semantic;

    // Check for imports from target package
    if !check_imports_from_package_semantic(&semantic, "@kunai-consulting/qwik") {
        return Ok(AnalysisResult {
            has_description: false,
            found_directly: false,
            candidate_components: Vec::new(),
        });
    }

    // Look for Checkbox.Description within Checkbox.Root
    let result = find_component_within_parent_semantic(&semantic, "Checkbox.Root", "Description");

    Ok(result)
}

// Legacy functions for backward compatibility
pub fn check_imports_from_package(source_text: &str, package_name: &str) -> bool {
    source_text.contains(package_name)
}

pub fn find_component_within_parent(
    source_text: &str,
    parent_component: &str,
    child_component: &str,
) -> AnalysisResult {
    let has_description =
        source_text.contains(&format!("{}.{}", parent_component, child_component));

    AnalysisResult {
        has_description,
        found_directly: has_description,
        candidate_components: Vec::new(),
    }
}

pub fn resolve_import_sources(
    _source_text: &str,
    _candidate_components: &mut [CandidateComponent],
) {
    // TODO: Use semantic analysis to find symbol references and their import sources
}

pub fn find_indirect_components(
    _candidate_components: &mut [CandidateComponent],
    _importer: &Path,
) -> Result<bool> {
    Ok(false)
}

pub fn analyze_file_for_description(file_path: &str) -> Result<bool> {
    let source_text = fs::read_to_string(file_path)?;
    let path = Path::new(file_path);

    parse_file_with_semantic(&source_text, path)?;
    Ok(source_text.contains("Checkbox.Description"))
}

fn resolve_import_path(import_source: &str, importer: &Path) -> Result<String> {
    if import_source.starts_with('.') {
        let parent = importer.parent().unwrap_or(Path::new("."));
        let resolved = parent.join(import_source);
        Ok(resolved.to_string_lossy().to_string())
    } else {
        Ok(import_source.to_string())
    }
}
