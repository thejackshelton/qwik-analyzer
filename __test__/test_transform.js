const { analyzeAndTransformCode } = require('./index.js');

async function testTransform() {
  const code = `
import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$(() => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Testing analyzeAndTransformCode function...');
  console.log('📄 Original code:');
  console.log(code);
  console.log('\n🔄 Transforming...');
  
  try {
    const result = await analyzeAndTransformCode(code, '/test/root.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed code:');
    console.log(result);
    
    if (result === code) {
      console.log('❌ Code was not transformed (identical output)');
    } else {
      console.log('✅ Code was transformed!');
    }
  } catch (error) {
    console.error('❌ Transform failed:', error);
  }
}

testTransform(); 