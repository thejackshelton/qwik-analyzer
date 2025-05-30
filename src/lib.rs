pub mod component_analyzer;
pub mod qwik_analyzer;

pub use qwik_analyzer::QwikAnalyzer;

use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CandidateComponent {
    pub component_name: String,
    pub import_source: Option<String>,
    pub resolved_path: Option<String>,
    pub provides_description: bool,
}

#[derive(Debug)]
pub struct AnalysisResult {
    pub has_description: bool,
    pub found_directly: bool,
    pub candidate_components: Vec<CandidateComponent>,
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

    // For now just return success
    Ok(())
}
