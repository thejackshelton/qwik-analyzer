use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, JSXElement};
use oxc_ast::AstKind;
use oxc_parser;
use oxc_semantic::Semantic;
use oxc_span;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{AnalysisResult, Result, Transformation};

/// Semantic symbol information for imports
#[derive(Debug, Clone)]
struct ImportSymbol {
    local_name: String,
    imported_name: String,
    module_source: String,
}

/// Information about a component that calls isComponentPresent
#[derive(Debug, Clone)]
struct ComponentWithCheck {
    component_name: String,
    checks_for: String,
}

/// Analyze a file using cross-file component-aware analysis
pub fn analyze_file_with_semantics(
    file_path: &Path,
    module_specifier: Option<&str>,
) -> Result<AnalysisResult> {
    let source_text = fs::read_to_string(file_path)?;
    analyze_code_with_semantics(&source_text, file_path, module_specifier)
}

/// Analyze code content directly (for Vite integration)
pub fn analyze_code_with_semantics(
    source_text: &str,
    file_path: &Path,
    module_specifier: Option<&str>,
) -> Result<AnalysisResult> {
    let allocator = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(file_path).unwrap_or_default();

    // Parse the code
    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, source_text, source_type).parse();

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

    // Build import symbol table with optional module filtering
    let import_symbols = build_import_symbol_table(&semantic, module_specifier);

    // Find JSX elements that use imported components
    let jsx_elements = extract_jsx_elements(&semantic, &import_symbols);

    // Check if any imported components call isComponentPresent()
    let component_checks = analyze_imported_components(&import_symbols, file_path)?;

    // Check if any of the requested components are present in current JSX tree with recursive analysis
    let has_description = check_component_presence_with_recursive_analysis(
        &jsx_elements,
        &component_checks,
        file_path,
    )?;

    // Find and prepare transformations for isComponentPresent() calls
    let transformations = find_and_prepare_transformations(&semantic, has_description);

    println!("📊 Analysis result: {}", has_description);

    Ok(AnalysisResult {
        has_description,
        file_path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(),
        transformations,
    })
}

/// Build a symbol table of all imports using semantic analysis
fn build_import_symbol_table(
    semantic: &Semantic,
    module_specifier: Option<&str>,
) -> HashMap<String, ImportSymbol> {
    let mut symbols = HashMap::new();

    println!("🔍 Building import symbol table...");

    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            let module_source = import_decl.source.value.to_string();
            println!("📦 Processing import from: '{}'", module_source);

            if let Some(specifiers) = &import_decl.specifiers {
                for specifier in specifiers {
                    match specifier {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                            let local_name = spec.local.name.to_string();
                            let imported_name = spec.imported.name().to_string();

                            if let Some(module_specifier) = module_specifier {
                                if !module_source.contains(module_specifier) {
                                    continue;
                                }
                            }

                            symbols.insert(
                                local_name.clone(),
                                ImportSymbol {
                                    local_name: local_name.clone(),
                                    imported_name,
                                    module_source: module_source.clone(),
                                },
                            );

                            println!(
                                "   ✅ Named import: {} (local: {})",
                                spec.imported.name(),
                                local_name
                            );
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                            let local_name = spec.local.name.to_string();

                            if let Some(module_specifier) = module_specifier {
                                if !module_source.contains(module_specifier) {
                                    continue;
                                }
                            }

                            symbols.insert(
                                local_name.clone(),
                                ImportSymbol {
                                    local_name: local_name.clone(),
                                    imported_name: "default".to_string(),
                                    module_source: module_source.clone(),
                                },
                            );

                            println!("   ✅ Default import: {}", local_name);
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                            spec,
                        ) => {
                            let local_name = spec.local.name.to_string();

                            if let Some(module_specifier) = module_specifier {
                                if !module_source.contains(module_specifier) {
                                    continue;
                                }
                            }

                            symbols.insert(
                                local_name.clone(),
                                ImportSymbol {
                                    local_name: local_name.clone(),
                                    imported_name: "*".to_string(),
                                    module_source: module_source.clone(),
                                },
                            );

                            println!("   ✅ Namespace import: {}", local_name);
                        }
                    }
                }
            }
        }
    }

    symbols
}

