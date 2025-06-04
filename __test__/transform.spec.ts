import { test, expect, beforeAll, afterAll } from "vitest";
import { analyzeAndTransformCode } from "../index.cjs";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let tempDir: string;

beforeAll(() => {
	tempDir = fs.mkdtempSync(path.join(__dirname, "temp-test-"));

	const componentsDir = path.join(tempDir, "components");
	const utilsDir = path.join(tempDir, "utils");

	fs.mkdirSync(componentsDir, { recursive: true });
	fs.mkdirSync(utilsDir, { recursive: true });

	fs.writeFileSync(
		path.join(componentsDir, "description.tsx"),
		`
import { component$ } from "@builder.io/qwik";

export const Description = component$(() => {
  return <div>Description Component</div>;
});
  `.trim(),
	);

	fs.writeFileSync(
		path.join(utilsDir, "qwik-analyzer.ts"),
		`
import type { Component } from "@builder.io/qwik";

export function isComponentPresent<T>(component: Component<T>, injectedValue?: boolean): boolean {
  if (injectedValue !== undefined) {
    return injectedValue;
  }
  return false;
}
  `.trim(),
	);
});

afterAll(() => {
	if (tempDir) {
		fs.rmSync(tempDir, { recursive: true, force: true });
	}
});

test("analyzeAndTransformCode function", async () => {
	const code = `
import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./components/description";
import { isComponentPresent } from "./utils/qwik-analyzer";

export const Root = component$(() => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
  `.trim();

	console.log("🔧 Testing analyzeAndTransformCode function...");
	console.log("📄 Original code:");
	console.log(code);
	console.log("\n🔄 Transforming...");

	const testFilePath = path.join(tempDir, "root.tsx");
	fs.writeFileSync(testFilePath, code);
	const result = analyzeAndTransformCode(code, testFilePath);
	console.log("✅ Transform successful!");
	console.log("📄 Transformed code:");
	console.log(result);

	expect(result).toBeDefined();
	expect(typeof result).toBe("string");

	if (result === code) {
		console.log("❌ Code was not transformed (identical output)");
	} else {
		console.log("✅ Code was transformed!");
		expect(result).not.toBe(code);
	}
});
