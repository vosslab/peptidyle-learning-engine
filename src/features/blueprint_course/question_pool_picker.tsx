// Accessible existing-Pool selector for Blueprint Assessment authoring.

import { For, Match, Show, Switch, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionPoolEditNumber } from "../../../generated/api/QuestionPoolEditNumber";
import type { QuestionPoolLibraryClient } from "../../api/question_pool_library";
import "./question_pool_picker.css";
import { CourseClassificationSummary } from "../../components/course_classification_summary";

type LoadState = "loading" | "ready" | "empty" | "error";

export interface QuestionPoolPickerSelection {
  readonly questionPoolId: QuestionId;
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

/** One dialog that selects a published Pool and previews its current membership. */
export function QuestionPoolPicker(props: QuestionPoolPickerProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [items, setItems] = createSignal<ReadonlyArray<QuestionPoolLibrarySummary>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loadingMore, setLoadingMore] = createSignal(false);
  const [selected, setSelected] = createSignal<QuestionPoolLibrarySummary>();
  const [detail, setDetail] = createSignal<QuestionPoolView>();
  const [detailState, setDetailState] = createSignal<LoadState>("empty");
  const [message, setMessage] = createSignal("Loading published Question Pools.");
  const [messageIsError, setMessageIsError] = createSignal(false);
  let detailRequest = 0;
  let dialog!: HTMLDialogElement;

  function cancel(): void {
    if (dialog.open) dialog.close();
    props.onCancel();
    queueMicrotask(() => props.trigger?.focus());
  }

  async function load(reset: boolean): Promise<void> {
    if (reset) {
      setState("loading");
      setMessage("Loading published Question Pools.");
      setMessageIsError(false);
    } else {
      setLoadingMore(true);
    }
    try {
      const page = await props.client.listQuestionPools(
        reset ? undefined : (cursor() ?? undefined),
      );
      const nextItems = reset ? page.items : [...items(), ...page.items];
      setItems(nextItems);
      setCursor(page.nextCursor);
      setState(nextItems.length === 0 ? "empty" : "ready");
      setMessage(
        nextItems.length === 0
          ? "No published Question Pools are available."
          : "Choose one published Question Pool to inspect its current membership.",
      );
      setMessageIsError(false);
    } catch {
      if (reset) setState("error");
      setMessage("Question Pools could not load. Try again.");
      setMessageIsError(true);
    } finally {
      setLoadingMore(false);
    }
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

  onMount(() => void load(true));
  onCleanup(() => {
    detailRequest += 1;
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
          <button type="button" onClick={() => void load(true)}>
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
              <ul class="question-pool-picker-results">
                <For each={items()}>
                  {(item) => (
                    <li>
                      <label>
                        <input
                          type="radio"
                          name="question-pool"
                          checked={selected()?.questionPoolId === item.questionPoolId}
                          onInput={() => void selectPool(item)}
                        />
                        <span>
                          <strong>{item.metadata.title}</strong>
                          <small>{item.metadata.description}</small>
                          <small>
                            {item.questionPoolId}, Edit {item.questionPoolEditNumber};{" "}
                            {item.memberCount} {item.memberCount === 1 ? "member" : "members"}
                          </small>
                        </span>
                      </label>
                    </li>
                  )}
                </For>
              </ul>
              <Show when={cursor() !== null}>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={loadingMore()}
                  onClick={() => void load(false)}
                >
                  {loadingMore() ? "Loading more Pools..." : "Load more Pools"}
                </button>
              </Show>
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
                  <ol class="question-pool-picker-members">
                    <For each={detail()?.members ?? []}>
                      {(member) => (
                        <li>
                          <strong>
                            {member.question.question_library.summary.metadata.questionTitle}
                          </strong>
                          <span>
                            {member.questionRevisionTuple.questionId}, Revision{" "}
                            {member.questionRevisionTuple.revisionNumber}
                          </span>
                        </li>
                      )}
                    </For>
                  </ol>
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