/// Extract all JSX element names from the semantic tree
fn extract_jsx_elements(
    semantic: &Semantic,
    import_symbols: &HashMap<String, ImportSymbol>,
) -> Vec<String> {
    let mut elements = Vec::new();

    println!("🔍 Extracting JSX elements...");

    for node in semantic.nodes().iter() {
        if let AstKind::JSXElement(jsx_element) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_element, import_symbols) {
                println!("🏷️  Found JSX element: '{}'", element_name);
                elements.push(element_name);
            }
        }
    }

    elements
}

/// Extract JSX element name with proper semantic resolution
fn extract_jsx_element_name(
    jsx_element: &JSXElement,
    import_symbols: &HashMap<String, ImportSymbol>,
) -> Option<String> {
    match &jsx_element.opening_element.name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => {
            let name = identifier.name.to_string();

            // Check if this identifier is an imported symbol
            if let Some(symbol) = import_symbols.get(&name) {
                Some(format!("{} (from {})", name, symbol.module_source))
            } else {
                Some(name)
            }
        }
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXElementName::MemberExpression(member_expr) => {
            // Handle member expressions like DummyComp.Description
            let object_name = extract_jsx_member_object_name(&member_expr.object, import_symbols)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        oxc_ast::ast::JSXElementName::NamespacedName(namespaced) => {
            Some(format!("{}:{}", namespaced.namespace.name, namespaced.name))
        }
        oxc_ast::ast::JSXElementName::ThisExpression(_) => None,
    }
}

/// Extract object name from JSX member expression with semantic resolution
fn extract_jsx_member_object_name(
    object: &oxc_ast::ast::JSXMemberExpressionObject,
    import_symbols: &HashMap<String, ImportSymbol>,
) -> Option<String> {
    match object {
        oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
            let name = identifier.name.to_string();

            // Return the local alias name as used in JSX
            if let Some(_symbol) = import_symbols.get(&name) {
                Some(name)
            } else {
                Some(name)
            }
        }
        oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(member_expr) => {
            let object_name = extract_jsx_member_object_name(&member_expr.object, import_symbols)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        oxc_ast::ast::JSXMemberExpressionObject::ThisExpression(_) => None,
    }
}

/// Analyze imported components to see if they call isComponentPresent()
fn analyze_imported_components(
    import_symbols: &HashMap<String, ImportSymbol>,
    current_file: &Path,
) -> Result<Vec<ComponentWithCheck>> {
    let mut component_checks = Vec::new();

    println!("🔍 Analyzing imported components for isComponentPresent() calls...");

    for (local_name, symbol) in import_symbols {
        // Skip non-relative imports for now (e.g., '@builder.io/qwik')
        if !symbol.module_source.starts_with('.') {
            continue;
        }

        // Resolve the import path
        match resolve_import_path(&symbol.module_source, current_file) {
            Ok(resolved_path) => {
                println!("📂 Analyzing component file: {}", resolved_path);

                // Analyze the component file for isComponentPresent() calls
                if let Ok(checks) = find_component_checks_in_file(&resolved_path) {
                    for check in checks {
                        // Map the component check to the local name used in current file
                        let component_name = format!("{}.{}", local_name, check.component_name);
                        let checks_for = check.checks_for.clone();

                        component_checks.push(ComponentWithCheck {
                            component_name: component_name.clone(),
                            checks_for: checks_for.clone(),
                        });

                        println!(
                            "✅ Component '{}' checks for '{}'",
                            component_name, checks_for
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "⚠️ Could not resolve import '{}': {}",
                    symbol.module_source, e
                );
            }
        }
    }

    Ok(component_checks)
}

/// Find isComponentPresent() calls in a specific file
fn find_component_checks_in_file(file_path: &str) -> Result<Vec<ComponentWithCheck>> {
    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(Path::new(file_path)).unwrap_or_default();

    // Parse the file
    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        return Ok(Vec::new());
    }

    // Build semantic information
    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
    let semantic = semantic_ret.semantic;

    let mut checks = Vec::new();

    // First, try to find isComponentPresent calls directly in this file
    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "isComponentPresent" {
                    if let Some(component_name) = extract_component_argument(call_expr) {
                        checks.push(ComponentWithCheck {
                            component_name: "Root".to_string(), // Assume it's in Root for now
                            checks_for: component_name,
                        });
                    }
                }
            }
        }
    }

    // If no direct calls found, check if this is an index file that exports other components
    if checks.is_empty() {
        println!("🔍 No direct isComponentPresent calls found, checking exports...");

        // Look for imports and exports that might point to actual component files
        for node in semantic.nodes().iter() {
            if let AstKind::ImportDeclaration(import_decl) = node.kind() {
                let import_source = import_decl.source.value.to_string();

                // Check if this import might be for a component that calls isComponentPresent
                if import_source.starts_with('.')
                    && (import_source.contains("root") || import_source.contains("Root"))
                {
                    println!(
                        "📂 Found potential Root component import: {}",
                        import_source
                    );

                    // Resolve and analyze the Root component file
                    if let Ok(resolved_path) =
                        resolve_import_path(&import_source, Path::new(file_path))
                    {
                        println!("📂 Analyzing Root component file: {}", resolved_path);

                        if let Ok(root_checks) = find_component_checks_in_file(&resolved_path) {
                            checks.extend(root_checks);
                        }
                    }
                }
            }
        }
    }

    Ok(checks)
}

