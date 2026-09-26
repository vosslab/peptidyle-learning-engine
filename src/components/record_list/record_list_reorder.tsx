import { createContext, createSignal, useContext, type JSX } from "solid-js";

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
  if (
    sourceIndex < 0 ||
    sourceIndex >= rows.length ||
    destinationIndex < 0 ||
    destinationIndex >= rows.length ||
    sourceIndex === destinationIndex
  )
    return [...rows];
  const reorderedRows = [...rows];
  const [row] = reorderedRows.splice(sourceIndex, 1);
  if (row === undefined) return reorderedRows;
  reorderedRows.splice(destinationIndex, 0, row);
  return reorderedRows;
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

export type ControlledRecordReorder = {
  /** Resolve only after the caller-owned record IDs reflect an accepted move. */
  readonly onMove: (sourceIndex: number, destinationIndex: number) => void | Promise<void>;
  readonly recordLabel: (recordId: string) => string;
  readonly isDisabled?: (recordId: string) => boolean;
};

type ControlledDraggedRecord = {
  readonly id: string;
};

type ControlledRecordReorderContext = {
  readonly recordIds: () => ReadonlyArray<string>;
  readonly reorder: ControlledRecordReorder;
  readonly moveByOffset: (recordId: string, offset: -1 | 1) => void;
  readonly startDrag: (recordId: string, event: DragEvent) => void;
  readonly allowDrop: (recordId: string, event: DragEvent) => void;
  readonly completeDrop: (recordId: string, event: DragEvent) => void;
  readonly endDrag: () => void;
};

const ControlledRecordReorderContext = createContext<ControlledRecordReorderContext>();

export function useControlledRecordReorder(): ControlledRecordReorderContext | undefined {
  return useContext(ControlledRecordReorderContext);
}

export type ControlledRecordReorderProviderProps = {
  readonly recordIds: () => ReadonlyArray<string>;
  readonly reorder: ControlledRecordReorder;
  readonly ariaLabel?: string;
  readonly children: JSX.Element;
};

export type RecordMoveControlsProps = {
  readonly recordLabel: string;
  readonly index: () => number;
  readonly count: () => number;
  readonly disabled: () => boolean;
  readonly move: (offset: -1 | 1) => void;
  readonly startDrag: (event: DragEvent) => void;
  readonly allowDrop: (event: DragEvent) => void;
  readonly completeDrop: (event: DragEvent) => void;
  readonly endDrag: () => void;
  readonly rootRef?: (element: HTMLDivElement) => void;
};

/** Shared native keyboard and drag controls for one controlled ordered record. */
export function RecordMoveControls(props: RecordMoveControlsProps): JSX.Element {
  function handleMoveKeyDown(event: KeyboardEvent): void {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    props.move(event.key === "ArrowUp" ? -1 : 1);
  }

  return (
    <div
      class="record-controlled-reorder-actions"
      role="group"
      aria-label={`Actions for ${props.recordLabel}`}
      ref={props.rootRef}
    >
      <button
        class="quiet-action"
        type="button"
        data-record-controlled-reorder-direction="earlier"
        disabled={props.disabled() || props.index() === 0}
        onClick={() => props.move(-1)}
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
        data-record-controlled-reorder-direction="later"
        disabled={props.disabled() || props.index() === props.count() - 1}
        onClick={() => props.move(1)}
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
        draggable={!props.disabled()}
        disabled={props.disabled()}
        onDragStart={props.startDrag}
        onDragOver={props.allowDrop}
        onDrop={props.completeDrop}
        onDragEnd={props.endDrag}
        aria-label={`Drag ${props.recordLabel} to a new position`}
      >
        Drag to reorder
      </button>
    </div>
  );
}

/**
 * Shares controlled movement completion, announcement, and stable-ID focus across ordered record
 * containers. It owns neither the caller's rows nor persistence and error presentation.
 */
