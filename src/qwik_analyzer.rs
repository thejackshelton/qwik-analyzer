use std::path::Path;

use crate::{component_analyzer, parse_file_with_semantic, AnalysisResult, Result};

pub struct QwikAnalyzer {
    debug_mode: bool,
}

impl QwikAnalyzer {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    /// Analyzes a file to determine if it contains Checkbox.Description
    pub fn analyze_file(&self, file_path: &Path) -> Result<AnalysisResult> {
        if self.debug_mode {
            println!("[qwik-analyzer] Analyzing file: {}", file_path.display());
        }

        // Use semantic analysis instead of string-based analysis
        let result = component_analyzer::analyze_file_with_semantics(file_path)?;

        if self.debug_mode {
            println!(
                "[qwik-analyzer] Analysis result: has_description = {}",
                result.has_description
            );
        }

        Ok(result)
    }

    /// Transform the code by adding static props
    pub fn transform_code(
        &self,
        source_text: &str,
        file_path: &Path,
        has_description: bool,
    ) -> Result<Option<String>> {
        if self.debug_mode {
            println!(
                "[qwik-analyzer] Transforming code for {} with has_description={}",
                file_path.display(),
                has_description
            );
        }

        // Test parsing the file
        parse_file_with_semantic(source_text, file_path)?;

        // TODO: Implement AST transformation
        // For now, return None to indicate no transformation needed
        Ok(None)
    }
}
