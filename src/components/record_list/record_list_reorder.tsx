import { createContext, createSignal, useContext, type JSX } from "solid-js";

import { reordered } from "../../features/blueprint_forks/blueprint_fork_apply_model";

type DraggedRecord = {
  readonly id: string;
  readonly label: string;
  readonly index: number;
};

type RecordListReorderDirection = "earlier" | "later";

type RecordListReorderContext = {
  readonly onMove: (sourceIndex: number, destinationIndex: number) => void;
  readonly announceMove: (recordLabel: string, destinationIndex: number) => void;
  readonly focusMovedRecord: (
    recordId: string,
    preferredDirection: RecordListReorderDirection,
  ) => void;
  readonly draggedRecord: () => DraggedRecord | undefined;
  readonly setDraggedRecord: (record: DraggedRecord | undefined) => void;
};

const RecordListReorderContext = createContext<RecordListReorderContext>();
const RecordListReorderProvider = RecordListReorderContext.Provider;

/** Moves one record to an absolute destination index without changing the caller's state. */
export function reorderedRecordListRows<Row>(
  rows: ReadonlyArray<Row>,
  sourceIndex: number,
  destinationIndex: number,
): Row[] {
  return reordered(rows, sourceIndex, destinationIndex - sourceIndex);
}

export type RecordListReorderProps = {
  /** Caller-owned state update; persistence timing and failure handling remain at the caller. */
  readonly onMove: (sourceIndex: number, destinationIndex: number) => void;
  readonly children: JSX.Element;
};

/**
 * Provides one live move announcement and drag state to reorder controls inside a RecordList.
 * It deliberately owns no rows, disabled policy, or persistence behavior.
 */
export function RecordListReorder(props: RecordListReorderProps): JSX.Element {
  const [movementAnnouncement, setMovementAnnouncement] = createSignal("");
  const [draggedRecord, setDraggedRecord] = createSignal<DraggedRecord>();
  let recordListReorderRoot: HTMLDivElement | undefined;

  function announceMove(recordLabel: string, destinationIndex: number): void {
    setMovementAnnouncement(`${recordLabel} moved to position ${destinationIndex + 1}.`);
  }

  function assignRecordListReorderRoot(root: HTMLDivElement): void {
    recordListReorderRoot = root;
  }

  function focusMovedRecord(
    recordId: string,
    preferredDirection: RecordListReorderDirection,
  ): void {
    queueMicrotask(() => {
      if (recordListReorderRoot === undefined) return;
      const recordRows = recordListReorderRoot.querySelectorAll<HTMLElement>("[data-record-id]");
      const movedRecordRow = Array.from(recordRows).find(
        (recordRow) => recordRow.dataset.recordId === recordId,
      );
      const preferredControl = movedRecordRow?.querySelector<HTMLButtonElement>(
        `[data-record-list-reorder-direction="${preferredDirection}"]:not(:disabled)`,
      );
      const fallbackControl = movedRecordRow?.querySelector<HTMLButtonElement>(
        "[data-record-list-reorder-direction]:not(:disabled)",
      );
      (preferredControl ?? fallbackControl)?.focus();
    });
  }

  const context: RecordListReorderContext = {
    onMove: props.onMove,
    announceMove,
    focusMovedRecord,
    draggedRecord,
    setDraggedRecord,
  };

  return (
    <RecordListReorderProvider value={context}>
      <div ref={assignRecordListReorderRoot}>
        {props.children}
        <p class="visually-hidden" role="status" aria-live="polite">
          {movementAnnouncement()}
        </p>
      </div>
    </RecordListReorderProvider>
  );
}

export type RecordListReorderControlsProps = {
  /** The stable RecordList recordId for focus restoration after a successful move. */
  readonly recordId: string;
  /** Human-readable record title used in the live move announcement. */
  readonly recordLabel: string;
  /** Reactive position supplied by the caller's RecordList row loop. */
  readonly index: () => number;
  /** Reactive record count supplied by the caller's current rows. */
  readonly count: () => number;
  /** The caller's current policy decision, such as an unsaved or busy state. */
  readonly disabled: boolean;
};

