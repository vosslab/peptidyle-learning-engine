import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type {
  AssessmentQuestionPickerEntry,
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInput,
} from "../../api/assessment_release";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import {
  appendAvailableFixedQuestion,
  moveAssessmentEntry,
  questionRevisionKey,
  removeAssessmentEntry,
} from "./assessment_workspace_questions_model";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";

const MAX_ASSIGNMENT_ENTRIES = 1024;

export type QuestionEditDirtyEvent =
  "title" | "move" | "remove" | "add" | "saveSucceeded" | "saveFailed";

/** Keeps the leave guard active until the current structural edit was persisted successfully. */
export function nextQuestionEditDirty(current: boolean, event: QuestionEditDirtyEvent): boolean {
  if (event === "saveSucceeded") return false;
  if (event === "saveFailed") return current;
  return true;
}

/** Builds the closed save payload from the editable Assessment workspace values only. */
export function questionSaveInput(
  current: LiveAssessmentWorkspace,
  title: string,
  entries: ReadonlyArray<AssessmentEntry>,
): SaveLiveAssessmentInput {
  return {
    title,
    instructions: current.instructions,
    entries,
    dueAt: current.dueAt,
    availableAt: current.availableAt,
    closesAt: current.closesAt,
    lateWorkRule: current.lateWorkRule,
    assessmentAttemptTimeLimitSeconds: current.assessmentAttemptTimeLimitSeconds,
    attemptLimit: current.attemptLimit,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule: current.studentFeedbackReleaseRule,
  };
}

function entryId(): AssessmentEntryId {
  return crypto.randomUUID();
}

function AssessmentEntrySummary(props: {
  readonly entry: AssessmentEntry;
  readonly description: (reference: AssessmentQuestionPickerEntry["reference"]) => string;
}): JSX.Element {
  if (props.entry.kind === "questionPool") {
    return (
      <>
        <strong>Question pool</strong> - {props.entry.selectionCount} selected from{" "}
        {props.entry.items.length} pinned Questions
        <ul>
          <For each={props.entry.items}>
            {(item) => (
              <li data-question-pool-item={item.id}>
                {item.reference.questionId} * Revision {item.reference.revisionNumber} (
                {item.availability})
              </li>
            )}
          </For>
        </ul>
      </>
    );
  }
  return (
    <>
      <strong>{props.entry.reference.questionId}</strong> * Revision{" "}
      {props.entry.reference.revisionNumber}: {props.description(props.entry.reference)} (
      {props.entry.availability})
    </>
  );
}

