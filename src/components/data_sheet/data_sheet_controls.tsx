import { For, Show, createEffect, type Accessor, type JSX } from "solid-js";

import type {
  DataSheetMode,
  DataSheetPresentation,
  DataSheetSelection,
  DataSheetSelectionToolbar,
} from "./data_sheet_state";
import { DataSheetColumnControls } from "./data_sheet_table";
import type { DataSheetTableState } from "./data_sheet_table_state";
import type { RecordAction } from "./record_content";
import { RecordActionControl } from "./record_action_control";
import type { RecordField, RecordListQuery, RecordSortBinding } from "./record_fields";
import {
  RecordPageControls,
  RecordPageSizeControl,
  type RecordPageControlsProps,
} from "./record_page_controls";
import { RecordFilterControls } from "./record_filter_controls";
import { RecordSortMenu, recordSortOptions } from "./record_sort_controls";

const modes: ReadonlyArray<{ readonly id: DataSheetMode; readonly label: string }> = [
  { id: "compact", label: "Compact" },
  { id: "list", label: "List" },
  { id: "cards", label: "Cards" },
  { id: "visual_boxes", label: "Visual Boxes" },
];

function DataSheetModeControls(props: {
  readonly mode: Accessor<DataSheetMode>;
  readonly onChange: (mode: DataSheetMode) => void;
}): JSX.Element {
  return (
    <div class="record-list__mode-controls" role="group" aria-label="Record display">
      <For each={modes}>
        {(option) => (
          <button
            class="quiet-action"
            type="button"
            aria-pressed={props.mode() === option.id}
            onClick={() => props.onChange(option.id)}
          >
            {option.label}
          </button>
        )}
      </For>
    </div>
  );
}

function DataSheetSelectionControls(props: {
  readonly loadedCount: number;
  readonly selectedCount: Accessor<number>;
  readonly toolbar: DataSheetSelectionToolbar;
}): JSX.Element {
  const actions = (): ReadonlyArray<RecordAction> => props.toolbar.actions?.() ?? [];
  const disabled = (): boolean => props.toolbar.disabled?.() === true;
  return (
    <Show when={props.loadedCount > 0 || props.selectedCount() > 0}>
      <details class="record-list__selection-disclosure" aria-label="Selected search results">
        <summary aria-live="polite">Selection ({props.selectedCount()})</summary>
        <div class="record-list__selection-controls">
          <p aria-live="polite">
            <strong>{props.selectedCount()} selected</strong> from {props.loadedCount} loaded
            results
          </p>
          <Show when={props.toolbar.onSelectLoaded}>
            {(selectLoaded) => (
              <button
                type="button"
                class="quiet-action"
                disabled={disabled()}
                onClick={() => selectLoaded()()}
              >
                Select loaded results
              </button>
            )}
          </Show>
          <Show when={props.toolbar.onClearSelection}>
            {(clearSelection) => (
              <button
                type="button"
                class="quiet-action"
                disabled={disabled() || props.selectedCount() === 0}
                onClick={() => clearSelection()()}
              >
                Clear selection
              </button>
            )}
          </Show>
          <For each={actions()}>
            {(action) => <RecordActionControl action={() => action} toolbar disabled={disabled} />}
          </For>
        </div>
      </details>
    </Show>
  );
}

/** Surrounding DataSheet controls. Query changes remain callbacks to the owning query layer. */
export function DataSheetControls<Row, SortKey extends string, FilterKey extends string>(props: {
  readonly presentation: DataSheetPresentation<FilterKey>;
  readonly table: DataSheetTableState<Row, SortKey, FilterKey>;
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly query?: RecordListQuery<SortKey, FilterKey>;
  readonly selection?: DataSheetSelection<Row>;
  readonly loadedCount: number;
  readonly pagination?: RecordPageControlsProps;
}): JSX.Element {
  let options!: HTMLDetailsElement;
  // Header filters use the same picker even when surrounding options are collapsed.
  createEffect(() => {
    if (props.presentation.openFilterKey() !== null) options.open = true;
  });
  const tabular = (): boolean =>
    props.presentation.mode() === "compact" || props.presentation.mode() === "list";
  const filterCount = (): number =>
    [...(props.query?.filters?.values.values() ?? [])].reduce(
      (count, values) => count + values.length,
      0,
    );
  const headingSortKeys = (): ReadonlySet<SortKey> =>
    new Set(
      [
        ...props.fields.filter((field) => field.role === "identity"),
        ...props.table.visibleFields(),
      ].flatMap((field) => (field.sortKey === undefined ? [] : [field.sortKey])),
    );
  const otherSortFields = (): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> =>
    props.fields.filter(
      (field) => field.sortKey !== undefined && !headingSortKeys().has(field.sortKey),
    );
  const otherSortBinding = (): RecordSortBinding<SortKey> | undefined => {
    const binding = props.query?.sort;
    if (binding === undefined) return undefined;
    return {
      ...binding,
      choices: binding.choices?.filter((choice) => !headingSortKeys().has(choice.sort.key)),
    };
  };
  const otherSortSummary = (): string => {
    const binding = otherSortBinding();
    const selected = binding?.value;
    if (binding === undefined || selected === null || selected === undefined) return "Sort";
    return (
      recordSortOptions(otherSortFields(), binding).find(
        (option) =>
          option.sort.key === selected.key && option.sort.direction === selected.direction,
      )?.label ?? "Sort"
    );
  };
  return (
    <div class="data-sheet-controls">
      <DataSheetModeControls mode={props.presentation.mode} onChange={props.presentation.setMode} />
      <div class="data-sheet-controls__secondary">
        <details
          class="data-sheet-controls__options"
          ref={(element) => {
            options = element;
          }}
        >
          <summary>Options{filterCount() ? ` (${filterCount()} filters)` : ""}</summary>
          <div class="data-sheet-controls__options-body">
            <Show when={props.pagination}>
              {(binding) => <RecordPageSizeControl {...binding()} />}
            </Show>
            <Show when={tabular()}>
              <DataSheetColumnControls table={props.table} />
            </Show>
            <Show when={props.query?.sort}>
              {(binding) => (
                <Show
                  when={tabular()}
                  fallback={<RecordSortMenu fields={props.fields} binding={binding()} />}
                >
                  <Show when={recordSortOptions(otherSortFields(), otherSortBinding()!).length > 0}>
                    <details class="record-sort__disclosure">
                      <summary>{otherSortSummary()}</summary>
                      <RecordSortMenu fields={otherSortFields()} binding={otherSortBinding()!} />
                    </details>
                  </Show>
                </Show>
              )}
            </Show>
            <Show when={props.query?.filters}>
              {(binding) => (
                <RecordFilterControls
                  fields={props.fields}
                  binding={binding()}
                  compact
                  openKey={props.presentation.openFilterKey()}
                  onOpen={props.presentation.openFilter}
                  onClose={props.presentation.closeFilter}
                />
              )}
            </Show>
          </div>
        </details>
        <Show when={props.selection?.kind === "checkbox" && props.selection.toolbar}>
          <DataSheetSelectionControls
            loadedCount={props.loadedCount}
            selectedCount={() => props.selection!.selectedIds().size}
            toolbar={props.selection!.toolbar!}
          />
        </Show>
      </div>
      <Show when={props.pagination}>{(binding) => <RecordPageControls {...binding()} />}</Show>
    </div>
  );
}
