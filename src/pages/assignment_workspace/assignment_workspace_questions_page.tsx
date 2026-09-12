import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentEntry } from "../../../generated/api/AssignmentEntry";
import type { AssignmentEntryId } from "../../../generated/api/AssignmentEntryId";
import type {
  AssignmentQuestionPickerEntry,
  SaveLiveAssignmentInput,
} from "../../api/assignment_release";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssignmentWorkspaceConflictError } from "../../api/http_client/assignment_release";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import { assignmentWorkspacePath } from "./assignment_workspace_paths";
import {
  appendAvailableFixedQuestion,
  moveAssignmentEntry,
  questionRevisionKey,
  removeAssignmentEntry,
} from "./assignment_workspace_questions_model";

const MAX_ASSIGNMENT_ENTRIES = 1024;

function saveInput(
  current: SaveLiveAssignmentInput,
  title: string,
  entries: ReadonlyArray<AssignmentEntry>,
): SaveLiveAssignmentInput {
  return { ...current, title, entries };
}

function entryId(): AssignmentEntryId {
  return crypto.randomUUID();
}

function AssignmentEntrySummary(props: {
  readonly entry: AssignmentEntry;
  readonly description: (reference: AssignmentQuestionPickerEntry["reference"]) => string;
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

/** Edits the complete normalized content owned by the current Assignment. */
export function AssignmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assignment().workspace;
  const [entries, setEntries] = createSignal<ReadonlyArray<AssignmentEntry>>(initial.entries);
  const [title, setTitle] = createSignal(initial.title);
  const [available, setAvailable] = createSignal<ReadonlyArray<AssignmentQuestionPickerEntry>>([]);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);

  const descriptions = createMemo(() => {
    const known = new Map<string, string>();
    for (const question of workspace.assignment().workspace.questions)
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
        await applicationApi.client.listLiveAssignmentQuestionPicker(workspace.courseReference),
      );
    } catch {
      setMessage(
        "Available published Questions could not load. Existing Assignment Entries remain here.",
      );
    }
  }

  function description(reference: AssignmentQuestionPickerEntry["reference"]): string {
    return descriptions().get(questionRevisionKey(reference)) ?? "Published Question";
  }

  function move(index: number, offset: -1 | 1): void {
    setEntries((current) => moveAssignmentEntry(current, index, offset));
    setMessage("Entry order changed. Save Questions when ready.");
  }

  function remove(index: number): void {
    setEntries((current) => removeAssignmentEntry(current, index));
    setMessage("Entry removed. Save Questions when ready.");
  }

  function add(candidate: AssignmentQuestionPickerEntry): void {
    setEntries((current) => appendAvailableFixedQuestion(current, candidate, entryId()));
    setMessage(
      "Available published Question added with its exact revision pin. Save Questions when ready.",
    );
  }

  async function save(): Promise<void> {
    if (needsReload()) {
      setMessage("Reload the latest assignment before saving. Your current Entries remain here.");
      return;
    }
    setBusy(true);
    try {
      await workspace.save(saveInput(workspace.assignment().workspace, title(), entries()));
      setMessage("Questions and order saved. Review assignment policies when you are ready.");
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssignmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assignment changed elsewhere. Reload latest assignment before saving; your current Entries remain here."
          : "Questions were not saved. Try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function reload(): Promise<void> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssignment();
      setEntries(latest.workspace.entries);
      setTitle(latest.workspace.title);
      setNeedsReload(false);
      setMessage("Latest assignment loaded. Review its complete ordered Entries.");
    } catch {
      setMessage("The latest assignment could not load. Your current Entries remain here.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="assignment-workspace-questions" aria-labelledby="assignment-questions-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-questions-heading">Questions</h1>
        <p class="page-lede">
          Every Entry retains its exact Question Revision and stable identity for future Attempts.
        </p>
      </header>
      <Show when={message()}>
        {(value) => (
          <p class="assignment-workspace-save-message" role="status">
            {value()}
          </p>
        )}
      </Show>
      <label class="assignment-editor-field">
        Assignment title
        <input value={title()} onInput={(event) => setTitle(event.currentTarget.value)} />
      </label>
      <section class="assignment-editor-panel" aria-labelledby="selected-questions-heading">
        <h2 id="selected-questions-heading">Ordered Assignment Entries</h2>
        <Show when={entries().length > 0} fallback={<p>No Entries are selected.</p>}>
          <ol>
            <For each={entries()}>
              {(entry, index) => (
                <li data-assignment-entry={entry.id}>
                  <AssignmentEntrySummary entry={entry} description={description} />{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === 0}
                    onClick={() => move(index(), -1)}
                  >
                    Move up
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === entries().length - 1}
                    onClick={() => move(index(), 1)}
                  >
                    Move down
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy()}
                    aria-label={`Remove Assignment Entry ${index() + 1}`}
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
      <section class="assignment-editor-panel" aria-labelledby="available-questions-heading">
        <h2 id="available-questions-heading">Available published Questions</h2>
        <p class="assignment-editor-note">
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
      <p class="assignment-editor-actions">
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
            Reload latest assignment
          </button>
        </Show>
      </p>
      <p class="assignment-workspace-next-actions">
        <A
          class="quiet-link"
          href={assignmentWorkspacePath(
            workspace.courseReference,
            workspace.assignmentReference,
            "policies",
          )}
        >
          Review assignment policies
        </A>
      </p>
    </section>
  );
}
