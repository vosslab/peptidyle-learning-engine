// library_browse_rows.tsx - Question Library RecordList presentations and result window.

import { A } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
  type ParentProps,
} from "solid-js";

import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import { BloomClassificationText } from "../components/bloom_classification";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import {
  createRecordListPresentation,
  recordListPresentationSwitcher,
} from "../components/record_list/record_list_presentation";
import {
  recordListWindow,
  recordListWindowScrollTopForRecord,
  type RecordListWindow,
} from "../components/record_list/record_list_window";
import type { RecordRegion } from "../components/record_list/region_spec";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { questionLink, webworkFormatLabel } from "./library_page_helpers";
import {
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
  type QuestionLibraryBrowseRow,
  type QuestionLibraryBrowseState,
} from "./library_page_model";
import "./library_browse_record_list.css";

const OVERSCAN_ROW_COUNT = 5;
const DRAFT_QUESTIONS_PATH = buildRoutePath("questionDrafts", {});

type LibraryBrowsePresentation = "scan" | "preview";

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

function recordId(row: QuestionLibraryBrowseRow): string {
  return row.displayId;
}

function recordListState(
  browseState: QuestionLibraryBrowseState,
  hasRows: boolean,
  onRetry: () => void,
): RecordListState {
  if (browseState.kind === "loading") {
    return hasRows
      ? { kind: "ready" }
      : { kind: "loading", label: "Loading published questions..." };
  }
  if (browseState.kind === "error") {
    if (hasRows) return { kind: "ready" };
    return {
      kind: "error",
      title: "The library could not load",
      message: "Your filters are still here. Check the connection and try again.",
      retry: onRetry,
      retryLabel: "Try again",
    };
  }
  return { kind: "ready" };
}

function selectionControl(
  row: QuestionLibraryBrowseRow,
  props: LibraryBrowseRowsProps,
): JSX.Element {
  const selected = (): boolean => props.selectedIds().has(row.displayId);
  const selectionDisabled = (): boolean =>
    props.editorBusy() ||
    (!selected() && props.selectedIds().size >= MAX_BULK_QUESTION_METADATA_ITEMS);

  return (
    <Show when={props.mayMutateLibrary}>
      <label class="library-browse-record-list__selection">
        <input
          type="checkbox"
          checked={selected()}
          disabled={selectionDisabled()}
          onChange={(event) => props.onUpdateSelection(row.displayId, event.currentTarget.checked)}
        />
        <span class="sr-only">Select {row.questionTitle}</span>
      </label>
    </Show>
  );
}

function openQuestionAction(
  row: QuestionLibraryBrowseRow,
  props: LibraryBrowseRowsProps,
): JSX.Element {
  const returnToken = props.returnTokenFor(row);
  return (
    <A
      class="quiet-link"
      href={questionLink(row, returnToken)}
      onClick={(event) => {
        if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)
          return;
        const token = new URL(event.currentTarget.href).searchParams.get(
          QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
        );
        if (token !== null) props.onSaveReturnState(token);
      }}
    >
      Open question
    </A>
  );
}

function scanRegions(
  props: LibraryBrowseRowsProps,
): ReadonlyArray<RecordRegion<QuestionLibraryBrowseRow>> {
  return [
    {
      id: "question",
      role: "identity",
      width: "minmax(16rem, 2fr)",
      align: "start",
      priority: "required",
      content: (row) => (
        <div class="library-browse-record-list__question">
          {selectionControl(row, props)}
          <div>
            <h2>{row.questionTitle}</h2>
            <p>{row.summary}</p>
          </div>
        </div>
      ),
    },
    {
      id: "classification",
      role: "status",
      width: "minmax(12rem, 1fr)",
      align: "start",
      priority: "high",
      content: (row) => (
        <div class="library-browse-record-list__classification">
          <p>{row.disciplineName}</p>
          <Show when={row.disciplineIsRetired}>
            <p class="library-browse-record-list__retired">Retired discipline</p>
          </Show>
          <Show when={row.bloom}>{(bloom) => <BloomClassificationText bloom={bloom()} />}</Show>
        </div>
      ),
    },
    {
      id: "authors",
      role: "metadata",
      width: "minmax(10rem, 1fr)",
      align: "start",
      priority: "medium",
      content: (row) => (
        <div>
          <p>Authors: {row.authorNames.join(", ")}</p>
          <Show when={webworkFormatLabel(row.questionFormat)}>
            {(format) => <p>Format: {format()}</p>}
          </Show>
        </div>
      ),
    },
    {
      id: "actions",
      role: "actions",
      width: "auto",
      align: "end",
      priority: "required",
      content: (row) => openQuestionAction(row, props),
    },
  ];
}

