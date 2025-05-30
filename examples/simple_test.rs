use qwik_analyzer::QwikAnalyzer;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Qwik Analyzer");

    let analyzer = QwikAnalyzer::new(true); // Enable debug mode

    // Test files
    let test_files = [
        ("Direct Example", "examples/test_files/direct_example.tsx"),
        (
            "Indirect Example",
            "examples/test_files/indirect_example.tsx",
        ),
        ("Heyo Component", "examples/test_files/heyo.tsx"),
    ];

    for (name, file_path) in test_files {
        println!("\n📁 Analyzing: {}", name);
        println!("   Path: {}", file_path);

        let path = Path::new(file_path);

        if !path.exists() {
            println!("   ❌ File not found");
            continue;
        }

        match analyzer.analyze_file(path) {
            Ok(result) => {
                println!("   ✅ Analysis complete");
                println!("   📊 Results:");
                println!("      - Has description: {}", result.has_description);
                println!("      - Found directly: {}", result.found_directly);
                println!(
                    "      - Candidate components: {}",
                    result.candidate_components.len()
                );

                if result.has_description {
                    println!("   🎯 This file should get _staticHasDescription=true");
                } else {
                    println!("   📝 This file should get _staticHasDescription=false");
                }
            }
            Err(e) => {
                println!("   ❌ Analysis failed: {}", e);
            }
        }
    }

    println!("\n🎉 Analysis complete!");
    Ok(())
}
