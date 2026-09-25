import { For, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";

export type RecordDetailListProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly renderRecord: (row: Row) => JSX.Element;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
};

/** Full review, discussion, form, or comparison records without compact-row rules. */
export function RecordDetailList<Row>(props: RecordDetailListProps<Row>): JSX.Element {
  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <section class="record-detail-list" role="list" aria-label={props.ariaLabel}>
        <For each={props.rows}>
          {(row) => (
            <article
              class="record-detail-list__item"
              role="listitem"
              data-record-id={props.recordId(row)}
            >
              {props.renderRecord(row)}
            </article>
          )}
        </For>
      </section>
    </RecordCollectionStateView>
  );
}
