import { component$ } from "@builder.io/qwik";
import type { DocumentHead } from "@builder.io/qwik-city";
import IndirectExample from "../examples/indirect_example";
import DirectExample from "../examples/direct_example";
import AliasExample from "../examples/aliased_example";
import CheckboxExample from "../examples/checkbox";

export default component$(() => {
	return <CheckboxExample />;
});

export const head: DocumentHead = {
	title: "Welcome to Qwik",
	meta: [
		{
			name: "description",
			content: "Qwik site description",
		},
	],
};
