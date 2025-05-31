import { component$ } from "@builder.io/qwik";
import { DummyComp } from "../components/dummy-comp";

export const Heyo = component$(() => {
  return (
    <div>
      <p>This is the Heyo component, providing a description.</p>
      <DummyComp.Description />
    </div>
  );
}); 