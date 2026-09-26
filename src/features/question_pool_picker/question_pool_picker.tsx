// Accessible existing-Pool selector for Blueprint Assessment authoring.

import { Match, Switch, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { QuestionPoolEditNumber } from "../../../generated/api/QuestionPoolEditNumber";
import type { QuestionPoolLibraryClient } from "../../api/question_pool_library";
import "./question_pool_picker.css";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import { RecordList, type RecordContent } from "../../components/record_list/record_list";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../../components/record_list/record_page_controls";
import { RecordSequence } from "../../components/record_list/record_sequence";

type LoadState = "loading" | "ready" | "empty" | "error";

export interface QuestionPoolPickerSelection {
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
  readonly memberCount: number;
}

export interface QuestionPoolPickerProps {
  readonly client: QuestionPoolLibraryClient;
  readonly trigger: HTMLButtonElement | undefined;
  readonly onConfirm: (selection: QuestionPoolPickerSelection) => void;
  readonly onCancel: () => void;
}

function poolLabel(summary: QuestionPoolLibrarySummary): string {
  return `${summary.metadata.title} (${summary.questionPoolId}, Edit ${summary.questionPoolEditNumber})`;
}

function poolContent(summary: QuestionPoolLibrarySummary): RecordContent {
  return {
    title: summary.metadata.title,
    description: summary.metadata.description,
    details: [
      { kind: "text", label: "Question Pool ID", value: summary.questionPoolId },
      { kind: "text", label: "Edit", value: String(summary.questionPoolEditNumber) },
      {
        kind: "text",
        label: "Members",
        value: String(summary.memberCount),
      },
    ],
    actions: [],
  };
}

function poolMemberContent(member: QuestionPoolView["members"][number]): RecordContent {
  const revision = member.publishedQuestionRevisionTuple;
  return {
    title: member.question.question_library.summary.metadata.questionTitle,
    details: [
      { kind: "text", label: "Published Question ID", value: revision.publishedQuestionId },
      { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
    ],
    actions: [],
  };
}

/** One dialog that selects a published Pool and previews its current membership. */
export function QuestionPoolPicker(props: QuestionPoolPickerProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [items, setItems] = createSignal<ReadonlyArray<QuestionPoolLibrarySummary>>([]);
  const [pageCursor, setPageCursor] = createSignal<string | null>(null);
  const [previousCursors, setPreviousCursors] = createSignal<ReadonlyArray<string | null>>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [pageSize, setPageSize] = createSignal<RecordPageSize>(50);
  const [loadingPage, setLoadingPage] = createSignal(false);
  const [selected, setSelected] = createSignal<QuestionPoolLibrarySummary>();
  const [detail, setDetail] = createSignal<QuestionPoolView>();
  const [detailState, setDetailState] = createSignal<LoadState>("empty");
  const [message, setMessage] = createSignal("Loading published Question Pools.");
  const [messageIsError, setMessageIsError] = createSignal(false);
  let detailRequest = 0;
  let listRequest = 0;
  let dialog!: HTMLDialogElement;

  function cancel(): void {
    if (dialog.open) dialog.close();
    props.onCancel();
    queueMicrotask(() => props.trigger?.focus());
  }

  async function loadPage(
    requestedCursor: string | null,
    requestedPreviousCursors: ReadonlyArray<string | null>,
  ): Promise<void> {
    const request = ++listRequest;
    setState("loading");
    setLoadingPage(true);
    setMessage("Loading published Question Pools.");
    setMessageIsError(false);
    try {
      const page = await props.client.listQuestionPools(requestedCursor ?? undefined, pageSize());
      if (request !== listRequest) return;
      setItems(page.items);
      setPageCursor(requestedCursor);
      setPreviousCursors(requestedPreviousCursors);
      setNextCursor(page.nextCursor);
      setState(page.items.length === 0 ? "empty" : "ready");
      setMessage(
        page.items.length === 0
          ? "No published Question Pools are available."
          : "Choose one published Question Pool to inspect its current membership.",
      );
      setMessageIsError(false);
    } catch {
      if (request !== listRequest) return;
      setState("error");
      setMessage("Question Pools could not load. Try again.");
      setMessageIsError(true);
    } finally {
      if (request === listRequest) setLoadingPage(false);
    }
  }

  function loadFirstPage(nextPageSize: RecordPageSize = pageSize()): void {
    if (nextPageSize !== pageSize()) setPageSize(nextPageSize);
    void loadPage(null, []);
  }

  function loadNextPage(): void {
    const cursor = nextCursor();
    if (cursor === null) return;
    void loadPage(cursor, [...previousCursors(), pageCursor()]);
  }

  function loadPreviousPage(): void {
    const cursors = previousCursors();
    const cursor = cursors.length === 0 ? undefined : cursors[cursors.length - 1];
    if (cursor === undefined) return;
    void loadPage(cursor, cursors.slice(0, -1));
  }

  async function selectPool(summary: QuestionPoolLibrarySummary): Promise<void> {
    const request = ++detailRequest;
    setSelected(summary);
    setDetail(undefined);
    setDetailState("loading");
    setMessage(`Loading ${poolLabel(summary)}.`);
    setMessageIsError(false);
    try {
      const loaded = await props.client.getQuestionPool(summary.questionPoolId);
      if (request !== detailRequest) return;
      if (
        loaded.questionPoolId !== summary.questionPoolId ||
        loaded.questionPoolEditNumber !== summary.questionPoolEditNumber
      ) {
        setDetailState("error");
        setMessage("That Question Pool changed. Reload the Pool list and choose it again.");
        setMessageIsError(true);
        return;
      }
      setDetail(loaded);
      setDetailState("ready");
      setMessage(`Selected ${loaded.questionPoolId}, Edit ${loaded.questionPoolEditNumber}.`);
      setMessageIsError(false);
    } catch {
      if (request !== detailRequest) return;
      setDetailState("error");
      setMessage("That Question Pool could not load. Choose it again or select another Pool.");
      setMessageIsError(true);
    }
  }

  function confirm(): void {
    const choice = selected();
    const loaded = detail();
    if (choice === undefined || loaded === undefined || detailState() !== "ready") {
      setMessage("Choose a Question Pool and wait for its current membership to load.");
      setMessageIsError(true);
      return;
    }
    if (dialog.open) dialog.close();
    props.trigger?.focus();
    props.onConfirm({
      questionPoolId: choice.questionPoolId,
      questionPoolEditNumber: choice.questionPoolEditNumber,
      memberCount: choice.memberCount,
    });
  }

  onMount(() => loadFirstPage());
  onCleanup(() => {
    detailRequest += 1;
    listRequest += 1;
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class="question-pool-picker-dialog"
      aria-labelledby="question-pool-picker-heading"
      aria-describedby="question-pool-picker-instructions"
      ref={(element) => {
        dialog = element;
        queueMicrotask(() => dialog.showModal());
      }}
      onCancel={(event) => {
        event.preventDefault();
        cancel();
      }}
    >
      <header class="question-pool-picker-header">
        <div>
          <p class="eyebrow">Question Pool selection</p>
          <h2 id="question-pool-picker-heading">Choose a published Question Pool</h2>
          <p id="question-pool-picker-instructions">
            Select an existing Pool by its Title and inspect its Description. Saving the Blueprint
            records the current Pool membership resolved by the server.
          </p>
        </div>
        <button class="quiet-action" type="button" onClick={cancel}>
          Close picker
        </button>
      </header>

      <p class="question-pool-picker-status" role={messageIsError() ? "alert" : "status"}>
        {message()}
      </p>

      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading published Question Pools...</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => loadFirstPage()}>
            Retry loading Question Pools
          </button>
        </Match>
        <Match when={state() === "empty"}>
          <p>No published Question Pools are available to add.</p>
        </Match>
        <Match when={state() === "ready"}>
          <div class="question-pool-picker-layout">
            <section aria-labelledby="question-pool-results-heading">
              <h3 id="question-pool-results-heading">Published Pools</h3>
              <RecordList
                rows={items()}
                content={poolContent}
                selection={{
                  kind: "radio",
                  selectedIds: () =>
                    selected() === undefined
                      ? new Set<string>()
                      : new Set([selected()!.questionPoolId]),
                  onChange: (item, checked) => {
                    if (checked) void selectPool(item);
                  },
                }}
                recordId={(item) => item.questionPoolId}
                state={{ kind: "ready" }}
                ariaLabel="Published Question Pools"
                emptyState={{ title: "No published Question Pools are available." }}
              />
              <RecordPageControls
                ariaLabel="Published Question Pool pages"
                hasPrevious={previousCursors().length > 0}
                hasNext={nextCursor() !== null}
                loading={loadingPage()}
                onPrevious={loadPreviousPage}
                onNext={loadNextPage}
                pageSize={pageSize()}
                onPageSizeChange={loadFirstPage}
              />
            </section>

            <section aria-labelledby="question-pool-detail-heading">
              <h3 id="question-pool-detail-heading">Pool members</h3>
              <Switch>
                <Match when={detailState() === "loading"}>
                  <p>Loading current Pool membership...</p>
                </Match>
                <Match when={detailState() === "error"}>
                  <button
                    type="button"
                    onClick={() => {
                      const current = selected();
                      if (current !== undefined) void selectPool(current);
                    }}
                  >
                    Retry this Pool
                  </button>
                </Match>
                <Match when={detail() !== undefined}>
                  <h4>{detail()!.metadata.title}</h4>
                  <p>{detail()!.metadata.description}</p>
                  <CourseClassificationSummary value={detail()!.metadata} />
                  <RecordSequence
                    rows={detail()?.members ?? []}
                    content={poolMemberContent}
                    recordId={(member) =>
                      `${member.publishedQuestionRevisionTuple.publishedQuestionId}:${member.publishedQuestionRevisionTuple.revisionNumber}`
                    }
                    state={{ kind: "ready" }}
                    ariaLabel="Ordered selected Pool members"
                    emptyState={{ title: "No members are in this Question Pool." }}
                  />
                </Match>
                <Match when={true}>
                  <p>Choose a Pool to inspect its ordered members.</p>
                </Match>
              </Switch>
            </section>
          </div>
        </Match>
      </Switch>

      <footer class="question-pool-picker-footer">
        <button class="quiet-action" type="button" onClick={cancel}>
          Cancel
        </button>
        <button type="button" disabled={detailState() !== "ready"} onClick={confirm}>
          Add Question Pool
        </button>
      </footer>
    </dialog>
  );
}
