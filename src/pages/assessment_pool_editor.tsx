// assessment_pool_editor.tsx - accessible editor controls for one Question Pool Assessment Entry.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { QuestionPoolPreview } from "../api/contracts";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";

import type {
  AssessmentQuestionRow,
  AssessmentEditorQuestionPoolAssessmentEntry,
} from "./assessment_editor_model";
import {
  parseExactQuestionIds,
  validateQuestionPoolAssessmentEntry,
} from "./assessment_editor_model";

export interface AssessmentPoolEditorProps {
  readonly entry: AssessmentEditorQuestionPoolAssessmentEntry;
  readonly entryIndex: number;
  readonly entryCount: number;
  readonly resolveQuestionPoolItems: (
    questionIds: ReadonlyArray<string>,
  ) => Promise<ReadonlyArray<AssessmentQuestionRow>>;
  readonly onChange: (entry: AssessmentEditorQuestionPoolAssessmentEntry) => void;
  readonly onMove: (direction: -1 | 1) => void;
  readonly onRemove: () => void;
  readonly onMessage: (message: string) => void;
  readonly preview: QuestionPoolPreview | undefined;
  readonly previewBusy: boolean;
  readonly onPreview: () => void;
  readonly onChooseQuestionPoolItems: (trigger: HTMLButtonElement) => void;
}

function poolLabel(entryIndex: number): string {
  return `Question pool ${entryIndex + 1}`;
}

