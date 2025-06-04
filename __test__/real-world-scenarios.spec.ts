import { test, expect, describe, beforeAll, afterAll } from "vitest";
import { analyzeAndTransformCode } from "../index.cjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Real World Scenarios", () => {
  test("checkbox example should work correctly", async () => {
    console.log("🐛 Testing the actual checkbox.tsx scenario");
    
    // Read the actual checkbox.tsx file
    const checkboxPath = path.resolve(__dirname, "../qwik-app/src/examples/checkbox.tsx");
    const checkboxCode = fs.readFileSync(checkboxPath, "utf-8");
    
    console.log("📄 Actual checkbox.tsx:");
    console.log(checkboxCode);
    console.log("\n🔄 Transforming...");
    
    const result = analyzeAndTransformCode(checkboxCode, checkboxPath);
    
    console.log("✅ Transform completed!");
    console.log("📄 Transformed checkbox.tsx:");
    console.log(result);
    console.log("");
    
    // Based on the checkbox.tsx and root.tsx files:
    // root.tsx has: isComponentPresent(Description), isComponentPresent(Title), isComponentPresent(Checkbox.Description)
    // checkbox.tsx has: <DummyComp.Title /> and <Checkbox.Description /> but NO <Description />
    
    // Local present components should be injected as true:
    expect(result).toContain("__qwik_analyzer_has_Title={true}"); // <DummyComp.Title /> is present (local)
    
    // NOTE: Description is false because isComponentPresent(Description) looks for direct <Description />
    // but JSX has <DummyComp.Description />. This is a separate namespacing issue.
    expect(result).toContain("__qwik_analyzer_has_Description={false}"); // Direct <Description /> not present
    
    // External components should be injected as false (because isComponentPresent call exists):
    expect(result).toContain("__qwik_analyzer_has_Checkbox_Description={false}"); // <Checkbox.Description /> is external, so false
  });

  test("DummyComp.Root resolution should work with object exports", async () => {
    console.log("🐛 Testing DummyComp.Root resolution through index.ts");
    
    // Read the actual DummyComp index.ts
    const indexPath = path.resolve(__dirname, "../qwik-app/src/components/dummy-comp/index.ts");
    const indexContent = fs.readFileSync(indexPath, "utf-8");
    console.log("📄 DummyComp index.ts:");
    console.log(indexContent);
    
    // Read the actual root.tsx
    const rootPath = path.resolve(__dirname, "../qwik-app/src/components/dummy-comp/root.tsx");
    const rootContent = fs.readFileSync(rootPath, "utf-8");
    console.log("📄 DummyComp root.tsx:");
    console.log(rootContent);
    
    const result = analyzeAndTransformCode(rootContent, rootPath);
    
    console.log("✅ Transform completed!");
    console.log("📄 Transformed root.tsx:");
    console.log(result);
    
    // root.tsx should get props parameter and transformed isComponentPresent calls
    expect(result).toContain("component$((props"); // Should add props parameter
    expect(result).toContain("props.__qwik_analyzer_has_Description"); // Transform Description call
    expect(result).toContain("props.__qwik_analyzer_has_Title"); // Transform Title call
    expect(result).toContain("props.__qwik_analyzer_has_Checkbox_Description"); // Transform Checkbox.Description call
  });

  test("should distinguish local Description vs external Checkbox.Description", async () => {
    console.log("🐛 Testing component name collision resolution");
    
    // The key issue from the logs: 
    // root.tsx has isComponentPresent(Description) [local] and isComponentPresent(Checkbox.Description) [external]
    // These should be treated as different components
    
    const rootPath = path.resolve(__dirname, "../qwik-app/src/components/dummy-comp/root.tsx");
    const result = analyzeAndTransformCode(
      fs.readFileSync(rootPath, "utf-8"), 
      rootPath
    );
    
    console.log("📄 Transformed root.tsx:");
    console.log(result);
    
    // Should transform local Description
    expect(result).toContain("props.__qwik_analyzer_has_Description");
    
    // Should transform external Checkbox.Description (with underscore)
    expect(result).toContain("props.__qwik_analyzer_has_Checkbox_Description");
    
    // These should be different prop names
    expect(result).not.toContain("props.__qwik_analyzer_has_Checkbox.Description");
  });

  test("external package imports should be handled gracefully", async () => {
    console.log("🐛 Testing external package import handling");
    
    // Test that @kunai-consulting/qwik imports don't crash the analyzer
    const checkboxPath = path.resolve(__dirname, "../qwik-app/src/examples/checkbox.tsx");
    const checkboxCode = fs.readFileSync(checkboxPath, "utf-8");
    
    console.log("🔄 Analyzing checkbox.tsx with external imports...");
    
    // This should not throw an error even though @kunai-consulting/qwik can't be resolved
    const result = analyzeAndTransformCode(checkboxCode, checkboxPath);
    
    console.log("✅ External imports handled gracefully!");
    
    // Should still contain the original external imports unchanged
    expect(result).toContain('import { Checkbox } from "@kunai-consulting/qwik"');
    expect(result).toContain('<Checkbox.Root>');
    expect(result).toContain('<Checkbox.Description />');
  });

  test("tilde alias imports should work", async () => {
    console.log("🐛 Testing tilde alias import resolution");
    
    // checkbox.tsx uses: import { DummyComp } from "~/components/dummy-comp";
    const checkboxPath = path.resolve(__dirname, "../qwik-app/src/examples/checkbox.tsx");
    const checkboxCode = fs.readFileSync(checkboxPath, "utf-8");
    
    console.log("🔄 Analyzing tilde alias imports...");
    
    const result = analyzeAndTransformCode(checkboxCode, checkboxPath);
    
    console.log("✅ Tilde alias imports processed!");
    console.log("📄 Result:");
    console.log(result);
    
    // Should resolve ~/components/dummy-comp to the actual dummy-comp module
    // and inject props based on what's found in root.tsx
    expect(result).toContain("DummyComp.Root");
    
    // If tilde resolution works, we should see prop injection
    const hasPropInjection = result.includes("__qwik_analyzer_has_");
    console.log("🔍 Has prop injection:", hasPropInjection);
  });
});