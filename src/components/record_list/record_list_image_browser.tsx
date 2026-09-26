import { For, createSignal, type JSX } from "solid-js";

import {
  RecordList,
  type RecordContent,
  type RecordListEmptyState,
  type RecordListSelection,
  type RecordListState,
} from "./record_list";

export type RecordListImageBrowserProps<Row> = {
  readonly rows: ReadonlyArray<Row>;
  readonly recordId: (row: Row) => string;
  readonly state: RecordListState;
  readonly ariaLabel: string;
  readonly emptyState: RecordListEmptyState;
  /** One ordinary adapter supplies image, name, description, and any actions. */
  readonly content: (row: Row) => RecordContent;
  readonly selection?: RecordListSelection<Row>;
};

type ImageBrowserPresentation = "gallery" | "list";

/**
 * Displays the same ordinary RecordList content as a visual Gallery or compact List.
 * The browser owns only its local presentation choice; callers retain record selection.
 */
export function RecordListImageBrowser<Row>(props: RecordListImageBrowserProps<Row>): JSX.Element {
  const [presentation, setPresentation] = createSignal<ImageBrowserPresentation>("gallery");
  const presentations: ReadonlyArray<{
    readonly id: ImageBrowserPresentation;
    readonly label: string;
  }> = [
    { id: "gallery", label: "Gallery" },
    { id: "list", label: "List" },
  ];

  return (
    <section class="record-list-image-browser" data-record-list-presentation={presentation()}>
      <div
        aria-label="Record presentation"
        class="record-list-image-browser__switcher"
        role="group"
      >
        <For each={presentations}>
          {(option) => (
            <button
              aria-pressed={presentation() === option.id}
              class="quiet-action"
              type="button"
              onClick={() => setPresentation(option.id)}
            >
              {option.label}
            </button>
          )}
        </For>
      </div>
      <RecordList
        ariaLabel={props.ariaLabel}
        content={props.content}
        emptyState={props.emptyState}
        presentation={presentation()}
        recordId={props.recordId}
        rows={props.rows}
        selection={props.selection}
        state={props.state}
      />
    </section>
  );
}
