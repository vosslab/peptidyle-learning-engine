import { For, Show, type JSX } from "solid-js";

import type { RecordRegion } from "./region_spec";
import "./record_list.css";

export type RecordListState =
  | { readonly kind: "ready" }
  | { readonly kind: "loading"; readonly label?: string }
  | {
      readonly kind: "error";
      readonly title?: string;
      readonly message: string;
      readonly retry?: () => void;
      readonly retryLabel?: string;
    };

export type RecordListEmptyState = {
  readonly title: string;
  readonly message?: string;
};

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

function loadingLabel(state: RecordListState): string {
  if (state.kind !== "loading") return "Loading records...";
  const label = state.label ?? "Loading records...";
  return label;
}

function errorTitle(state: RecordListState): string {
  if (state.kind !== "error") return "Records unavailable";
  const title = state.title ?? "Records unavailable";
  return title;
}

function errorMessage(state: RecordListState): string {
  if (state.kind !== "error") return "Records could not be loaded.";
  return state.message;
}

function errorRetryLabel(state: RecordListState): string {
  if (state.kind !== "error") return "Retry";
  const retryLabel = state.retryLabel ?? "Retry";
  return retryLabel;
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
    <Show
      when={props.state.kind === "ready"}
      fallback={
        <Show
          when={props.state.kind === "loading"}
          fallback={
            <section class="record-list__state record-list__state--error" role="alert">
              <h2>{errorTitle(props.state)}</h2>
              <p>{errorMessage(props.state)}</p>
              <Show when={props.state.kind === "error" && props.state.retry !== undefined}>
                <button
                  class="quiet-action"
                  type="button"
                  onClick={() => props.state.kind === "error" && props.state.retry?.()}
                >
                  {errorRetryLabel(props.state)}
                </button>
              </Show>
            </section>
          }
        >
          <p class="record-list__state record-list__state--loading" role="status">
            {loadingLabel(props.state)}
          </p>
        </Show>
      }
    >
      <Show
        when={props.rows.length > 0}
        fallback={
          <section class="record-list__state record-list__state--empty" role="status">
            <h2>{props.emptyState.title}</h2>
            <Show when={props.emptyState.message}>{(message) => <p>{message()}</p>}</Show>
          </section>
        }
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
              <article
                class="record-list__row"
                role="listitem"
                data-record-id={props.recordId(row)}
              >
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
      </Show>
    </Show>
  );
}
