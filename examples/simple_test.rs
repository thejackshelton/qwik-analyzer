use qwik_analyzer::QwikAnalyzer;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Qwik Analyzer with Real Qwik Project");

    let analyzer = QwikAnalyzer::new(true);

    // Test files from the real Qwik project
    let test_files = [
        ("Direct Example", "qwik-app/src/examples/direct_example.tsx"),
        (
            "Indirect Example",
            "qwik-app/src/examples/indirect_example.tsx",
        ),
        ("Heyo Component", "qwik-app/src/examples/heyo.tsx"),
    ];

    for (name, file_path) in test_files {
        println!("\n📁 Analyzing: {}", name);
        println!("   Path: {}", file_path);

        let path = Path::new(file_path);
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

                for (i, candidate) in result.candidate_components.iter().enumerate() {
                    println!(
                        "        {}. {} (import: {:?}, provides: {})",
                        i + 1,
                        candidate.component_name,
                        candidate.import_source,
                        candidate.provides_description
                    );
                }

                if result.has_description {
                    println!("   🎯 This file should get _staticHasDescription=true");
                } else {
                    println!("   📝 This file should get _staticHasDescription=false");
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    println!("\n🎉 Analysis complete!");
    Ok(())
}
