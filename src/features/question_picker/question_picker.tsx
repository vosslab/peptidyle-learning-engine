// question_picker.tsx - accessible shared Question ID selector for Instructor workflows.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { QuestionSearchQuery, QuestionSearchResult } from "../../pages/library_page_model";
import { EMPTY_QUESTION_SEARCH_QUERY } from "../../pages/library_page_model";
import "./question_picker.css";
import {
  QuestionPickerSession,
  moveQuestionPickerSelection,
  questionPickerSelection,
  toggleQuestionPickerSelection,
  type QuestionPickerCurationActions,
  type QuestionPickerSelection,
  type QuestionPickerSelectionMode,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
  type QuestionPickerState,
} from "./question_picker_model";

export interface QuestionPickerProps {
  readonly repository: QuestionPickerSourceRepository;
  readonly sources: ReadonlyArray<QuestionPickerSource>;
  readonly mode: QuestionPickerSelectionMode;
  /** Required per destination: Question Folders use 200; Assignments use their remaining cap. */
  readonly maximumSelection: number;
  readonly onConfirm: (selection: QuestionPickerSelection) => void;
  readonly onCancel: () => void;
  readonly trigger: HTMLButtonElement | undefined;
  readonly curationActions?: QuestionPickerCurationActions;
  readonly confirmLabel?: string;
  /** Optional destination-specific explanation of how picker selection proceeds. */
  readonly instructions?: string;
  readonly title?: string;
}

function sourceKey(source: QuestionPickerSource): string {
  if (source.kind === "folder") return `folder:${source.folder}`;
  if (source.kind === "retainedAssignment") {
    return `retained:${source.retainedAssignment.course}:${source.retainedAssignment.assignment}`;
  }
  return source.kind;
}

function sourceFromKey(
  sources: ReadonlyArray<QuestionPickerSource>,
  key: string,
): QuestionPickerSource | undefined {
  return sources.find((source) => sourceKey(source) === key);
}

function rowIsSelected(selection: QuestionPickerSelection, row: QuestionSearchResult): boolean {
  return selection.questionIds.includes(row.displayId);
}

function selectedCopy(
  selection: QuestionPickerSelection,
  mode: QuestionPickerSelectionMode,
): string {
  if (mode === "none") return "Browse questions and open a question for its full details.";
  if (selection.questionIds.length === 0)
    return "Select question results to prepare an ordered list.";
  if (mode === "one") return `Selected ${selection.questions[0]?.row.title ?? "question"}.`;
  return `${selection.questionIds.length} questions selected in order.`;
}

function facetValues(
  state: QuestionPickerState,
  facet:
    | "authorName"
    | "backend"
    | "tag"
    | "questionType"
    | "classification"
    | "capability"
    | "questionLicense"
    | "evidence"
    | "usedInMyCourses",
): ReadonlyArray<{ readonly value: string; readonly count: number }> {
  return state.aggregates.filter((aggregate) => aggregate.facet === facet);
}

/**
 * One native dialog with source, D1 filters, result selection, and an ordered
 * tray. Parents own curation persistence and receive public Question IDs only.
 */
