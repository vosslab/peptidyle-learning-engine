// One shared-search dialog for adding either fixed Questions or reusable Question Pools.

import { Show, createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type {
  QuestionLibraryBrowseRepository,
  LibrarySearchRow,
} from "../../pages/library_page_model";
import { questionLibrarySearchDefinition } from "../../pages/question_library_search_definition";
import { SearchControls } from "../search/search_controls";
import { SearchResults } from "../search/search_results";
import { SearchSelectionBar } from "../search/search_selection_bar";
import { createSearchState } from "../search/search_state";
import {
  assessmentContentPickerSelection,
  type AssessmentContentPickerSelection,
} from "./assessment_content_picker_model";
import "./assessment_content_picker.css";

export type { AssessmentContentPickerSelection } from "./assessment_content_picker_model";

export interface AssessmentContentPickerProps {
  readonly repository: QuestionLibraryBrowseRepository;
  readonly trigger: HTMLButtonElement | undefined;
  readonly maximumQuestionSelection: number;
  readonly onConfirm: (selection: AssessmentContentPickerSelection) => void;
  readonly onCancel: () => void;
}

/** Selects an ordered Question batch or one Pool with the common request, paging, and display UI. */
export function AssessmentContentPicker(props: AssessmentContentPickerProps): JSX.Element {
  const state = createSearchState(questionLibrarySearchDefinition(props.repository), {
    runOnMount: true,
  });
  const [selectedRows, setSelectedRows] = createSignal<ReadonlyMap<string, LibrarySearchRow>>(
    new Map(),
  );
  let dialog!: HTMLDialogElement;

  // SearchSession clears selection for submit, chips, sort, and Clear. Keep retained row facts
  // only for IDs that the shared session still owns, including selections on another page.
  createEffect(() => {
    const selectedIds = state.selectedIds();
    setSelectedRows((current) => {
      const next = new Map([...current].filter(([id]) => selectedIds.has(id)));
      return next.size === current.size ? current : next;
    });
  });

  function cancel(): void {
    if (dialog.open) dialog.close();
    props.onCancel();
    queueMicrotask(() => props.trigger?.focus());
  }

  function clearSelection(): void {
    state.clearSelection();
    setSelectedRows(new Map());
  }

  function choose(row: LibrarySearchRow, checked: boolean): void {
    if (!checked) {
      state.select(row, false);
      setSelectedRows((current) => {
        const next = new Map(current);
        next.delete(row.displayId);
        return next;
      });
      return;
    }
    if (row.kind === "pool") {
      clearSelection();
      state.select(row, true);
      setSelectedRows(new Map([[row.displayId, row]]));
      return;
    }
    if ([...selectedRows().values()].some((value) => value.kind === "pool")) clearSelection();
    const questionCount = [...selectedRows().values()].filter(
      (value) => value.kind === "question",
    ).length;
    if (questionCount >= props.maximumQuestionSelection) return;
    state.select(row, true);
    setSelectedRows((current) => new Map(current).set(row.displayId, row));
  }

  function changeQuery(change: Partial<ReturnType<typeof state.query>>): void {
    clearSelection();
    void state.apply({ ...state.query(), ...change });
  }

  function submit(): void {
    clearSelection();
    void state.submit();
  }

  function selectLoadedQuestions(): void {
    const current = state.state();
    if (current.kind === "empty") return;
    clearSelection();
    const questions = current.rows
      .filter(
        (row): row is Extract<LibrarySearchRow, { readonly kind: "question" }> =>
          row.kind === "question",
      )
      .slice(0, props.maximumQuestionSelection);
    for (const row of questions) state.select(row, true);
    setSelectedRows(new Map(questions.map((row) => [row.displayId, row])));
  }

  function confirm(): void {
    const rows = [...selectedRows().values()];
    const choice = assessmentContentPickerSelection(rows);
    if (choice === undefined) return;
    if (dialog.open) dialog.close();
    props.trigger?.focus();
    props.onConfirm(choice);
  }

  onMount(() => queueMicrotask(() => dialog.showModal()));
  onCleanup(() => {
    if (dialog.open) dialog.close();
  });

  return (
    <dialog
      class="assessment-content-picker-dialog"
      aria-labelledby="assessment-content-picker-heading"
      aria-describedby="assessment-content-picker-instructions"
      ref={(element) => (dialog = element)}
      onCancel={(event) => {
        event.preventDefault();
        cancel();
      }}
    >
      <header class="assessment-content-picker-dialog__header">
        <div>
          <p class="eyebrow">Assessment content</p>
          <h2 id="assessment-content-picker-heading">Choose published Assessment content</h2>
          <p id="assessment-content-picker-instructions">
            Questions keep their exact published Revision. Importing a Question Pool uses its
            current edit number and current member count.
          </p>
        </div>
        <button class="quiet-action" type="button" onClick={cancel}>
          Close picker
        </button>
      </header>
      <div class="assessment-content-picker-dialog__controls">
        <SearchControls
          state={state}
          textLabel="Search published Assessment content"
          textPlaceholder="Title or concept"
          displayAriaLabel="Assessment content result display"
          onSubmit={submit}
        >
          <label>
            Show
            <select
              value={state.query().kind}
              onChange={(event) =>
                changeQuery({
                  kind: event.currentTarget.value as "both" | "questions" | "pools",
                })
              }
            >
              <option value="both">Questions and Question Pools</option>
              <option value="questions">Questions only</option>
              <option value="pools">Question Pools only</option>
            </select>
          </label>
          <label>
            Question membership
            <select
              value={state.query().membership}
              disabled={state.query().kind === "pools"}
              onChange={(event) =>
                changeQuery({ membership: event.currentTarget.value as "noPool" | "all" })
              }
            >
              <option value="noPool">Questions in no Pool</option>
              <option value="all">All Questions</option>
            </select>
          </label>
        </SearchControls>
      </div>
      <div class="assessment-content-picker-dialog__results">
        <SearchSelectionBar
          selectedCount={() => state.selectedIds().size}
          loadedCount={() => {
            const current = state.state();
            return current.kind === "empty" ? 0 : current.rows.length;
          }}
          onSelectLoaded={selectLoadedQuestions}
          onClearSelection={() => {
            clearSelection();
          }}
        >
          <span class="assessment-content-picker-dialog__selection-copy">
            <Show when={selectedRows().size > 0}>
              <span>{selectedRows().size} selected</span>
            </Show>
          </span>
        </SearchSelectionBar>
        <SearchResults
          state={state}
          ariaLabel="Published Assessment content"
          emptyState={{
            title: "No published Assessment content matches these choices.",
            message: "Try different text or membership choices.",
          }}
          selection={() => ({
            kind: "checkbox",
            selectedIds: state.selectedIds,
            onChange: choose,
          })}
        />
      </div>
      <footer class="assessment-content-picker-dialog__footer">
        <button class="quiet-action" type="button" onClick={cancel}>
          Cancel
        </button>
        <button type="button" disabled={selectedRows().size === 0} onClick={confirm}>
          Add selected content
        </button>
      </footer>
    </dialog>
  );
}
