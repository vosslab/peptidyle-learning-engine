// One controlled vocabulary selector shared by publication and bulk metadata editing.

import { For, Show, createResource, type JSX } from "solid-js";
import "./content_classification_select.css";

type VocabularyItem = { readonly uuid: string; readonly name: string; readonly isRetired: boolean };

export function ContentClassificationSelect(props: {
  readonly label: string;
  readonly value: string | null;
  readonly parentUuid?: string | null;
  readonly required?: boolean;
  readonly disabled?: boolean;
  /** Discovery filters may intentionally select retired values; authoring cannot. */
  readonly allowRetired?: boolean;
  readonly load: (parentUuid: string) => Promise<ReadonlyArray<VocabularyItem>>;
  readonly onChange: (uuid: string | null) => void;
}): JSX.Element {
  const [items, { refetch }] = createResource(
    () => (props.parentUuid === undefined ? "root" : props.parentUuid || false),
    props.load,
  );
  return (
    <label class="content-classification-select">
      <span>
        {props.label}
        {props.required ? " (required)" : " (optional)"}
      </span>
      <select
        value={props.value ?? ""}
        required={props.required}
        disabled={
          props.disabled || items.loading || Boolean(items.error) || props.parentUuid === null
        }
        onChange={(event) => props.onChange(event.currentTarget.value || null)}
      >
        <option value="">{items.loading ? "Loading..." : `Select ${props.label}`}</option>
        <For each={items.error ? [] : items()}>
          {(item) => (
            // Apply the current UUID when async choices arrive, even if it has not changed.
            <option
              value={item.uuid}
              selected={props.value === item.uuid}
              disabled={item.isRetired && !props.allowRetired && props.value !== item.uuid}
            >
              {item.name}
              {item.isRetired ? " (retired)" : ""}
            </option>
          )}
        </For>
      </select>
      <Show when={Boolean(items.error)}>
        <span role="status">{props.label} choices could not be loaded.</span>
        <button type="button" disabled={props.disabled} onClick={() => void refetch()}>
          Retry {props.label} choices
        </button>
      </Show>
      <Show when={!items.loading && !items.error && items()?.length === 0}>
        <span>No {props.label} choices are available.</span>
      </Show>
    </label>
  );
}
