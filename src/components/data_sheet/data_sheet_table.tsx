import { For, Show, createMemo, type Accessor, type JSX } from "solid-js";

import {
  defaultRecordActionsWidth,
  recordColumnWidth,
  visibleTableWidth,
} from "./record_column_visibility";
import type { RecordField, RecordListQuery } from "./record_fields";
import { RecordFilterButton } from "./record_filter_controls";
import { RecordSortHeading } from "./record_sort_controls";
import type { DataSheetPresentation, DataSheetSelection } from "./data_sheet_state";
import type { DataSheetTableState } from "./data_sheet_table_state";
import { CompactRecordActions, fieldText, RecordItem } from "./record_item";
import { RecordDetails, type RecordContent } from "./record_content";
import {
  RecordCollectionEmptyNotice,
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import { useRecordInteractions } from "./record_interactions";
import { HeaderRow } from "./header_row";

export type DataSheetTableColumn<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly resizeLabel?: string;
  readonly minWidth?: number;
  readonly cell: (row: Row) => JSX.Element;
  readonly width?: string;
  readonly align?: "start" | "center" | "end";
  readonly sortDirection?: () => "ascending" | "descending" | undefined;
};

export type DataSheetTableRowHeader<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly resizeLabel?: string;
  readonly minWidth?: number;
  readonly content: (row: Row) => JSX.Element;
  readonly width?: string;
  readonly sortDirection?: () => "ascending" | "descending" | undefined;
};

type TableViewProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly columns: ReadonlyArray<DataSheetTableColumn<Row>>;
  readonly rowId: (row: Row) => string;
  readonly rowHeader: DataSheetTableRowHeader<Row>;
  /** Include the row-header ID once; omitted for the usual first-column layout. */
  readonly orderedColumnIds?: ReadonlyArray<string>;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly columnWidths?: ReadonlyMap<string, number>;
  readonly onColumnWidthsChange?: (widths: ReadonlyMap<string, number>) => void;
  readonly onColumnMove?: (id: string, targetId: string) => void;
  readonly movableColumnIds?: ReadonlyArray<string>;
  readonly minimumWidth?: number;
};

