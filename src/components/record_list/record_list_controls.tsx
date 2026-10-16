import { For, Show, type Accessor, type JSX } from "solid-js";

import type { RecordAction } from "./record_list";
import { RecordActionControl } from "./record_action_control";

export type RecordListMode = "compact" | "list" | "poster";

const modes: ReadonlyArray<{ readonly id: RecordListMode; readonly label: string }> = [
  { id: "compact", label: "Compact" },
  { id: "list", label: "List" },
  { id: "poster", label: "Visual boxes" },
];

export function RecordListModeControls(props: {
  readonly mode: Accessor<RecordListMode>;
  readonly onChange: (mode: RecordListMode) => void;
}): JSX.Element {
  return (
    <div class="record-list__mode-controls" role="group" aria-label="Record display">
      <span>Display</span>
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

export type RecordListSelectionToolbar = {
  readonly disabled?: Accessor<boolean>;
  readonly onSelectLoaded?: () => void;
  readonly onClearSelection?: () => void;
  readonly actions?: Accessor<ReadonlyArray<RecordAction>>;
};

export function RecordListSelectionControls(props: {
  readonly loadedCount: number;
  readonly selectedCount: Accessor<number>;
  readonly toolbar: RecordListSelectionToolbar;
}): JSX.Element {
  const actions = (): ReadonlyArray<RecordAction> => props.toolbar.actions?.() ?? [];
  const disabled = (): boolean => props.toolbar.disabled?.() === true;
  return (
    <Show when={props.loadedCount > 0 || props.selectedCount() > 0}>
      <section class="record-list__selection-controls" aria-label="Selected search results">
        <p aria-live="polite">
          <strong>{props.selectedCount()} selected</strong> from {props.loadedCount} loaded results
        </p>
        <div>
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
      </section>
    </Show>
  );
}
