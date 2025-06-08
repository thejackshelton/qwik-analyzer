import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { usePresence } from "../../../../src/vite/plugin";
import { Title } from "./title";
import { Checkbox } from "@kunai-consulting/qwik";

export const Root = component$(() => {
	const isDescription = usePresence(Description);
	const isTitle = usePresence(Title);
	const isCheckbox = usePresence(Checkbox.Description);

	return (
		<div>
			<Slot />
			<p>Description present: {isDescription ? "true" : "false"}</p>
			<p>Title present: {isTitle ? "true" : "false"}</p>
			<p>Checkbox description present: {isCheckbox ? "true" : "false"}</p>
		</div>
	);
});
