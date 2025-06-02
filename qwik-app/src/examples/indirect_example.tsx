import { component$ } from "@builder.io/qwik";
import { DummyComp } from "../components/dummy-comp";
import { Heyo } from "./heyo";

export default component$(() => {
	return (
		<DummyComp.Root>
			<button type="button">Some trigger</button>
			<Heyo />
		</DummyComp.Root>
	);
});
