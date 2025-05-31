import { component$, useStyles$ } from "@builder.io/qwik";
import { Checkbox } from "@kunai-consulting/qwik";
import { Heyo } from "./heyo";

export default component$(() => {
  
  return (
    <Checkbox.Root>
      <Checkbox.Trigger class="checkbox-trigger">
        <Checkbox.Indicator class="checkbox-indicator">
          Checked
        </Checkbox.Indicator>
      </Checkbox.Trigger>
      <Heyo />
    </Checkbox.Root>
  );
}); 