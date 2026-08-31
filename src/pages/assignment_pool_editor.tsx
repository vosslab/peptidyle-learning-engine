// assignment_pool_editor.tsx - accessible editor controls for one ordered pool entry.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { PoolDrawPreview } from "../api/contracts";
import { MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL } from "../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL";

import type {
  AssignmentCatalogRow,
  AssignmentEditorQuestionPoolEntry,
} from "./assignment_editor_model";
import {
  parseExactProblemDisplayReferences,
  validateQuestionPoolEntry,
} from "./assignment_editor_model";

export interface AssignmentPoolEditorProps {
  readonly entry: AssignmentEditorQuestionPoolEntry;
  readonly entryIndex: number;
  readonly entryCount: number;
  readonly resolveCandidates: (
    questionIds: ReadonlyArray<string>,
  ) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
  readonly onChange: (entry: AssignmentEditorQuestionPoolEntry) => void;
  readonly onMove: (direction: -1 | 1) => void;
  readonly onRemove: () => void;
  readonly onMessage: (message: string) => void;
  readonly preview: PoolDrawPreview | undefined;
  readonly previewBusy: boolean;
  readonly onPreview: () => void;
  readonly onChooseCandidates: (trigger: HTMLButtonElement) => void;
}

function poolLabel(entryIndex: number): string {
  return `Question pool ${entryIndex + 1}`;
}

export function AssignmentPoolEditor(props: AssignmentPoolEditorProps): JSX.Element {
  const [candidateText, setCandidateText] = createSignal("");
  const [candidateError, setCandidateError] = createSignal("");
  const [candidateBusy, setCandidateBusy] = createSignal(false);

  function update(next: Partial<AssignmentEditorQuestionPoolEntry>): void {
    const entry = { ...props.entry, ...next };
    const error = validateQuestionPoolEntry(entry);
    setCandidateError(error ?? "");
    props.onChange(entry);
    props.onMessage(
      error === null
        ? "Pool updated. Review its candidates, then save the complete assignment definition."
        : `${error} Correct this pool before saving.`,
    );
  }

  async function addCandidates(): Promise<void> {
    let questionIds: ReadonlyArray<string>;
    try {
      questionIds = parseExactProblemDisplayReferences(candidateText());
    } catch (error: unknown) {
      setCandidateError(
        error instanceof Error ? error.message : "Candidate Question IDs are invalid.",
      );
      return;
    }
    const existing = new Set(props.entry.candidates.map((candidate) => candidate.questionId));
    if (questionIds.some((questionId) => existing.has(questionId))) {
      setCandidateError("Each candidate Question ID can appear only once in a pool.");
      return;
    }
    if (
      props.entry.candidates.length + questionIds.length >
      MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL
    ) {
      setCandidateError(
        `Keep this pool to ${MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL} candidate Question IDs or fewer, then check and add candidates.`,
      );
      return;
    }
    setCandidateBusy(true);
    try {
      const candidates = await props.resolveCandidates(questionIds);
      const nextCandidates = [...props.entry.candidates, ...candidates];
      const entry = { ...props.entry, candidates: nextCandidates };
      const error = validateQuestionPoolEntry(entry);
      setCandidateError(error ?? "");
      props.onChange(entry);
      setCandidateText("");
      props.onMessage(
        `Added ${candidates.length} candidate Question ID${candidates.length === 1 ? "" : "s"}. Set the draw count, then save the assignment.`,
      );
    } catch (error: unknown) {
      setCandidateError(
        error instanceof Error
          ? `${error.message} Your candidate Question IDs are still here.`
          : "Candidates could not be checked. Your candidate Question IDs are still here.",
      );
    } finally {
      setCandidateBusy(false);
    }
  }

  function removeCandidate(questionId: string): void {
    const candidates = props.entry.candidates.filter(
      (candidate) => candidate.questionId !== questionId,
    );
    update({ candidates });
  }
  function updateOrdering(value: string): void {
    if (value !== "candidateOrder" && value !== "randomized") return;
    update({ ordering: value });
  }

  return (
    <li class="assignment-editor-row assignment-editor-pool" aria-label={poolLabel(props.entryIndex)}>
      <div>
        <h3>Question pool</h3>
        <p>
          Draw algorithm v1. The server selects from the candidate Question IDs and records each
          student&apos;s actual draw as immutable evidence.
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
          {props.preview === undefined ? "Preview draw" : "Preview another draw"}
        </button>
        <button class="quiet-action" type="button" onClick={props.onRemove}>
          Remove pool
        </button>
      </div>
      <div class="assignment-editor-pool-fields">
        <label class="assignment-editor-field">
          Draw count
          <input
            type="number"
            min="1"
            max={props.entry.candidates.length || undefined}
            value={props.entry.drawCount}
            onInput={(event) => update({ drawCount: Number(event.currentTarget.value) })}
          />
        </label>
        <label class="assignment-editor-field">
          Points per drawn question
          <input
            inputmode="decimal"
            value={props.entry.pointsPerItem}
            onInput={(event) => update({ pointsPerItem: event.currentTarget.value })}
          />
        </label>
        <label class="assignment-editor-field">
          Delivery order
          <select
            value={props.entry.ordering}
            onChange={(event) => updateOrdering(event.currentTarget.value)}
          >
            <option value="candidateOrder">Candidate order</option>
            <option value="randomized">Randomized</option>
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
              Draw {preview().drawCount} in {preview().ordering} order with Draw algorithm v1.
            </p>
            <h5>Candidate questions</h5>
            <ul>
              <For each={preview().candidates}>
                {(candidate) => (
                  <li>
                    <strong>{candidate.questionId}</strong> {candidate.title}
                  </li>
                )}
              </For>
            </ul>
            <h5>Server-sampled draw</h5>
            <ol>
              <For each={preview().sampled}>
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
        class="assignment-editor-pool-candidates"
        aria-labelledby={`pool-candidates-${props.entryIndex}`}
      >
        <h4 id={`pool-candidates-${props.entryIndex}`}>Candidate Question IDs</h4>
        <p class="assignment-editor-note">
          Add candidates in the order you want preserved when delivery order is Candidate order.
        </p>
        <ul>
          <For each={props.entry.candidates} fallback={<li>No candidates yet.</li>}>
            {(candidate) => (
              <li>
                <span>
                  <strong>{candidate.questionId}</strong> {candidate.title}
                </span>
                <button
                  class="quiet-action"
                  type="button"
                  aria-label={`Remove ${candidate.questionId} from this question pool`}
                  onClick={() => removeCandidate(candidate.questionId)}
                >
                  Remove
                </button>
              </li>
            )}
          </For>
        </ul>
        <label class="assignment-editor-field">
          Add candidate Question IDs
          <textarea
            rows="2"
            value={candidateText()}
            placeholder="7K3-M9QP, 7K4-M9QP"
            aria-invalid={candidateError() !== ""}
            aria-describedby={`pool-candidate-help-${props.entryIndex}`}
            onInput={(event) => {
              setCandidateText(event.currentTarget.value);
              setCandidateError("");
            }}
          />
        </label>
        <p id={`pool-candidate-help-${props.entryIndex}`} class="assignment-editor-note">
          Paste canonical Question IDs separated by commas or lines. The library checks each one
          before it becomes a candidate.
        </p>
        <button
          class="quiet-action"
          type="button"
          disabled={candidateBusy()}
          onClick={() => void addCandidates()}
        >
          Check and add candidates
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={candidateBusy()}
          onClick={(event) => props.onChooseCandidates(event.currentTarget)}
        >
          Choose candidates
        </button>
        <Show when={candidateError()}>
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