function previewRegions(
  props: LibraryBrowseRowsProps,
): ReadonlyArray<RecordRegion<QuestionLibraryBrowseRow>> {
  return [
    {
      id: "question-preview",
      role: "identity",
      width: "minmax(18rem, 2fr)",
      align: "start",
      priority: "required",
      content: (row) => (
        <div class="library-browse-record-list__question">
          {selectionControl(row, props)}
          <div>
            <h2>{row.questionTitle}</h2>
            <p>{row.summary}</p>
            <p class="library-browse-record-list__authors">By {row.authorNames.join(", ")}</p>
          </div>
        </div>
      ),
    },
    {
      id: "learning-metadata",
      role: "status",
      width: "minmax(14rem, 1fr)",
      align: "start",
      priority: "high",
      content: (row) => (
        <div class="library-browse-record-list__classification">
          <p>{row.disciplineName}</p>
          <Show when={row.bloom}>{(bloom) => <BloomClassificationText bloom={bloom()} />}</Show>
          <Show when={webworkFormatLabel(row.questionFormat)}>
            {(format) => <p>Format: {format()}</p>}
          </Show>
        </div>
      ),
    },
    {
      id: "question-id",
      role: "metadata",
      width: "auto",
      align: "end",
      priority: "medium",
      content: (row) => (
        <CopyableQuestionId
          questionTitle={row.questionTitle}
          displayId={row.displayId}
          presentation="compact"
        />
      ),
    },
    {
      id: "actions",
      role: "actions",
      width: "auto",
      align: "end",
      priority: "required",
      content: (row) => openQuestionAction(row, props),
    },
  ];
}

