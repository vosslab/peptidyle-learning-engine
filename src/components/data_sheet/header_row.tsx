import { For, Show, type JSX } from "solid-js";

import type { DataSheetTableColumn, DataSheetTableRowHeader } from "./data_sheet_table";

type HeaderRowProps<Row> = {
  readonly columnIds: () => ReadonlyArray<string>;
  readonly rowHeader: DataSheetTableRowHeader<Row>;
  readonly column: (id: string) => DataSheetTableColumn<Row> | undefined;
  readonly canMove: (id: string) => boolean;
  readonly onStartDrag: (event: DragEvent, id: string) => void;
  readonly onDragOver: (event: DragEvent, id: string) => void;
  readonly onDrop: (event: DragEvent, id: string) => void;
  readonly onDragEnd: () => void;
  readonly columnWidths?: ReadonlyMap<string, number>;
  readonly canResize: boolean;
  readonly onResizeStart: (id: string) => number;
  readonly onResize: (id: string, minimum: number, width: number) => void;
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
    if (drag?.pointerId === event.pointerId)
      props.resize(drag.startWidth + (event.clientX - drag.startX) / drag.zoom);
  }
  function onPointerEnd(event: PointerEvent): void {
    if (drag?.pointerId !== event.pointerId) return;
    event.stopPropagation();
    drag = undefined;
  }
  function onKeyDown(event: KeyboardEvent): void {
    const step = event.shiftKey ? 32 : 16;
    const width =
      event.key === "ArrowLeft"
        ? props.start() - step
        : event.key === "ArrowRight"
          ? props.start() + step
          : event.key === "Home"
            ? props.minWidth
            : undefined;
    if (width === undefined) return;
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

/** DataSheetTable's heading view; column mechanics and collection input are forwarded. */
export function HeaderRow<Row>(props: HeaderRowProps<Row>): JSX.Element {
  const label = (id: string): string | undefined =>
    id === props.rowHeader.id ? props.rowHeader.resizeLabel : props.column(id)?.resizeLabel;
  const minimum = (id: string): number =>
    id === props.rowHeader.id
      ? (props.rowHeader.minWidth ?? 160)
      : (props.column(id)?.minWidth ?? 96);
  return (
    <thead>
      <tr>
        <For each={props.columnIds()}>
          {(id) => (
            <th
              scope="col"
              aria-sort={
                id === props.rowHeader.id
                  ? (props.rowHeader.sortDirection?.() ?? "none")
                  : (props.column(id)?.sortDirection?.() ?? "none")
              }
              data-record-column-id={id}
              draggable={props.canMove(id)}
              onDragStart={(event) => props.onStartDrag(event, id)}
              onDragOver={(event) => props.onDragOver(event, id)}
              onDrop={(event) => props.onDrop(event, id)}
              onDragEnd={props.onDragEnd}
              style={{
                width: id === props.rowHeader.id ? props.rowHeader.width : props.column(id)?.width,
                "text-align": props.column(id)?.align ?? "start",
              }}
            >
              {id === props.rowHeader.id ? props.rowHeader.header : props.column(id)?.header}
              <Show when={props.canMove(id)}>
                <span
                  class="record-table__drag-grip"
                  aria-hidden="true"
                  title="Drag to move column"
                >
                  {"\u22ee\u22ee"}
                </span>
              </Show>
              <Show when={props.canResize && label(id) !== undefined}>
                <ColumnResizeHandle
                  label={label(id)!}
                  minWidth={minimum(id)}
                  width={props.columnWidths?.get(id)}
                  start={() => props.onResizeStart(id)}
                  resize={(width) => props.onResize(id, minimum(id), width)}
                />
              </Show>
            </th>
          )}
        </For>
      </tr>
    </thead>
  );
}