export function AssessmentPoolEditor(props: AssessmentPoolEditorProps): JSX.Element {
  const [entryText, setEntryText] = createSignal("");
  const [entryError, setEntryError] = createSignal("");
  const [entryBusy, setEntryBusy] = createSignal(false);

  function update(next: Partial<AssessmentEditorQuestionPoolAssessmentEntry>): void {
    const entry = { ...props.entry, ...next };
    const error = validateQuestionPoolAssessmentEntry(entry);
    setEntryError(error ?? "");
    props.onChange(entry);
    props.onMessage(
      error === null
        ? "Question Pool updated. Review its Items, then save the complete Assessment Content."
        : `${error} Correct this pool before saving.`,
    );
  }

  async function addQuestionPoolItems(): Promise<void> {
    let questionIds: ReadonlyArray<string>;
    try {
      questionIds = parseExactQuestionIds(entryText());
    } catch (error: unknown) {
      setEntryError(
        error instanceof Error ? error.message : "Question Pool Item Question IDs are invalid.",
      );
      return;
    }
    const existing = new Set(props.entry.items.map((item) => item.questionId));
    if (questionIds.some((questionId) => existing.has(questionId))) {
      setEntryError("Each Question Pool Item Question ID can appear only once in a Question Pool.");
      return;
    }
    if (
      props.entry.items.length + questionIds.length >
      MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
    ) {
      setEntryError(
        `Keep this pool to ${MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY} Question Pool Item Question IDs or fewer, then check and add Items.`,
      );
      return;
    }
    setEntryBusy(true);
    try {
      const items = await props.resolveQuestionPoolItems(questionIds);
      const nextItems = [...props.entry.items, ...items];
      const entry = { ...props.entry, items: nextItems };
      const error = validateQuestionPoolAssessmentEntry(entry);
      setEntryError(error ?? "");
      props.onChange(entry);
      setEntryText("");
      props.onMessage(
        `Added ${items.length} Question Pool Item ID${items.length === 1 ? "" : "s"}. Set the selection count, then save the Assessment.`,
      );
    } catch (error: unknown) {
      setEntryError(
        error instanceof Error
          ? `${error.message} Your Question Pool Item Question IDs are still here.`
          : "Question Pool Items could not be checked. Your Question Pool Item Question IDs are still here.",
      );
    } finally {
      setEntryBusy(false);
    }
  }

  function removeQuestionPoolItem(questionId: string): void {
    const items = props.entry.items.filter((item) => item.questionId !== questionId);
    update({ items });
  }
  function updateSelectedQuestionOrder(value: string): void {
    if (value !== "questionPoolOrder" && value !== "randomOrder") return;
    update({ selectionRule: { selectedQuestionOrder: value } });
  }

  return (
    <li
      class="assessment-editor-row assessment-editor-pool"
      aria-label={poolLabel(props.entryIndex)}
    >
      <div>
        <h3>Question pool</h3>
        <p>
          The server selects from the Question Pool Item Question IDs and records each
          Student&apos;s exact Question Pool Selection as immutable evidence.
        </p>
      </div>
      <div class="assessment-editor-row-actions">
        <button
          class="quiet-action"
          type="button"
          disabled={props.entryIndex === 0}
          aria-label="Move question pool earlier"
          onClick={() => props.onMove(-1)}
        >
          &uarr;
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={props.entryIndex === props.entryCount - 1}
          aria-label="Move question pool later"
          onClick={() => props.onMove(1)}
        >
          &darr;
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={props.previewBusy}
          onClick={props.onPreview}
        >
          {props.preview === undefined ? "Preview selection" : "Preview another selection"}
        </button>
        <button class="quiet-action" type="button" onClick={props.onRemove}>
          Remove pool
        </button>
      </div>
      <div class="assessment-editor-pool-fields">
        <label class="assessment-editor-field">
          Selection count
          <input
            type="number"
            min="1"
            max={props.entry.items.length || undefined}
            value={props.entry.selectionCount}
            onInput={(event) => update({ selectionCount: Number(event.currentTarget.value) })}
          />
        </label>
        <label class="assessment-editor-field">
          Points per selected Question
          <input
            inputmode="decimal"
            value={props.entry.pointsPerItem}
            onInput={(event) => update({ pointsPerItem: event.currentTarget.value })}
          />
        </label>
        <label class="assessment-editor-field">
          Selected Question order
          <select
            value={props.entry.selectionRule.selectedQuestionOrder}
            onChange={(event) => updateSelectedQuestionOrder(event.currentTarget.value)}
          >
            <option value="questionPoolOrder">Question Pool Item order</option>
            <option value="randomOrder">Random order</option>
          </select>
        </label>
      </div>
      <Show when={props.preview}>
        {(preview) => (
          <section
            class="assessment-editor-pool-preview"
            aria-labelledby={`pool-preview-${props.entryIndex}`}
          >
            <h4 id={`pool-preview-${props.entryIndex}`}>{preview().questionPoolLabel}</h4>
            <p>
              Selected {preview().selectionCount} in {preview().selectionRule.selectedQuestionOrder}
              .
            </p>
            <h5>Question Pool Items</h5>
            <ul>
              <For each={preview().items}>
                {(item) => (
                  <li>
                    <strong>{item.questionId}</strong> {item.questionTitle}
                  </li>
                )}
              </For>
            </ul>
            <h5>Server-selected Questions</h5>
            <ol>
              <For each={preview().selectedItems}>
                {(sample) => (
                  <li>
                    <strong>{sample.questionId}</strong> {sample.questionTitle}
                  </li>
                )}
              </For>
            </ol>
            <p class="assessment-editor-note">
              This is an instructor-only sample. It does not create student work or alter grades.
            </p>
          </section>
        )}
      </Show>
      <section
        class="assessment-editor-pool-entries"
        aria-labelledby={`pool-entries-${props.entryIndex}`}
      >
        <h4 id={`pool-entries-${props.entryIndex}`}>Question Pool Item Question IDs</h4>
        <p class="assessment-editor-note">
          Add Items in the order you want preserved when delivery order is Question Pool Item order.
        </p>
        <ul>
          <For each={props.entry.items} fallback={<li>No Items yet.</li>}>
            {(entry) => (
              <li>
                <span>
                  <strong>{entry.questionId}</strong> {entry.questionTitle}
                </span>
                <button
                  class="quiet-action"
                  type="button"
                  aria-label={`Remove ${entry.questionId} from this question pool`}
                  onClick={() => removeQuestionPoolItem(entry.questionId)}
                >
                  Remove
                </button>
              </li>
            )}
          </For>
        </ul>
        <label class="assessment-editor-field">
          Add Question Pool Item Question IDs
          <textarea
            rows="2"
            value={entryText()}
            placeholder="ABCD-ZEFG, BCDE-FGHA"
            aria-invalid={entryError() !== ""}
            aria-describedby={`pool-entry-help-${props.entryIndex}`}
            onInput={(event) => {
              setEntryText(event.currentTarget.value);
              setEntryError("");
            }}
          />
        </label>
        <p id={`pool-entry-help-${props.entryIndex}`} class="assessment-editor-note">
          Paste canonical Question IDs separated by commas or lines. The library checks each one
          before it becomes a Question Pool Item.
        </p>
        <button
          class="quiet-action"
          type="button"
          disabled={entryBusy()}
          onClick={() => void addQuestionPoolItems()}
        >
          Check and add Question Pool Items
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={entryBusy()}
          onClick={(event) => props.onChooseQuestionPoolItems(event.currentTarget)}
        >
          Choose Questions for pool
        </button>
        <Show when={entryError()}>
          {(error) => (
            <p class="inline-error" role="alert">
              {error()}
            </p>
          )}
        </Show>
      </section>
    </li>
  );
}
