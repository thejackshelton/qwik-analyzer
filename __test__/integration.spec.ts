/**
 * Integration test for qwik-analyzer NAPI + Vite plugin
 * Tests both the Rust NAPI bindings and the real example files
 */

import { describe, it, expect } from "vitest";
import { analyzeFile, analyzeFileChanged } from "../index.cjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface TestCase {
	name: string;
	file: string;
	expectedHasComponent: boolean;
	description: string;
	moduleSpecifier?: string;
}

const testCases: TestCase[] = [
	{
		name: "Direct Example",
		file: "../qwik-app/src/examples/direct_example.tsx",
		expectedHasComponent: true,
		description:
			"Should detect DummyComp.Description directly within DummyComp.Root",
		moduleSpecifier: "../components/dummy-comp",
	},
	{
		name: "Indirect Example",
		file: "../qwik-app/src/examples/indirect_example.tsx",
		expectedHasComponent: true,
		description:
			"Should detect DummyComp.Description via imported Heyo component with recursive analysis",
		moduleSpecifier: "../components/dummy-comp",
	},
	{
		name: "Heyo Component",
		file: "../qwik-app/src/examples/heyo.tsx",
		expectedHasComponent: false,
		description:
			"Should return false - contains DummyComp.Description but not within DummyComp.Root",
		moduleSpecifier: "../components/dummy-comp",
	},
	{
		name: "Aliased Example",
		file: "../qwik-app/src/examples/aliased_example.tsx",
		expectedHasComponent: true,
		description:
			"Should detect aliased DummyComp.Description within DummyComp.Root",
		moduleSpecifier: "../components/dummy-comp",
	},
	{
		name: "Checkbox Example",
		file: "../qwik-app/src/examples/checkbox.tsx",
		expectedHasComponent: true,
		description:
			"Should detect DummyComp.Description (contains Checkbox.Description which matches)",
		moduleSpecifier: "../components/dummy-comp",
	},
	{
		name: "Slot Example",
		file: "../qwik-app/src/examples/slot_example.tsx",
		expectedHasComponent: true,
		description:
			"Should return true - MyTest.Root contains isComponentPresent call for MyTestChild",
		moduleSpecifier: "../components/my-test",
	},
];

describe("qwik-analyzer integration tests", () => {
	it("Slot Example - MyTest.Child detection: Should return true - MyTest.Root contains isComponentPresent call for MyTestChild", async () => {
		const filePath = path.resolve(__dirname, "../qwik-app/src/examples/slot_example.tsx");

		expect(fs.existsSync(filePath)).toBe(true);

		const result = await analyzeFile(filePath, "../components/my-test", "MyTestChild");
		expect(result.hasComponent).toBe(true);
	});
	for (const testCase of testCases) {
		it(`${testCase.name}: ${testCase.description}`, async () => {
			const filePath = path.resolve(__dirname, testCase.file);

			expect(fs.existsSync(filePath)).toBe(true);

			const result = await analyzeFile(filePath);
			expect(result.hasComponent).toBe(testCase.expectedHasComponent);
		});

		it(`${testCase.name}: should handle file change events`, () => {
			const filePath = path.resolve(__dirname, testCase.file);

			expect(fs.existsSync(filePath)).toBe(true);

			expect(() => analyzeFileChanged(filePath, "update")).not.toThrow();
		});
	}

	it("should handle non-existent files gracefully", async () => {
		expect(() => analyzeFile("/nonexistent/file.tsx")).toThrow(
			expect.objectContaining({
				message: expect.stringMatching(
					/Analysis failed: (No such file or directory \(os error 2\)|The system cannot find the path specified\. \(os error 3\))/,
				),
			}),
		);
	});
});