/** Edits the complete normalized content owned by the current Assessment. */
export function AssessmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assessment().workspace;
  const [entries, setEntries] = createSignal<ReadonlyArray<AssessmentEntry>>(initial.entries);
  const [title, setTitle] = createSignal(initial.title);
  const [available, setAvailable] = createSignal<ReadonlyArray<AssessmentQuestionPickerEntry>>([]);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);

  const descriptions = createMemo(() => {
    const known = new Map<string, string>();
    for (const question of workspace.assessment().workspace.questions)
      known.set(questionRevisionKey(question.reference), question.description);
    for (const question of available())
      known.set(questionRevisionKey(question.reference), question.description);
    return known;
  });
  const availableToAdd = createMemo(() =>
    available().filter(
      (candidate) =>
        !entries().some(
          (entry) =>
            entry.kind === "fixedQuestion" &&
            questionRevisionKey(entry.reference) === questionRevisionKey(candidate.reference),
        ),
    ),
  );

  onMount(() => void loadAvailable());

  async function loadAvailable(): Promise<void> {
    try {
      setAvailable(
        await applicationApi.client.listLiveAssessmentQuestionPicker(workspace.courseReference),
      );
    } catch {
      setMessage(
        "Available published Questions could not load. Existing Assessment Entries remain here.",
      );
    }
  }

  function description(reference: AssessmentQuestionPickerEntry["reference"]): string {
    return descriptions().get(questionRevisionKey(reference)) ?? "Published Question";
  }

  function move(index: number, offset: -1 | 1): void {
    setEntries((current) => moveAssessmentEntry(current, index, offset));
    setDirty((current) => nextQuestionEditDirty(current, "move"));
    setMessage("Entry order changed. Save Questions when ready.");
  }

  function remove(index: number): void {
    setEntries((current) => removeAssessmentEntry(current, index));
    setDirty((current) => nextQuestionEditDirty(current, "remove"));
    setMessage("Entry removed. Save Questions when ready.");
  }

  function add(candidate: AssessmentQuestionPickerEntry): void {
    setEntries((current) => appendAvailableFixedQuestion(current, candidate, entryId()));
    setDirty((current) => nextQuestionEditDirty(current, "add"));
    setMessage(
      "Available published Question added with its exact revision pin. Save Questions when ready.",
    );
  }

  async function save(): Promise<boolean> {
    if (needsReload()) {
      setMessage("Reload the latest assessment before saving. Your current Entries remain here.");
      return false;
    }
    setBusy(true);
    try {
      await workspace.save(questionSaveInput(workspace.assessment().workspace, title(), entries()));
      setDirty((current) => nextQuestionEditDirty(current, "saveSucceeded"));
      setMessage("Questions and order saved. Review assessment policies when you are ready.");
      return true;
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssessmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assessment changed elsewhere. Reload latest assessment before saving; your current Entries remain here."
          : "Questions were not saved. Try again.",
      );
      setDirty((current) => nextQuestionEditDirty(current, "saveFailed"));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function reload(): Promise<void> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssessment();
      setEntries(latest.workspace.entries);
      setTitle(latest.workspace.title);
      setNeedsReload(false);
      setMessage("Latest assessment loaded. Review its complete ordered Entries.");
    } catch {
      setMessage("The latest assessment could not load. Your current Entries remain here.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="assessment-workspace-questions" aria-labelledby="assessment-questions-heading">
      <UnsavedChangesGuard dirty={dirty} save={save} />
      <header class="assessment-workspace-header">
        <p class="eyebrow">Assessment workspace</p>
        <h1 id="assessment-questions-heading">Questions</h1>
        <p class="page-lede">
          Every Entry retains its exact Question Revision and stable identity for future Attempts.
        </p>
      </header>
      <Show when={message()}>
        {(value) => (
          <p class="assessment-workspace-save-message" role="status">
            {value()}
          </p>
        )}
      </Show>
      <label class="assessment-editor-field">
        Assessment title
        <input
          value={title()}
          onInput={(event) => {
            setTitle(event.currentTarget.value);
            setDirty((current) => nextQuestionEditDirty(current, "title"));
          }}
        />
      </label>
      <section class="assessment-editor-panel" aria-labelledby="selected-questions-heading">
        <h2 id="selected-questions-heading">Ordered Assessment Entries</h2>
        <Show when={entries().length > 0} fallback={<p>No Entries are selected.</p>}>
          <ol>
            <For each={entries()}>
              {(entry, index) => (
                <li data-assessment-entry={entry.id}>
                  <AssessmentEntrySummary entry={entry} description={description} />{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === 0}
                    onClick={() => move(index(), -1)}
                  >
                    Move earlier
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === entries().length - 1}
                    onClick={() => move(index(), 1)}
                  >
                    Move later
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy()}
                    aria-label={`Remove Assessment Entry ${index() + 1}`}
                    onClick={() => remove(index())}
                  >
                    Remove
                  </button>
                </li>
              )}
            </For>
          </ol>
        </Show>
      </section>
      <section class="assessment-editor-panel" aria-labelledby="available-questions-heading">
        <h2 id="available-questions-heading">Available published Questions</h2>
        <p class="assessment-editor-note">
          Adding a Question pins the exact Available revision shown here.
        </p>
        <Show
          when={availableToAdd().length > 0}
          fallback={<p>No additional Available Questions are ready to add.</p>}
        >
          <ul>
            <For each={availableToAdd()}>
              {(candidate) => (
                <li>
                  <strong>{candidate.reference.questionId}</strong> * Revision{" "}
                  {candidate.reference.revisionNumber}: {candidate.description}{" "}
                  <button
                    type="button"
                    disabled={busy() || entries().length >= MAX_ASSIGNMENT_ENTRIES}
                    onClick={() => add(candidate)}
                  >
                    Add Question
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
      <p class="assessment-editor-actions">
        <button
          class="primary-action"
          type="button"
          disabled={busy() || needsReload()}
          onClick={() => void save()}
        >
          {busy() ? "Saving Questions..." : "Save Questions and order"}
        </button>
        <Show when={needsReload()}>
          <button type="button" onClick={() => void reload()}>
            Reload latest assessment
          </button>
        </Show>
      </p>
      <p class="assessment-workspace-next-actions">
        <A
          class="quiet-link"
          href={assessmentWorkspacePath(
            workspace.courseReference,
            workspace.assessmentReference,
            "policies",
          )}
        >
          Review assessment policies
        </A>
      </p>
    </section>
  );
}
