// assignment_pool_editor.tsx - accessible editor controls for one ordered pool entry.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { QuestionPoolPreview } from "../api/contracts";
import { MAX_QUESTION_POOL_ENTRIES_PER_ASSIGNMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ENTRIES_PER_ASSIGNMENT_ENTRY";

import type {
  AssignmentQuestionRow,
  AssignmentEditorQuestionPoolEntry,
} from "./assignment_editor_model";
import { parseExactQuestionIds, validateQuestionPoolEntry } from "./assignment_editor_model";

export interface AssignmentPoolEditorProps {
  readonly entry: AssignmentEditorQuestionPoolEntry;
  readonly entryIndex: number;
  readonly entryCount: number;
  readonly resolveEntries: (
    questionIds: ReadonlyArray<string>,
  ) => Promise<ReadonlyArray<AssignmentQuestionRow>>;
  readonly onChange: (entry: AssignmentEditorQuestionPoolEntry) => void;
  readonly onMove: (direction: -1 | 1) => void;
  readonly onRemove: () => void;
  readonly onMessage: (message: string) => void;
  readonly preview: QuestionPoolPreview | undefined;
  readonly previewBusy: boolean;
  readonly onPreview: () => void;
  readonly onChooseEntries: (trigger: HTMLButtonElement) => void;
}

function poolLabel(entryIndex: number): string {
  return `Question pool ${entryIndex + 1}`;
}

export function AssignmentPoolEditor(props: AssignmentPoolEditorProps): JSX.Element {
  const [entryText, setEntryText] = createSignal("");
  const [entryError, setEntryError] = createSignal("");
  const [entryBusy, setEntryBusy] = createSignal(false);

  function update(next: Partial<AssignmentEditorQuestionPoolEntry>): void {
    const entry = { ...props.entry, ...next };
    const error = validateQuestionPoolEntry(entry);
    setEntryError(error ?? "");
    props.onChange(entry);
    props.onMessage(
      error === null
        ? "Pool updated. Review its entries, then save the complete assignment definition."
        : `${error} Correct this pool before saving.`,
    );
  }

  async function addEntries(): Promise<void> {
    let questionIds: ReadonlyArray<string>;
    try {
      questionIds = parseExactQuestionIds(entryText());
    } catch (error: unknown) {
      setEntryError(
        error instanceof Error ? error.message : "Question Pool Entry Question IDs are invalid.",
      );
      return;
    }
    const existing = new Set(props.entry.entries.map((entry) => entry.questionId));
    if (questionIds.some((questionId) => existing.has(questionId))) {
      setEntryError("Each entry Question ID can appear only once in a pool.");
      return;
    }
    if (
      props.entry.entries.length + questionIds.length >
      MAX_QUESTION_POOL_ENTRIES_PER_ASSIGNMENT_ENTRY
    ) {
      setEntryError(
        `Keep this pool to ${MAX_QUESTION_POOL_ENTRIES_PER_ASSIGNMENT_ENTRY} Question Pool Entry Question IDs or fewer, then check and add entries.`,
      );
      return;
    }
    setEntryBusy(true);
    try {
      const entries = await props.resolveEntries(questionIds);
      const nextEntries = [...props.entry.entries, ...entries];
      const entry = { ...props.entry, entries: nextEntries };
      const error = validateQuestionPoolEntry(entry);
      setEntryError(error ?? "");
      props.onChange(entry);
      setEntryText("");
      props.onMessage(
        `Added ${entries.length} entry Question ID${entries.length === 1 ? "" : "s"}. Set the selection count, then save the Assignment.`,
      );
    } catch (error: unknown) {
      setEntryError(
        error instanceof Error
          ? `${error.message} Your Question Pool Entry Question IDs are still here.`
          : "Question Pool Entries could not be checked. Your Question Pool Entry Question IDs are still here.",
      );
    } finally {
      setEntryBusy(false);
    }
  }

  function removeEntry(questionId: string): void {
    const entries = props.entry.entries.filter((entry) => entry.questionId !== questionId);
    update({ entries });
  }
  function updateSelectedQuestionOrder(value: string): void {
    if (value !== "questionPoolOrder" && value !== "randomOrder") return;
    update({ selectionRule: { selectedQuestionOrder: value } });
  }

  return (
    <li
      class="assignment-editor-row assignment-editor-pool"
      aria-label={poolLabel(props.entryIndex)}
    >
      <div>
        <h3>Question pool</h3>
        <p>
          The server selects from the Question Pool Entry Question IDs and records each
          Student&apos;s exact Question Pool Selection as immutable evidence.
        </p>
      </div>
      <div class="assignment-editor-row-actions">
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
      <div class="assignment-editor-pool-fields">
        <label class="assignment-editor-field">
          Selection count
          <input
            type="number"
            min="1"
            max={props.entry.entries.length || undefined}
            value={props.entry.selectionCount}
            onInput={(event) => update({ selectionCount: Number(event.currentTarget.value) })}
          />
        </label>
        <label class="assignment-editor-field">
          Points per selected Question
          <input
            inputmode="decimal"
            value={props.entry.pointsPerItem}
            onInput={(event) => update({ pointsPerItem: event.currentTarget.value })}
          />
        </label>
        <label class="assignment-editor-field">
          Selected Question order
          <select
            value={props.entry.selectionRule.selectedQuestionOrder}
            onChange={(event) => updateSelectedQuestionOrder(event.currentTarget.value)}
          >
            <option value="questionPoolOrder">Question Pool Entry order</option>
            <option value="randomOrder">Random order</option>
          </select>
        </label>
      </div>
      <Show when={props.preview}>
        {(preview) => (
          <section
            class="assignment-editor-pool-preview"
            aria-labelledby={`pool-preview-${props.entryIndex}`}
          >
            <h4 id={`pool-preview-${props.entryIndex}`}>{preview().questionPoolLabel}</h4>
            <p>
              Selected {preview().selectionCount} in {preview().selectionRule.selectedQuestionOrder}
              .
            </p>
            <h5>Question Pool Entries</h5>
            <ul>
              <For each={preview().entries}>
                {(entry) => (
                  <li>
                    <strong>{entry.questionId}</strong> {entry.title}
                  </li>
                )}
              </For>
            </ul>
            <h5>Server-selected Questions</h5>
            <ol>
              <For each={preview().selected}>
                {(sample) => (
                  <li>
                    <strong>{sample.questionId}</strong> {sample.title}
                  </li>
                )}
              </For>
            </ol>
            <p class="assignment-editor-note">
              This is an instructor-only sample. It does not create student work or alter grades.
            </p>
          </section>
        )}
      </Show>
      <section
        class="assignment-editor-pool-entries"
        aria-labelledby={`pool-entries-${props.entryIndex}`}
      >
        <h4 id={`pool-entries-${props.entryIndex}`}>Question Pool Entry Question IDs</h4>
        <p class="assignment-editor-note">
          Add entries in the order you want preserved when delivery order is Question Pool Entry
          order.
        </p>
        <ul>
          <For each={props.entry.entries} fallback={<li>No entries yet.</li>}>
            {(entry) => (
              <li>
                <span>
                  <strong>{entry.questionId}</strong> {entry.title}
                </span>
                <button
                  class="quiet-action"
                  type="button"
                  aria-label={`Remove ${entry.questionId} from this question pool`}
                  onClick={() => removeEntry(entry.questionId)}
                >
                  Remove
                </button>
              </li>
            )}
          </For>
        </ul>
        <label class="assignment-editor-field">
          Add Question Pool Entry Question IDs
          <textarea
            rows="2"
            value={entryText()}
            placeholder="7K3-M9QP, 7K4-M9QP"
            aria-invalid={entryError() !== ""}
            aria-describedby={`pool-entry-help-${props.entryIndex}`}
            onInput={(event) => {
              setEntryText(event.currentTarget.value);
              setEntryError("");
            }}
          />
        </label>
        <p id={`pool-entry-help-${props.entryIndex}`} class="assignment-editor-note">
          Paste canonical Question IDs separated by commas or lines. The library checks each one
          before it becomes a entry.
        </p>
        <button
          class="quiet-action"
          type="button"
          disabled={entryBusy()}
          onClick={() => void addEntries()}
        >
          Check and add entries
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={entryBusy()}
          onClick={(event) => props.onChooseEntries(event.currentTarget)}
        >
          Choose entries
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
