import { For, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import type { RecordRegion } from "./region_spec";

export type RecordSequenceProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly regions: ReadonlyArray<RecordRegion<Row>>;
  readonly recordId: (row: Row) => string;
  readonly state: RecordCollectionState;
  readonly ariaLabel: string;
  readonly emptyState: RecordCollectionEmptyState;
};

/** Native ordered records. Callers keep move controls and persistence inside their regions. */
export function RecordSequence<Row>(props: RecordSequenceProps<Row>): JSX.Element {
  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <ol class="record-sequence" aria-label={props.ariaLabel}>
        <For each={props.rows}>
          {(row) => (
            <li class="record-sequence__item" data-record-id={props.recordId(row)}>
              <For each={props.regions}>
                {(region) => (
                  <div
                    class="record-sequence__region"
                    classList={{
                      "record-sequence__region--identity": region.role === "identity",
                      "record-sequence__region--actions": region.role === "actions",
                    }}
                    data-record-region-id={region.id}
                  >
                    {region.content(row)}
                  </div>
                )}
              </For>
            </li>
          )}
        </For>
      </ol>
    </RecordCollectionStateView>
  );
}