/// Extract function name from call expression
fn extract_function_name(call_expr: &CallExpression) -> Option<String> {
    match &call_expr.callee {
        oxc_ast::ast::Expression::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

/// Extract component argument from isComponentPresent() call
fn extract_component_argument(call_expr: &CallExpression) -> Option<String> {
    if let Some(first_arg) = call_expr.arguments.first() {
        match &first_arg {
            oxc_ast::ast::Argument::Identifier(identifier) => Some(identifier.name.to_string()),
            _ => None,
        }
    } else {
        None
    }
}

/// Check if requested components are present with recursive subtree analysis
fn check_component_presence_with_recursive_analysis(
    jsx_elements: &[String],
    component_checks: &[ComponentWithCheck],
    current_file: &Path,
) -> Result<bool> {
    if component_checks.is_empty() {
        println!("❌ No imported components with isComponentPresent() calls found");
        return Ok(false);
    }

    println!("🔍 Checking component presence with recursive analysis...");

    for check in component_checks {
        println!(
            "🎯 Component '{}' checks for '{}'",
            check.component_name, check.checks_for
        );

        // Check if the component that makes the check is used in JSX
        let component_used = jsx_elements
            .iter()
            .any(|element| element.contains(&check.component_name));

        if component_used {
            println!("✅ Found component '{}' being used", check.component_name);

            // First check if target component is directly in current JSX tree
            let direct_found = jsx_elements.iter().any(|element| {
                element.contains(&check.checks_for)
                    || element.contains(&format!(".{}", check.checks_for))
            });

            if direct_found {
                println!(
                    "✅ Found target component '{}' directly in JSX tree!",
                    check.checks_for
                );
                return Ok(true);
            }

            // If not found directly, recursively check imported components within the Root subtree
            println!("🔍 Recursively analyzing components within Root subtree...");

            if recursively_check_jsx_subtree(jsx_elements, &check.checks_for, current_file)? {
                println!(
                    "✅ Found target component '{}' in recursive JSX analysis!",
                    check.checks_for
                );
                return Ok(true);
            }

            println!(
                "❌ Target component '{}' not found in JSX tree or subtrees",
                check.checks_for
            );
        } else {
            println!(
                "❌ Component '{}' not used in this JSX tree",
                check.component_name
            );
        }
    }

    Ok(false)
}

/// Recursively analyze JSX subtree by following component imports
fn recursively_check_jsx_subtree(
    jsx_elements: &[String],
    target_component: &str,
    current_file: &Path,
) -> Result<bool> {
    // Extract component names that are not part of the target module (like "Heyo")
    for element in jsx_elements {
        // Skip elements that contain dots (they're likely from the target module)
        if element.contains('.') {
            continue;
        }

        // Skip basic HTML elements
        if element.starts_with(char::is_lowercase) {
            continue;
        }

        println!("🔍 Recursively analyzing component: {}", element);

        // Try to find and analyze this component file
        if let Ok(component_file) = find_component_file(element, current_file) {
            println!("📂 Found component file: {}", component_file);

            // Analyze the component file recursively
            if let Ok(has_target) = analyze_component_for_target(&component_file, target_component)
            {
                if has_target {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Find the file for a given component name
fn find_component_file(component_name: &str, current_file: &Path) -> Result<String> {
    let current_dir = current_file
        .parent()
        .ok_or("Could not get parent directory")?;
    let component_file_name = component_name.to_lowercase();

    // Try common component file patterns
    for pattern in &[
        format!("./{}.tsx", component_file_name),
        format!("./{}.ts", component_file_name),
        format!("./{}.jsx", component_file_name),
        format!("./{}.js", component_file_name),
    ] {
        let resolved_path = current_dir.join(pattern);
        if resolved_path.exists() {
            return Ok(resolved_path.to_string_lossy().to_string());
        }
    }

    Err(format!("Could not find component file for: {}", component_name).into())
}

/// Analyze a component file to see if it contains the target component
fn analyze_component_for_target(file_path: &str, target_component: &str) -> Result<bool> {
    let source_text = fs::read_to_string(file_path)?;
    let allocator = Allocator::default();
    let source_type = oxc_span::SourceType::from_path(Path::new(file_path)).unwrap_or_default();

    // Parse the file
    let oxc_parser::ParserReturn {
        program, errors, ..
    } = oxc_parser::Parser::new(&allocator, &source_text, source_type).parse();

    if !errors.is_empty() {
        return Ok(false);
    }

    // Build semantic information
    let semantic_ret = oxc_semantic::SemanticBuilder::new().build(&program);
    let semantic = semantic_ret.semantic;

    // Build import symbol table (no filtering for recursive analysis)
    let import_symbols = build_import_symbol_table(&semantic, None);

    // Extract JSX elements and check for target
    let jsx_elements = extract_jsx_elements(&semantic, &import_symbols);

    for element in jsx_elements {
        if element.contains(target_component) || element.contains(&format!(".{}", target_component))
        {
            println!(
                "✅ Found target '{}' in component file: {}",
                target_component, file_path
            );
            return Ok(true);
        }
    }

    Ok(false)
}

/// Resolve import path relative to importer
fn resolve_import_path(import_source: &str, importer: &Path) -> Result<String> {
    let importer_dir = importer.parent().ok_or("Could not get parent directory")?;

    let resolved = if import_source.starts_with('.') {
        // Relative import
        importer_dir.join(import_source)
    } else {
        // Absolute or node_modules import
        return Err("Non-relative imports not supported".into());
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

    // If not found, return error
    Err(format!("Could not resolve import: {}", import_source).into())
}

/// Find isComponentPresent() calls and prepare transformations
fn find_and_prepare_transformations(
    semantic: &Semantic,
    has_description: bool,
) -> Vec<Transformation> {
    let mut transformations = Vec::new();
    let source_text = semantic.source_text();

    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "isComponentPresent" {
                    // Only transform if this is a proper function call (not part of import/export)
                    // Check if the call has arguments and is in a valid context
                    if !call_expr.arguments.is_empty() {
                        // Get the span of the call expression
                        let span = call_expr.span;
                        let start = span.start as u32;
                        let end = span.end as u32;
                        let replacement = if has_description { "true" } else { "false" };

                        // Extract the actual source text for debugging
                        let actual_text = if (start as usize) < source_text.len()
                            && (end as usize) <= source_text.len()
                        {
                            &source_text[(start as usize)..(end as usize)]
                        } else {
                            "INVALID_SPAN"
                        };

                        println!(
                            "🔄 Preparing transformation: {}..{} -> {} (call: {})",
                            start, end, replacement, function_name
                        );
                        println!("📝 Actual source text at span: '{}'", actual_text);

                        // Show context around the span for debugging
                        let context_start = if start >= 20 { start - 20 } else { 0 };
                        let context_end = std::cmp::min(end + 20, source_text.len() as u32);
                        let context =
                            &source_text[(context_start as usize)..(context_end as usize)];
                        println!("📖 Context: '{}'", context);

                        transformations.push(Transformation {
                            start,
                            end,
                            replacement: replacement.to_string(),
                        });
                    }
                }
            }
        }
    }

    transformations
}
