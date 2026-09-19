// library_browse_rows.tsx - virtual Question Library result window and row actions.

import { A } from "@solidjs/router";
import { For, Show, type Accessor, type JSX, type ParentProps } from "solid-js";

import { BloomClassificationText } from "../components/bloom_classification";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { questionLink, webworkFormatLabel } from "./library_page_helpers";
import {
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  questionLibraryBrowseVirtualWindow,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryBrowseState,
} from "./library_page_model";

const OVERSCAN_ROWS = 5;
const DRAFT_QUESTIONS_PATH = buildRoutePath("questionDrafts", {});

export interface LibraryBrowseRowsProps {
  readonly mayMutateLibrary: boolean;
  readonly displayedRows: Accessor<ReadonlyArray<QuestionLibraryBrowseRow>>;
  readonly selectedIds: Accessor<ReadonlySet<string>>;
  readonly editorBusy: Accessor<boolean>;
  readonly browseState: Accessor<QuestionLibraryBrowseState>;
  readonly scrollTop: Accessor<number>;
  readonly viewportHeight: Accessor<number>;
  readonly rowHeightPx: Accessor<number>;
  readonly setLibraryWindow: (element: HTMLDivElement | undefined) => void;
  readonly onScroll: (scrollTop: number, viewportHeight: number) => void;
  readonly onNeedMore: () => void;
  readonly onRetry: () => void;
  readonly onUpdateSelection: (questionId: string, checked: boolean) => void;
  readonly onSelectLoaded: () => void;
  readonly onClearSelection: () => void;
  readonly onOpenMetadataEditor: () => void;
  readonly returnTokenFor: (row: QuestionLibraryBrowseRow) => string;
  readonly onSaveReturnState: (token: string) => void;
}

export function LibraryBrowseRows(props: ParentProps<LibraryBrowseRowsProps>): JSX.Element {
  const virtualWindow = (): Readonly<{
    readonly offset: number;
    readonly rows: ReadonlyArray<QuestionLibraryBrowseRow>;
  }> =>
    questionLibraryBrowseVirtualWindow(
      props.displayedRows(),
      props.scrollTop(),
      props.viewportHeight(),
      props.rowHeightPx(),
      OVERSCAN_ROWS,
    );

  function handleScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLDivElement)) {
      return;
    }
    props.onScroll(target.scrollTop, target.clientHeight);
    const current = props.browseState();
    if (
      current.kind === "ready" &&
      target.scrollTop + target.clientHeight >= target.scrollHeight - props.rowHeightPx() * 3
    ) {
      props.onNeedMore();
    }
  }

  return (
    <>
      <Show
        when={
          props.mayMutateLibrary &&
          (props.displayedRows().length > 0 || props.selectedIds().size > 0)
        }
      >
        <section class="question-library-bulk-toolbar" aria-label="Bulk Question actions">
          <p aria-live="polite">
            <strong>{props.selectedIds().size} selected</strong> from {props.displayedRows().length}{" "}
            loaded Questions
          </p>
          <div>
            <button
              type="button"
              class="quiet-action"
              disabled={props.editorBusy() || props.displayedRows().length === 0}
              onClick={props.onSelectLoaded}
            >
              Select loaded Questions
            </button>
            <button
              type="button"
              class="quiet-action"
              disabled={props.editorBusy() || props.selectedIds().size === 0}
              onClick={props.onClearSelection}
            >
              Clear selection
            </button>
            <button
              type="button"
              class="primary-action"
              disabled={props.editorBusy() || props.selectedIds().size === 0}
              onClick={() => void props.onOpenMetadataEditor()}
            >
              Edit shared metadata
            </button>
          </div>
          <p class="question-library-bulk-help">
            Select loaded Questions affects only results fetched into this browser, never every
            Question in the library. Each bulk update is limited to{" "}
            {MAX_BULK_QUESTION_METADATA_ITEMS}.
          </p>
        </section>
      </Show>
      {props.children}
      <Show when={props.browseState().kind === "error"}>
        <section class="route-error" role="alert">
          <h2>The library could not load</h2>
          <p>Your filters are still here. Check the connection and try again.</p>
          <button class="primary-action" type="button" onClick={() => void props.onRetry()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={props.browseState().kind === "empty"}>
        <section class="empty-state" aria-label="No matching published questions">
          <h2>No published questions match these filters</h2>
          <p>Use the global Question Library to find and reuse published Questions.</p>
          <p>Try a shorter search or choose a broader topic.</p>
          <Show when={DRAFT_QUESTIONS_PATH}>
            {(path) => (
              <A class="primary-action" href={path()} children="Create a Draft Question" />
            )}
          </Show>
        </section>
      </Show>
      <Show when={props.displayedRows().length > 0}>
        <div
          class="question-library-window"
          role="region"
          aria-label="Published questions"
          tabIndex={0}
          ref={props.setLibraryWindow}
          onScroll={handleScroll}
          style={`--ple-question-library-loaded-block-size:${props.displayedRows().length * props.rowHeightPx()}px`}
        >
          <div
            style={{
              height: `${props.displayedRows().length * props.rowHeightPx()}px`,
              position: "relative",
            }}
          >
            <div
              class="question-library-window-slice"
              style={{ top: `${virtualWindow().offset}px` }}
            >
              <For each={virtualWindow().rows}>
                {(row) => (
                  <LibraryBrowseRow
                    row={row}
                    rowHeightPx={props.rowHeightPx()}
                    mayMutateLibrary={props.mayMutateLibrary}
                    selected={props.selectedIds().has(row.displayId)}
                    selectionDisabled={
                      props.editorBusy() ||
                      (!props.selectedIds().has(row.displayId) &&
                        props.selectedIds().size >= MAX_BULK_QUESTION_METADATA_ITEMS)
                    }
                    returnToken={props.returnTokenFor(row)}
                    onUpdateSelection={props.onUpdateSelection}
                    onSaveReturnState={props.onSaveReturnState}
                  />
                )}
              </For>
            </div>
          </div>
          <Show when={props.browseState().kind === "loading"}>
            <p class="loading-state" role="status">
              Loading more published questions...
            </p>
          </Show>
        </div>
      </Show>
      <Show when={props.browseState().kind === "loading" && props.displayedRows().length === 0}>
        <p class="loading-state" role="status">
          Loading published questions...
        </p>
      </Show>
    </>
  );
}

