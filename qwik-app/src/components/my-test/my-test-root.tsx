import { component$, Slot } from "@builder.io/qwik";
import { usePresence } from "../../../../src/vite/plugin";
import { MyTestChild } from "./my-test-child";

export const MyTestRoot = component$((props) => {
	const isChild = usePresence(MyTestChild);

	console.log("PROPS BRO: ", props);

	return (
		<div>
			<Slot />

			<p>Is child presentddd: {isChild ? "Yes" : "No"}</p>
		</div>
	);
});
