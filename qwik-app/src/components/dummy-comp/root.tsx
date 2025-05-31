import { component$, Slot } from "@builder.io/qwik";
import { Description } from "./description";
import { isComponentPresent } from "../../utils/qwik-analyzer";

export const Root = component$(() => {
  const isDescription = isComponentPresent(Description);

  return (  
    <div>
      <Slot />
      <p>Description present: {isDescription ? "true" : "false"}</p>
    </div>
  );
});