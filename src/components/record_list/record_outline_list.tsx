import { Show, type Accessor, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import {
  ControlledRecordMoveControls,
  ControlledRecordReorderProvider,
  useControlledRecordReorder,
  type ControlledRecordReorder,
} from "./record_list_reorder";

export type RecordOutlineReorder = ControlledRecordReorder & {
  /** The caller-owned current sibling order for this exact outline level. */
  readonly recordIds: Accessor<ReadonlyArray<string>>;
};

export type RecordOutlineListProps = {
  readonly state: RecordCollectionState;
  readonly isEmpty: boolean;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly children: JSX.Element;
  readonly reorder?: RecordOutlineReorder;
};

/** Native nested-record container. Compose nested lists inside outline items. */
export function RecordOutlineList(props: RecordOutlineListProps): JSX.Element {
  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.isEmpty}
      emptyState={props.emptyState}
    >
      <Show when={props.reorder} fallback={<NativeOutlineList {...props} />}>
        {(reorder) => (
          <ControlledRecordReorderProvider
            recordIds={reorder().recordIds}
            reorder={reorder()}
            ariaLabel={props.ariaLabel}
          >
            <NativeOutlineList {...props} />
          </ControlledRecordReorderProvider>
        )}
      </Show>
    </RecordCollectionStateView>
  );
}

function NativeOutlineList(props: RecordOutlineListProps): JSX.Element {
  return (
    <ol class="record-outline-list" aria-label={props.ariaLabel}>
      {props.children}
    </ol>
  );
}

export type RecordOutlineItemProps = {
  readonly recordId: string;
  readonly children: JSX.Element;
};

/** One outline record whose children may include a nested RecordOutlineList or RecordSequence. */
export function RecordOutlineItem(props: RecordOutlineItemProps): JSX.Element {
  const reorder = useControlledRecordReorder();
  return (
    <li class="record-outline-list__item" data-record-id={props.recordId}>
      {props.children}
      <Show when={reorder !== undefined && reorder.recordIds().includes(props.recordId)}>
        <ControlledRecordMoveControls recordId={props.recordId} />
      </Show>
    </li>
  );
}
