import {
  Show,
  createContext,
  createMemo,
  createRenderEffect,
  createSignal,
  on,
  onCleanup,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import {
  ControlledRecordReorderProvider,
  RecordMoveControls,
  type RecordReorderBinding,
  useControlledRecordReorder,
} from "./record_reorder";

/** The same live domain command used by the editor's Add or Remove button. */
export type RecordTransferCommand = {
  readonly label: string;
  readonly disabled: boolean;
  readonly run: () => void | Promise<void>;
};

/** Panel keys are local to one editor's RecordTransferProvider. */
export type RecordTransferBinding<Row> = {
  readonly panelKey: string;
  readonly targetPanelKey: string;
  readonly command: (row: Row) => RecordTransferCommand;
};

type RecordGesture = {
  readonly sourcePanel: string;
  readonly targetPanel: string;
  readonly resolve: () => RecordTransferCommand | undefined;
  readonly clearReorder: () => void;
};

type RecordTransferContext = {
  readonly gesture: Accessor<RecordGesture | undefined>;
  readonly start: (gesture: RecordGesture) => void;
  readonly clear: () => void;
};

const RecordTransferContext = createContext<RecordTransferContext>();
const RecordInteractionStateContext = createContext<Accessor<object>>();

/** Private SearchBlock seam: identity is the current search-state snapshot. */
export function RecordInteractionStateProvider(props: {
  readonly state: object;
  readonly children: JSX.Element;
}): JSX.Element {
  return (
    <RecordInteractionStateContext.Provider value={() => props.state}>
      {props.children}
    </RecordInteractionStateContext.Provider>
  );
}

/** One editor-local gesture; it owns no records, draft, search state, or persistence. */
export function RecordTransferProvider(props: { readonly children: JSX.Element }): JSX.Element {
  const [gesture, setGesture] = createSignal<RecordGesture>();
  function clear(): void {
    const current = gesture();
    setGesture(undefined);
    current?.clearReorder();
  }
  const context: RecordTransferContext = {
    gesture,
    clear,
    start: (next) => {
      clear();
      setGesture(next);
    },
  };
  onCleanup(clear);
  return (
    <RecordTransferContext.Provider value={context}>
      {props.children}
    </RecordTransferContext.Provider>
  );
}

type RecordInteractions = {
  readonly enabled: () => boolean;
  readonly canDrag: (id: string) => boolean;
  readonly canReorder: (id: string) => boolean;
  readonly canDrop: (id?: string) => boolean;
  readonly startDrag: (id: string, event: DragEvent) => void;
  readonly allowDrop: (event: DragEvent, id?: string) => void;
  readonly completeDrop: (event: DragEvent, id?: string) => void;
  readonly clear: () => void;
};

const RecordInteractionsContext = createContext<RecordInteractions>();

export function useRecordInteractions(): RecordInteractions | undefined {
  return useContext(RecordInteractionsContext);
}

/** Connects native record interactions to the current DataSheet and its optional domain
 * bindings.
 */
function RecordInteractionsProvider<Row>(props: {
  readonly transfer?: RecordTransferBinding<Row>;
  readonly row: (id: string) => Row | undefined;
  readonly recordIds: ReadonlyArray<string>;
  readonly ready: boolean;
  readonly reorderEnabled: boolean;
  readonly children: JSX.Element;
}): JSX.Element {
  const transfer = useContext(RecordTransferContext);
  const lifecycle = useContext(RecordInteractionStateContext);
  const reorder = useControlledRecordReorder();
  const present = (id: string): boolean => props.ready && props.recordIds.includes(id);
  const resolve = (id: string): RecordTransferCommand | undefined => {
    if (!present(id)) return undefined;
    const row = props.row(id);
    return row === undefined ? undefined : props.transfer?.command(row);
  };
  const canReorder = (id: string): boolean =>
    props.reorderEnabled &&
    present(id) &&
    reorder?.orderForRecord(id).includes(id) === true &&
    !reorder.isDisabled(id);
  const canTransfer = (id: string): boolean => {
    const command = resolve(id);
    return transfer !== undefined && command !== undefined && !command.disabled;
  };
  function clear(): void {
    transfer?.clear();
    reorder?.endDrag();
  }
  // State identity changes include retained rows, loading, errors, filters, and page transitions.
  createRenderEffect(on(() => lifecycle?.() ?? props.recordIds, clear, { defer: true }));
  onCleanup(clear);

  function transferCommand(id?: string): RecordTransferCommand | undefined {
    const gesture = transfer?.gesture();
    const binding = props.transfer;
    if (
      !props.ready ||
      gesture === undefined ||
      binding === undefined ||
      gesture.sourcePanel === binding.panelKey ||
      gesture.targetPanel !== binding.panelKey ||
      binding.targetPanelKey !== gesture.sourcePanel ||
      (id !== undefined && !present(id))
    )
      return undefined;
    // ASVS 2.2.1: use the current source row and domain eligibility, never drag payload data.
    const command = gesture.resolve();
    return command?.disabled === false ? command : undefined;
  }
  const context: RecordInteractions = {
    enabled: () => props.reorderEnabled || (transfer !== undefined && props.transfer !== undefined),
    canDrag: (id) => canReorder(id) || canTransfer(id),
    canReorder,
    canDrop: (id) => transferCommand(id) !== undefined,
    clear,
    startDrag: (id, event) => {
      clear();
      if (!canReorder(id) && !canTransfer(id)) {
        event.preventDefault();
        return;
      }
      const binding = props.transfer;
      if (transfer && binding) {
        transfer.start({
          sourcePanel: binding.panelKey,
          targetPanel: binding.targetPanelKey,
          resolve: () => resolve(id),
          clearReorder: () => reorder?.endDrag(),
        });
      }
      if (canReorder(id)) reorder?.startDrag(id, event);
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", "Move record");
      }
    },
    allowDrop: (event, id) => {
      if (transferCommand(id)) {
        event.preventDefault();
        event.stopPropagation();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
      } else if (id !== undefined && present(id)) reorder?.allowDrop(id, event);
    },
    completeDrop: (event, id) => {
      const command = transferCommand(id);
      event.stopPropagation();
      if (command) {
        event.preventDefault();
        clear();
        void command.run();
      } else if (id !== undefined && present(id)) {
        reorder?.completeDrop(id, event);
        clear();
      } else clear();
    },
  };
  return (
    <RecordInteractionsContext.Provider value={context}>
      {props.children}
    </RecordInteractionsContext.Provider>
  );
}

