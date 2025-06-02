import { test, expect } from "vitest";
import { analyzeFile } from "../index.js";

test("direct example analysis", () => {
	console.log("Testing direct example that contains Description...");

	const result = analyzeFile("./qwik-app/src/examples/direct_example.tsx");

	console.log("Direct Example Result:", JSON.stringify(result, null, 2));
	expect(result).toBeDefined();
	expect(result.filePath).toBe("./qwik-app/src/examples/direct_example.tsx");
});
