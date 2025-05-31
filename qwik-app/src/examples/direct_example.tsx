import { component$, useStyles$ } from "@builder.io/qwik";
import { Checkbox } from "@kunai-consulting/qwik";
import styles from "./checkbox.css?inline";

export default component$(() => {
  useStyles$(styles);

  return (
    <Checkbox.Root>
      <div
        style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}
      >
        <Checkbox.Trigger class="checkbox-trigger">
          <Checkbox.Indicator class="checkbox-indicator">
            Checked
          </Checkbox.Indicator>
        </Checkbox.Trigger>
        <Checkbox.Label>I accept the Terms and Conditions</Checkbox.Label>
      </div>
      <Checkbox.Description style={{ color: "#b8c1cc" }}>
        By checking this box, you acknowledge that you have read, understood, and agree to
        our Terms of Service and Privacy Policy. This includes consent to process your
        personal data as described in our policies.
      </Checkbox.Description>
    </Checkbox.Root>
  );
}); 