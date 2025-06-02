import { component$ } from "@builder.io/qwik";
import { DummyComp as MyComp } from "../components/dummy-comp";

export default component$(() => {
	return (
		<MyComp.Root>
			<button type="button">Some trigger</button>
			<MyComp.Description />
		</MyComp.Root>
	);
});
