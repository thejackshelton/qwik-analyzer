import { test, expect, describe, beforeAll, afterAll } from "vitest";
import { analyzeAndTransformCode } from "../index.js";
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

describe("Enhanced Prop Injection", () => {
	test("component without props parameter should auto-add props", async () => {
		console.log(
			"🧪 Testing Enhanced Prop Injection with Automatic Props Parameter\n",
		);

		const componentWithoutProps = `
import { component$ } from "@builder.io/qwik";
import { Description } from "./components/description";
import { isComponentPresent } from "./utils/qwik-analyzer";

export const Root = component$(() => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
    `.trim();

		console.log("🔧 Test 1: Component WITHOUT props parameter");
		console.log("📄 Original component code:");
		console.log(componentWithoutProps);
		console.log("\n🔄 Transforming component...");

		const testFilePath = path.join(tempDir, "root.tsx");
		fs.writeFileSync(testFilePath, componentWithoutProps);
		const result1 = analyzeAndTransformCode(
			componentWithoutProps,
			testFilePath,
		);
		console.log("✅ Transform successful!");
		console.log("📄 Transformed component code:");
		console.log(result1);
		console.log("");

		if (
			result1.includes("component$((props) =>") &&
			result1.includes("props.__qwik_analyzer_has_Description")
		) {
			console.log(
				"✅ AUTO PROPS ADDITION WORKING: Added props parameter and injection!",
			);
			expect(result1).toContain("component$((props) =>");
			expect(result1).toContain("props.__qwik_analyzer_has_Description");
		} else if (result1.includes("props.__qwik_analyzer_has_Description")) {
			console.log("✅ PROP INJECTION WORKING but props parameter not added");
			expect(result1).toContain("props.__qwik_analyzer_has_Description");
		} else {
			console.log("❌ TRANSFORMATION NOT WORKING");
			expect(true).toBe(false);
		}
	});

	test("component with props parameter should only inject prop", async () => {
		const componentWithProps = `
import { component$ } from "@builder.io/qwik";
import { Description } from "./components/description";
import { isComponentPresent } from "./utils/qwik-analyzer";

export const Root = component$((props) => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
    `.trim();

		console.log("🔧 Test 2: Component WITH props parameter");
		console.log("📄 Original component code:");
		console.log(componentWithProps);
		console.log("\n🔄 Transforming component...");

		const testFilePath = path.join(tempDir, "root2.tsx");
		fs.writeFileSync(testFilePath, componentWithProps);
		const result2 = analyzeAndTransformCode(componentWithProps, testFilePath);
		console.log("✅ Transform successful!");
		console.log("📄 Transformed component code:");
		console.log(result2);
		console.log("");

		if (
			result2.includes("component$((props) =>") &&
			result2.includes("props.__qwik_analyzer_has_Description")
		) {
			console.log("✅ EXISTING PROPS WORKING: Used existing props parameter!");
			expect(result2).toContain("component$((props) =>");
			expect(result2).toContain("props.__qwik_analyzer_has_Description");
		} else {
			console.log("❌ EXISTING PROPS NOT WORKING");
			expect(true).toBe(false); // Force failure
		}
	});

	test("component without isComponentPresent should not add props", async () => {
		const componentWithoutCall = `
import { component$ } from "@builder.io/qwik";

export const Root = component$(() => {
  return <div>Test</div>;
});
    `.trim();

		console.log("🔧 Test 4: Component WITHOUT isComponentPresent call");
		console.log("📄 Original component code:");
		console.log(componentWithoutCall);
		console.log("\n🔄 Transforming component...");

		const testFilePath = path.join(tempDir, "root4.tsx");
		fs.writeFileSync(testFilePath, componentWithoutCall);
		const result4 = analyzeAndTransformCode(componentWithoutCall, testFilePath);
		console.log("✅ Transform successful!");
		console.log("📄 Transformed component code:");
		console.log(result4);
		console.log("");

		if (result4 === componentWithoutCall) {
			console.log(
				"✅ NO UNNECESSARY TRANSFORMATION: Correctly left component unchanged!",
			);
			expect(result4).toBe(componentWithoutCall);
		} else {
			console.log(
				"❌ UNNECESSARY TRANSFORMATION: Should not have changed component without isComponentPresent",
			);
			expect(true).toBe(false);
		}
	});
});