export function LibraryBrowseRows(props: ParentProps<LibraryBrowseRowsProps>): JSX.Element {
  const presentation = createRecordListPresentation<LibraryBrowsePresentation>({
    variants: [
      { id: "scan", label: "Scan" },
      { id: "preview", label: "Preview" },
    ],
  });
  const presentationSwitcher = recordListPresentationSwitcher(
    presentation,
    "Question result presentation",
  );
  const [measuredRecordHeightsPx, setMeasuredRecordHeightsPx] = createSignal<
    ReadonlyMap<string, number>
  >(new Map());
  const [focusedRecordId, setFocusedRecordId] = createSignal<string>();
  let windowElement: HTMLDivElement | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let mutationObserver: MutationObserver | undefined;

  const selectedRegions = (): ReadonlyArray<RecordRegion<QuestionLibraryBrowseRow>> => {
    const regions = presentation.variant() === "scan" ? scanRegions(props) : previewRegions(props);
    return regions;
  };
  const virtualWindow = (): RecordListWindow<QuestionLibraryBrowseRow> =>
    recordListWindow({
      records: props.displayedRows(),
      recordId,
      estimatedRecordHeightPx: props.rowHeightPx(),
      measuredRecordHeightsPx: measuredRecordHeightsPx(),
      scrollTopPx: props.scrollTop(),
      viewportHeightPx: props.viewportHeight(),
      overscanPx: props.rowHeightPx() * OVERSCAN_ROW_COUNT,
      focusedRecordId: focusedRecordId(),
    });

  function refreshMeasurements(): void {
    if (windowElement === undefined) return;
    const priorMeasurements = measuredRecordHeightsPx();
    const nextMeasurements = new Map(priorMeasurements);
    for (const element of windowElement.querySelectorAll<HTMLElement>(".record-list__row")) {
      const currentRecordId = element.dataset.recordId;
      if (currentRecordId !== undefined)
        nextMeasurements.set(currentRecordId, element.offsetHeight);
    }
    const measurementsChanged =
      nextMeasurements.size !== priorMeasurements.size ||
      [...nextMeasurements].some(
        ([currentRecordId, height]) => priorMeasurements.get(currentRecordId) !== height,
      );
    if (measurementsChanged) setMeasuredRecordHeightsPx(nextMeasurements);
  }

  function observeRows(): void {
    if (windowElement === undefined || resizeObserver === undefined) return;
    resizeObserver.disconnect();
    for (const element of windowElement.querySelectorAll<HTMLElement>(".record-list__row")) {
      resizeObserver.observe(element);
    }
    refreshMeasurements();
  }

  function setWindowElement(element: HTMLDivElement): void {
    windowElement = element;
    props.setLibraryWindow(element);
    mutationObserver?.observe(element, { childList: true, subtree: true });
    observeRows();
  }

  function handleScroll(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLDivElement)) return;
    props.onScroll(target.scrollTop, target.clientHeight);
    const current = props.browseState();
    if (
      current.kind === "ready" &&
      target.scrollTop + target.clientHeight >= target.scrollHeight - props.rowHeightPx() * 3
    ) {
      props.onNeedMore();
    }
  }

  function handleFocusIn(event: FocusEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const recordElement = target.closest<HTMLElement>("[data-record-id]");
    const nextFocusedRecordId = recordElement?.dataset.recordId;
    if (nextFocusedRecordId === undefined || windowElement === undefined) return;
    setFocusedRecordId(nextFocusedRecordId);
    const nextScrollTopPx = recordListWindowScrollTopForRecord(
      {
        records: props.displayedRows(),
        recordId,
        estimatedRecordHeightPx: props.rowHeightPx(),
        measuredRecordHeightsPx: measuredRecordHeightsPx(),
      },
      nextFocusedRecordId,
      windowElement.clientHeight,
      "nearest",
      windowElement.scrollTop,
    );
    if (nextScrollTopPx !== undefined && nextScrollTopPx !== windowElement.scrollTop) {
      windowElement.scrollTop = nextScrollTopPx;
      props.onScroll(nextScrollTopPx, windowElement.clientHeight);
    }
  }

  function handleFocusOut(): void {
    queueMicrotask(() => {
      const activeRecordElement = document.activeElement?.closest<HTMLElement>("[data-record-id]");
      setFocusedRecordId(activeRecordElement?.dataset.recordId);
    });
  }

  onMount(() => {
    resizeObserver = new ResizeObserver(refreshMeasurements);
    mutationObserver = new MutationObserver(observeRows);
    if (windowElement !== undefined) {
      mutationObserver.observe(windowElement, { childList: true, subtree: true });
      observeRows();
    }
    onCleanup(() => {
      resizeObserver?.disconnect();
      mutationObserver?.disconnect();
      props.setLibraryWindow(undefined);
    });
  });

  createEffect(() => {
    props.displayedRows();
    presentation.variant();
    measuredRecordHeightsPx();
    queueMicrotask(observeRows);
  });

  const rowsAvailable = (): boolean => props.displayedRows().length > 0;
  const recordListClass = (): string =>
    `library-browse-record-list library-browse-record-list--${presentation.variant()}`;

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
            Select loaded Questions affects only results fetched into this browser, never every
            Question in the library. Each bulk update is limited to{" "}
            {MAX_BULK_QUESTION_METADATA_ITEMS}.
          </p>
        </section>
      </Show>
      {props.children}
      <Show when={props.browseState().kind !== "initial"}>
        <Show when={presentationSwitcher}>
          {(switcher) => (
            <div
              class="library-browse-record-list__presentations"
              role="group"
              aria-label={switcher().ariaLabel}
            >
              <For each={switcher().variants}>
                {(variant) => (
                  <button
                    type="button"
                    class="quiet-action"
                    aria-pressed={switcher().selectedVariant() === variant.id}
                    onClick={() => switcher().selectVariant(variant.id)}
                  >
                    {variant.label}
                  </button>
                )}
              </For>
            </div>
          )}
        </Show>
        <Show
          when={rowsAvailable()}
          fallback={
            <>
              <RecordList
                rows={[]}
                regions={selectedRegions()}
                recordId={recordId}
                state={recordListState(props.browseState(), false, props.onRetry)}
                ariaLabel="Published questions"
                emptyState={{
                  title: "No published questions match these filters",
                  message: "Try a shorter search or choose a broader topic.",
                }}
              />
              <Show when={props.browseState().kind === "empty" && DRAFT_QUESTIONS_PATH}>
                {(path) => (
                  <A class="primary-action" href={path()} children="Create a Draft Question" />
                )}
              </Show>
            </>
          }
        >
          <div
            class="library-browse-record-list__window"
            role="region"
            aria-label="Published questions"
            tabIndex={0}
            ref={setWindowElement}
            onScroll={handleScroll}
            onFocusIn={handleFocusIn}
            onFocusOut={handleFocusOut}
          >
            <div
              class="library-browse-record-list__spacer"
              style={{ height: `${virtualWindow().topSpacerHeightPx}px` }}
            />
            <div class={recordListClass()}>
              <RecordList
                rows={virtualWindow().records}
                regions={selectedRegions()}
                recordId={recordId}
                state={recordListState(props.browseState(), true, props.onRetry)}
                ariaLabel="Published questions"
                emptyState={{ title: "No published questions match these filters" }}
              />
            </div>
            <div
              class="library-browse-record-list__spacer"
              style={{ height: `${virtualWindow().bottomSpacerHeightPx}px` }}
            />
          </div>
        </Show>
        <Show when={props.browseState().kind === "loading" && rowsAvailable()}>
          <p class="loading-state" role="status">
            Loading more published questions...
          </p>
        </Show>
        <Show when={props.browseState().kind === "error" && rowsAvailable()}>
          <section class="route-error" role="alert">
            <h2>The library could not load more questions</h2>
            <p>
              Your loaded results and selection are still here. Check the connection and try again.
            </p>
            <button class="primary-action" type="button" onClick={() => void props.onRetry()}>
              Try again
            </button>
          </section>
        </Show>
      </Show>
    </>
  );
}
