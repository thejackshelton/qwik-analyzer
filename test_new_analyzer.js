const { analyzeFile } = require('./index.js');

console.log('🚀 Testing new semantic analysis approach...\n');

// Test the root component that has isComponentPresent() call
analyzeFile('./qwik-app/src/components/dummy-comp/root.tsx')
  .then(result => {
    console.log('Root Component Analysis:');
    console.log(JSON.stringify(result, null, 2));
    console.log('');
    
    // Test direct example
    return analyzeFile('./qwik-app/src/examples/direct_example.tsx');
  })
  .then(result => {
    console.log('Direct Example Analysis:');
    console.log(JSON.stringify(result, null, 2));
    console.log('');
    
    // Test aliased example
    return analyzeFile('./qwik-app/src/examples/aliased_example.tsx');
  })
  .then(result => {
    console.log('Aliased Example Analysis:');
    console.log(JSON.stringify(result, null, 2));
  })
  .catch(err => console.error('Error:', err)); 