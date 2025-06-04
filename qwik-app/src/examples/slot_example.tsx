import { component$ } from "@builder.io/qwik";
import type { DocumentHead } from "@builder.io/qwik-city";
import { MyTest } from "../components/my-test";

export default component$(() => {
	return (
		<>
			<MyTest.Root>
				<MyTest.Child />
			</MyTest.Root>
		</>
	);
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
