use oxc_allocator::Allocator;
use oxc_parser;
use oxc_span::SourceType;
use std::fs;
use std::path::Path;

use crate::{AnalysisResult, Result};

pub mod jsx_analysis;
pub mod import_resolver;
pub mod component_presence;
pub mod transformations;
pub mod utils;

use jsx_analysis::extract_imported_jsx_components;
use component_presence::find_presence_calls;
use transformations::{transform_file, transform_components};
use utils::debug;

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
        if let Ok(calls) = find_presence_calls(semantic, &jsx_component, file_path) {
            all_component_calls.extend(calls);
        }
    }

    for call in &mut all_component_calls {
        call.is_present_in_subtree = component_presence::has_component(semantic, &call.component_name, file_path)?;
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
        let current_file_transformations = transform_file(semantic, &all_component_calls)?;
        transformations.extend(current_file_transformations);
    }

    let current_file_component_transformations = transform_components(semantic, file_path)?;
    transformations.extend(current_file_component_transformations);

    Ok(AnalysisResult {
        has_description: has_any_component,
        file_path: file_path.to_string_lossy().to_string(),
        dependencies: Vec::new(),
        transformations,
    })
} 