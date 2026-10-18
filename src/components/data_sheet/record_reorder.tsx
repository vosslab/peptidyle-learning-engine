import { createContext, createSignal, useContext, type JSX } from "solid-js";

type RecordListReorderDirection = "earlier" | "later";

/**
 * Destination index for a native drag that lands on another position in the current order.
 * A drop on the same position, an out-of-range position, or a disabled control does not move.
 */
export function nativeDragIndex(
  sourceIndex: number,
  destinationIndex: number,
  count: number,
  disabled: boolean,
): number | undefined {
  // ASVS 2.2.1: accept a drop only inside the current order and only when the control is enabled.
  if (
    disabled ||
    sourceIndex < 0 ||
    destinationIndex < 0 ||
    sourceIndex >= count ||
    destinationIndex >= count ||
    sourceIndex === destinationIndex
  )
    return undefined;
  return destinationIndex;
}

/**
 * Destination index for a native drag that lands on another record in the current order.
 * A drop on the same record, a missing record, or a disabled record does not move.
 */
export function nativeDragDestinationIndex(
  recordIds: readonly string[],
  draggedId: string,
  dropId: string,
  isDisabled: (recordId: string) => boolean,
): number | undefined {
  // ASVS 2.2.1: accept a drop only when both records are in the current order and enabled.
  if (draggedId === dropId || isDisabled(draggedId) || isDisabled(dropId)) return undefined;
  const sourceIndex = recordIds.indexOf(draggedId);
  const destinationIndex = recordIds.indexOf(dropId);
  return nativeDragIndex(sourceIndex, destinationIndex, recordIds.length, false);
}

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

/** Full authored sibling order; filtered/paged display order is never used for movement. */
export type RecordReorderBinding = {
  readonly orderForRecord: (recordId: string) => ReadonlyArray<string>;
  readonly onMove: (sourceId: string, targetId: string) => void | Promise<void>;
  readonly recordLabel: (recordId: string) => string;
  readonly isDisabled?: (recordId: string) => boolean;
};

type ControlledDraggedRecord = {
  readonly id: string;
};

type ControlledRecordReorderContext = {
  readonly orderForRecord: (recordId: string) => ReadonlyArray<string>;
  readonly recordLabel: (recordId: string) => string;
  readonly isDisabled: (recordId: string) => boolean;
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
  readonly root?: () => HTMLElement | undefined;
  readonly onBeforeMove?: () => void;
  readonly ariaLabel?: string;
  readonly children: JSX.Element;
  readonly binding?: RecordReorderBinding;
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
  readonly dragLabel?: string;
  readonly dragDisabled?: () => boolean;
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
        <span class="record-interaction__icon" aria-hidden="true">
          &uarr;
        </span>
        <span class="record-interaction__label">&uarr; Move earlier</span>
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
        <span class="record-interaction__icon" aria-hidden="true">
          &darr;
        </span>
        <span class="record-interaction__label">&darr; Move later</span>
      </button>
      <button
        class="quiet-action"
        type="button"
        draggable={!(props.dragDisabled?.() ?? props.disabled())}
        disabled={props.dragDisabled?.() ?? props.disabled()}
        onDragStart={props.startDrag}
        onDragOver={props.allowDrop}
        onDrop={props.completeDrop}
        onDragEnd={props.endDrag}
        aria-label={props.dragLabel ?? `Drag ${props.recordLabel} to a new position`}
        title={props.dragLabel ?? "Drag to reorder"}
      >
        <span class="record-interaction__icon" aria-hidden="true">
          {"\u22ee\u22ee"}
        </span>
        <span class="record-interaction__label">{props.dragLabel ?? "Drag to reorder"}</span>
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

  const orderForRecord = (recordId: string): ReadonlyArray<string> =>
    props.binding?.orderForRecord(recordId) ?? [];
  const recordLabel = (recordId: string): string =>
    props.binding?.recordLabel(recordId) ?? "Record";

  function recordIndex(recordId: string): number {
    return orderForRecord(recordId).indexOf(recordId);
  }

  function recordIsDisabled(recordId: string): boolean {
    return props.binding?.isDisabled?.(recordId) === true;
  }

  function focusMovedRecord(
    recordId: string,
    preferredDirection: RecordListReorderDirection,
  ): void {
    requestAnimationFrame(() => {
      const focusRoot =
        props.root?.() ??
        Array.from(document.querySelectorAll<HTMLOListElement>("ol[aria-label]")).find(
          (element) => element.getAttribute("aria-label") === props.ariaLabel,
        );
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
    const before = orderForRecord(recordId);
    const sourceIndex = before.indexOf(recordId);
    if (
      recordIsDisabled(recordId) ||
      sourceIndex < 0 ||
      destinationIndex < 0 ||
      destinationIndex >= before.length ||
      sourceIndex === destinationIndex ||
      recordIsDisabled(before[destinationIndex]!)
    )
      return;
    const targetId = before[destinationIndex]!;
    setDraggedRecord(undefined);
    props.onBeforeMove?.();
    try {
      await props.binding?.onMove(recordId, targetId);
    } catch {
      return;
    }
    await new Promise<void>((resolve) => queueMicrotask(resolve));
    const after = orderForRecord(recordId);
    const actualIndex = after.indexOf(recordId);
    if (actualIndex !== destinationIndex || after.every((id, index) => id === before[index]))
      return;
    setMovementAnnouncement(`${recordLabel(recordId)} moved to position ${actualIndex + 1}.`);
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
    if (
      dragged === undefined ||
      nativeDragDestinationIndex(
        orderForRecord(dragged.id),
        dragged.id,
        recordId,
        recordIsDisabled,
      ) === undefined
    )
      return;
    event.preventDefault();
    if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "move";
  }

  function completeDrop(recordId: string, event: DragEvent): void {
    event.preventDefault();
    const dragged = draggedRecord();
    setDraggedRecord(undefined);
    if (dragged === undefined) return;
    const destinationIndex = nativeDragDestinationIndex(
      orderForRecord(dragged.id),
      dragged.id,
      recordId,
      recordIsDisabled,
    );
    if (destinationIndex === undefined) return;
    void requestMove(dragged.id, destinationIndex);
  }

  const context: ControlledRecordReorderContext = {
    orderForRecord,
    recordLabel,
    isDisabled: recordIsDisabled,
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
