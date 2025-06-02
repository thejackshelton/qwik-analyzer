const { analyzeAndTransformCode } = require('./index.js');

async function testEnhancedPropInjection() {
  console.log('🧪 Testing Enhanced Prop Injection with Automatic Props Parameter\n');

  // Test 1: Component WITHOUT props parameter (should auto-add props)
  const componentWithoutProps = `
import { component$ } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$(() => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Test 1: Component WITHOUT props parameter');
  console.log('📄 Original component code:');
  console.log(componentWithoutProps);
  console.log('\n🔄 Transforming component...');
  
  try {
    const result1 = await analyzeAndTransformCode(componentWithoutProps, '/test/root.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed component code:');
    console.log(result1);
    console.log('');
    
    if (result1.includes('component$((props) =>') && result1.includes('props.__qwik_analyzer_has_Description')) {
      console.log('✅ AUTO PROPS ADDITION WORKING: Added props parameter and injection!');
    } else if (result1.includes('props.__qwik_analyzer_has_Description')) {
      console.log('✅ PROP INJECTION WORKING but props parameter not added');
    } else {
      console.log('❌ TRANSFORMATION NOT WORKING');
    }
  } catch (error) {
    console.error('❌ Component transformation failed:', error);
  }

  console.log('\n' + '='.repeat(80) + '\n');

  // Test 2: Component WITH props parameter (should only inject prop, not add parameter)
  const componentWithProps = `
import { component$ } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$((props) => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Test 2: Component WITH props parameter');
  console.log('📄 Original component code:');
  console.log(componentWithProps);
  console.log('\n🔄 Transforming component...');
  
  try {
    const result2 = await analyzeAndTransformCode(componentWithProps, '/test/root2.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed component code:');
    console.log(result2);
    console.log('');
    
    if (result2.includes('component$((props) =>') && result2.includes('props.__qwik_analyzer_has_Description')) {
      console.log('✅ EXISTING PROPS WORKING: Used existing props parameter!');
    } else {
      console.log('❌ EXISTING PROPS NOT WORKING');
    }
  } catch (error) {
    console.error('❌ Component transformation failed:', error);
  }

  console.log('\n' + '='.repeat(80) + '\n');

  // Test 3: Component with OTHER parameters (should add props as first param)
  const componentWithOtherParams = `
import { component$ } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$((signal) => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Test 3: Component with OTHER parameters');
  console.log('📄 Original component code:');
  console.log(componentWithOtherParams);
  console.log('\n🔄 Transforming component...');
  
  try {
    const result3 = await analyzeAndTransformCode(componentWithOtherParams, '/test/root3.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed component code:');
    console.log(result3);
    console.log('');
    
    if (result3.includes('component$((props, signal) =>') && result3.includes('props.__qwik_analyzer_has_Description')) {
      console.log('✅ PROPS PREPEND WORKING: Added props as first parameter!');
    } else {
      console.log('❌ PROPS PREPEND NOT WORKING');
    }
  } catch (error) {
    console.error('❌ Component transformation failed:', error);
  }

  console.log('\n' + '='.repeat(80) + '\n');

  // Test 4: Component WITHOUT isComponentPresent (should NOT add props)
  const componentWithoutCall = `
import { component$ } from "@builder.io/qwik";

export const Root = component$(() => {
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Test 4: Component WITHOUT isComponentPresent call');
  console.log('📄 Original component code:');
  console.log(componentWithoutCall);
  console.log('\n🔄 Transforming component...');
  
  try {
    const result4 = await analyzeAndTransformCode(componentWithoutCall, '/test/root4.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed component code:');
    console.log(result4);
    console.log('');
    
    if (result4 === componentWithoutCall) {
      console.log('✅ NO UNNECESSARY TRANSFORMATION: Correctly left component unchanged!');
    } else {
      console.log('❌ UNNECESSARY TRANSFORMATION: Should not have changed component without isComponentPresent');
    }
  } catch (error) {
    console.error('❌ Component transformation failed:', error);
  }
}

testEnhancedPropInjection().catch(console.error); 