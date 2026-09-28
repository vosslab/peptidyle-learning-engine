// display_mode_toggle.tsx - small explicit Light/Dark switch for the Ribbon.

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { JSX } from "solid-js";

export interface DisplayModeToggleProps {
  readonly mode: DisplayMode;
  readonly disabled: boolean;
  readonly onSwitch: () => void;
}

/** The label names the result of activation, keeping the current state unambiguous. */
export function DisplayModeToggle(props: DisplayModeToggleProps): JSX.Element {
  const nextMode = (): DisplayMode => (props.mode === "light" ? "dark" : "light");
  return (
    <button
      class="ple-app-ribbon__display-mode-toggle"
      type="button"
      aria-label={`Switch to ${nextMode() === "dark" ? "Dark" : "Light"}`}
      title={`Switch to ${nextMode() === "dark" ? "Dark" : "Light"}`}
      disabled={props.disabled}
      onClick={props.onSwitch}
    >
      <span aria-hidden="true">{props.mode === "light" ? "\u25d0" : "\u2600"}</span>
    </button>
  );
}
