use oxc_allocator::Allocator;
use oxc_ast::ast::JSXElement;
use oxc_ast::AstKind;
use oxc_parser;
use oxc_semantic::Semantic;
use oxc_span;
use std::fs;
use std::path::Path;

use crate::{AnalysisResult, CandidateComponent, Result};

/// Internal result for component analysis with more details
#[derive(Debug)]
struct DetailedAnalysisResult {
    has_description: bool,
    candidate_components: Vec<CandidateComponent>,
}

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
fn find_component_within_parent_semantic(
    semantic: &Semantic,
    parent_component: &str,
    child_component: &str,
) -> DetailedAnalysisResult {
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

    DetailedAnalysisResult {
        has_description: found_directly,
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
            file_path: file_path.to_string_lossy().to_string(),
            dependencies: Vec::new(),
        });
    }

    // Look for Checkbox.Description within Checkbox.Root
    let mut result =
        find_component_within_parent_semantic(&semantic, "Checkbox.Root", "Description");

    // If not found directly, perform indirect analysis
    if !result.has_description && !result.candidate_components.is_empty() {
        println!("🔄 Starting indirect analysis...");

        // Resolve import sources for candidate components
        resolve_import_sources_semantic(&semantic, &mut result.candidate_components);

        // Recursively analyze candidate components
        if find_indirect_components(&mut result.candidate_components, file_path)? {
            result.has_description = true;
            println!("✅ Found Checkbox.Description through indirect analysis!");
        }
    }

    Ok(AnalysisResult {
        has_description: result.has_description,
        file_path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(), // TODO: Extract actual dependencies
    })
}

/// Resolve import sources for candidate components using semantic analysis
pub fn resolve_import_sources_semantic(
    semantic: &Semantic,
    candidate_components: &mut [CandidateComponent],
) {
    println!(
        "🔍 Resolving import sources for {} candidates",
        candidate_components.len()
    );

    // Walk through all nodes to find import declarations
    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            let import_source = import_decl.source.value.to_string();
            println!("📦 Processing import from: '{}'", import_source);

            // Check each import specifier
            if let Some(specifiers) = &import_decl.specifiers {
                for specifier in specifiers {
                    match specifier {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                            let imported_name = spec.local.name.to_string();
                            println!("   - Named import: '{}'", imported_name);

                            // Find matching candidate component
                            for candidate in candidate_components.iter_mut() {
                                if candidate.component_name == imported_name {
                                    println!(
                                        "✅ Matched '{}' to import source '{}'",
                                        imported_name, import_source
                                    );
                                    candidate.import_source = Some(import_source.clone());
                                }
                            }
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                            let imported_name = spec.local.name.to_string();
                            println!("   - Default import: '{}'", imported_name);

                            // Find matching candidate component
                            for candidate in candidate_components.iter_mut() {
                                if candidate.component_name == imported_name {
                                    println!(
                                        "✅ Matched '{}' to import source '{}'",
                                        imported_name, import_source
                                    );
                                    candidate.import_source = Some(import_source.clone());
                                }
                            }
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                            spec,
                        ) => {
                            let imported_name = spec.local.name.to_string();
                            println!("   - Namespace import: '{}'", imported_name);

                            // Find matching candidate component
                            for candidate in candidate_components.iter_mut() {
                                if candidate
                                    .component_name
                                    .starts_with(&format!("{}.", imported_name))
                                {
                                    println!(
                                        "✅ Matched '{}' to import source '{}'",
                                        candidate.component_name, import_source
                                    );
                                    candidate.import_source = Some(import_source.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Log results
    for candidate in candidate_components.iter() {
        if let Some(import_source) = &candidate.import_source {
            println!(
                "📋 Candidate '{}' resolved to '{}'",
                candidate.component_name, import_source
            );
        } else {
            println!(
                "⚠️  Candidate '{}' could not be resolved",
                candidate.component_name
            );
        }
    }
}

/// Recursively find components that provide descriptions
pub fn find_indirect_components(
    candidate_components: &mut [CandidateComponent],
    importer: &Path,
) -> Result<bool> {
    let mut found_description = false;

    for candidate in candidate_components.iter_mut() {
        if let Some(import_source) = &candidate.import_source {
            println!(
                "🔍 Analyzing candidate '{}' from '{}'",
                candidate.component_name, import_source
            );

            // Resolve the import path
            match resolve_import_path(import_source, importer) {
                Ok(resolved_path) => {
                    candidate.resolved_path = Some(resolved_path.clone());
                    println!("📂 Resolved to: {}", resolved_path);

                    // Analyze the resolved component file
                    match analyze_component_for_description(Path::new(&resolved_path)) {
                        Ok(provides_description) => {
                            candidate.provides_description = provides_description;
                            if provides_description {
                                println!(
                                    "✅ Component '{}' provides description!",
                                    candidate.component_name
                                );
                                found_description = true;
                            } else {
                                println!(
                                    "❌ Component '{}' does not provide description",
                                    candidate.component_name
                                );
                            }
                        }
                        Err(e) => {
                            println!(
                                "⚠️  Could not analyze component '{}': {}",
                                candidate.component_name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "⚠️  Could not resolve import path for '{}': {}",
                        candidate.component_name, e
                    );
                }
            }
        } else {
            println!(
                "⚠️  Candidate '{}' has no import source",
                candidate.component_name
            );
        }
    }

    Ok(found_description)
}

/// Analyze a component file to see if it contains Checkbox.Description
fn analyze_component_for_description(file_path: &Path) -> Result<bool> {
    println!("🔍 Analyzing component file: {}", file_path.display());

    if !file_path.exists() {
        println!("❌ File does not exist: {}", file_path.display());
        return Ok(false);
    }

    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(file_path).unwrap_or_default();

    // Parse the file
    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        eprintln!("Parser errors in {}: {:?}", file_path.display(), errors);
        return Ok(false);
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

    // Check if this component contains Checkbox.Description anywhere
    let contains_description = contains_checkbox_description_anywhere(&semantic);

    println!(
        "📊 File {} contains Checkbox.Description: {}",
        file_path.display(),
        contains_description
    );

    Ok(contains_description)
}

/// Resolve import path relative to importer
fn resolve_import_path(import_source: &str, importer: &Path) -> Result<String> {
    let importer_dir = importer.parent().ok_or("Could not get parent directory")?;

    let resolved = if import_source.starts_with('.') {
        // Relative import
        importer_dir.join(import_source)
    } else {
        // Absolute or node_modules import - for now, just return as-is
        return Ok(import_source.to_string());
    };

    // Try different extensions
    for ext in &[".tsx", ".ts", ".jsx", ".js"] {
        let with_ext = resolved.with_extension(&ext[1..]);
        if with_ext.exists() {
            return Ok(with_ext.to_string_lossy().to_string());
        }

        // Also try with index file
        let index_path = resolved.join(format!("index{}", ext));
        if index_path.exists() {
            return Ok(index_path.to_string_lossy().to_string());
        }
    }

    // If not found, return the resolved path anyway
    Ok(resolved.to_string_lossy().to_string())
}

/// Check if semantic analysis contains Checkbox.Description anywhere
pub fn contains_checkbox_description_anywhere(semantic: &Semantic) -> bool {
    for node in semantic.nodes().iter() {
        if let AstKind::JSXElement(jsx_element) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_element) {
                if element_name == "Checkbox.Description" {
                    return true;
                }
            }
        }
    }
    false
}
