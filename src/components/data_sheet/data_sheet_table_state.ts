import {
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  type Accessor,
  type Setter,
} from "solid-js";

import {
  recordColumnWidth,
  visibleRecordFields,
  type ColumnChoice,
} from "./data_sheet_column_visibility";
import type { RecordField } from "./data_sheet_fields";
import type { DataSheetMode } from "./data_sheet_state";

type DataSheetTableStateOptions<Row, SortKey extends string, FilterKey extends string> = {
  readonly mode: Accessor<DataSheetMode>;
  readonly fields: Accessor<ReadonlyArray<RecordField<Row, SortKey, FilterKey>>>;
  readonly identityField: Accessor<RecordField<Row, SortKey, FilterKey> | undefined>;
  readonly hasRecordActions: Accessor<boolean>;
};

export type DataSheetTableState<Row, SortKey extends string, FilterKey extends string> = {
  readonly fields: Accessor<ReadonlyArray<RecordField<Row, SortKey, FilterKey>>>;
  readonly columnWidths: Accessor<ReadonlyMap<string, number>>;
  readonly setColumnWidths: (widths: ReadonlyMap<string, number>) => void;
  readonly columnChoices: Accessor<ReadonlyMap<string, ColumnChoice>>;
  readonly orderedFieldIds: Accessor<ReadonlyArray<string>>;
  readonly visibleFields: Accessor<ReadonlyArray<RecordField<Row, SortKey, FilterKey>>>;
  readonly availableWidth: Accessor<number>;
  readonly rootFontSize: Accessor<number>;
  readonly moveColumn: (id: string, targetId: string) => void;
  readonly moveColumnBy: (id: string, direction: -1 | 1) => void;
  readonly chooseColumn: (id: string, choice: ColumnChoice | "auto") => void;
  readonly resetColumns: () => void;
  readonly observeTableWidth: (element: HTMLDivElement) => void;
};

/** One instance's two temporary table configurations, retained across mode changes. */
export function createDataSheetTableState<Row, SortKey extends string, FilterKey extends string>(
  options: DataSheetTableStateOptions<Row, SortKey, FilterKey>,
): DataSheetTableState<Row, SortKey, FilterKey> {
  const fields = createMemo((): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> => {
    if (options.mode() !== "list") return options.fields();
    return [
      ...options.fields().map((field) => ({
        ...field,
        defaultColumn: {
          ...field.defaultColumn,
          width:
            field.role === "identity"
              ? "24rem"
              : field.defaultColumn.visible === true
                ? field.defaultColumn.width
                : "14rem",
          visible: field.role === "identity" || field.defaultColumn.visible === true,
        },
      })),
      {
        id: "record_details",
        label: "Details",
        value: () => null,
        defaultColumn: { priority: 0, width: "20rem", visible: true },
      },
    ];
  });
  const identityWidth = (): number =>
    options.mode() === "list"
      ? 24 * rootFontSize()
      : recordColumnWidth(options.identityField()?.defaultColumn.width, rootFontSize());
  function createConfiguration(): {
    columnWidths: Accessor<ReadonlyMap<string, number>>;
    setColumnWidths: Setter<ReadonlyMap<string, number>>;
    columnChoices: Accessor<ReadonlyMap<string, ColumnChoice>>;
    setColumnChoices: Setter<ReadonlyMap<string, ColumnChoice>>;
    columnOrder: Accessor<ReadonlyArray<string>>;
    setColumnOrder: Setter<ReadonlyArray<string>>;
  } {
    const [columnWidths, setColumnWidths] = createSignal<ReadonlyMap<string, number>>(new Map());
    const [columnChoices, setColumnChoices] = createSignal<ReadonlyMap<string, ColumnChoice>>(
      new Map(),
    );
    const [columnOrder, setColumnOrder] = createSignal<ReadonlyArray<string>>([]);
    return {
      columnWidths,
      setColumnWidths,
      columnChoices,
      setColumnChoices,
      columnOrder,
      setColumnOrder,
    };
  }
  const compact = createConfiguration();
  const list = createConfiguration();
  const active = (): ReturnType<typeof createConfiguration> =>
    options.mode() === "list" ? list : compact;
  const columnWidths = (): ReadonlyMap<string, number> => active().columnWidths();
  const columnChoices = (): ReadonlyMap<string, ColumnChoice> => active().columnChoices();
  const columnOrder = (): ReadonlyArray<string> => active().columnOrder();
  const setColumnWidths = (widths: ReadonlyMap<string, number>): void => {
    active().setColumnWidths(widths);
  };
  const setColumnChoices = (choices: ReadonlyMap<string, ColumnChoice>): void => {
    active().setColumnChoices(choices);
  };
  const setColumnOrder: Setter<ReadonlyArray<string>> = (value) => active().setColumnOrder(value);
  const [availableWidth, setAvailableWidth] = createSignal(0);
  const [rootFontSize, setRootFontSize] = createSignal(16);
  let widthObserver: ResizeObserver | undefined;

  createEffect(() => {
    const available = new Set([...options.fields().map((field) => field.id), "record_details"]);
    for (const configuration of [compact, list]) {
      configuration.setColumnOrder((current) => {
        const retained = current.filter((id) => available.has(id));
        return retained.length === current.length ? current : retained;
      });
    }
  });
  onCleanup(() => widthObserver?.disconnect());

  const orderedFieldIds = createMemo((): ReadonlyArray<string> => {
    const declared = fields().map((field) => field.id);
    const available = new Set(declared);
    const retained = columnOrder().filter((id) => available.has(id));
    const retainedIds = new Set(retained);
    return [...retained, ...declared.filter((id) => !retainedIds.has(id))];
  });
  const visibleFields = createMemo(() =>
    visibleRecordFields(
      fields(),
      columnChoices(),
      availableWidth(),
      rootFontSize(),
      options.hasRecordActions(),
      identityWidth(),
    ),
  );

  function moveColumn(id: string, targetId: string): void {
    if (id === targetId) return;
    const next = [...orderedFieldIds()];
    const from = next.indexOf(id);
    const target = next.indexOf(targetId);
    if (from < 0 || target < 0) return;
    next.splice(from, 1);
    next.splice(target, 0, id);
    setColumnOrder(next);
  }
  function moveColumnBy(id: string, direction: -1 | 1): void {
    const next = orderedFieldIds();
    const neighbor = next[next.indexOf(id) + direction];
    if (neighbor !== undefined) moveColumn(id, neighbor);
  }
  function chooseColumn(id: string, choice: ColumnChoice | "auto"): void {
    const next = new Map(columnChoices());
    if (choice === "auto") next.delete(id);
    else next.set(id, choice);
    setColumnChoices(next);
  }
  function resetColumns(): void {
    setColumnChoices(new Map());
    setColumnWidths(new Map());
    setColumnOrder([]);
  }
  function observeTableWidth(element: HTMLDivElement): void {
    widthObserver?.disconnect();
    const readFontSize = (): number =>
      Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    setRootFontSize(readFontSize());
    widthObserver = new ResizeObserver(([entry]) => {
      if (entry === undefined) return;
      setRootFontSize(readFontSize());
      // Hidden tables retain their last usable layout and the controls it renders.
      if (entry.contentRect.width > 0) setAvailableWidth(entry.contentRect.width);
    });
    widthObserver.observe(element);
  }
  return {
    fields,
    columnWidths,
    setColumnWidths,
    columnChoices,
    orderedFieldIds,
    visibleFields,
    availableWidth,
    rootFontSize,
    moveColumn,
    moveColumnBy,
    chooseColumn,
    resetColumns,
    observeTableWidth,
  };
}
