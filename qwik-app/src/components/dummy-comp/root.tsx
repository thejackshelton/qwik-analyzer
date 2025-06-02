import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "qwik-analyzer";
import { Title } from "./title";
import { Checkbox } from "@kunai-consulting/qwik"

export const Root = component$(() => {
	const isDescription = isComponentPresent(Description);
	const isTitle = isComponentPresent(Title);
	const isCheckbox = isComponentPresent(Checkbox.Description);

	return (
		<div>
			<Slot />
			<p>Description present: {isDescription ? "true" : "false"}</p>
			<p>Title present: {isTitle ? "true" : "false"}</p>
			<p>Checkbox present: {isCheckbox ? "true" : "false"}</p>
		</div>
	);
});
