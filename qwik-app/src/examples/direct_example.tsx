import { component$ } from "@builder.io/qwik";
import { DummyComp } from "../components/dummy-comp";

export default component$(() => {
	return (
		<DummyComp.Root>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: "8px",
					marginBottom: "8px",
				}}
			>
				<button type="button">Some trigger</button>
				<label>I accept the Terms and Conditions</label>
			</div>
			<DummyComp.Description />
			<DummyComp.Title />
		</DummyComp.Root>
	);
});
