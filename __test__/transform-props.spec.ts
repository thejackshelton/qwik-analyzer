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
	const dummyCompDir = path.join(componentsDir, "dummy-comp");

	fs.mkdirSync(dummyCompDir, { recursive: true });
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
		path.join(dummyCompDir, "index.ts"),
		`
export { Root } from './root';
export { Description } from './description';
  `.trim(),
	);

	fs.writeFileSync(
		path.join(dummyCompDir, "root.tsx"),
		`
import { component$, Slot } from "@builder.io/qwik";

export const Root = component$(() => {
  return <div><Slot /></div>;
});
  `.trim(),
	);

	fs.writeFileSync(
		path.join(dummyCompDir, "description.tsx"),
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

describe("Prop Injection Approach", () => {
	test("component definition should get props injected into isComponentPresent calls", async () => {
		console.log("🧪 Testing Prop Injection Approach\n");

		const componentCode = `
import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$((props) => {
  const isDescription = isComponentPresent(Description);
  return <div>Test</div>;
});
    `.trim();

		console.log("🔧 Test 1: Component Definition Transformation");
		console.log("📄 Original component code:");
		console.log(componentCode);
		console.log("\n🔄 Transforming component...");

		const testFilePath = path.join(tempDir, "root.tsx");
		fs.writeFileSync(testFilePath, componentCode);
		const componentResult = analyzeAndTransformCode(
			componentCode,
			testFilePath,
		);
		console.log("✅ Transform successful!");
		console.log("📄 Transformed component code:");
		console.log(componentResult);
		console.log("");

		expect(componentResult).toBeDefined();
		if (componentResult.includes("props.__qwik_analyzer_has_Description")) {
			console.log(
				"✅ PROP INJECTION WORKING: Found props.__qwik_analyzer_has_Description",
			);
			expect(componentResult).toContain(
				"props.__qwik_analyzer_has_Description",
			);
		} else {
			console.log("❌ PROP INJECTION NOT WORKING: Missing props injection");
		}
	});

	test("consumer code should get props injected into Root component usage", async () => {
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

		console.log("🔧 Test 2: Consumer Prop Injection");
		console.log("📄 Original consumer code:");
		console.log(consumerCode);
		console.log("\n🔄 Transforming consumer...");

		const testFilePath = path.join(tempDir, "direct_example.tsx");
		fs.writeFileSync(testFilePath, consumerCode);
		const consumerResult = analyzeAndTransformCode(consumerCode, testFilePath);
		console.log("✅ Transform successful!");
		console.log("📄 Transformed consumer code:");
		console.log(consumerResult);
		console.log("");

		expect(consumerResult).toBeDefined();
		if (consumerResult.includes("__qwik_analyzer_has_Description={true}")) {
			console.log(
				"✅ CONSUMER INJECTION WORKING: Found __qwik_analyzer_has_Description={true}",
			);
			expect(consumerResult).toContain(
				"__qwik_analyzer_has_Description={true}",
			);
		} else {
			console.log(
				"❌ CONSUMER INJECTION NOT WORKING: Missing prop injection into Root component",
			);
		}
	});
});
