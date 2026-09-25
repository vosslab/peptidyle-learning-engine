import { type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";

export type RecordOutlineListProps = {
  readonly state: RecordCollectionState;
  readonly isEmpty: boolean;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
  readonly children: JSX.Element;
};

/** Native nested-record container. Compose nested lists inside outline items. */
export function RecordOutlineList(props: RecordOutlineListProps): JSX.Element {
  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.isEmpty}
      emptyState={props.emptyState}
    >
      <ol class="record-outline-list" aria-label={props.ariaLabel}>
        {props.children}
      </ol>
    </RecordCollectionStateView>
  );
}

export type RecordOutlineItemProps = {
  readonly recordId: string;
  readonly children: JSX.Element;
};

/** One outline record whose children may include a nested RecordOutlineList or RecordSequence. */
export function RecordOutlineItem(props: RecordOutlineItemProps): JSX.Element {
  return (
    <li class="record-outline-list__item" data-record-id={props.recordId}>
      {props.children}
    </li>
  );
}
