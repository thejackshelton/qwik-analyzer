/**
 * Integration test for qwik-analyzer NAPI + Vite plugin
 * Tests both the Rust NAPI bindings and the real example files
 */

import { describe, it, expect } from "vitest";
import { analyzeFile, analyzeFileChanged } from "../index.js";
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
];

describe("qwik-analyzer integration tests", () => {
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
				message: expect.stringContaining(
					"Analysis failed: No such file or directory (os error 2)",
				),
			}),
		);
	});
});