function LibraryBrowseRow(props: {
  readonly row: QuestionLibraryBrowseRow;
  readonly rowHeightPx: number;
  readonly mayMutateLibrary: boolean;
  readonly selected: boolean;
  readonly selectionDisabled: boolean;
  readonly returnToken: string;
  readonly onUpdateSelection: (questionId: string, checked: boolean) => void;
  readonly onSaveReturnState: (token: string) => void;
}): JSX.Element {
  return (
    <article class="question-library-row" style={{ height: `${props.rowHeightPx}px` }}>
      <Show when={props.mayMutateLibrary}>
        <label class="question-library-row-selection">
          <input
            type="checkbox"
            checked={props.selected}
            disabled={props.selectionDisabled}
            onChange={(event) =>
              props.onUpdateSelection(props.row.displayId, event.currentTarget.checked)
            }
          />
          <span class="sr-only">Select {props.row.questionTitle}</span>
        </label>
      </Show>
      <h2>{props.row.questionTitle}</h2>
      <p class="question-library-row-summary">{props.row.summary}</p>
      <Show when={props.row.bloom}>
        {(bloom) => (
          <p class="question-library-row-bloom">
            <BloomClassificationText bloom={bloom()} />
          </p>
        )}
      </Show>
      <p class="question-library-row-authors" aria-label="Question Authors">
        Authors: {props.row.authorNames.join(", ")}
        <span>
          {" "}
          · Discipline: {props.row.disciplineName}
          <Show when={props.row.disciplineIsRetired}> (retired)</Show>
        </span>
        <Show when={webworkFormatLabel(props.row.questionFormat)}>
          {(format) => <> · Format: {format()}</>}
        </Show>
      </p>
      <CopyableQuestionId
        questionTitle={props.row.questionTitle}
        displayId={props.row.displayId}
        presentation="compact"
      />
      <A
        class="quiet-link"
        href={questionLink(props.row, props.returnToken)}
        onClick={(event) => {
          if (
            event.button !== 0 ||
            event.metaKey ||
            event.ctrlKey ||
            event.shiftKey ||
            event.altKey
          ) {
            return;
          }
          const token = new URL(event.currentTarget.href).searchParams.get(
            QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
          );
          if (token !== null) props.onSaveReturnState(token);
        }}
      >
        Open question
      </A>
    </article>
  );
}