export function ControlledRecordReorderProvider(
  props: ControlledRecordReorderProviderProps,
): JSX.Element {
  const [movementAnnouncement, setMovementAnnouncement] = createSignal("");
  const [draggedRecord, setDraggedRecord] = createSignal<ControlledDraggedRecord>();

  function recordIndex(recordId: string): number {
    return props.recordIds().indexOf(recordId);
  }

  function recordIsDisabled(recordId: string): boolean {
    return props.reorder.isDisabled?.(recordId) === true;
  }

  function focusMovedRecord(
    recordId: string,
    preferredDirection: RecordListReorderDirection,
  ): void {
    requestAnimationFrame(() => {
      const focusRoot = Array.from(
        document.querySelectorAll<HTMLOListElement>("ol[aria-label]"),
      ).find((element) => element.getAttribute("aria-label") === props.ariaLabel);
      const movedRecord = Array.from(
        focusRoot?.querySelectorAll<HTMLElement>("[data-record-id]") ?? [],
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
    const before = props.recordIds();
    const sourceIndex = before.indexOf(recordId);
    if (
      recordIsDisabled(recordId) ||
      sourceIndex < 0 ||
      destinationIndex < 0 ||
      destinationIndex >= before.length ||
      sourceIndex === destinationIndex
    )
      return;
    try {
      await props.reorder.onMove(sourceIndex, destinationIndex);
    } catch {
      return;
    }
    await new Promise<void>((resolve) => queueMicrotask(resolve));
    const after = props.recordIds();
    const actualIndex = after.indexOf(recordId);
    if (actualIndex !== destinationIndex || after.every((id, index) => id === before[index]))
      return;
    setMovementAnnouncement(
      `${props.reorder.recordLabel(recordId)} moved to position ${actualIndex + 1}.`,
    );
    focusMovedRecord(recordId, actualIndex < sourceIndex ? "earlier" : "later");
  }

  function moveByOffset(recordId: string, offset: -1 | 1): void {
    void requestMove(recordId, recordIndex(recordId) + offset);
  }

  function startDrag(recordId: string, event: DragEvent): void {
    if (recordIsDisabled(recordId) || recordIndex(recordId) < 0) return;
    setDraggedRecord({ id: recordId });
    if (event.dataTransfer !== null) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", recordId);
    }
  }

  function allowDrop(recordId: string, event: DragEvent): void {
    const dragged = draggedRecord();
    if (dragged === undefined || dragged.id === recordId || recordIsDisabled(recordId)) return;
    event.preventDefault();
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "move";
  }

  function completeDrop(recordId: string, event: DragEvent): void {
    event.preventDefault();
    const dragged = draggedRecord();
    setDraggedRecord(undefined);
    if (dragged === undefined || recordIsDisabled(recordId)) return;
    void requestMove(dragged.id, recordIndex(recordId));
  }

  const context: ControlledRecordReorderContext = {
    recordIds: props.recordIds,
    reorder: props.reorder,
    moveByOffset,
    startDrag,
    allowDrop,
    completeDrop,
    endDrag: (): void => setDraggedRecord(undefined),
  };

  return (
    <ControlledRecordReorderContext.Provider value={context}>
      {props.children}
      <p class="visually-hidden" role="status" aria-live="polite">
        {movementAnnouncement()}
      </p>
    </ControlledRecordReorderContext.Provider>
  );
}

export function ControlledRecordMoveControls(props: { readonly recordId: string }): JSX.Element {
  const context = useControlledRecordReorder();
  if (context === undefined)
    throw new Error("ControlledRecordMoveControls must be inside ControlledRecordReorderProvider.");
  const recordIndex = (): number => context.recordIds().indexOf(props.recordId);
  const disabled = (): boolean => context.reorder.isDisabled?.(props.recordId) === true;
  const recordLabel = (): string => context.reorder.recordLabel(props.recordId);

  return (
    <RecordMoveControls
      recordLabel={recordLabel()}
      index={recordIndex}
      count={() => context.recordIds().length}
      disabled={disabled}
      move={(offset) => context.moveByOffset(props.recordId, offset)}
      startDrag={(event) => context.startDrag(props.recordId, event)}
      allowDrop={(event) => context.allowDrop(props.recordId, event)}
      completeDrop={(event) => context.completeDrop(props.recordId, event)}
      endDrag={context.endDrag}
    />
  );
}
