import { A } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { SaveLiveAssignmentInput } from "../../api/assignment_release";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssignmentWorkspaceConflictError } from "../../api/http_client/assignment_release";
import { QuestionPicker, questionLibraryPickerSources } from "../../features/question_picker";
import { createAssignmentEditorRepository } from "../assignment_editor_repository";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";
import { assignmentWorkspacePath } from "./assignment_workspace_paths";

const MAX_ASSIGNMENT_QUESTION_SELECTION = 1024;

function withQuestionIds(
  workspace: Omit<SaveLiveAssignmentInput, "questionIds">,
  title: string,
  questionIds: ReadonlyArray<QuestionId>,
): SaveLiveAssignmentInput {
  return {
    title,
    instructions: workspace.instructions,
    questionIds,
    dueAt: workspace.dueAt,
    lateWorkRule: workspace.lateWorkRule,
    assignmentAttemptTimeLimitSeconds: workspace.assignmentAttemptTimeLimitSeconds,
    attemptLimit: workspace.attemptLimit,
    activityRules: workspace.activityRules,
    studentFeedbackReleaseRule: workspace.studentFeedbackReleaseRule,
  };
}

/** Removes one selected Question while retaining the authored order of every other Question. */
function removeSelectedQuestionAt(
  questionIds: ReadonlyArray<QuestionId>,
  index: number,
): ReadonlyArray<QuestionId> {
  if (index < 0 || index >= questionIds.length) return questionIds;
  return questionIds.filter((_questionId, currentIndex) => currentIndex !== index);
}

/** Selects and orders only the direct resource's fixed Questions. */
export function AssignmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const applicationApi = useApplicationApi();
  const [available, setAvailable] = createSignal<
    ReadonlyArray<{ readonly questionId: QuestionId; readonly description: string }>
  >([]);
  const [selected, setSelected] = createSignal<ReadonlyArray<QuestionId>>(
    workspace.assignment().workspace.questions.map((question) => question.questionId),
  );
  const [title, setTitle] = createSignal(workspace.assignment().workspace.title);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [pickerOpen, setPickerOpen] = createSignal(false);
  const pickerRepository = createAssignmentEditorRepository(applicationApi.client);
  let pickerTrigger: HTMLButtonElement | undefined;

  onMount(() => {
    void loadAvailable();
  });

  async function loadAvailable(): Promise<void> {
    try {
      setAvailable(
        await applicationApi.client.listLiveAssignmentQuestionPicker(workspace.courseReference),
      );
    } catch {
      setMessage("Question choices could not load. Current selected Questions remain available.");
    }
  }

  const selectedEntries = (): ReadonlyArray<{
    readonly questionId: QuestionId;
    readonly description: string;
  }> =>
    selected().map(
      (questionId) =>
        available().find((entry) => entry.questionId === questionId) ?? {
          questionId,
          description: questionId,
        },
    );

  function move(index: number, offset: -1 | 1): void {
    setSelected((current) => {
      const target = index + offset;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target]!, next[index]!];
      return next;
    });
  }

  function remove(index: number): void {
    setSelected((current) => removeSelectedQuestionAt(current, index));
    setMessage("Question removed. Save Questions and order when ready.");
  }

  function addPickerSelection(
    questionIds: ReadonlyArray<string>,
    questions: ReadonlyArray<{ readonly questionId: string; readonly description: string }>,
  ): void {
    const known = new Set(selected());
    const addedQuestionIds = questionIds.filter((questionId) => !known.has(questionId));
    if (addedQuestionIds.length === 0) {
      setMessage("Every selected Question is already in this assignment.");
      setPickerOpen(false);
      return;
    }
    setSelected((current) => [...current, ...addedQuestionIds]);
    setAvailable((current) => {
      const currentById = new Set(current.map((entry) => entry.questionId));
      const additions = questions
        .filter((question) => addedQuestionIds.includes(question.questionId))
        .filter((question) => !currentById.has(question.questionId))
        .map((question) => ({
          questionId: question.questionId,
          description: question.description,
        }));
      return [...current, ...additions];
    });
    setMessage("Selected Questions added. Save Questions and order when ready.");
    setPickerOpen(false);
  }

  async function save(): Promise<void> {
    if (needsReload()) {
      setMessage(
        "Reload the latest assignment before saving. Your selected Questions remain here.",
      );
      return;
    }
    setBusy(true);
    try {
      await workspace.save(withQuestionIds(workspace.assignment().workspace, title(), selected()));
      setMessage("Questions and order saved. Review assignment policies when you are ready.");
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssignmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assignment changed elsewhere. Reload latest assignment before saving; your selected Questions remain here."
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
      setSelected(latest.workspace.questions.map((question) => question.questionId));
      setTitle(latest.workspace.title);
      setNeedsReload(false);
      setMessage("Latest assignment loaded. Review its Questions and order.");
    } catch {
      setMessage("The latest assignment could not load. Your selected Questions remain here.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="assignment-workspace-questions" aria-labelledby="assignment-questions-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-questions-heading">Questions</h1>
        <p class="page-lede">Select and order the Questions students receive.</p>
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
        <h2 id="selected-questions-heading">Selected Questions and order</h2>
        <p>
          <button
            class="quiet-action"
            type="button"
            disabled={
              busy() || needsReload() || selected().length >= MAX_ASSIGNMENT_QUESTION_SELECTION
            }
            ref={(element) => {
              pickerTrigger = element;
            }}
            onClick={() => setPickerOpen(true)}
          >
            Search question library
          </button>
        </p>
        <Show when={selectedEntries().length > 0} fallback={<p>No Questions are selected.</p>}>
          <ol>
            <For each={selectedEntries()}>
              {(entry, index) => (
                <li>
                  <strong>{entry.questionId}</strong> {entry.description}{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === 0}
                    onClick={() => move(index(), -1)}
                  >
                    Move up
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy() || index() === selectedEntries().length - 1}
                    onClick={() => move(index(), 1)}
                  >
                    Move down
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy()}
                    aria-label={`Remove Question ${entry.questionId}`}
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
      <Show when={pickerOpen()}>
        <QuestionPicker
          repository={pickerRepository.questionPickerRepository}
          sources={questionLibraryPickerSources(true)}
          mode="many"
          maximumSelection={MAX_ASSIGNMENT_QUESTION_SELECTION - selected().length}
          trigger={pickerTrigger}
          title="Choose assignment questions"
          confirmLabel="Add selected questions"
          onConfirm={(selection) =>
            addPickerSelection(
              selection.questionIds,
              selection.questions.map((question) => ({
                questionId: question.questionId,
                description: question.row.questionTitle,
              })),
            )
          }
          onCancel={() => setPickerOpen(false)}
        />
      </Show>
    </section>
  );
}
