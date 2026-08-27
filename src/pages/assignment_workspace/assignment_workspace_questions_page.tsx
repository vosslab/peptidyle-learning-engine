// assignment_workspace_questions_page.tsx - Questions-owned assignment title and ordered definition.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import { AssignmentEditorContentList } from "../assignment_editor_content_list";
import { createAssignmentEditorCatalogController } from "../assignment_editor_catalog_controller";
import { ASSIGNMENT_EDITOR_STYLES } from "../assignment_editor_styles";
import {
  appendFixedEntries,
  appendSelectionGroup,
  assignmentContentInput,
  assignmentEditorDraftFrom,
  fixedEntries,
  moveAssignmentEntry,
  parseExactProblemDisplayReferences,
  questionBackendLabel,
  validateAssignmentEditorDraft,
  type AssignmentCatalogRow,
  type AssignmentEditorDraft,
  type AssignmentEditorSelectionGroupEntry,
} from "../assignment_editor_model";
import { AssignmentEditorProblemPicker } from "../assignment_editor_problem_picker";
import { createAssignmentEditorPickerController } from "../assignment_editor_picker_controller";
import { createAssignmentEditorReuseController } from "../assignment_editor_reuse_controller";
import { resolveAssignmentContentSaveFailure } from "../../api/http_client";
import type { PoolDrawPreview } from "../../api/contracts";

import { assignmentWorkspaceCreatePath, assignmentWorkspacePath } from "./assignment_workspace_nav";
import { useAssignmentWorkspace } from "./assignment_workspace_live_page";

function previewRevision(revision: string): TeachingOperationRevision {
  const match = /^"([1-9][0-9]*)"$/u.exec(revision);
  const value = match?.[1];
  if (value === undefined || BigInt(value) > 9_223_372_036_854_775_807n)
    throw new Error("Save the assignment before requesting a pool sample.");
  return value;
}

