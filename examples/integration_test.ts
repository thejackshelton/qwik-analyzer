#!/usr/bin/env node

/**
 * Integration test for qwik-analyzer NAPI + Vite plugin
 * Tests both the Rust NAPI bindings and the real example files
 */

import { analyzeFile, analyzeFileChanged } from '../index.js';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface TestCase {
  name: string;
  file: string;
  expectedHasDescription: boolean;
  description: string;
}

async function runIntegrationTests() {
  console.log('🚀 Running qwik-analyzer integration tests...\n');

  const testCases: TestCase[] = [
    {
      name: 'Direct Example',
      file: '../qwik-app/src/examples/direct_example.tsx',
      expectedHasDescription: true,
      description: 'Should detect Checkbox.Description directly within Checkbox.Root'
    },
    {
      name: 'Indirect Example', 
      file: '../qwik-app/src/examples/indirect_example.tsx',
      expectedHasDescription: true,
      description: 'Should detect Checkbox.Description via imported Heyo component'
    },
    {
      name: 'Heyo Component',
      file: '../qwik-app/src/examples/heyo.tsx', 
      expectedHasDescription: false,
      description: 'Should return false - contains Checkbox.Description but not within Checkbox.Root'
    }
  ];

  let passedTests = 0;
  const totalTests = testCases.length;

  for (const testCase of testCases) {
    console.log(`🔍 Testing: ${testCase.name}`);
    console.log(`   ${testCase.description}`);
    
    const filePath = path.resolve(__dirname, testCase.file);
    
    if (!fs.existsSync(filePath)) {
      console.log(`   ❌ File not found: ${filePath}`);
      continue;
    }

    try {
      // Test the NAPI binding
      const result = await analyzeFile(filePath);
      
      console.log(`   📊 Analysis result: ${JSON.stringify(result, null, 2)}`);
      
      if (result.hasDescription === testCase.expectedHasDescription) {
        console.log(`   ✅ PASSED - Expected: ${testCase.expectedHasDescription}, Got: ${result.hasDescription}`);
        passedTests++;
      } else {
        console.log(`   ❌ FAILED - Expected: ${testCase.expectedHasDescription}, Got: ${result.hasDescription}`);
      }

      // Test file change event
      console.log('   🔄 Testing file change event...');
      await analyzeFileChanged(filePath, 'update');
      console.log('   ✅ File change event processed successfully');

    } catch (error) {
      console.log(`   ❌ ERROR: ${(error as Error).message}`);
    }
    
    console.log(''); // Add spacing
  }

  // Summary
  console.log('📈 Test Summary:');
  console.log(`   Total tests: ${totalTests}`);
  console.log(`   Passed: ${passedTests}`);
  console.log(`   Failed: ${totalTests - passedTests}`);
  
  if (passedTests === totalTests) {
    console.log('   🎉 All tests passed!');
  } else {
    console.log('   ⚠️  Some tests failed.');
  }

  // Test error handling
  console.log('\n🧪 Testing error handling...');
  try {
    await analyzeFile('/nonexistent/file.tsx');
    console.log('   ❌ Should have thrown an error for non-existent file');
  } catch (error) {
    console.log(`   ✅ Correctly handled non-existent file: ${(error as Error).message}`);
  }

  console.log('\n🏁 Integration tests complete!');
}

// Run the tests
runIntegrationTests().catch(console.error); 