import { For, type JSX } from "solid-js";

import { selectChange } from "../controlled_form_change";
import type { RecordField, RecordSort, RecordSortBinding } from "./data_sheet_fields";
import "./data_sheet_sort_controls.css";

function sameSort<SortKey extends string>(
  left: RecordSort<SortKey> | null,
  right: RecordSort<SortKey>,
): boolean {
  return left?.key === right.key && left.direction === right.direction;
}

export function DataSheetSortHeading<SortKey extends string>(props: {
  readonly label: string;
  readonly sortKey: SortKey;
  readonly binding: RecordSortBinding<SortKey>;
}): JSX.Element {
  const direction = (): "ascending" | "descending" | null =>
    props.binding.value?.key === props.sortKey ? props.binding.value.direction : null;
  return (
    <button
      type="button"
      class="record-sort__heading"
      aria-label={`Sort by ${props.label}${direction() ? `, ${direction()}` : ""}`}
      onClick={() =>
        props.binding.onChange({
          key: props.sortKey,
          direction: direction() === "ascending" ? "descending" : "ascending",
        })
      }
      onDragStart={(event) => event.preventDefault()}
    >
      {props.label}
      <span class="record-sort__direction" aria-hidden="true">
        {direction() === "ascending" ? "\u2191" : direction() === "descending" ? "\u2193" : ""}
      </span>
    </button>
  );
}

/** Field orders and extra query orders share their labels across the menu and compact summary. */
export function recordSortOptions<Row, SortKey extends string, FilterKey extends string>(
  fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>,
  binding: RecordSortBinding<SortKey>,
): ReadonlyArray<{ readonly label: string; readonly sort: RecordSort<SortKey> }> {
  const choices: Array<{ readonly label: string; readonly sort: RecordSort<SortKey> }> = [];
  const seen = new Set<string>();
  for (const field of fields) {
    if (field.sortKey === undefined || seen.has(field.sortKey)) continue;
    seen.add(field.sortKey);
    choices.push(
      {
        label: `${field.label} (ascending)`,
        sort: { key: field.sortKey, direction: "ascending" },
      },
      {
        label: `${field.label} (descending)`,
        sort: { key: field.sortKey, direction: "descending" },
      },
    );
  }
  for (const choice of binding.choices ?? []) {
    if (!choices.some((entry) => sameSort(entry.sort, choice.sort))) choices.push(choice);
  }
  return choices;
}

/** The menu exposes the same query orders when column headings are not present. */
export function DataSheetSortMenu<Row, SortKey extends string, FilterKey extends string>(props: {
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly binding: RecordSortBinding<SortKey>;
}): JSX.Element {
  const options = (): ReadonlyArray<{
    readonly label: string;
    readonly sort: RecordSort<SortKey>;
  }> => recordSortOptions(props.fields, props.binding);
  const selected = (): string => {
    const index = options().findIndex((option) => sameSort(props.binding.value, option.sort));
    return index < 0 ? "" : String(index);
  };

  return (
    <label class="record-sort__menu">
      <span>Order results</span>
      <select
        value={selected()}
        onChange={(event) =>
          selectChange(event.currentTarget, selected(), (requested) => {
            const option = options()[Number(requested)];
            if (option) props.binding.onChange(option.sort);
          })
        }
      >
        <option value="" disabled>
          Choose order
        </option>
        <For each={options()}>
          {(option, index) => <option value={index()}>{option.label}</option>}
        </For>
      </select>
    </label>
  );
}
