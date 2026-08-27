// problem_picker.tsx - accessible shared Question ID selector for instructor workflows.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { CatalogBrowseQuery, CatalogBrowseRow } from "../../pages/library_page_model";
import { EMPTY_CATALOG_QUERY } from "../../pages/library_page_model";
import "./problem_picker.css";
import {
  ProblemPickerSession,
  moveProblemPickerSelection,
  problemPickerSelection,
  toggleProblemPickerSelection,
  type ProblemPickerCurationActions,
  type ProblemPickerSelection,
  type ProblemPickerSelectionMode,
  type ProblemPickerSource,
  type ProblemPickerSourceRepository,
  type ProblemPickerState,
} from "./problem_picker_model";

export interface ProblemPickerProps {
  readonly repository: ProblemPickerSourceRepository;
  readonly sources: ReadonlyArray<ProblemPickerSource>;
  readonly mode: ProblemPickerSelectionMode;
  /** Required per destination: collections use 200; assignments use their remaining cap. */
  readonly maximumSelection: number;
  readonly onConfirm: (selection: ProblemPickerSelection) => void;
  readonly onCancel: () => void;
  readonly trigger: HTMLButtonElement | undefined;
  readonly curationActions?: ProblemPickerCurationActions;
  readonly confirmLabel?: string;
  /** Optional destination-specific explanation of how picker selection proceeds. */
  readonly instructions?: string;
  readonly title?: string;
}

function sourceKey(source: ProblemPickerSource): string {
  if (source.kind === "collection") return `collection:${source.collection}`;
  if (source.kind === "retainedAssignment") {
    return `retained:${source.retainedAssignment.course}:${source.retainedAssignment.assignment}`;
  }
  return source.kind;
}

function sourceFromKey(
  sources: ReadonlyArray<ProblemPickerSource>,
  key: string,
): ProblemPickerSource | undefined {
  return sources.find((source) => sourceKey(source) === key);
}

function rowIsSelected(selection: ProblemPickerSelection, row: CatalogBrowseRow): boolean {
  return selection.questionIds.includes(row.displayId);
}

function selectedCopy(selection: ProblemPickerSelection, mode: ProblemPickerSelectionMode): string {
  if (mode === "none") return "Browse questions and open a question for its full details.";
  if (selection.questionIds.length === 0)
    return "Select question results to prepare an ordered list.";
  if (mode === "one") return `Selected ${selection.questions[0]?.row.title ?? "question"}.`;
  return `${selection.questionIds.length} questions selected in order.`;
}

function facetValues(
  state: ProblemPickerState,
  group:
    | "byline"
    | "backend"
    | "tag"
    | "responseFamily"
    | "taxonomy"
    | "capability"
    | "license"
    | "evidence"
    | "usedInMyCourses",
): ReadonlyArray<{ readonly value: string; readonly count: number }> {
  return state.aggregates.filter((aggregate) => aggregate.group === group);
}

/**
 * One native dialog with source, D1 filters, result selection, and an ordered
 * tray. Parents own curation persistence and receive public Question IDs only.
 */
