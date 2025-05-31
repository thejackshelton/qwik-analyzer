use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

pub mod component_analyzer;
pub mod qwik_analyzer;

pub use qwik_analyzer::QwikAnalyzer;

use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

#[derive(Debug, Clone)]
pub struct CandidateComponent {
    pub component_name: String,
    pub import_source: Option<String>,
    pub resolved_path: Option<String>,
    pub provides_description: bool,
}

#[derive(Debug)]
#[napi(object)]
pub struct AnalysisResult {
    pub has_description: bool,
    pub file_path: String,
    pub dependencies: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
pub fn analyze_file_changed(file_path: String, event: String) -> napi::Result<()> {
    let analyzer = QwikAnalyzer::new(false);
    let path = Path::new(&file_path);

    match event.as_str() {
        "create" | "update" => {
            // Just analyze the file - no caching needed
            match analyzer.analyze_file(path) {
                Ok(_result) => {
                    // Analysis complete - results available for next transform/load
                }
                Err(e) => {
                    eprintln!("Failed to analyze {}: {}", file_path, e);
                }
            }
        }
        "delete" => {
            // File deleted - nothing to do
        }
        _ => {
            // Unknown event, ignore
        }
    }

    Ok(())
}

#[napi]
pub fn analyze_file(file_path: String) -> napi::Result<AnalysisResult> {
    let analyzer = QwikAnalyzer::new(false);
    let path = Path::new(&file_path);

    match analyzer.analyze_file(path) {
        Ok(result) => Ok(AnalysisResult {
            has_description: result.has_description,
            file_path: file_path.clone(),
            dependencies: vec![], // TODO: Extract from component analysis
        }),
        Err(e) => Err(Error::new(
            Status::GenericFailure,
            format!("Analysis failed: {}", e),
        )),
    }
}