/** Keyboard and native-drag controls for one RecordList row. */
export function RecordListReorderControls(props: RecordListReorderControlsProps): JSX.Element {
  const context = useContext(RecordListReorderContext);
  if (context === undefined)
    throw new Error("RecordListReorderControls must be inside RecordListReorder.");
  const reorderContext = context;

  function moveRecord(
    sourceIndex: number,
    destinationIndex: number,
    recordId: string,
    recordLabel: string,
  ): void {
    if (
      props.disabled ||
      sourceIndex === destinationIndex ||
      destinationIndex < 0 ||
      destinationIndex >= props.count()
    )
      return;
    reorderContext.onMove(sourceIndex, destinationIndex);
    reorderContext.announceMove(recordLabel, destinationIndex);
    reorderContext.focusMovedRecord(recordId, destinationIndex < sourceIndex ? "earlier" : "later");
  }

  function moveFromKeyboard(offset: -1 | 1): void {
    moveRecord(props.index(), props.index() + offset, props.recordId, props.recordLabel);
  }

  function handleMoveKeyDown(event: KeyboardEvent): void {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    moveFromKeyboard(event.key === "ArrowUp" ? -1 : 1);
  }

  function startDrag(event: DragEvent): void {
    if (props.disabled) return;
    const draggedRecord: DraggedRecord = {
      id: props.recordId,
      label: props.recordLabel,
      index: props.index(),
    };
    reorderContext.setDraggedRecord(draggedRecord);
    if (event.dataTransfer !== null) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", draggedRecord.label);
    }
  }

  function allowDrop(event: DragEvent): void {
    const draggedRecord = reorderContext.draggedRecord();
    if (props.disabled || draggedRecord === undefined || draggedRecord.id === props.recordId)
      return;
    event.preventDefault();
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "move";
  }

  function completeDrop(event: DragEvent): void {
    event.preventDefault();
    const draggedRecord = reorderContext.draggedRecord();
    reorderContext.setDraggedRecord(undefined);
    if (draggedRecord === undefined) return;
    moveRecord(draggedRecord.index, props.index(), draggedRecord.id, draggedRecord.label);
  }

  return (
    <div
      class="record-list__reorder-actions"
      role="group"
      aria-label={`Actions for ${props.recordLabel}`}
    >
      <button
        class="quiet-action"
        type="button"
        data-record-list-reorder-direction="earlier"
        disabled={props.disabled || props.index() === 0}
        onClick={() => moveFromKeyboard(-1)}
        onKeyDown={handleMoveKeyDown}
        aria-keyshortcuts="ArrowUp"
        aria-label={`Move ${props.recordLabel} earlier`}
        title="Use the Up Arrow to move earlier."
      >
        &uarr; Move earlier
      </button>
      <button
        class="quiet-action"
        type="button"
        data-record-list-reorder-direction="later"
        disabled={props.disabled || props.index() === props.count() - 1}
        onClick={() => moveFromKeyboard(1)}
        onKeyDown={handleMoveKeyDown}
        aria-keyshortcuts="ArrowDown"
        aria-label={`Move ${props.recordLabel} later`}
        title="Use the Down Arrow to move later."
      >
        &darr; Move later
      </button>
      <button
        class="quiet-action"
        type="button"
        draggable={!props.disabled}
        disabled={props.disabled}
        onPointerDown={() => {
          if (props.disabled) return;
          reorderContext.setDraggedRecord({
            id: props.recordId,
            label: props.recordLabel,
            index: props.index(),
          });
        }}
        onDragStart={startDrag}
        onDragOver={allowDrop}
        onDrop={completeDrop}
        onDragEnd={() => reorderContext.setDraggedRecord(undefined)}
        aria-label={`Drag ${props.recordLabel} to a new position`}
      >
        Drag to reorder
      </button>
    </div>
  );
}
