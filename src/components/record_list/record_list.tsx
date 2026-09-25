import { For, Show, type JSX } from "solid-js";

import {
  RecordCollectionStateView,
  type RecordCollectionEmptyState,
  type RecordCollectionState,
} from "./record_collection_state";
import type { RecordRegion } from "./region_spec";
import "./record_list.css";

export type RecordListState = RecordCollectionState;
export type RecordListEmptyState = RecordCollectionEmptyState;

export type RecordListProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly regions: ReadonlyArray<RecordRegion<Row>>;
  readonly recordId: (row: Row) => string;
  readonly state: RecordListState;
  readonly ariaLabel: string;
  readonly emptyState: RecordListEmptyState;
};

function gridTemplateColumns<Row>(
  regions: ReadonlyArray<RecordRegion<Row>>,
  hiddenPriorities: ReadonlyArray<"high" | "medium" | "low">,
): string {
  const visibleRegions = regions.filter(
    (region) => region.priority === "required" || !hiddenPriorities.includes(region.priority),
  );
  const tracks = visibleRegions.map((region) => region.width);
  const template = tracks.join(" ");
  return template;
}

/**
 * Renders consistently aligned records and their shared loading, empty, and error states.
 * Presentation variants, reordering, and windowing compose outside this focused core.
 */
export function RecordList<Row>(props: RecordListProps<Row>): JSX.Element {
  const allTemplateColumns = (): string => gridTemplateColumns(props.regions, []);
  const withoutLowTemplateColumns = (): string => gridTemplateColumns(props.regions, ["low"]);
  const withoutMediumTemplateColumns = (): string =>
    gridTemplateColumns(props.regions, ["low", "medium"]);
  const requiredTemplateColumns = (): string =>
    gridTemplateColumns(props.regions, ["low", "medium", "high"]);

  return (
    <RecordCollectionStateView
      state={props.state}
      isEmpty={props.rows.length === 0}
      emptyState={props.emptyState}
    >
      <div
        class="record-list"
        role="list"
        aria-label={props.ariaLabel}
        style={{
          "--record-list-columns-all": allTemplateColumns(),
          "--record-list-columns-without-low": withoutLowTemplateColumns(),
          "--record-list-columns-without-medium": withoutMediumTemplateColumns(),
          "--record-list-columns-required": requiredTemplateColumns(),
        }}
      >
        <Show when={props.regions.some((region) => region.header !== undefined)}>
          <div class="record-list__header" aria-hidden="true">
            <For each={props.regions}>
              {(region) => (
                <div
                  class="record-list__header-region"
                  data-record-region-id={region.id}
                  data-record-list-priority={region.priority}
                  style={{
                    "align-self": region.align,
                    "justify-self": region.align,
                  }}
                >
                  {region.header}
                </div>
              )}
            </For>
          </div>
        </Show>
        <For each={props.rows}>
          {(row) => (
            <article class="record-list__row" role="listitem" data-record-id={props.recordId(row)}>
              <For each={props.regions}>
                {(region) => (
                  <div
                    class="record-list__region"
                    classList={{
                      "record-list__region--identity": region.role === "identity",
                      "record-list__region--metadata": region.role === "metadata",
                      "record-list__region--status": region.role === "status",
                      "record-list__region--actions": region.role === "actions",
                    }}
                    data-record-region-id={region.id}
                    data-record-list-priority={region.priority}
                    style={{
                      "align-self": region.align,
                      "justify-self": region.align,
                    }}
                  >
                    {region.content(row)}
                  </div>
                )}
              </For>
            </article>
          )}
        </For>
      </div>
    </RecordCollectionStateView>
  );
}
