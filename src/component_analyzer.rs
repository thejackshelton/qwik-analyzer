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

    println!("🔍 Building import symbol table...");
    let import_symbols = build_import_symbol_table(semantic);

    println!("🔍 Extracting JSX elements...");
    let jsx_elements = extract_jsx_elements(semantic);

    for element in &jsx_elements {
        println!("🏷️  Found JSX element: '{}'", element);
    }

    println!("🔍 Analyzing imported components for isComponentPresent() calls...");

    // Check if this file contains isComponentPresent calls (this is a component definition)
    let component_transformations = find_and_prepare_component_transformations(semantic);

    // Check if this file uses Root components (this is a consumer)
    let (has_description, consumer_transformations) =
        analyze_root_component_usage(semantic, &import_symbols);

    let mut all_transformations = Vec::new();
    all_transformations.extend(component_transformations);
    all_transformations.extend(consumer_transformations);

    println!("📊 Analysis result: {}", has_description);

    Ok(AnalysisResult {
        has_description,
        file_path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(),
        transformations: all_transformations,
    })
}

/// Build a symbol table of all imported symbols
fn build_import_symbol_table(semantic: &Semantic) -> Vec<ImportSymbol> {
    let mut symbols = Vec::new();

    for node in semantic.nodes().iter() {
        if let AstKind::ImportDeclaration(import_decl) = node.kind() {
            let module_source = import_decl.source.value.to_string();

            if let Some(specifiers) = &import_decl.specifiers {
                for spec in specifiers {
                    match spec {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                            let local_name = spec.local.name.to_string();
                            let imported_name = spec.imported.name().to_string();

                            symbols.push(ImportSymbol {
                                local_name: local_name.clone(),
                                imported_name,
                                module_source: module_source.clone(),
                            });
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                            let local_name = spec.local.name.to_string();

                            symbols.push(ImportSymbol {
                                local_name: local_name.clone(),
                                imported_name: "default".to_string(),
                                module_source: module_source.clone(),
                            });
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                            spec,
                        ) => {
                            let local_name = spec.local.name.to_string();

                            symbols.push(ImportSymbol {
                                local_name: local_name.clone(),
                                imported_name: "*".to_string(),
                                module_source: module_source.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    symbols
}

/// Extract all JSX element names from the semantic tree
fn extract_jsx_elements(semantic: &Semantic) -> Vec<String> {
    let mut elements = Vec::new();

    println!("🔍 Extracting JSX elements...");

    for node in semantic.nodes().iter() {
        if let AstKind::JSXElement(jsx_element) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name(jsx_element) {
                println!("🏷️  Found JSX element: '{}'", element_name);
                elements.push(element_name);
            }
        }
    }

    elements
}

/// Extract JSX element name with proper semantic resolution
fn extract_jsx_element_name(jsx_element: &JSXElement) -> Option<String> {
    match &jsx_element.opening_element.name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => {
            let name = identifier.name.to_string();
            Some(name)
        }
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXElementName::MemberExpression(member_expr) => {
            // Handle member expressions like DummyComp.Description
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
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
) -> Option<String> {
    match object {
        oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
            let name = identifier.name.to_string();
            Some(name)
        }
        oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(member_expr) => {
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        oxc_ast::ast::JSXMemberExpressionObject::ThisExpression(_) => None,
    }
}

/// Analyze imported components to see if they call isComponentPresent()
fn analyze_imported_components(
    import_symbols: &Vec<ImportSymbol>,
    current_file: &Path,
) -> Result<Vec<ComponentWithCheck>> {
    let mut component_checks = Vec::new();

    println!("🔍 Analyzing imported components for isComponentPresent() calls...");

    for symbol in import_symbols {
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
                        let component_name =
                            format!("{}.{}", symbol.local_name, check.component_name);
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
    let import_symbols = build_import_symbol_table(&semantic);

    // Extract JSX elements and check for target
    let jsx_elements = extract_jsx_elements(&semantic);

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

/// Find isComponentPresent() calls in component definitions and prepare prop-based transformations
fn find_and_prepare_component_transformations(semantic: &Semantic) -> Vec<Transformation> {
    let mut transformations = Vec::new();
    let source_text = semantic.source_text();

    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "isComponentPresent" {
                    if !call_expr.arguments.is_empty() {
                        let call_span = call_expr.span;
                        let start = call_span.start as u32;
                        let end = call_span.end as u32;

                        // Extract the component argument
                        if let Some(component_arg) = call_expr.arguments.first() {
                            if let Some(component_name) =
                                extract_component_name_from_argument(component_arg)
                            {
                                // Check if we need to add props parameter to the component function
                                if let Some(props_transformation) =
                                    check_and_add_props_parameter(semantic, call_span.start)
                                {
                                    transformations.push(props_transformation);
                                }

                                // Transform: isComponentPresent(Description)
                                // ->        isComponentPresent(Description, props.__qwik_analyzer_has_Description)
                                let prop_name = format!("__qwik_analyzer_has_{}", component_name);
                                let replacement = format!(
                                    "isComponentPresent({}, props.{})",
                                    component_name, prop_name
                                );

                                transformations.push(Transformation {
                                    start,
                                    end,
                                    replacement: replacement.clone(),
                                });

                                println!("🔄 Preparing component transformation: {}..{} -> {} (call: isComponentPresent)", start, end, replacement);
                            }
                        }
                    }
                }
            }
        }
    }

    transformations
}

/// Check if component function needs props parameter and add it if missing
fn check_and_add_props_parameter(
    semantic: &Semantic,
    call_position: u32,
) -> Option<Transformation> {
    // Find the component$ function that contains this call
    for node in semantic.nodes().iter() {
        if let AstKind::CallExpression(call_expr) = node.kind() {
            // Check if this is a component$ call
            if let Some(function_name) = extract_function_name(call_expr) {
                if function_name == "component$" {
                    // Check if this call contains our isComponentPresent call
                    let call_start = call_expr.span.start;
                    let call_end = call_expr.span.end;

                    if call_position >= call_start && call_position <= call_end {
                        // Found the component$ call that contains our isComponentPresent
                        // Check if it has a props parameter
                        return check_component_arrow_function_params(call_expr);
                    }
                }
            }
        }
    }

    None
}

/// Check component$() arrow function parameters and add props if missing
fn check_component_arrow_function_params(call_expr: &CallExpression) -> Option<Transformation> {
    if let Some(first_arg) = call_expr.arguments.first() {
        if let oxc_ast::ast::Argument::ArrowFunctionExpression(arrow_fn) = first_arg {
            // Check if the function already has parameters
            if arrow_fn.params.items.is_empty() && arrow_fn.params.rest.is_none() {
                // No parameters - we need to add props
                let params_span = arrow_fn.params.span;
                let start = params_span.start as u32;
                let end = params_span.end as u32;

                // Transform () => { ... } to (props) => { ... }
                let replacement = "(props)".to_string();

                println!(
                    "🔄 Adding props parameter: {}..{} -> {}",
                    start, end, replacement
                );

                return Some(Transformation {
                    start,
                    end,
                    replacement,
                });
            } else {
                // Function already has parameters - check if one is named 'props'
                let has_props = arrow_fn.params.items.iter().any(|param| {
                    if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(ident) =
                        &param.pattern.kind
                    {
                        ident.name.as_str() == "props"
                    } else {
                        false
                    }
                });

                if !has_props {
                    // Has parameters but no 'props' - we need to add props as first parameter
                    let params_start = arrow_fn.params.span.start as u32;

                    // Insert props as first parameter
                    let insertion_point = params_start + 1; // After the opening (
                    let replacement = "props, ".to_string();

                    println!(
                        "🔄 Adding props as first parameter at position {}",
                        insertion_point
                    );

                    return Some(Transformation {
                        start: insertion_point,
                        end: insertion_point,
                        replacement,
                    });
                }
            }
        }
    }

    None
}

/// Analyze Root component usage and generate consumer-side prop injections
fn analyze_root_component_usage(
    semantic: &Semantic,
    import_symbols: &Vec<ImportSymbol>,
) -> (bool, Vec<Transformation>) {
    let mut transformations = Vec::new();
    let mut overall_has_description = false;

    // Find JSX elements that are Root components
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name_from_opening(jsx_opening) {
                // Check if this is a Root component (e.g., "DummyComp.Root")
                if element_name.ends_with(".Root") {
                    println!("🎯 Found Root component usage: {}", element_name);

                    // Analyze the subtree of this Root component for target components
                    let has_description_in_subtree =
                        analyze_subtree_for_target_components(semantic, node);

                    if has_description_in_subtree {
                        overall_has_description = true;

                        // Generate prop injection transformation
                        let jsx_span = jsx_opening.span;
                        let start = jsx_span.start as u32;
                        let end = jsx_span.end as u32;

                        // Find insertion point for the prop (before closing >)
                        let source_text = semantic.source_text();
                        let jsx_text = &source_text[start as usize..end as usize];

                        // Insert the prop before the closing >
                        if let Some(closing_pos) = jsx_text.rfind('>') {
                            let insertion_point = start + closing_pos as u32;
                            let prop_injection = " __qwik_analyzer_has_Description={true}";

                            transformations.push(Transformation {
                                start: insertion_point,
                                end: insertion_point,
                                replacement: prop_injection.to_string(),
                            });

                            println!(
                                "🔄 Preparing consumer transformation: inject prop at position {}",
                                insertion_point
                            );
                        }
                    }
                }
            }
        }
    }

    (overall_has_description, transformations)
}

/// Extract component name from a function call argument
fn extract_component_name_from_argument(argument: &oxc_ast::ast::Argument) -> Option<String> {
    match argument {
        oxc_ast::ast::Argument::Identifier(identifier) => Some(identifier.name.to_string()),
        _ => None,
    }
}

/// Extract JSX element name from opening element
fn extract_jsx_element_name_from_opening(
    jsx_opening: &oxc_ast::ast::JSXOpeningElement,
) -> Option<String> {
    match &jsx_opening.name {
        oxc_ast::ast::JSXElementName::Identifier(identifier) => Some(identifier.name.to_string()),
        oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
            Some(identifier.name.to_string())
        }
        oxc_ast::ast::JSXElementName::MemberExpression(member_expr) => {
            // Handle member expressions like DummyComp.Description
            let object_name = extract_jsx_member_object_name(&member_expr.object)?;
            let property_name = &member_expr.property.name;
            Some(format!("{}.{}", object_name, property_name))
        }
        _ => None,
    }
}

/// Analyze the subtree of a Root component for target components like Description
fn analyze_subtree_for_target_components(
    semantic: &Semantic,
    root_node: &oxc_semantic::AstNode,
) -> bool {
    // This is a simplified version - in practice, you'd want to traverse the JSX tree
    // and look for Description components within this Root's children

    // For now, let's look for any Description usage in the entire file
    // In a more sophisticated implementation, we'd traverse only the children of this specific Root
    for node in semantic.nodes().iter() {
        if let AstKind::JSXOpeningElement(jsx_opening) = node.kind() {
            if let Some(element_name) = extract_jsx_element_name_from_opening(jsx_opening) {
                if element_name.contains("Description") {
                    println!(
                        "✅ Found Description component in subtree: {}",
                        element_name
                    );
                    return true;
                }
            }
        }
    }

    false
}
