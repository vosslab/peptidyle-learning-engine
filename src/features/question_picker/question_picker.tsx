// question_picker.tsx - shared-search Question selector for Instructor workflows.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { QuestionImageAssetId } from "../../../generated/api/QuestionImageAssetId";
import type { QuestionImageUrlResolver } from "../../components/question_renderer";
import type { RecordListSelection } from "../../components/record_list/record_list";
import { reorderedRecordListRows } from "../../components/record_list/record_list_reorder";
import type { QuestionLibraryBrowseRow } from "../../pages/library_page_model";
import { RecordSequence } from "../../components/record_list/record_sequence";
import { SearchControls } from "../search/search_controls";
import { SearchResults } from "../search/search_results";
import { SearchSelectionBar } from "../search/search_selection_bar";
import { createSearchState } from "../search/search_state";
import "./question_picker.css";
import { QuestionPickerInspection } from "./question_picker_inspection";
import {
  inspectQuestionPickerRow,
  questionPickerInspectionView,
  questionPickerSearchDefinition,
  questionPickerSelection,
  toggleQuestionPickerSelection,
  type QuestionPickerEligibility,
  type QuestionPickerInspectionView,
  type QuestionPickerSelection,
  type QuestionPickerSelectionMode,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "./question_picker_model";

export interface QuestionPickerProps {
  readonly repository: QuestionPickerSourceRepository;
  readonly sources: ReadonlyArray<QuestionPickerSource>;
  readonly mode: QuestionPickerSelectionMode;
  readonly maximumSelection: number;
  readonly initialSelection?: QuestionPickerSelection;
  readonly onConfirm: (selection: QuestionPickerSelection) => void;
  readonly onCancel: () => void;
  readonly trigger: HTMLButtonElement | undefined;
  readonly confirmLabel?: string;
  readonly instructions?: string;
  readonly title?: string;
  /** Pool-member contexts use the owning Pool's UUID, Type, and Backend before rows render. */
  readonly eligibility?: QuestionPickerEligibility;
  readonly loadQuestionInspection?: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => Promise<QuestionDetails>;
  readonly questionRevisionPreviewDocumentUrl?: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly questionImageUrl?: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
    questionImageAssetId: QuestionImageAssetId,
  ) => string;
}

type PickerInspection =
  | { readonly kind: "closed" }
  | { readonly kind: "loading"; readonly key: string }
  | {
      readonly kind: "ready";
      readonly key: string;
      readonly view: QuestionPickerInspectionView;
      readonly previewDocumentUrl: string | null;
    }
  | { readonly kind: "error"; readonly key: string; readonly message: string };

function sourceKey(source: QuestionPickerSource): string {
  if (source.kind === "retainedAssessment") {
    return `retained:${source.retainedAssessment.courseInstanceId}:${source.retainedAssessment.assessmentId}`;
  }
  return source.kind;
}

function sourceFromKey(
  sources: ReadonlyArray<QuestionPickerSource>,
  key: string,
): QuestionPickerSource | undefined {
  return sources.find((source) => sourceKey(source) === key);
}

function selectedCopy(
  selection: QuestionPickerSelection,
  mode: QuestionPickerSelectionMode,
): string {
  if (mode === "none") return "Browse Questions and inspect their details.";
  if (selection.questionIds.length === 0)
    return "Select Question results to prepare an ordered list.";
  if (mode === "one") return `Selected ${selection.questions[0]?.row.questionTitle ?? "Question"}.`;
  return `${selection.questionIds.length} Questions selected in order.`;
}

