// library_browse_rows.tsx - one selectable Question Library scan for the current page.

import { A } from "@solidjs/router";
import { Show, onCleanup, onMount, type JSX, type ParentProps } from "solid-js";

import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import {
  RecordList,
  type RecordContent,
  type RecordFact,
  type RecordListSelection,
  type RecordListState,
} from "../components/record_list/record_list";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { questionLink, webworkFormatLabel } from "./library_page_helpers";
import {
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryBrowseState,
} from "./library_page_model";
import "./library_browse_record_list.css";

const DRAFT_QUESTIONS_PATH = buildRoutePath("questionDrafts", {});

export interface LibraryBrowseRowsProps {
  readonly mayMutateLibrary: boolean;
  readonly displayedRows: () => ReadonlyArray<QuestionLibraryBrowseRow>;
  readonly selectedIds: () => ReadonlySet<string>;
  readonly editorBusy: () => boolean;
  readonly browseState: () => QuestionLibraryBrowseState;
  readonly setLibraryWindow: (element: HTMLDivElement | undefined) => void;
  readonly onScroll: (scrollTop: number) => void;
  readonly onRetry: () => void;
  readonly onUpdateSelection: (questionId: string, checked: boolean) => void;
  readonly onSelectLoaded: () => void;
  readonly onClearSelection: () => void;
  readonly onOpenMetadataEditor: () => void;
  readonly returnTokenFor: (row: QuestionLibraryBrowseRow) => string;
  readonly onSaveReturnState: (token: string) => void;
}

function recordId(row: QuestionLibraryBrowseRow): string {
  return row.displayId;
}

function recordListState(
  browseState: QuestionLibraryBrowseState,
  onRetry: () => void,
): RecordListState {
  if (browseState.kind === "loading") {
    return { kind: "loading", label: "Loading published questions..." };
  }
  if (browseState.kind === "error") {
    return {
      kind: "error",
      title: "The library could not load",
      message: "Your filters and selection are still here. Check the connection and try again.",
      retry: onRetry,
      retryLabel: "Try again",
    };
  }
  return { kind: "ready" };
}

function questionDetails(row: QuestionLibraryBrowseRow): ReadonlyArray<RecordFact> {
  const details: RecordFact[] = [
    {
      kind: "text",
      label: "Authors",
      value: row.authorNames.length > 0 ? row.authorNames.join(", ") : "None",
    },
    { kind: "text", label: "Discipline", value: row.disciplineName },
  ];
  if (row.disciplineIsRetired) {
    details.push({ kind: "text", label: "Discipline status", value: "Retired discipline" });
  }
  if (row.bloom !== null) {
    details.push({
      kind: "text",
      label: "Bloom",
      value: `${row.bloom.cognitiveProcess} / ${row.bloom.knowledgeDimension}`,
    });
  }
  const format = webworkFormatLabel(row.questionFormat);
  if (format !== null) {
    details.push({ kind: "text", label: "Format", value: format });
  }
  details.push({
    kind: "questionId",
    questionTitle: row.questionTitle,
    displayId: row.displayId,
  });
  return details;
}

export function LibraryBrowseRows(props: ParentProps<LibraryBrowseRowsProps>): JSX.Element {
  let windowElement: HTMLDivElement | undefined;

  function questionContent(row: QuestionLibraryBrowseRow): RecordContent {
    const summary = row.summary.trim();
    const returnToken = props.returnTokenFor(row);
    return {
      title: row.questionTitle,
      description: summary.length > 0 ? summary : undefined,
      details: questionDetails(row),
      actions: [
        {
          id: "open",
          kind: "link",
          label: "Open",
          href: questionLink(row, returnToken),
          title: `Open ${row.questionTitle}`,
          onFollow: (event): void => {
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
          },
        },
      ],
    };
  }

  const selection = (): RecordListSelection<QuestionLibraryBrowseRow> | undefined => {
    if (!props.mayMutateLibrary) return undefined;
    return {
      kind: "checkbox",
      selectedIds: props.selectedIds,
      disabled: (row) =>
        props.editorBusy() ||
        (!props.selectedIds().has(row.displayId) &&
          props.selectedIds().size >= MAX_BULK_QUESTION_METADATA_ITEMS),
      onChange: (row, checked) => props.onUpdateSelection(row.displayId, checked),
    };
  };

  function setWindowElement(element: HTMLDivElement): void {
    windowElement = element;
    props.setLibraryWindow(element);
  }

  function handleScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLDivElement)) return;
    props.onScroll(target.scrollTop);
  }

  function keepFocusedRecordVisible(event: FocusEvent): void {
    const target = event.target;
    if (!(target instanceof Element) || windowElement === undefined) return;
    const recordElement = target.closest<HTMLElement>("[data-record-id]");
    if (recordElement === null) return;
    const windowRect = windowElement.getBoundingClientRect();
    const recordRect = recordElement.getBoundingClientRect();
    if (recordRect.top < windowRect.top) {
      windowElement.scrollTop -= windowRect.top - recordRect.top;
    } else if (recordRect.bottom > windowRect.bottom) {
      windowElement.scrollTop += recordRect.bottom - windowRect.bottom;
    }
    props.onScroll(windowElement.scrollTop);
  }

  onMount(() => {
    onCleanup(() => {
      props.setLibraryWindow(undefined);
    });
  });

  const rowsAvailable = (): boolean => props.displayedRows().length > 0;
  const listSelection = selection;

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
            Questions on this page
          </p>
          <div>
            <button
              type="button"
              class="quiet-action"
              disabled={props.editorBusy() || !rowsAvailable()}
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
            Select loaded Questions affects only this page, never every Question in the library.
            Each bulk update is limited to {MAX_BULK_QUESTION_METADATA_ITEMS}.
          </p>
        </section>
      </Show>
      {props.children}
      <Show when={props.browseState().kind !== "initial"}>
        <div
          class="library-browse-record-list__window"
          role="region"
          aria-label="Published questions"
          tabIndex={0}
          ref={setWindowElement}
          onScroll={handleScroll}
          onFocusIn={keepFocusedRecordVisible}
        >
          <RecordList
            rows={props.displayedRows()}
            content={questionContent}
            recordId={recordId}
            selection={listSelection()}
            state={recordListState(props.browseState(), props.onRetry)}
            ariaLabel="Published questions"
            emptyState={{
              title: "No published questions match these filters",
              message: "Try a shorter search or choose a broader topic.",
            }}
          />
        </div>
        <Show when={props.browseState().kind === "empty" && DRAFT_QUESTIONS_PATH}>
          {(path) => (
            <A class="primary-action" href={path()}>
              Create a Draft Question
            </A>
          )}
        </Show>
      </Show>
    </>
  );
}
