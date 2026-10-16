import { For, Show, type JSX } from "solid-js";

import {
  RecordCollectionEmptyNotice,
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";

export type RecordTableColumn<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly resizeLabel?: string;
  readonly minWidth?: number;
  readonly cell: (row: Row) => JSX.Element;
  readonly width?: string;
  readonly align?: "start" | "center" | "end";
  readonly sortDirection?: () => "ascending" | "descending" | undefined;
};

export type RecordTableRowHeader<Row> = {
  readonly id: string;
  readonly header: JSX.Element;
  readonly resizeLabel?: string;
  readonly minWidth?: number;
  readonly content: (row: Row) => JSX.Element;
  readonly width?: string;
  readonly sortDirection?: () => "ascending" | "descending" | undefined;
};

export type RecordTableProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly columns: ReadonlyArray<RecordTableColumn<Row>>;
  readonly rowId: (row: Row) => string;
  readonly rowHeader: RecordTableRowHeader<Row>;
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

function ColumnResizeHandle(props: {
  readonly label: string;
  readonly minWidth: number;
  readonly width?: number;
  readonly start: () => number;
  readonly resize: (width: number) => void;
}): JSX.Element {
  let drag: { pointerId: number; startX: number; startWidth: number; zoom: number } | undefined;

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const button = event.currentTarget as HTMLButtonElement;
    const header = button.closest("th")!;
    drag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startWidth: props.start(),
      zoom: header.getBoundingClientRect().width / header.offsetWidth || 1,
    };
    button.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent): void {
    if (drag?.pointerId !== event.pointerId) return;
    props.resize(drag.startWidth + (event.clientX - drag.startX) / drag.zoom);
  }

  function onPointerEnd(event: PointerEvent): void {
    if (drag?.pointerId !== event.pointerId) return;
    event.stopPropagation();
    drag = undefined;
  }

  function onKeyDown(event: KeyboardEvent): void {
    const step = event.shiftKey ? 32 : 16;
    let width: number;
    switch (event.key) {
      case "ArrowLeft":
        width = props.start() - step;
        break;
      case "ArrowRight":
        width = props.start() + step;
        break;
      case "Home":
        width = props.minWidth;
        break;
      default:
        return;
    }
    event.preventDefault();
    event.stopPropagation();
    props.resize(width);
  }

  return (
    <button
      class="record-table__resize-handle"
      type="button"
      aria-label={`Resize ${props.label} column${props.width === undefined ? "" : `, ${Math.round(props.width)} pixels`}. Use Left and Right arrows; Home sets minimum width.`}
      title={`Resize ${props.label} column. Drag, or use Left and Right arrows; Home sets minimum width.`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onDragStart={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
      onKeyDown={onKeyDown}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
    />
  );
}

/** Semantic records with named columns and one row header per record. */
export function RecordTable<Row>(props: RecordTableProps<Row>): JSX.Element {
  let tableElement!: HTMLTableElement;
  let draggedColumnId: string | undefined;
  const columnIds = (): ReadonlyArray<string> =>
    props.orderedColumnIds ?? [props.rowHeader.id, ...props.columns.map((column) => column.id)];
  const column = (id: string): RecordTableColumn<Row> | undefined =>
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
          <thead>
            <tr>
              <For each={columnIds()}>
                {(id) => (
                  <th
                    scope="col"
                    aria-sort={
                      id === props.rowHeader.id
                        ? (props.rowHeader.sortDirection?.() ?? "none")
                        : (column(id)?.sortDirection?.() ?? "none")
                    }
                    data-record-column-id={id}
                    draggable={canMove(id)}
                    onDragStart={(event) => startColumnDrag(event, id)}
                    onDragOver={(event) => allowColumnDrop(event, id)}
                    onDrop={(event) => dropColumn(event, id)}
                    onDragEnd={() => {
                      draggedColumnId = undefined;
                    }}
                    style={{
                      width: id === props.rowHeader.id ? props.rowHeader.width : column(id)?.width,
                      "text-align": column(id)?.align ?? "start",
                    }}
                  >
                    {id === props.rowHeader.id ? props.rowHeader.header : column(id)?.header}
                    <Show when={canMove(id)}>
                      <span
                        class="record-table__drag-grip"
                        aria-hidden="true"
                        title="Drag to move column"
                      >
                        {"\u22ee\u22ee"}
                      </span>
                    </Show>
                    <Show
                      when={
                        props.onColumnWidthsChange &&
                        (id === props.rowHeader.id
                          ? props.rowHeader.resizeLabel
                          : column(id)?.resizeLabel)
                      }
                    >
                      <ColumnResizeHandle
                        label={
                          (id === props.rowHeader.id
                            ? props.rowHeader.resizeLabel
                            : column(id)?.resizeLabel)!
                        }
                        minWidth={
                          id === props.rowHeader.id
                            ? (props.rowHeader.minWidth ?? 160)
                            : (column(id)?.minWidth ?? 96)
                        }
                        width={props.columnWidths?.get(id)}
                        start={() => startResize(id)}
                        resize={(width) =>
                          resize(
                            id,
                            id === props.rowHeader.id
                              ? (props.rowHeader.minWidth ?? 160)
                              : (column(id)?.minWidth ?? 96),
                            width,
                          )
                        }
                      />
                    </Show>
                  </th>
                )}
              </For>
            </tr>
          </thead>
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
                <tr data-record-id={props.rowId(row)}>
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
