// compact_short_name_field.tsx - Instructor short-name entry with compact-navigation guidance.

import type { JSX } from "solid-js";

/**
 * Short names stay available above about 16 characters.
 * The guidance asks Instructors to keep them compact when practical.
 */
export function CompactShortNameField(props: {
  readonly label: string;
  readonly name: string;
  readonly value: string;
  readonly maxLength: number;
  readonly helpId: string;
  readonly onInput: (value: string) => void;
  readonly inputRef?: (element: HTMLInputElement) => void;
}): JSX.Element {
  return (
    <label>
      {props.label}
      <input
        ref={props.inputRef}
        name={props.name}
        value={props.value}
        maxlength={props.maxLength}
        autocomplete="off"
        required
        aria-describedby={props.helpId}
        onInput={(event) => props.onInput(event.currentTarget.value)}
      />
      <small id={props.helpId}>For compact navigation; about 16 characters when practical.</small>
    </label>
  );
}
