const { analyzeFile } = require('./qwik-analyzer.darwin-arm64.node');

console.log('Testing direct example that contains Description...');

// Test with module specifier filtering
const result = analyzeFile('./qwik-app/src/examples/direct_example.tsx', '../components/dummy-comp');

console.log('Direct Example Result:', JSON.stringify(result, null, 2)); 