/** Semantic records with named columns and one row header per record. */
function TableView<Row>(props: TableViewProps<Row>): JSX.Element {
  const interactions = useRecordInteractions();
  let tableElement!: HTMLTableElement;
  let draggedColumnId: string | undefined;
  const columnIds = (): ReadonlyArray<string> =>
    props.orderedColumnIds ?? [props.rowHeader.id, ...props.columns.map((column) => column.id)];
  const column = (id: string): DataSheetTableColumn<Row> | undefined =>
    props.columns.find((entry) => entry.id === id);
  const canMove = (id: string): boolean =>
    props.onColumnMove !== undefined &&
    id !== "record_actions" &&
    (props.movableColumnIds === undefined || props.movableColumnIds.includes(id));

  function measureWidths(): Map<string, number> {
    const widths = new Map<string, number>();
    for (const header of tableElement.querySelectorAll<HTMLTableCellElement>(
      "thead th[data-record-column-id]",
    )) {
      const id = header.dataset.recordColumnId;
      if (id) widths.set(id, header.offsetWidth);
    }
    return widths;
  }

  function startResize(id: string): number {
    const widths = new Map(props.columnWidths);
    for (const [visibleId, width] of measureWidths()) {
      if (!widths.has(visibleId)) widths.set(visibleId, width);
    }
    props.onColumnWidthsChange?.(widths);
    return widths.get(id) ?? 0;
  }

  function resize(id: string, minimum: number, width: number): void {
    const widths = new Map(props.columnWidths);
    widths.set(id, Math.max(minimum, Math.round(width)));
    props.onColumnWidthsChange?.(widths);
  }

  function tableWidth(): string | undefined {
    if (!props.columnWidths?.size) return undefined;
    const measured = columnIds().map((id) => props.columnWidths?.get(id));
    if (measured.some((width) => width === undefined)) return undefined;
    return `${measured.reduce<number>((sum, width) => sum + (width ?? 0), 0)}px`;
  }

  function startColumnDrag(event: DragEvent, id: string): void {
    if (!canMove(id)) return;
    if (
      event.target instanceof Element &&
      event.target.closest("button, input, select, textarea")
    ) {
      event.preventDefault();
      return;
    }
    draggedColumnId = id;
    event.dataTransfer?.setData("text/plain", id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function allowColumnDrop(event: DragEvent, id: string): void {
    if (!draggedColumnId || !canMove(id) || draggedColumnId === id) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  function dropColumn(event: DragEvent, id: string): void {
    if (!draggedColumnId || !canMove(id)) return;
    event.preventDefault();
    props.onColumnMove?.(draggedColumnId, id);
    draggedColumnId = undefined;
  }

  return (
    <RecordCollectionStateView state={props.state} isEmpty={false} emptyState={props.emptyState}>
      <div
        class="record-table__scroll"
        onFocusIn={(event) => {
          if (event.target instanceof HTMLElement) {
            event.target.scrollIntoView({ block: "nearest", inline: "nearest" });
          }
        }}
      >
        <table
          ref={(element) => {
            tableElement = element;
          }}
          class="record-table"
          aria-label={props.ariaLabel}
          style={{
            width: tableWidth(),
            "min-inline-size": tableWidth()
              ? "0"
              : props.minimumWidth
                ? `max(100%, ${props.minimumWidth}px)`
                : undefined,
          }}
        >
          <colgroup>
            <For each={columnIds()}>
              {(id) => (
                <col
                  style={{
                    width:
                      props.columnWidths?.get(id) === undefined
                        ? undefined
                        : `${props.columnWidths?.get(id)}px`,
                  }}
                />
              )}
            </For>
          </colgroup>
          <HeaderRow
            columnIds={columnIds}
            rowHeader={props.rowHeader}
            column={column}
            canMove={canMove}
            onStartDrag={startColumnDrag}
            onDragOver={allowColumnDrop}
            onDrop={dropColumn}
            onDragEnd={() => {
              draggedColumnId = undefined;
            }}
            columnWidths={props.columnWidths}
            canResize={props.onColumnWidthsChange !== undefined}
            onResizeStart={startResize}
            onResize={resize}
          />
          <tbody>
            <Show when={props.rows.length === 0 && props.state.kind === "ready"}>
              <tr class="record-table__empty-row">
                <td colspan={columnIds().length}>
                  <RecordCollectionEmptyNotice emptyState={props.emptyState} />
                </td>
              </tr>
            </Show>
            <For each={props.rows}>
              {(row) => (
                <tr
                  data-record-id={props.rowId(row)}
                  classList={{
                    "record-list__row--transfer-target": interactions?.canDrop(props.rowId(row)),
                  }}
                  onDragOver={(event) => interactions?.allowDrop(event, props.rowId(row))}
                  onDrop={(event) => interactions?.completeDrop(event, props.rowId(row))}
                >
                  <For each={columnIds()}>
                    {(id) => (
                      <Show
                        when={id === props.rowHeader.id}
                        fallback={
                          <td
                            data-record-column-id={id}
                            style={{ "text-align": column(id)?.align ?? "start" }}
                          >
                            {column(id)?.cell(row)}
                          </td>
                        }
                      >
                        <th scope="row" data-record-column-id={id}>
                          {props.rowHeader.content(row)}
                        </th>
                      </Show>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </RecordCollectionStateView>
  );
}

export type DataSheetTableProps<Row, SortKey extends string, FilterKey extends string> = {
  readonly recordIds: ReadonlyArray<string>;
  readonly row: (id: string) => Row;
  readonly content: (row: Row) => RecordContent;
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
  readonly selection?: DataSheetSelection<Row>;
  readonly selectionName: string;
  readonly identityField?: RecordField<Row, SortKey, FilterKey>;
  readonly hasRecordActions: boolean;
  readonly query?: RecordListQuery<SortKey, FilterKey>;
  readonly presentation: DataSheetPresentation<FilterKey>;
  readonly table: DataSheetTableState<Row, SortKey, FilterKey>;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
};

/** Surrounding column UI invokes the same temporary state as DataSheetTable. */
export function DataSheetColumnControls<
  Row,
  SortKey extends string,
  FilterKey extends string,
>(props: { readonly table: DataSheetTableState<Row, SortKey, FilterKey> }): JSX.Element {
  // ASVS 1.2.1: Solid renders column labels as text, preserving the control structure.
  const fields = (): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> => props.table.fields();
  let columnControlsBody: HTMLDivElement | undefined;
  function moveColumnFromControl(id: string, direction: -1 | 1, trigger: HTMLButtonElement): void {
    props.table.moveColumnBy(id, direction);
    queueMicrotask(() => {
      if (trigger.isConnected && !trigger.disabled) {
        trigger.focus();
        return;
      }
      const group = Array.from(
        columnControlsBody?.querySelectorAll<HTMLDivElement>("[data-record-column-id]") ?? [],
      ).find((element) => element.dataset.recordColumnId === id);
      const preferred = group?.querySelector<HTMLButtonElement>(
        `[data-record-list-column-direction="${direction}"]:not(:disabled)`,
      );
      const fallback = group?.querySelector<HTMLButtonElement>(
        "[data-record-list-column-direction]:not(:disabled)",
      );
      (preferred ?? fallback)?.focus();
    });
  }
  return (
    <Show when={fields().length > 0}>
      <details class="record-list__column-controls">
        <summary>Columns</summary>
        <div
          class="record-list__column-controls-body"
          ref={(element) => (columnControlsBody = element)}
        >
          <For each={props.table.orderedFieldIds()}>
            {(id) => {
              const field = (): RecordField<Row, SortKey, FilterKey> | undefined =>
                fields().find((entry) => entry.id === id);
              return (
                <div class="record-list__column-move" data-record-column-id={id}>
                  <span>{field()?.label}</span>
                  <button
                    type="button"
                    aria-label={`Move ${field()?.label} column left`}
                    data-record-list-column-direction={-1}
                    disabled={props.table.orderedFieldIds().indexOf(id) === 0}
                    onClick={(event) => moveColumnFromControl(id, -1, event.currentTarget)}
                  >
                    &larr;
                  </button>
                  <button
                    type="button"
                    aria-label={`Move ${field()?.label} column right`}
                    data-record-list-column-direction={1}
                    disabled={
                      props.table.orderedFieldIds().indexOf(id) ===
                      props.table.orderedFieldIds().length - 1
                    }
                    onClick={(event) => moveColumnFromControl(id, 1, event.currentTarget)}
                  >
                    &rarr;
                  </button>
                </div>
              );
            }}
          </For>
          <For each={fields().filter((field) => field.role !== "identity")}>
            {(field) => (
              <label class="record-list__column-choice">
                <span>{field.label}</span>
                <select
                  aria-label={`${field.label} column visibility`}
                  value={props.table.columnChoices().get(field.id) ?? "auto"}
                  onChange={(event) =>
                    props.table.chooseColumn(
                      field.id,
                      event.currentTarget.value as "auto" | "show" | "hide",
                    )
                  }
                >
                  <option value="auto">Automatic</option>
                  <option value="show">Show</option>
                  <option value="hide">Hide</option>
                </select>
              </label>
            )}
          </For>
          <button type="button" onClick={props.table.resetColumns}>
            Restore default columns
          </button>
        </div>
      </details>
    </Show>
  );
}

/** Compact and List table owner: columns, configuration controls, headings, and rows. */
export function DataSheetTable<Row, SortKey extends string, FilterKey extends string>(
  props: DataSheetTableProps<Row, SortKey, FilterKey>,
): JSX.Element {
  const fields = (): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> => props.table.fields();
  const identityField = (): RecordField<Row, SortKey, FilterKey> | undefined => props.identityField;
  const row = (id: string): Row => props.row(id);
  const recordIds = (): ReadonlyArray<string> => props.recordIds;
  const recordContent = (record: Row): RecordContent => props.content(record);
  const detailContent = (record: Row): RecordContent => {
    const content = recordContent(record);
    const visibleIds = new Set(props.table.visibleFields().map((field) => field.id));
    return {
      ...content,
      details: content.details.filter(
        (fact) =>
          fact.kind !== "text" || fact.fieldId === undefined || !visibleIds.has(fact.fieldId),
      ),
    };
  };
  const hasRecordActions = (): boolean => props.hasRecordActions;
  const selectionName = props.selectionName;
  const sortDirection = (key: SortKey | undefined): "ascending" | "descending" | undefined =>
    key !== undefined && props.query?.sort?.value?.key === key
      ? props.query.sort.value.direction
      : undefined;
  // ASVS 1.2.1: field values use Solid text interpolation, preserving HTML structure.
  const columns = createMemo((): ReadonlyArray<DataSheetTableColumn<string>> => {
    const dataColumns: ReadonlyArray<DataSheetTableColumn<string>> = props.table
      .visibleFields()
      .map((field) => ({
        id: field.id,
        header: (
          <span class="record-filters__heading">
            <Show
              when={field.sortKey !== undefined && props.query?.sort !== undefined}
              fallback={field.label}
            >
              <RecordSortHeading
                label={field.label}
                sortKey={field.sortKey!}
                binding={props.query!.sort!}
              />
            </Show>
            <Show when={field.filterKey !== undefined && props.query?.filters}>
              <RecordFilterButton
                label={field.label}
                count={props.query?.filters?.values.get(field.filterKey!)?.length ?? 0}
                onOpen={(trigger) => props.presentation.openFilter(field.filterKey!, trigger)}
              />
            </Show>
          </span>
        ),
        sortDirection: () => sortDirection(field.sortKey),
        resizeLabel: field.label,
        width: field.defaultColumn.width,
        cell: (recordId) => (
          <Show
            when={field.id === "record_details"}
            fallback={
              <span
                class={
                  props.presentation.mode() === "compact"
                    ? "record-list__compact-value"
                    : "record-list__list-value"
                }
                title={
                  field.render === undefined ? fieldText(field.value(row(recordId))) : undefined
                }
              >
                {field.render?.(row(recordId)) ?? fieldText(field.value(row(recordId)))}
              </span>
            }
          >
            <RecordDetails content={() => detailContent(row(recordId))} />
          </Show>
        ),
      }));
    return hasRecordActions()
      ? [
          ...dataColumns,
          {
            id: "record_actions",
            header: "Actions",
            resizeLabel: "Actions",
            minWidth: 112,
            width: `${defaultRecordActionsWidth(props.table.availableWidth(), props.table.rootFontSize())}px`,
            cell: (recordId) => (
              <CompactRecordActions
                recordId={recordId}
                content={() => recordContent(row(recordId))}
              />
            ),
          },
        ]
      : dataColumns;
  });
  const rowHeader = (): DataSheetTableRowHeader<string> => ({
    id: identityField()?.id ?? "record_identity",
    header: (
      <span class="record-filters__heading">
        <Show
          when={identityField()?.sortKey !== undefined && props.query?.sort !== undefined}
          fallback={identityField()?.label ?? "Record"}
        >
          <RecordSortHeading
            label={identityField()?.label ?? "Record"}
            sortKey={identityField()!.sortKey!}
            binding={props.query!.sort!}
          />
        </Show>
        <Show when={identityField()?.filterKey !== undefined && props.query?.filters}>
          <RecordFilterButton
            label={identityField()?.label ?? "Record"}
            count={props.query?.filters?.values.get(identityField()!.filterKey!)?.length ?? 0}
            onOpen={(trigger) =>
              props.presentation.openFilter(identityField()!.filterKey!, trigger)
            }
          />
        </Show>
      </span>
    ),
    sortDirection: () => sortDirection(identityField()?.sortKey),
    resizeLabel: identityField()?.label ?? "Record",
    minWidth: 160,
    width:
      props.presentation.mode() === "list"
        ? "24rem"
        : (identityField()?.defaultColumn.width ?? (fields().length ? "30%" : "65%")),
    content: (recordId) => (
      <RecordItem
        presentation={props.presentation.mode() === "list" ? "list" : "compact"}
        renderBody={props.renderBody}
        recordId={recordId}
        row={() => row(recordId)}
        content={recordContent}
        selection={props.selection}
        selectionName={selectionName}
      />
    ),
  });
  const orderedColumnIds = (): ReadonlyArray<string> | undefined => {
    if (fields().length === 0) return undefined;
    const shown = new Set(props.table.visibleFields().map((field) => field.id));
    return [
      ...(identityField() === undefined ? ["record_identity"] : []),
      ...props.table.orderedFieldIds().filter((id) => id === identityField()?.id || shown.has(id)),
      ...(hasRecordActions() ? ["record_actions"] : []),
    ];
  };
  return (
    <div
      class="data-sheet-table"
      classList={{
        "record-list--compact": props.presentation.mode() === "compact",
        "record-list--list": props.presentation.mode() === "list",
      }}
      ref={props.table.observeTableWidth}
    >
      <TableView
        rows={recordIds()}
        columns={columns()}
        rowId={(recordId) => recordId}
        rowHeader={rowHeader()}
        orderedColumnIds={orderedColumnIds()}
        onColumnMove={props.table.moveColumn}
        movableColumnIds={props.table.orderedFieldIds()}
        columnWidths={props.table.columnWidths()}
        onColumnWidthsChange={props.table.setColumnWidths}
        minimumWidth={visibleTableWidth(
          props.table.visibleFields(),
          props.table.columnWidths(),
          props.table.availableWidth(),
          props.table.rootFontSize(),
          hasRecordActions(),
          rowHeader().id,
          recordColumnWidth(
            props.presentation.mode() === "list" ? "24rem" : identityField()?.defaultColumn.width,
            props.table.rootFontSize(),
          ),
        )}
        state={props.state}
        ariaLabel={props.ariaLabel}
        emptyState={props.emptyState}
      />
    </div>
  );
}
