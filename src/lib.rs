use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

pub mod component_analyzer;

#[derive(Debug)]
#[napi(object)]
pub struct Transformation {
    pub start: u32,
    pub end: u32,
    pub replacement: String,
}

#[derive(Debug)]
#[napi(object)]
pub struct AnalysisResult {
    pub has_description: bool,
    pub file_path: String,
    pub dependencies: Vec<String>,
    pub transformations: Vec<Transformation>,
}

use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Parse a TypeScript/JavaScript file and return semantic information
pub fn parse_file_with_semantic(source_text: &str, file_path: &Path) -> Result<()> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_path).unwrap_or_default();

    let ParserReturn {
        program, errors, ..
    } = Parser::new(&allocator, source_text, source_type).parse();

    if !errors.is_empty() {
        eprintln!("Parser errors in {}: {:?}", file_path.display(), errors);
    }

    let semantic_ret = SemanticBuilder::new().build(&program);

    if !semantic_ret.errors.is_empty() {
        eprintln!(
            "Semantic errors in {}: {:?}",
            file_path.display(),
            semantic_ret.errors
        );
    }

    Ok(())
}

#[napi]
pub fn analyze_file(
    file_path: String,
    module_specifier: Option<String>,
) -> napi::Result<AnalysisResult> {
    let path = Path::new(&file_path);
    match component_analyzer::analyze_file_with_semantics(path, module_specifier.as_deref()) {
        Ok(result) => Ok(result),
        Err(e) => Err(napi::Error::new(
            napi::Status::GenericFailure,
            format!("Analysis failed: {}", e),
        )),
    }
}

#[napi]
pub fn analyze_file_changed(file_path: String, event: String, module_specifier: Option<String>) {
    if let Err(e) = analyze_file(file_path.clone(), module_specifier) {
        eprintln!("Error analyzing changed file {}: {}", file_path, e);
    }
}

/// Analyze and transform code content directly (for Vite integration)
#[napi]
pub fn analyze_and_transform_code(
    code: String,
    file_path: String,
    module_specifier: Option<String>,
) -> napi::Result<String> {
    let path = Path::new(&file_path);
    let result = match component_analyzer::analyze_code_with_semantics(
        &code,
        path,
        module_specifier.as_deref(),
    ) {
        Ok(result) => result,
        Err(e) => {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Analysis failed: {}", e),
            ))
        }
    };

    if result.transformations.is_empty() {
        return Ok(code);
    }

    // Apply transformations in reverse order to maintain offsets
    let mut transformed_code = code;
    let mut sorted_transformations = result.transformations;
    sorted_transformations.sort_by(|a, b| b.start.cmp(&a.start));

    for transformation in sorted_transformations {
        let start = transformation.start as usize;
        let end = transformation.end as usize;

        if start <= transformed_code.len() && end <= transformed_code.len() && start <= end {
            let before = &transformed_code[..start];
            let after = &transformed_code[end..];
            transformed_code = format!("{}{}{}", before, transformation.replacement, after);
        }
    }

    Ok(transformed_code)
}
