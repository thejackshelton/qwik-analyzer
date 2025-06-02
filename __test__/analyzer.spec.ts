import { test, expect, describe } from "vitest";
import { analyzeFile } from "../index.js";

describe("New semantic analysis approach", () => {
	test("should analyze root component with isComponentPresent call", async () => {
		console.log("🚀 Testing new semantic analysis approach...\n");

		const result = analyzeFile("./qwik-app/src/components/dummy-comp/root.tsx");
		console.log("Root Component Analysis:");
		console.log(JSON.stringify(result, null, 2));
		console.log("");

		expect(result).toBeDefined();
		expect(result.filePath).toBe(
			"./qwik-app/src/components/dummy-comp/root.tsx",
		);
	});

	test("should analyze direct example", async () => {
		const result = analyzeFile("./qwik-app/src/examples/direct_example.tsx");
		console.log("Direct Example Analysis:");
		console.log(JSON.stringify(result, null, 2));
		console.log("");

		expect(result).toBeDefined();
		expect(result.filePath).toBe("./qwik-app/src/examples/direct_example.tsx");
	});

	test("should analyze aliased example", async () => {
		const result = analyzeFile("./qwik-app/src/examples/aliased_example.tsx");
		console.log("Aliased Example Analysis:");
		console.log(JSON.stringify(result, null, 2));

		expect(result).toBeDefined();
		expect(result.filePath).toBe("./qwik-app/src/examples/aliased_example.tsx");
	});
});
