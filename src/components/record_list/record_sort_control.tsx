import { For, type JSX } from "solid-js";

import "./record_family.css";

export type RecordSortOption<Value extends string> = {
  readonly value: Value;
  readonly label: string;
};

export type RecordSortControlProps<Value extends string> = {
  /** Visible native label for the caller's query-order preference. */
  readonly label: string;
  /** Typed values that the caller understands and applies to its own query or local state. */
  readonly options: ReadonlyArray<RecordSortOption<Value>>;
  /** Caller-owned current value. */
  readonly value: Value;
  readonly disabled?: boolean;
  /** Receives a selected typed value; callers retain paging reset and query policy. */
  readonly onChange: (value: Value) => void;
};

/** A native controlled select for one result-order preference. */
export function RecordSortControl<Value extends string>(
  props: RecordSortControlProps<Value>,
): JSX.Element {
  return (
    <label class="record-sort-control">
      <span class="record-sort-control__label">{props.label}</span>
      <select
        value={props.value}
        disabled={props.disabled}
        onChange={(event): void => props.onChange(event.currentTarget.value as Value)}
      >
        <For each={props.options}>
          {(option) => <option value={option.value}>{option.label}</option>}
        </For>
      </select>
    </label>
  );
}