/** Keeps Questions edits local until the Instructor explicitly saves the complete ordered definition. */
export function AssignmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssignmentWorkspace();
  const [draft, setDraft] = createSignal<AssignmentEditorDraft>(
    assignmentEditorDraftFrom(workspace.assignment()),
  );
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [validationMessage, setValidationMessage] = createSignal("");
  const [directQuestionId, setDirectQuestionId] = createSignal("");
  const [directMessage, setDirectMessage] = createSignal("");
  const [targetItemId, setTargetItemId] = createSignal<string>();
  const [poolPreview, setPoolPreview] = createSignal<PoolDrawPreview>();
  const [needsReload, setNeedsReload] = createSignal(false);
  const [issuedWorkRecovery, setIssuedWorkRecovery] = createSignal(false);
  let questionIdInput: HTMLInputElement | undefined;

  const catalogController = createAssignmentEditorCatalogController(workspace.repository);
  const reuseController = createAssignmentEditorReuseController(
    workspace.repository,
    workspace.courseId,
    workspace.assignmentId,
  );

  /**
   * A stale revision is a recovery state, rather than a transient save error. Keep it sticky
   * while local edits continue so another action cannot submit the same stale revision. The
   * explicit reload is the deliberate, visible choice that replaces the local draft.
   */
  function update(next: AssignmentEditorDraft, nextMessage: string): void {
    setDraft(next);
    setPoolPreview(undefined);
    const failure = validateAssignmentEditorDraft(next);
    setValidationMessage(failure === null ? "" : `${failure} Correct the questions, then save.`);
    setMessage(nextMessage);
  }

  function replacePool(entryIndex: number, entry: AssignmentEditorSelectionGroupEntry): void {
    const entries = [...draft().entries];
    entries[entryIndex] = entry;
    update(
      { ...draft(), entries },
      "Pool updated. Review its candidates, then save questions and order.",
    );
  }

  function removeEntry(entryIndex: number): void {
    const entries = draft()
      .entries.filter((_entry, index) => index !== entryIndex)
      .map((entry, position) => ({ ...entry, position }));
    update(
      { ...draft(), entries },
      "Question removed. Save questions and order when the definition is ready.",
    );
  }

  function addPool(): void {
    if (draft().entries.length >= MAX_ASSIGNMENT_ORDERED_ENTRIES) {
      setValidationMessage(
        `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer, then save.`,
      );
      return;
    }
    update(
      appendSelectionGroup(draft()),
      "Question pool added. Add candidate Question IDs, set its draw count, then save questions and order.",
    );
  }

  function startReplacement(itemId: string): void {
    setTargetItemId(itemId);
    setDirectQuestionId("");
    catalogController.setSelected(undefined);
    queueMicrotask(() => questionIdInput?.focus());
  }

  function replaceFixedQuestion(row: AssignmentCatalogRow, itemId: string): void {
    const current = fixedEntries(draft()).find((entry) => entry.id === itemId);
    if (current === undefined) {
      setMessage(
        "That question is no longer in the local definition. Reload the assignment to continue.",
      );
      return;
    }
    if (
      fixedEntries(draft()).some(
        (entry) => entry.questionId === row.questionId && entry.id !== itemId,
      )
    ) {
      setDirectMessage(`${row.questionId} is already in this assignment.`);
      return;
    }
    const entries = draft().entries.map((entry) =>
      entry.kind === "fixed" && entry.id === itemId
        ? { ...entry, questionId: row.questionId, title: row.title, backend: row.backend }
        : entry,
    );
    update(
      { ...draft(), entries },
      `${row.questionId} will replace ${current.questionId} when you save questions and order.`,
    );
    setTargetItemId(undefined);
    catalogController.setSelected(undefined);
    setDirectQuestionId("");
  }

  function addRows(rows: ReadonlyArray<AssignmentCatalogRow>, success: string): void {
    const next = appendFixedEntries(draft(), rows);
    if (next === draft()) {
      setMessage("Every selected Question ID is already in this assignment.");
      return;
    }
    update(next, success);
  }

  async function chooseQuestionId(): Promise<void> {
    try {
      const row = await catalogController.lookup(directQuestionId());
      catalogController.setSelected(row);
      setMessage(
        targetItemId() === undefined
          ? `${row.questionId} is ready to add.`
          : `${row.questionId} is ready to replace the selected question.`,
      );
      setDirectMessage("");
    } catch (_error: unknown) {
      setDirectMessage("That Question ID could not be found. Check it and try again.");
    }
  }

  async function addQuestionIds(): Promise<void> {
    let questionIds: ReadonlyArray<string>;
    try {
      questionIds = parseExactProblemDisplayReferences(directQuestionId());
    } catch (error: unknown) {
      setDirectMessage(error instanceof Error ? error.message : "Question IDs are invalid.");
      return;
    }
    if (targetItemId() !== undefined) {
      setDirectMessage("Choose one replacement Question ID, then use the selected replacement.");
      return;
    }
    if (draft().entries.length + questionIds.length > MAX_ASSIGNMENT_ORDERED_ENTRIES) {
      setDirectMessage(
        `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer, then add Question IDs.`,
      );
      return;
    }
    try {
      const rows = await Promise.all(
        questionIds.map(
          async (questionId) => await workspace.repository.resolvePublished(questionId),
        ),
      );
      addRows(
        rows,
        `Added ${rows.length} Question ID${rows.length === 1 ? "" : "s"}. Save questions and order when ready.`,
      );
      setDirectQuestionId("");
      setDirectMessage("");
    } catch (_error: unknown) {
      setDirectMessage("A Question ID could not be found. Your Question IDs are still here.");
    }
  }

  async function saveQuestions(): Promise<boolean> {
    const current = draft();
    if (busy()) return false;
    if (issuedWorkRecovery()) {
      setMessage(
        "Create a new assignment to use these structural question changes. This assignment's issued learner work remains unchanged.",
      );
      return false;
    }
    if (needsReload()) {
      setMessage(
        "Reload the latest assignment before saving. Your entered title and question changes remain here until you choose Reload latest assignment.",
      );
      return false;
    }
    if (current.title.trim() === "") {
      setValidationMessage("Enter an assignment title before saving questions and order.");
      return false;
    }
    const failure = validateAssignmentEditorDraft(current);
    if (failure !== null) {
      setValidationMessage(`${failure} Correct the questions, then save.`);
      return false;
    }
    setBusy(true);
    try {
      const saved = await workspace.client.saveAssignmentContent(
        workspace.courseId,
        workspace.assignmentId,
        assignmentContentInput(current),
        current.revision,
      );
      workspace.replaceAssignment(saved);
      setDraft(assignmentEditorDraftFrom(saved));
      setPoolPreview(undefined);
      setNeedsReload(false);
      setValidationMessage("");
      setMessage("Questions and order saved. Review assignment policies when you are ready.");
      return true;
    } catch (error: unknown) {
      const failure = resolveAssignmentContentSaveFailure(error);
      if (failure.kind === "staleRevision") {
        setNeedsReload(true);
        setMessage(failure.message);
      } else if (failure.kind === "issuedLearnerWork") {
        setIssuedWorkRecovery(true);
        setMessage(failure.message);
      } else {
        setMessage(failure.message);
      }
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function reloadLatest(): Promise<void> {
    setBusy(true);
    try {
      const latest = await workspace.reloadAssignment();
      setDraft(assignmentEditorDraftFrom(latest));
      setPoolPreview(undefined);
      setNeedsReload(false);
      setIssuedWorkRecovery(false);
      setValidationMessage("");
      setMessage(
        "Latest assignment loaded; your local title and question changes were replaced. Review the current questions and order before saving.",
      );
    } catch (_error: unknown) {
      setMessage(
        "The latest assignment could not be loaded. Your local question changes remain here.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function previewPool(groupPosition: number): Promise<void> {
    const failure = validateAssignmentEditorDraft(draft());
    if (failure !== null) {
      setValidationMessage(`${failure} Correct the questions, then preview a pool draw.`);
      return;
    }
    setMessage("Saving questions, then generating a server sample for this pool.");
    if (!(await saveQuestions())) return;
    const saved = workspace.assignment();
    setBusy(true);
    try {
      const preview = await workspace.client.previewPoolDraw(
        workspace.courseReference,
        saved.reference,
        previewRevision(saved.revision),
        groupPosition,
      );
      setPoolPreview(preview);
      setMessage(`${preview.groupLabel} server sample is ready. It does not create learner work.`);
    } catch (_error: unknown) {
      setMessage("The pool sample could not be generated. The saved questions remain available.");
    } finally {
      setBusy(false);
    }
  }

  const pickerController = createAssignmentEditorPickerController({
    repository: workspace.repository,
    courseId: workspace.courseId,
    mode: { kind: "workspace", assignmentId: workspace.assignmentId },
    currentDraft: draft,
    editorBusy: busy,
    setBusy,
    onDraftChange: (next) =>
      update(next, "Selected questions added. Save questions and order when ready."),
    onReplacementPrepared: (row, itemId) => {
      catalogController.setSelected(row);
      setTargetItemId(itemId);
      setMessage(`${row.questionId} is ready to replace the selected question.`);
    },
    onMessage: setMessage,
    onError: (_error, fallback) => setMessage(fallback),
  });

  onMount(() => {
    void pickerController.loadSources();
    void reuseController.load();
  });

  return (
    <section class="assignment-workspace-questions" aria-labelledby="assignment-questions-heading">
      <style>{ASSIGNMENT_EDITOR_STYLES}</style>
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-questions-heading">Questions</h1>
        <p class="page-lede">
          Set the assignment title, ordered fixed questions, and question pools.
        </p>
      </header>
      <p role="status" aria-live="polite">
        {message()}
      </p>
      <Show when={needsReload()}>
        <section class="inline-error" role="alert">
          <p>
            Reload the latest assignment before saving another set of question changes. Reloading
            replaces your local title and question changes with the server version.
          </p>
          <button
            class="quiet-action"
            type="button"
            disabled={busy()}
            aria-label="Reload latest assignment and replace local changes"
            onClick={() => void reloadLatest()}
          >
            Reload latest assignment
          </button>
        </section>
      </Show>
      <Show when={issuedWorkRecovery()}>
        <section class="inline-error" role="alert">
          <p>
            Learner work has already been issued for this assignment. Your local question changes
            remain here, and the issued learner work remains unchanged.
          </p>
          <p>
            <A class="primary-link" href={assignmentWorkspaceCreatePath(workspace.courseReference)}>
              Create a new assignment
            </A>
          </p>
        </section>
      </Show>
      <Show when={validationMessage()}>
        {(value) => (
          <section class="inline-error" role="alert">
            <p>{value()}</p>
          </section>
        )}
      </Show>
      <fieldset class="assignment-editor-operation-boundary" disabled={busy()}>
        <div class="assignment-editor-actions">
          <button
            class="primary-action"
            type="button"
            disabled={busy() || needsReload() || issuedWorkRecovery()}
            onClick={() => void saveQuestions()}
          >
            Save questions and order
          </button>
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
        </div>
        <div class="assignment-editor-grid">
          <section class="assignment-editor-panel">
            <label class="assignment-editor-field" for="assignment-questions-title">
              Assignment title
              <input
                id="assignment-questions-title"
                value={draft().title}
                onInput={(event) =>
                  update(
                    { ...draft(), title: event.currentTarget.value },
                    "Title changed. Save questions and order when ready.",
                  )
                }
              />
            </label>
            <button class="quiet-action" type="button" onClick={addPool}>
              Add question pool
            </button>
            <p class="assignment-editor-note">
              A pool draws a configured number of candidates with the server&apos;s fixed Draw
              algorithm v1.
            </p>
            <Show
              when={draft().entries.length > 0}
              fallback={
                <section class="empty-state" aria-label="Empty question definition">
                  <p>Add at least one question.</p>
                  <p class="assignment-editor-note">
                    Search the library, reuse a saved selection, or enter a Question ID below.
                  </p>
                </section>
              }
            >
              <AssignmentEditorContentList
                entries={draft().entries}
                createMode={false}
                busy={busy()}
                preview={poolPreview()}
                resolveCandidates={async (questionIds) =>
                  await Promise.all(
                    questionIds.map(
                      async (questionId) => await workspace.repository.resolvePublished(questionId),
                    ),
                  )
                }
                onMove={(entryIndex, direction) =>
                  update(
                    moveAssignmentEntry(draft(), entryIndex, direction),
                    "Question order changed. Save questions and order when ready.",
                  )
                }
                onReplace={startReplacement}
                onRemoveFixed={(itemId) => {
                  const index = draft().entries.findIndex(
                    (entry) => entry.kind === "fixed" && entry.id === itemId,
                  );
                  if (index >= 0) removeEntry(index);
                }}
                onPoolChange={replacePool}
                onRemovePool={removeEntry}
                onMessage={setMessage}
                onPreviewPool={(groupPosition) => void previewPool(groupPosition)}
                onChoosePoolCandidates={(entryIndex, trigger) =>
                  pickerController.open({ kind: "pool", entryIndex }, trigger)
                }
              />
            </Show>
          </section>

          <section class="assignment-editor-panel">
            <h2>{targetItemId() === undefined ? "Add questions" : "Replace question"}</h2>
            <Show when={targetItemId() !== undefined}>
              <p class="assignment-editor-note">
                The replacement applies to future runs when you save questions and order. Issued
                work remains with its original question.
              </p>
            </Show>
            <label class="assignment-editor-field" for="assignment-question-id">
              {targetItemId() === undefined ? "Question IDs" : "Replacement Question ID"}
              <input
                id="assignment-question-id"
                ref={(element) => {
                  questionIdInput = element;
                }}
                value={directQuestionId()}
                placeholder="7K3-M9QP"
                aria-describedby="assignment-question-id-help"
                aria-invalid={directMessage() !== ""}
                onInput={(event) => {
                  setDirectQuestionId(event.currentTarget.value);
                  setDirectMessage("");
                  catalogController.setSelected(undefined);
                }}
              />
            </label>
            <p id="assignment-question-id-help" class="assignment-editor-note">
              Add one or more canonical Question IDs separated by commas or lines. Use one ID to
              choose a replacement.
            </p>
            <div class="assignment-editor-actions">
              <button class="quiet-action" type="button" onClick={() => void chooseQuestionId()}>
                Check Question ID
              </button>
              <button
                class="primary-action"
                type="button"
                disabled={
                  targetItemId() !== undefined && catalogController.selected() === undefined
                }
                onClick={() => {
                  const itemId = targetItemId();
                  const selected = catalogController.selected();
                  if (itemId !== undefined && selected !== undefined)
                    replaceFixedQuestion(selected, itemId);
                  else void addQuestionIds();
                }}
              >
                {targetItemId() === undefined ? "Add Question IDs" : "Use selected replacement"}
              </button>
              <button
                class="quiet-action"
                type="button"
                disabled={pickerController.sources().length === 0}
                onClick={(event) =>
                  pickerController.open(
                    targetItemId() === undefined
                      ? { kind: "fixed" }
                      : { kind: "replacement", itemId: targetItemId()! },
                    event.currentTarget,
                  )
                }
              >
                {targetItemId() === undefined ? "Search question library" : "Choose replacement"}
              </button>
            </div>
            <Show when={catalogController.selected()}>
              {(row) => (
                <p class="success-state">
                  Selected: {row().questionId} {row().title} ({questionBackendLabel(row().backend)})
                </p>
              )}
            </Show>
            <Show when={directMessage()}>
              {(value) => (
                <p class="inline-error" role="alert">
                  {value()}
                </p>
              )}
            </Show>

            <details class="assignment-editor-reuse">
              <summary>Reuse questions from a saved assignment</summary>
              <Show when={reuseController.message()}>
                {(value) => (
                  <p class="inline-error" role="alert">
                    {value()}
                  </p>
                )}
              </Show>
              <Show
                when={reuseController.reuse().length > 0}
                fallback={
                  <p class="assignment-editor-note">
                    No other saved assignments are available in this course yet.
                  </p>
                }
              >
                <label class="assignment-editor-field">
                  Source assignment
                  <select
                    value={reuseController.sourceIndex() ?? ""}
                    onChange={(event) =>
                      reuseController.chooseSource(Number(event.currentTarget.value))
                    }
                  >
                    <For each={reuseController.reuse()}>
                      {(assignment, index) => <option value={index()}>{assignment.title}</option>}
                    </For>
                  </select>
                </label>
                <div class="assignment-editor-reuse-checklist">
                  <For each={reuseController.selectedSource()?.questions ?? []}>
                    {(question, index) => (
                      <label>
                        <input
                          type="checkbox"
                          checked={reuseController.questionIndexes().has(index())}
                          onChange={(event) =>
                            reuseController.toggleQuestion(index(), event.currentTarget.checked)
                          }
                        />
                        <span>
                          <strong>{question.title}</strong>
                          <small>{question.questionId}</small>
                        </span>
                      </label>
                    )}
                  </For>
                </div>
                <div class="assignment-editor-reuse-actions">
                  <button
                    class="primary-action"
                    type="button"
                    onClick={() =>
                      addRows(
                        (reuseController.selectedSource()?.questions ?? []).filter(
                          (_question, index) => reuseController.questionIndexes().has(index),
                        ),
                        "Selected saved questions added. Save questions and order when ready.",
                      )
                    }
                  >
                    Add selected questions
                  </button>
                  <button
                    class="quiet-action"
                    type="button"
                    onClick={() =>
                      addRows(
                        reuseController.selectedSource()?.questions ?? [],
                        "Saved assignment questions added. Save questions and order when ready.",
                      )
                    }
                  >
                    Add entire assignment
                  </button>
                </div>
              </Show>
            </details>
          </section>
        </div>
      </fieldset>
      <AssignmentEditorProblemPicker
        repository={workspace.repository}
        controller={pickerController}
      />
    </section>
  );
}