/** One native dialog retains its tray and inspection while shared search owns discovery state. */
export function QuestionPicker(props: QuestionPickerProps): JSX.Element {
  const initialSource = props.sources[0];
  if (initialSource === undefined) throw new Error("Question Picker needs at least one source.");
  const [selection, setSelection] = createSignal<QuestionPickerSelection>(
    questionPickerSelection(
      props.mode,
      props.maximumSelection,
      props.initialSelection?.questions.map((question) => question.row) ?? [],
    ),
  );
  const [selectionMessage, setSelectionMessage] = createSignal(
    selectedCopy(selection(), props.mode),
  );
  const [inspection, setInspection] = createSignal<PickerInspection>({ kind: "closed" });
  let dialog!: HTMLDialogElement;
  const search = createSearchState(
    questionPickerSearchDefinition(props.repository, initialSource, props.eligibility, inspectRow),
  );

  function updateSelection(next: QuestionPickerSelection): void {
    setSelection(next);
    setSelectionMessage(selectedCopy(next, props.mode));
  }

  function updateLibraryQuery(change: Record<string, string | null>): void {
    void search.apply({
      ...search.query(),
      libraryQuery: { ...search.query().libraryQuery, ...change },
    });
  }

  function selectSource(event: Event): void {
    const target = event.currentTarget;
    if (!(target instanceof HTMLSelectElement)) return;
    const source = sourceFromKey(props.sources, target.value);
    if (source === undefined) return;
    updateSelection(questionPickerSelection(props.mode, props.maximumSelection, []));
    search.clearSelection();
    void search.apply({ ...search.query(), source });
  }

  function inspectionImageUrl(view: QuestionPickerInspectionView): QuestionImageUrlResolver {
    return (asset) =>
      new URL(
        props.questionImageUrl?.(view.publishedQuestionRevisionTuple, asset.questionImageAssetId) ??
          "",
        window.location.origin,
      );
  }

  function inspectRow(row: QuestionLibraryBrowseRow): void {
    const load = props.loadQuestionInspection;
    if (load === undefined) return;
    const request = inspectQuestionPickerRow(selection(), row);
    const key = `${request.publishedQuestionRevisionTuple.publishedQuestionId}:${request.publishedQuestionRevisionTuple.revisionNumber}`;
    setInspection({ kind: "loading", key });
    void load(request.publishedQuestionRevisionTuple).then(
      (details) => {
        const view = questionPickerInspectionView(details);
        const previewDocumentUrl =
          view.backend === "webwork"
            ? (props.questionRevisionPreviewDocumentUrl?.(view.publishedQuestionRevisionTuple) ??
              null)
            : null;
        setInspection((current) =>
          current.kind !== "closed" && current.key === key
            ? { kind: "ready", key, view, previewDocumentUrl }
            : current,
        );
      },
      () =>
        setInspection((current) =>
          current.kind !== "closed" && current.key === key
            ? {
                kind: "error",
                key,
                message: "This Question could not be inspected. The Assessment is unchanged.",
              }
            : current,
        ),
    );
  }

  function pickerRows(): ReadonlyArray<QuestionLibraryBrowseRow> {
    const current = search.state();
    return current.kind === "empty" ? [] : current.rows;
  }

  function toggleRow(row: QuestionLibraryBrowseRow, checked: boolean): void {
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
      search.select(row, checked);
    } catch (error: unknown) {
      setSelectionMessage(
        error instanceof Error ? error.message : "That Question could not be selected.",
      );
    }
  }

  function selectedRows(): RecordListSelection<QuestionLibraryBrowseRow> | undefined {
    if (props.mode === "none") return undefined;
    return {
      kind: props.mode === "one" ? "radio" : "checkbox",
      selectedIds: () => new Set(selection().questionIds),
      onChange: toggleRow,
    };
  }

  function cancel(): void {
    if (dialog.open) dialog.close();
    props.onCancel();
    queueMicrotask(() => props.trigger?.focus());
  }

  function confirm(): void {
    if (props.mode !== "none" && selection().questions.length === 0) {
      setSelectionMessage("Select at least one Question before continuing.");
      return;
    }
    if (dialog.open) dialog.close();
    props.trigger?.focus();
    props.onConfirm(selection());
  }

  const facetValues = (
    facet: string,
  ): ReadonlyArray<{ readonly value: string; readonly count: number }> =>
    (search.state().filterCounts ?? []).filter((item) => item.facet === facet);
  const inspectionError = (): string | undefined => {
    const current = inspection();
    return current.kind === "error" ? current.message : undefined;
  };
  const readyInspection = (): Extract<PickerInspection, { readonly kind: "ready" }> | undefined => {
    const current = inspection();
    return current.kind === "ready" ? current : undefined;
  };

  onMount(() => void search.open(search.query()));
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
        queueMicrotask(() => dialog.showModal());
      }}
      onCancel={(event) => {
        event.preventDefault();
        cancel();
      }}
    >
      <header class="question-picker-header">
        <div>
          <p class="eyebrow">Question selection</p>
          <h2 id="question-picker-heading">{props.title ?? "Choose published Questions"}</h2>
          <p id="question-picker-instructions">
            {props.instructions ??
              "Choose a source, refine the current Library result, then add Questions in the order you want to use them."}
          </p>
        </div>
        <button class="quiet-action" type="button" onClick={cancel}>
          Close picker
        </button>
      </header>

      <SearchControls
        state={search}
        textLabel="Search Questions"
        textPlaceholder="Title, concept, or tag"
        displayAriaLabel="Question result display"
      >
        <div class="question-picker-filters">
          <label>
            Question source
            <select value={sourceKey(search.query().source)} onChange={selectSource}>
              <For each={props.sources}>
                {(source) => <option value={sourceKey(source)}>{source.label}</option>}
              </For>
            </select>
          </label>
          <label>
            Question Author
            <select
              value={search.query().libraryQuery.authorName ?? ""}
              onChange={(event) =>
                updateLibraryQuery({ authorName: event.currentTarget.value || null })
              }
            >
              <option value="">All Question Authors</option>
              <For each={facetValues("authorName")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
          <label>
            Backend
            <select
              value={search.query().libraryQuery.backend ?? ""}
              disabled={props.eligibility !== undefined}
              onChange={(event) =>
                updateLibraryQuery({ backend: event.currentTarget.value || null })
              }
            >
              <option value="">All backends</option>
              <For each={facetValues("backend")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
          <label>
            Question Type
            <select
              value={search.query().libraryQuery.questionType ?? ""}
              disabled={props.eligibility !== undefined}
              onChange={(event) =>
                updateLibraryQuery({ questionType: event.currentTarget.value || null })
              }
            >
              <option value="">All Question Types</option>
              <For each={facetValues("questionType")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
          <label>
            Capability
            <select
              value={search.query().libraryQuery.capability ?? ""}
              onChange={(event) =>
                updateLibraryQuery({ capability: event.currentTarget.value || null })
              }
            >
              <option value="">All capabilities</option>
              <For each={facetValues("capability")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
          <label>
            Tag
            <select
              value={search.query().libraryQuery.tag ?? ""}
              onChange={(event) => updateLibraryQuery({ tag: event.currentTarget.value || null })}
            >
              <option value="">All tags</option>
              <For each={facetValues("tag")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
          <label>
            Question License
            <select
              value={search.query().libraryQuery.questionLicense ?? ""}
              onChange={(event) =>
                updateLibraryQuery({ questionLicense: event.currentTarget.value || null })
              }
            >
              <option value="">All Question Licenses</option>
              <For each={facetValues("questionLicense")}>
                {(facet) => (
                  <option value={facet.value}>{`${facet.value} (${facet.count})`}</option>
                )}
              </For>
            </select>
          </label>
        </div>
      </SearchControls>

      <p class="question-picker-status" role="status" aria-live="polite">
        {selectionMessage()}
      </p>
      <Show when={inspection().kind === "loading"}>
        <p class="question-picker-status" role="status">
          Loading Question inspection...
        </p>
      </Show>
      <Show when={inspectionError()}>
        {(message) => (
          <section class="route-error" role="alert">
            <h3>Question inspection unavailable</h3>
            <p>{message()}</p>
            <button
              class="quiet-action"
              type="button"
              onClick={() => setInspection({ kind: "closed" })}
            >
              Close inspection
            </button>
          </section>
        )}
      </Show>
      <Show when={readyInspection()}>
        {(ready) => (
          <QuestionPickerInspection
            view={ready().view}
            previewDocumentUrl={ready().previewDocumentUrl}
            questionImageUrl={inspectionImageUrl(ready().view)}
            onClose={() => setInspection({ kind: "closed" })}
          />
        )}
      </Show>

      <section class="question-picker-results" aria-label="Question results">
        <h3>Current results</h3>
        <SearchResults
          state={search}
          ariaLabel="Question results"
          selection={selectedRows}
          emptyState={{
            title: "No Questions match this source and filter.",
            message: "Use a shorter search or choose a broader source.",
          }}
        />
      </section>
      <Show when={props.mode !== "none"}>
        <SearchSelectionBar
          selectedCount={() => selection().questionIds.length}
          loadedCount={() => pickerRows().length}
          onSelectLoaded={() => pickerRows().forEach((row) => toggleRow(row, true))}
          onClearSelection={() => {
            updateSelection(questionPickerSelection(props.mode, props.maximumSelection, []));
            search.clearSelection();
          }}
        />
        <section class="question-picker-tray" aria-labelledby="question-picker-tray-heading">
          <h3 id="question-picker-tray-heading">Selected Questions</h3>
          <RecordSequence
            rows={selection().questions}
            content={(question) => ({
              title: question.row.questionTitle,
              details: [
                {
                  kind: "questionId",
                  questionTitle: question.row.questionTitle,
                  displayId: question.questionId,
                },
              ],
              actions: [],
            })}
            renderBody={(question) => (
              <button
                class="quiet-action"
                type="button"
                onClick={() => toggleRow(question().row, false)}
              >
                Remove
              </button>
            )}
            recordId={(question) => question.questionId}
            reorder={{
              onMove: (sourceIndex, destinationIndex) => {
                const nextRows = reorderedRecordListRows(
                  selection().questions,
                  sourceIndex,
                  destinationIndex,
                ).map((question) => question.row);
                updateSelection(
                  questionPickerSelection(props.mode, props.maximumSelection, nextRows),
                );
              },
              recordLabel: (question) => question.row.questionTitle,
            }}
            state={{ kind: "ready" }}
            ariaLabel="Selected Questions in order"
            emptyState={{
              title: "No Questions are selected.",
              message:
                "Choose a result to add it here. The order becomes the returned Question ID order.",
            }}
          />
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
          {props.confirmLabel ?? "Use selected Questions"}
        </button>
      </footer>
    </dialog>
  );
}
