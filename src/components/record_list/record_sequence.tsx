import { For, Show, createMemo, createSignal, type Accessor, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import { RecordSemanticContent, type RecordContent } from "./record_list";
import { RecordMoveControls } from "./record_list_reorder";

type RecordSequenceSharedProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly reorder?: RecordSequenceReorder<Row>;
};

export type RecordSequenceReorder<Row> = {
  /**
   * Receives a requested current position change. Resolve only after the controlled rows reflect an
   * accepted order; callers retain persistence, conflict, and error presentation.
   */
  readonly onMove: (sourceIndex: number, destinationIndex: number) => void | Promise<void>;
  readonly recordLabel: (row: Row) => string;
  /** Disables movement for one row without changing the caller-owned order. */
  readonly isDisabled?: (row: Row) => boolean;
};

export type RecordSequenceProps<Row> = RecordSequenceSharedProps<Row> & {
  readonly content: (row: Row) => RecordContent;
  /** Bounded caller-owned body under the shared identity, such as a Pool editor. */
  readonly renderBody?: (row: Accessor<Row>) => JSX.Element;
};

type DraggedRecord = {
  readonly id: string;
};

/** Native ordered records with shared semantic identity and an optional bounded body. */
export function RecordSequence<Row>(props: RecordSequenceProps<Row>): JSX.Element {
  const content = props.content;
  const rowsById = createMemo(() => new Map(props.rows.map((row) => [props.recordId(row), row])));
  const recordIds = createMemo(() => props.rows.map((row) => props.recordId(row)));
  const [movementAnnouncement, setMovementAnnouncement] = createSignal("");
  const [draggedRecord, setDraggedRecord] = createSignal<DraggedRecord>();
  let sequenceRoot: HTMLOListElement | undefined;

  function rowIndex(recordId: string): number {
    return recordIds().indexOf(recordId);
  }

  function rowIsDisabled(recordId: string): boolean {
    const row = rowsById().get(recordId);
    return row === undefined || props.reorder?.isDisabled?.(row) === true;
  }

  function focusMovedRecord(recordId: string, preferredDirection: "earlier" | "later"): void {
    queueMicrotask(() => {
      const movedRecord = Array.from(
        sequenceRoot?.querySelectorAll<HTMLElement>("[data-record-id]") ?? [],
      ).find((element) => element.dataset.recordId === recordId);
      const preferredControl = movedRecord?.querySelector<HTMLButtonElement>(
        `[data-record-controlled-reorder-direction="${preferredDirection}"]:not(:disabled)`,
      );
      const fallbackControl = movedRecord?.querySelector<HTMLButtonElement>(
        "[data-record-controlled-reorder-direction]:not(:disabled)",
      );
      (preferredControl ?? fallbackControl)?.focus();
    });
  }

  async function requestMove(recordId: string, destinationIndex: number): Promise<void> {
    const reorder = props.reorder;
    const before = recordIds();
    const sourceIndex = before.indexOf(recordId);
    const row = rowsById().get(recordId);
    if (
      reorder === undefined ||
      row === undefined ||
      rowIsDisabled(recordId) ||
      sourceIndex < 0 ||
      destinationIndex < 0 ||
      destinationIndex >= before.length ||
      sourceIndex === destinationIndex
    )
      return;
    try {
      await reorder.onMove(sourceIndex, destinationIndex);
    } catch {
      return;
    }
    await new Promise<void>((resolve) => queueMicrotask(resolve));
    const after = recordIds();
    const actualIndex = after.indexOf(recordId);
    if (actualIndex !== destinationIndex || after.every((id, index) => id === before[index]))
      return;
    setMovementAnnouncement(`${reorder.recordLabel(row)} moved to position ${actualIndex + 1}.`);
    focusMovedRecord(recordId, actualIndex < sourceIndex ? "earlier" : "later");
  }

  function moveByOffset(recordId: string, offset: -1 | 1): void {
    void requestMove(recordId, rowIndex(recordId) + offset);
  }

  function startDrag(recordId: string, event: DragEvent): void {
    if (rowIsDisabled(recordId)) return;
    const index = rowIndex(recordId);
    if (index < 0) return;
    setDraggedRecord({ id: recordId });
    if (event.dataTransfer !== null) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", recordId);
    }
  }

  function allowDrop(recordId: string, event: DragEvent): void {
    const dragged = draggedRecord();
    if (dragged === undefined || dragged.id === recordId || rowIsDisabled(recordId)) return;
    event.preventDefault();
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "move";
  }

  function completeDrop(recordId: string, event: DragEvent): void {
    event.preventDefault();
    const dragged = draggedRecord();
    setDraggedRecord(undefined);
    if (dragged === undefined || rowIsDisabled(recordId)) return;
    void requestMove(dragged.id, rowIndex(recordId));
  }

  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <ol
        class="record-sequence record-sequence--semantic"
        aria-label={props.ariaLabel}
        ref={(element) => {
          sequenceRoot = element;
        }}
      >
        <For each={recordIds()}>
          {(recordId) => {
            const row = (): Row => rowsById().get(recordId)!;
            return (
              <li
                class="record-sequence__item record-sequence__item--semantic"
                data-record-id={recordId}
              >
                <RecordSemanticContent
                  recordId={recordId}
                  content={createMemo(() => content(row()))}
                  renderBody={
                    props.renderBody === undefined
                      ? undefined
                      : (): JSX.Element => props.renderBody!(row)
                  }
                />
                <Show when={props.reorder !== undefined}>
                  <RecordMoveControls
                    recordLabel={props.reorder!.recordLabel(row())}
                    index={() => rowIndex(recordId)}
                    count={() => recordIds().length}
                    disabled={() => rowIsDisabled(recordId)}
                    move={(offset) => moveByOffset(recordId, offset)}
                    startDrag={(event) => startDrag(recordId, event)}
                    allowDrop={(event) => allowDrop(recordId, event)}
                    completeDrop={(event) => completeDrop(recordId, event)}
                    endDrag={(): void => setDraggedRecord(undefined)}
                  />
                </Show>
              </li>
            );
          }}
        </For>
      </ol>
      <Show when={props.reorder !== undefined}>
        <p class="visually-hidden" role="status" aria-live="polite">
          {movementAnnouncement()}
        </p>
      </Show>
    </RecordCollectionStateView>
  );
}
