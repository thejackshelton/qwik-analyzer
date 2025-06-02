const { analyzeAndTransformCode } = require('./index.js');

async function testPropInjection() {
  console.log('🧪 Testing Prop Injection Approach\n');

  // Test 1: Component definition (should get props injected into isComponentPresent calls)
  const componentCode = `
import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$((props) => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();
  
  console.log('🔧 Test 1: Component Definition Transformation');
  console.log('📄 Original component code:');
  console.log(componentCode);
  console.log('\n🔄 Transforming component...');
  
  try {
    const componentResult = await analyzeAndTransformCode(componentCode, '/test/root.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed component code:');
    console.log(componentResult);
    console.log('');
    
    if (componentResult.includes('props.__qwik_analyzer_has_Description')) {
      console.log('✅ PROP INJECTION WORKING: Found props.__qwik_analyzer_has_Description');
    } else {
      console.log('❌ PROP INJECTION NOT WORKING: Missing props injection');
    }
  } catch (error) {
    console.error('❌ Component transformation failed:', error);
  }

  console.log('\n' + '='.repeat(80) + '\n');

  // Test 2: Consumer code (should get props injected into Root component usage)
  const consumerCode = `
import { component$ } from "@builder.io/qwik";
import { DummyComp } from "./components/dummy-comp";

export default component$(() => {
  return (
    <div>
      <DummyComp.Root>
        <DummyComp.Description>Hello</DummyComp.Description>
      </DummyComp.Root>
    </div>
  );
});
  `.trim();
  
  console.log('🔧 Test 2: Consumer Prop Injection');
  console.log('📄 Original consumer code:');
  console.log(consumerCode);
  console.log('\n🔄 Transforming consumer...');
  
  try {
    const consumerResult = await analyzeAndTransformCode(consumerCode, '/test/direct_example.tsx');
    console.log('✅ Transform successful!');
    console.log('📄 Transformed consumer code:');
    console.log(consumerResult);
    console.log('');
    
    if (consumerResult.includes('__qwik_analyzer_has_Description={true}')) {
      console.log('✅ CONSUMER INJECTION WORKING: Found __qwik_analyzer_has_Description={true}');
    } else {
      console.log('❌ CONSUMER INJECTION NOT WORKING: Missing prop injection into Root component');
    }
  } catch (error) {
    console.error('❌ Consumer transformation failed:', error);
  }
}

testPropInjection().catch(console.error); 