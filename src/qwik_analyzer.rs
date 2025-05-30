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

        let source_text = std::fs::read_to_string(file_path)?;

        // Test parsing the file
        parse_file_with_semantic(&source_text, file_path)?;

        // Check if file imports from the target package
        let imports_target_package =
            component_analyzer::check_imports_from_package(&source_text, "@kunai-consulting/qwik");

        if !imports_target_package {
            if self.debug_mode {
                println!("[qwik-analyzer] No imports from target package, skipping");
            }
            return Ok(AnalysisResult {
                has_description: false,
                found_directly: false,
                candidate_components: Vec::new(),
            });
        }

        // Look for Checkbox.Description within Checkbox.Root
        let result = component_analyzer::find_component_within_parent(
            &source_text,
            "Checkbox.Root",
            "Description",
        );

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
