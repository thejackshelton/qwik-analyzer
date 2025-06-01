#!/usr/bin/env node

// Test the NAPI bindings for qwik-analyzer
import { analyzeFile, analyzeFileChanged } from '../index.js';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function testNAPIBindings() {
  console.log('🚀 Testing NAPI bindings for qwik-analyzer...\n');

  // Create a test file with Checkbox components
  const testFile = path.join(__dirname, 'test_checkbox.tsx');
  const testContent = `
import { Checkbox } from '@kunai-consulting/qwik';

export default function MyComponent() {
  return (
    <div>
      <Checkbox.Root>
        <Checkbox.Description>This has a description!</Checkbox.Description>
      </Checkbox.Root>
    </div>
  );
}
`;

  // Write test file
  fs.writeFileSync(testFile, testContent);
  console.log('📝 Created test file:', testFile);

  try {
    // Test 1: Direct analysis
    console.log('\n🔍 Test 1: Direct file analysis');
    const result = await analyzeFile(testFile);
    console.log('   Result:', JSON.stringify(result, null, 2));
    
    if (result.hasDescription) {
      console.log('   ✅ Correctly detected Checkbox.Description!');
    } else {
      console.log('   ❌ Failed to detect Checkbox.Description');
    }

    // Test 2: File change event
    console.log('\n🔍 Test 2: File change event handling');
    await analyzeFileChanged(testFile, 'update');
    console.log('   ✅ File change event processed successfully');

    // Test 3: Non-existent file
    console.log('\n🔍 Test 3: Non-existent file handling');
    try {
      await analyzeFile('/nonexistent/file.tsx');
      console.log('   ❌ Should have failed for non-existent file');
    } catch (error) {
      console.log('   ✅ Correctly handled non-existent file:', (error as Error).message);
    }

  } catch (error) {
    console.error('❌ NAPI test failed:', error);
  } finally {
    // Clean up
    if (fs.existsSync(testFile)) {
      fs.unlinkSync(testFile);
      console.log('\n🧹 Cleaned up test file');
    }
  }

  console.log('\n🏁 NAPI binding tests complete!');
}

// Run the test
testNAPIBindings().catch(console.error); 