/** Shared handle in every RecordItem mode; existing domain buttons remain the transfer
 * equivalent.
 */
export function RecordInteractionControls(props: {
  readonly recordId: string;
  readonly recordLabel: string;
}): JSX.Element {
  const interactions = useRecordInteractions();
  const reorder = useControlledRecordReorder();
  const order = (): ReadonlyArray<string> => reorder?.orderForRecord(props.recordId) ?? [];
  const hasReorder = (): boolean => order().includes(props.recordId);
  return (
    <Show when={interactions?.enabled()}>
      <Show
        when={hasReorder()}
        fallback={
          <button
            class="quiet-action record-list__drag-handle"
            type="button"
            draggable={interactions?.canDrag(props.recordId)}
            disabled={!interactions?.canDrag(props.recordId)}
            onDragStart={(event) => interactions?.startDrag(props.recordId, event)}
            onDragEnd={() => interactions?.clear()}
            aria-label={`Drag ${props.recordLabel} to the other panel`}
            title="Drag to transfer"
          >
            <span class="record-interaction__icon" aria-hidden="true">
              {"\u22ee\u22ee"}
            </span>
            <span class="record-interaction__label">Drag to transfer</span>
          </button>
        }
      >
        <RecordMoveControls
          recordLabel={props.recordLabel}
          index={() => order().indexOf(props.recordId)}
          count={() => order().length}
          disabled={() => !interactions?.canReorder(props.recordId)}
          dragDisabled={() => !interactions?.canDrag(props.recordId)}
          move={(offset) => {
            if (!interactions?.canReorder(props.recordId)) return;
            interactions.clear();
            reorder?.moveByOffset(props.recordId, offset);
          }}
          startDrag={(event) => interactions?.startDrag(props.recordId, event)}
          allowDrop={(event) => interactions?.allowDrop(event, props.recordId)}
          completeDrop={(event) => interactions?.completeDrop(event, props.recordId)}
          endDrag={() => interactions?.clear()}
        />
      </Show>
    </Show>
  );
}

/** DataSheet empty regions and record rows use the same live destination validation. */
function RecordEmptyDropTarget(): JSX.Element {
  const interactions = useRecordInteractions();
  return (
    <Show when={interactions?.canDrop()}>
      <p class="record-list__empty-drop-target" role="status">
        Drop here to transfer
      </p>
    </Show>
  );
}

/** DataSheet-local composition keeps column gestures outside record drag state. */
export function DataSheetInteractions<Row>(props: {
  readonly reorder?: RecordReorderBinding;
  readonly transfer?: RecordTransferBinding<Row>;
  readonly row: (id: string) => Row | undefined;
  readonly recordIds: ReadonlyArray<string>;
  readonly ready: boolean;
  readonly ariaLabel: string;
  readonly children: JSX.Element;
}): JSX.Element {
  const transfer = useContext(RecordTransferContext);
  const reorder = createMemo((): RecordReorderBinding | undefined => {
    const binding = props.reorder;
    if (binding === undefined) return undefined;
    return {
      ...binding,
      isDisabled: (id) => !props.ready || binding.isDisabled?.(id) === true,
    };
  });
  let root: HTMLDivElement | undefined;
  return (
    <ControlledRecordReorderProvider
      binding={reorder()}
      root={() => root}
      ariaLabel={props.ariaLabel}
      onBeforeMove={() => transfer?.clear()}
    >
      <RecordInteractionsProvider
        transfer={props.transfer}
        row={props.row}
        recordIds={props.recordIds}
        ready={props.ready}
        reorderEnabled={props.reorder !== undefined}
      >
        <RecordInteractionRegion
          rootRef={(element) => {
            root = element;
          }}
        >
          {props.children}
          <Show when={props.recordIds.length === 0}>
            <RecordEmptyDropTarget />
          </Show>
        </RecordInteractionRegion>
      </RecordInteractionsProvider>
    </ControlledRecordReorderProvider>
  );
}

function RecordInteractionRegion(props: {
  readonly rootRef: (element: HTMLDivElement) => void;
  readonly children: JSX.Element;
}): JSX.Element {
  const interactions = useRecordInteractions();
  return (
    <div
      ref={props.rootRef}
      class="data-sheet-block"
      classList={{ "record-list--transfer-target": interactions?.canDrop() }}
      onDragOver={(event) => interactions?.allowDrop(event)}
      onDrop={(event) => interactions?.completeDrop(event)}
    >
      {props.children}
    </div>
  );
}
