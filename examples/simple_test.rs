// Use the correct crate name as defined in Cargo.toml
extern crate qwik_analyzer;

use qwik_analyzer::QwikAnalyzer;
use std::path::Path;

fn main() {
    println!("🚀 Testing qwik-analyzer...");

    let analyzer = QwikAnalyzer::new(true); // Enable debug mode

    // Test files that should contain Checkbox.Description within Checkbox.Root
    let test_files = [
        "examples/test_files/direct_example.tsx",
        "examples/test_files/indirect_example.tsx",
    ];

    for file_path in &test_files {
        let path = Path::new(file_path);
        println!("\n📂 Analyzing: {}", path.display());

        if !path.exists() {
            println!("❌ File does not exist: {}", file_path);
            continue;
        }

        match analyzer.analyze_file(path) {
            Ok(result) => {
                println!("✅ Analysis successful!");
                println!("   has_component: {}", result.has_component);
            }
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
            }
        }
    }

    println!("\n🏁 Test complete!");
}
