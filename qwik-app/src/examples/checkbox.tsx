import { component$ } from "@builder.io/qwik";
import { DummyComp } from "~/components/dummy-comp";
import { Root } from "~/components/dummy-comp/root";
import { Checkbox } from "@kunai-consulting/qwik";

export default component$(() => {
  return (
    <DummyComp.Root>
      <Checkbox.Root>
        <Checkbox.Description />
      </Checkbox.Root>

      <DummyComp.Title />
      
    </DummyComp.Root>
  )
})