export function QuestionPicker(props: QuestionPickerProps): JSX.Element {
  const [source, setSource] = createSignal<QuestionPickerSource | undefined>(props.sources[0]);
  const [query, setQuery] = createSignal<QuestionSearchQuery>(EMPTY_QUESTION_SEARCH_QUERY);
  const [state, setState] = createSignal<QuestionPickerState>({
    kind: "loading",
    rows: [],
    aggregates: [],
    nextCursor: null,
  });
  const [selection, setSelection] = createSignal<QuestionPickerSelection>(
    questionPickerSelection(props.mode, props.maximumSelection, []),
  );
  const [selectionMessage, setSelectionMessage] = createSignal(
    selectedCopy(selection(), props.mode),
  );
  const session = new QuestionPickerSession(props.repository, setState);
  let dialog!: HTMLDialogElement;

  function updateSelection(next: QuestionPickerSelection): void {
    setSelection(next);
    setSelectionMessage(selectedCopy(next, props.mode));
  }

  function resultRows(): ReadonlyArray<QuestionSearchResult> {
    const current = state();
    return current.kind === "empty" ? [] : current.rows;
  }

  function hasNextPage(): boolean {
    const current = state();
    return current.kind === "ready" && current.nextCursor !== null;
  }

  function updateQuery(change: Partial<QuestionSearchQuery>): void {
    setQuery((current) => ({ ...current, ...change }));
  }

  function loadCurrent(): void {
    const currentSource = source();
    if (currentSource === undefined) return;
    void session.reset(currentSource, query());
  }

  function selectSource(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLSelectElement)) return;
    const next = sourceFromKey(props.sources, target.value);
    if (next === undefined) return;
    setSource(next);
    void session.reset(next, query());
  }

  function toggleRow(row: QuestionSearchResult, checked: boolean): void {
    try {
      updateSelection(
        toggleQuestionPickerSelection(
          props.mode,
          props.maximumSelection,
          selection(),
          row,
          checked,
        ),
      );
    } catch (error: unknown) {
      setSelectionMessage(
        error instanceof Error ? error.message : "That question could not be selected.",
      );
    }
  }

  function cancel(): void {
    if (dialog.open) dialog.close();
    props.onCancel();
    queueMicrotask(() => props.trigger?.focus());
  }

  function confirm(): void {
    if (props.mode !== "none" && selection().questions.length === 0) {
      setSelectionMessage("Select at least one question before continuing.");
      return;
    }
    if (dialog.open) dialog.close();
    props.trigger?.focus();
    props.onConfirm(selection());
  }

  onMount(() => {
    const initial = source();
    if (initial !== undefined) void session.reset(initial, query());
  });

  onCleanup(() => {
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class="question-picker-dialog"
      aria-labelledby="question-picker-heading"
      aria-describedby="question-picker-instructions"
      ref={(element) => {
        dialog = element;
        queueMicrotask(() => {
          dialog.showModal();
        });
      }}
      onCancel={(event) => {
        event.preventDefault();
        cancel();
      }}
    >
      <header class="question-picker-header">
        <div>
          <p class="eyebrow">Question selection</p>
          <h2 id="question-picker-heading">{props.title ?? "Choose published questions"}</h2>
          <p id="question-picker-instructions">
            {props.instructions ??
              "Choose a source, refine the current library result, then add questions in the order you want to use them."}
          </p>
        </div>
        <button class="quiet-action" type="button" onClick={cancel}>
          Close picker
        </button>
      </header>

      <form
        class="question-picker-controls"
        onSubmit={(event) => {
          event.preventDefault();
          loadCurrent();
        }}
      >
        <label>
          Question source
          <select
            value={source() === undefined ? "" : sourceKey(source()!)}
            onChange={selectSource}
          >
            <For each={props.sources}>
              {(candidate) => <option value={sourceKey(candidate)}>{candidate.label}</option>}
            </For>
          </select>
        </label>
        <label class="question-picker-search-control">
          Search questions
          <input
            ref={(element) => queueMicrotask(() => element.focus())}
            type="search"
            value={query().search}
            maxlength={256}
            onInput={(event) => updateQuery({ search: event.currentTarget.value })}
            placeholder="Title, concept, or tag"
          />
        </label>
        <label>
          Question Author
          <select
            value={query().authorName ?? ""}
            onChange={(event) => updateQuery({ authorName: event.currentTarget.value || null })}
          >
            <option value="">All Question Authors</option>
            <For each={facetValues(state(), "authorName")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Backend
          <select
            value={query().backend ?? ""}
            onChange={(event) => updateQuery({ backend: event.currentTarget.value || null })}
          >
            <option value="">All backends</option>
            <For each={facetValues(state(), "backend")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Question Type
          <select
            value={query().questionType ?? ""}
            onChange={(event) => updateQuery({ questionType: event.currentTarget.value || null })}
          >
            <option value="">All Question Types</option>
            <For each={facetValues(state(), "questionType")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Classification
          <select
            value={query().classification ?? ""}
            onChange={(event) => updateQuery({ classification: event.currentTarget.value || null })}
          >
            <option value="">All classifications</option>
            <For each={facetValues(state(), "classification")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Capability
          <select
            value={query().capability ?? ""}
            onChange={(event) => updateQuery({ capability: event.currentTarget.value || null })}
          >
            <option value="">All capabilities</option>
            <For each={facetValues(state(), "capability")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Question License
          <select
            value={query().questionLicense ?? ""}
            onChange={(event) =>
              updateQuery({ questionLicense: event.currentTarget.value || null })
            }
          >
            <option value="">All Question Licenses</option>
            <For each={facetValues(state(), "questionLicense")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Evidence
          <select
            value={query().evidence ?? ""}
            onChange={(event) => updateQuery({ evidence: event.currentTarget.value || null })}
          >
            <option value="">Any evidence state</option>
            <For each={facetValues(state(), "evidence")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          My course use
          <select
            value={query().usedInMyCourses ?? ""}
            onChange={(event) =>
              updateQuery({ usedInMyCourses: event.currentTarget.value || null })
            }
          >
            <option value="">Any course use</option>
            <For each={facetValues(state(), "usedInMyCourses")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Tag
          <select
            value={query().tag ?? ""}
            onChange={(event) => updateQuery({ tag: event.currentTarget.value || null })}
          >
            <option value="">All tags</option>
            <For each={facetValues(state(), "tag")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <button class="primary-action" type="submit" disabled={source() === undefined}>
          Search questions
        </button>
      </form>

      <p class="question-picker-status" role="status" aria-live="polite">
        {selectionMessage()}
      </p>

      <Show when={state().kind === "error"}>
        <section class="route-error" role="alert">
          <h3>Question results could not load</h3>
          <p>
            Your search, filters, and selected questions remain available. Try loading this source
            again.
          </p>
          <button class="primary-action" type="button" onClick={() => void session.retry()}>
            Retry source
          </button>
        </section>
      </Show>
      <Show when={state().kind === "empty"}>
        <section class="empty-state">
          <h3>No questions match this source and filter</h3>
          <p>Use a shorter search or choose a broader source.</p>
        </section>
      </Show>
      <Show when={resultRows().length > 0}>
        <section class="question-picker-results" aria-label="Question results">
          <h3>Current results</h3>
          <ul>
            <For each={resultRows()}>
              {(row) => (
                <li>
                  <Show
                    when={props.mode !== "none"}
                    fallback={
                      <article class="question-picker-result">
                        <span>
                          <strong>{row.title}</strong>
                          <span>{row.summary}</span>
                          <small>{`${row.displayId} - ${row.classifications.join(" / ") || "No classification"}`}</small>
                        </span>
                      </article>
                    }
                  >
                    <label class="question-picker-result">
                      <input
                        type={props.mode === "one" ? "radio" : "checkbox"}
                        name={props.mode === "one" ? "question-picker-choice" : undefined}
                        checked={rowIsSelected(selection(), row)}
                        onChange={(event) => toggleRow(row, event.currentTarget.checked)}
                      />
                      <span>
                        <strong>{row.title}</strong>
                        <span>{row.summary}</span>
                        <small>{`${row.displayId} - ${row.classifications.join(" / ") || "No classification"}`}</small>
                      </span>
                    </label>
                  </Show>
                </li>
              )}
            </For>
          </ul>
          <Show when={hasNextPage()}>
            <button class="quiet-action" type="button" onClick={() => void session.loadNext()}>
              Load more results
            </button>
          </Show>
        </section>
      </Show>

      <Show when={props.mode !== "none"}>
        <section class="question-picker-tray" aria-labelledby="question-picker-tray-heading">
          <h3 id="question-picker-tray-heading">Selected questions</h3>
          <Show
            when={selection().questions.length > 0}
            fallback={
              <p>
                Choose a result to add it here. The order becomes the returned Question ID order.
              </p>
            }
          >
            <ol>
              <For each={selection().questions}>
                {(question, index) => (
                  <li>
                    <span>
                      <strong>{question.row.title}</strong> <small>{question.questionId}</small>
                    </span>
                    <span class="question-picker-tray-actions">
                      <button
                        class="quiet-action"
                        type="button"
                        disabled={index() === 0}
                        aria-label={`Move ${question.row.title} earlier`}
                        onClick={() =>
                          updateSelection(
                            moveQuestionPickerSelection(
                              props.mode,
                              props.maximumSelection,
                              selection(),
                              index(),
                              -1,
                            ),
                          )
                        }
                      >
                        Earlier
                      </button>
                      <button
                        class="quiet-action"
                        type="button"
                        disabled={index() === selection().questions.length - 1}
                        aria-label={`Move ${question.row.title} later`}
                        onClick={() =>
                          updateSelection(
                            moveQuestionPickerSelection(
                              props.mode,
                              props.maximumSelection,
                              selection(),
                              index(),
                              1,
                            ),
                          )
                        }
                      >
                        Later
                      </button>
                      <button
                        class="quiet-action"
                        type="button"
                        onClick={() => toggleRow(question.row, false)}
                      >
                        Remove
                      </button>
                    </span>
                  </li>
                )}
              </For>
            </ol>
          </Show>
          <Show when={props.curationActions !== undefined && selection().questions.length > 0}>
            <div class="question-picker-curation-actions">
              <button
                class="quiet-action"
                type="button"
                onClick={() =>
                  props.curationActions?.request({
                    kind: "addToCollection",
                    selection: selection(),
                  })
                }
              >
                Add to Question Folder
              </button>
            </div>
          </Show>
        </section>
      </Show>

      <footer class="question-picker-footer">
        <button class="quiet-action" type="button" onClick={cancel}>
          Cancel
        </button>
        <button
          class="primary-action"
          type="button"
          disabled={props.mode !== "none" && selection().questions.length === 0}
          onClick={confirm}
        >
          {props.confirmLabel ?? "Use selected questions"}
        </button>
      </footer>
    </dialog>
  );
}
