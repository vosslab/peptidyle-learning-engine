import { createSignal, type Accessor, type Setter } from "solid-js";

import type { RecordAction } from "./record_content";

export type DataSheetMode = "compact" | "list" | "cards" | "visual_boxes";

export type DataSheetSelectionToolbar = {
  readonly disabled?: Accessor<boolean>;
  readonly onSelectLoaded?: () => void;
  readonly onClearSelection?: () => void;
  readonly actions?: Accessor<ReadonlyArray<RecordAction>>;
};

/** Caller-owned selection identity and commands; DataSheet never owns selected records. */
export type DataSheetSelection<Row> = {
  readonly kind: "checkbox" | "radio";
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly disabled?: (row: Row) => boolean;
  readonly onChange: (row: Row, selected: boolean) => void;
  readonly toolbar?: DataSheetSelectionToolbar;
};

export type DataSheetPresentation<FilterKey extends string> = {
  readonly mode: Accessor<DataSheetMode>;
  readonly setMode: Setter<DataSheetMode>;
  readonly openFilterKey: Accessor<FilterKey | null>;
  readonly openFilter: (key: FilterKey, trigger: HTMLButtonElement) => void;
  readonly closeFilter: () => void;
};

/** Collection presentation only; DataSheetTable owns column configuration. */
export function createDataSheetPresentation<FilterKey extends string>(
  initialMode: DataSheetMode = "compact",
): DataSheetPresentation<FilterKey> {
  const [mode, setMode] = createSignal<DataSheetMode>(initialMode);
  const [openFilterKey, setOpenFilterKey] = createSignal<FilterKey | null>(null);
  let filterTrigger: HTMLButtonElement | undefined;
  function openFilter(key: FilterKey, trigger: HTMLButtonElement): void {
    filterTrigger = trigger;
    setOpenFilterKey(() => key);
  }
  function closeFilter(): void {
    setOpenFilterKey(null);
    if (filterTrigger?.isConnected) filterTrigger.focus();
    filterTrigger = undefined;
  }
  return { mode, setMode, openFilterKey, openFilter, closeFilter };
}