export function ProblemPicker(props: ProblemPickerProps): JSX.Element {
  const [source, setSource] = createSignal<ProblemPickerSource | undefined>(props.sources[0]);
  const [query, setQuery] = createSignal<CatalogBrowseQuery>(EMPTY_CATALOG_QUERY);
  const [state, setState] = createSignal<ProblemPickerState>({
    kind: "loading",
    rows: [],
    aggregates: [],
    nextCursor: null,
  });
  const [selection, setSelection] = createSignal<ProblemPickerSelection>(
    problemPickerSelection(props.mode, props.maximumSelection, []),
  );
  const [selectionMessage, setSelectionMessage] = createSignal(
    selectedCopy(selection(), props.mode),
  );
  const session = new ProblemPickerSession(props.repository, setState);
  let dialog!: HTMLDialogElement;

  function updateSelection(next: ProblemPickerSelection): void {
    setSelection(next);
    setSelectionMessage(selectedCopy(next, props.mode));
  }

  function resultRows(): ReadonlyArray<CatalogBrowseRow> {
    const current = state();
    return current.kind === "empty" ? [] : current.rows;
  }

  function hasNextPage(): boolean {
    const current = state();
    return current.kind === "ready" && current.nextCursor !== null;
  }

  function updateQuery(change: Partial<CatalogBrowseQuery>): void {
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

  function toggleRow(row: CatalogBrowseRow, checked: boolean): void {
    try {
      updateSelection(
        toggleProblemPickerSelection(props.mode, props.maximumSelection, selection(), row, checked),
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
      class="problem-picker-dialog"
      aria-labelledby="problem-picker-heading"
      aria-describedby="problem-picker-instructions"
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
      <header class="problem-picker-header">
        <div>
          <p class="eyebrow">Question selection</p>
          <h2 id="problem-picker-heading">{props.title ?? "Choose published questions"}</h2>
          <p id="problem-picker-instructions">
            {props.instructions ??
              "Choose a source, refine the current library result, then add questions in the order you want to use them."}
          </p>
        </div>
        <button class="quiet-action" type="button" onClick={cancel}>
          Close picker
        </button>
      </header>

      <form
        class="problem-picker-controls"
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
        <label class="problem-picker-search-control">
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
          Byline
          <select
            value={query().byline ?? ""}
            onChange={(event) => updateQuery({ byline: event.currentTarget.value || null })}
          >
            <option value="">All bylines</option>
            <For each={facetValues(state(), "byline")}>
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
          Response family
          <select
            value={query().responseFamily ?? ""}
            onChange={(event) => updateQuery({ responseFamily: event.currentTarget.value || null })}
          >
            <option value="">All response families</option>
            <For each={facetValues(state(), "responseFamily")}>
              {(facet) => <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>}
            </For>
          </select>
        </label>
        <label>
          Topic
          <select
            value={query().taxonomy ?? ""}
            onChange={(event) => updateQuery({ taxonomy: event.currentTarget.value || null })}
          >
            <option value="">All topics</option>
            <For each={facetValues(state(), "taxonomy")}>
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
          License
          <select
            value={query().license ?? ""}
            onChange={(event) => updateQuery({ license: event.currentTarget.value || null })}
          >
            <option value="">All licenses</option>
            <For each={facetValues(state(), "license")}>
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

      <p class="problem-picker-status" role="status" aria-live="polite">
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
        <section class="problem-picker-results" aria-label="Question results">
          <h3>Current results</h3>
          <ul>
            <For each={resultRows()}>
              {(row) => (
                <li>
                  <Show
                    when={props.mode !== "none"}
                    fallback={
                      <article class="problem-picker-result">
                        <span>
                          <strong>{row.title}</strong>
                          <span>{row.summary}</span>
                          <small>{`${row.displayId} - ${row.taxonomy.join(" / ") || "No topic label"}`}</small>
                        </span>
                      </article>
                    }
                  >
                    <label class="problem-picker-result">
                      <input
                        type={props.mode === "one" ? "radio" : "checkbox"}
                        name={props.mode === "one" ? "problem-picker-choice" : undefined}
                        checked={rowIsSelected(selection(), row)}
                        onChange={(event) => toggleRow(row, event.currentTarget.checked)}
                      />
                      <span>
                        <strong>{row.title}</strong>
                        <span>{row.summary}</span>
                        <small>{`${row.displayId} - ${row.taxonomy.join(" / ") || "No topic label"}`}</small>
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
        <section class="problem-picker-tray" aria-labelledby="problem-picker-tray-heading">
          <h3 id="problem-picker-tray-heading">Selected questions</h3>
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
                    <span class="problem-picker-tray-actions">
                      <button
                        class="quiet-action"
                        type="button"
                        disabled={index() === 0}
                        aria-label={`Move ${question.row.title} earlier`}
                        onClick={() =>
                          updateSelection(
                            moveProblemPickerSelection(
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
                            moveProblemPickerSelection(
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
            <div class="problem-picker-curation-actions">
              <button
                class="quiet-action"
                type="button"
                onClick={() =>
                  props.curationActions?.request({ kind: "favorite", selection: selection() })
                }
              >
                Update Favorites
              </button>
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
                Add to collection
              </button>
            </div>
          </Show>
        </section>
      </Show>

      <footer class="problem-picker-footer">
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
