// assignment_workspace_questions_page.tsx - Questions-owned assignment title and ordered definition.

import { A } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onMount, type JSX } from "solid-js";

import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import { AssignmentEditorContentList } from "../assignment_editor_content_list";
import { createAssignmentEditorQuestionLookupController } from "../assignment_editor_question_lookup_controller";
import {
  appendFixedEntries,
  appendQuestionPool,
  assignmentContentInput,
  assignmentEditorDraftFrom,
  fixedEntries,
  moveAssignmentEntry,
  parseExactProblemDisplayReferences,
  questionBackendLabel,
  validateAssignmentEditorDraft,
  type AssignmentQuestionRow,
  type AssignmentEditorDraft,
  type AssignmentEditorQuestionPoolEntry,
} from "../assignment_editor_model";
import { AssignmentEditorQuestionPicker } from "../assignment_editor_question_picker";
import { createAssignmentEditorPickerController } from "../assignment_editor_picker_controller";
import { createAssignmentEditorReuseController } from "../assignment_editor_reuse_controller";
import {
  resolveAssignmentContentSaveFailure,
  resolveAssignmentFixedItemReplacementFailure,
} from "../../api/http_client";
import type { QuestionPoolPreview } from "../../api/contracts";

import { assignmentWorkspacePath } from "./assignment_workspace_nav";
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
  const [poolPreview, setPoolPreview] = createSignal<QuestionPoolPreview>();
  const [hasUnsavedContent, setHasUnsavedContent] = createSignal(false);
  const [needsReload, setNeedsReload] = createSignal(false);
  const [successorRevisionRequired, setSuccessorRevisionRequired] = createSignal(false);
  let questionIdInput: HTMLInputElement | undefined;
  let saveQuestionsButton: HTMLButtonElement | undefined;
  let reloadLatestButton: HTMLButtonElement | undefined;
  let replaceSelectedQuestionButton: HTMLButtonElement | undefined;

  const questionLookupController = createAssignmentEditorQuestionLookupController(
    workspace.repository,
  );
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
    setHasUnsavedContent(true);
    const failure = validateAssignmentEditorDraft(next);
    setValidationMessage(failure === null ? "" : `${failure} Correct the questions, then save.`);
    setMessage(nextMessage);
  }

  function replacePool(entryIndex: number, entry: AssignmentEditorQuestionPoolEntry): void {
    const entries = [...draft().entries];
    entries[entryIndex] = entry;
    update(
      { ...draft(), entries },
      "Pool updated. Review its candidates, then save questions and order.",
    );
  }

  function removeEntry(entryIndex: number): void {
    const entries = draft().entries.filter((_entry, index) => index !== entryIndex);
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
      appendQuestionPool(draft()),
      "Question Pool added. Add candidate Question IDs, set its selection count, then save questions and order.",
    );
  }

  function focusReplacementRecovery(): void {
    if (needsReload() || successorRevisionRequired()) {
      queueMicrotask(() => reloadLatestButton?.focus());
      return;
    }
    queueMicrotask(() => saveQuestionsButton?.focus());
  }

  function replacementBlocked(): boolean {
    if (!hasUnsavedContent() && !needsReload() && !successorRevisionRequired()) return false;
    if (successorRevisionRequired()) {
      setMessage(
        "A successor Draft Assignment Revision is required before structural question changes can be saved.",
      );
    } else if (needsReload()) {
      setMessage(
        "Reload the latest assignment before replacing a question. Your selected replacement remains available after reload.",
      );
    } else if (questionLookupController.selected() === undefined) {
      setMessage(
        "Save questions and order before choosing a replacement. Replacement happens in a separate action after you select a question.",
      );
    } else {
      setMessage(
        "Save questions and order before replacing a question. The selected replacement is not committed with the full definition.",
      );
    }
    focusReplacementRecovery();
    return true;
  }

  function replacementTargetTitle(): string | undefined {
    const itemId = targetItemId();
    if (itemId === undefined) return undefined;
    return fixedEntries(draft()).find((entry) => entry.id === itemId)?.title;
  }

  function replacementActionLabel(): string {
    const currentTitle = replacementTargetTitle();
    const selectedTitle = questionLookupController.selected()?.title;
    return currentTitle !== undefined && selectedTitle !== undefined
      ? `Replace ${currentTitle} with ${selectedTitle}`
      : "Replace the selected question";
  }

  function startReplacement(itemId: string): void {
    if (replacementBlocked()) return;
    setTargetItemId(itemId);
    setDirectQuestionId("");
    questionLookupController.setSelected(undefined);
    queueMicrotask(() => questionIdInput?.focus());
  }

  function focusReplacedRow(questionId: string): void {
    queueMicrotask(() => {
      const row = [...document.querySelectorAll<HTMLElement>(".assignment-editor-row")].find(
        (candidate) => candidate.textContent?.includes(questionId) === true,
      );
      const button = [...(row?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find(
        (candidate) => candidate.textContent?.trim() === "Replace",
      );
      button?.focus();
    });
  }

  async function replaceFixedQuestion(row: AssignmentQuestionRow, itemId: string): Promise<void> {
    if (busy() || replacementBlocked()) return;
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
    setBusy(true);
    try {
      const saved = await workspace.client.replaceAssignmentFixedItem(
        workspace.courseId,
        workspace.assignmentId,
        workspace.assignment().reference,
        itemId,
        row.questionId,
        workspace.assignment().revision,
      );
      workspace.replaceAssignment(saved);
      setDraft(assignmentEditorDraftFrom(saved));
      setPoolPreview(undefined);
      setHasUnsavedContent(false);
      setNeedsReload(false);
      setSuccessorRevisionRequired(false);
      setValidationMessage("");
      setTargetItemId(undefined);
      questionLookupController.setSelected(undefined);
      setDirectQuestionId("");
      setDirectMessage("");
      setMessage(
        `${row.title} now replaces ${current.title} for future runs. Issued Student work remains unchanged.`,
      );
      focusReplacedRow(row.questionId);
    } catch (error: unknown) {
      const failure = resolveAssignmentFixedItemReplacementFailure(error);
      if (failure.kind === "staleRevision") setNeedsReload(true);
      setMessage(failure.message);
    } finally {
      setBusy(false);
    }
  }

  function addRows(rows: ReadonlyArray<AssignmentQuestionRow>, success: string): void {
    const next = appendFixedEntries(draft(), rows);
    if (next === draft()) {
      setMessage("Every selected Question ID is already in this assignment.");
      return;
    }
    update(next, success);
  }

  async function chooseQuestionId(): Promise<void> {
    setBusy(true);
    setMessage("Checking the supplied Question ID.");
    try {
      const row = await questionLookupController.lookup(directQuestionId());
      questionLookupController.setSelected(row);
      setMessage(
        targetItemId() === undefined
          ? `${row.questionId} is ready to add.`
          : `${row.questionId} is ready to replace the selected question.`,
      );
      setDirectMessage("");
    } catch (_error: unknown) {
      const failure = "That Question ID could not be found. Check it and try again.";
      setDirectMessage(failure);
      setMessage(failure);
    } finally {
      setBusy(false);
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
    setBusy(true);
    setMessage("Checking the supplied Question IDs and adding valid questions.");
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
      const failure = "A Question ID could not be found. Your Question IDs are still here.";
      setDirectMessage(failure);
      setMessage(failure);
    } finally {
      setBusy(false);
    }
  }

  async function saveQuestions(): Promise<boolean> {
    const current = draft();
    if (busy()) return false;
    if (!hasUnsavedContent()) {
      setMessage("Questions and order are already saved in the current assignment revision.");
      return true;
    }
    if (successorRevisionRequired()) {
      setMessage(
        "Create a successor Draft Assignment Revision to use these structural question changes. Existing Student work remains pinned to this revision.",
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
        workspace.assignment().reference,
        assignmentContentInput(current),
        current.revision,
      );
      workspace.replaceAssignment(saved);
      setDraft(assignmentEditorDraftFrom(saved));
      setPoolPreview(undefined);
      setHasUnsavedContent(false);
      setNeedsReload(false);
      setValidationMessage("");
      setMessage("Questions and order saved. Review assignment policies when you are ready.");
      return true;
    } catch (error: unknown) {
      const failure = resolveAssignmentContentSaveFailure(error);
      if (failure.kind === "staleRevision") {
        setNeedsReload(true);
        setMessage(failure.message);
      } else if (failure.kind === "successorRevisionRequired") {
        setSuccessorRevisionRequired(true);
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
    let reloaded = false;
    try {
      const latest = await workspace.reloadAssignment();
      setDraft(assignmentEditorDraftFrom(latest));
      setPoolPreview(undefined);
      setHasUnsavedContent(false);
      setNeedsReload(false);
      setSuccessorRevisionRequired(false);
      setValidationMessage("");
      setMessage(
        "Latest assignment loaded; your local title and question changes were replaced. Review the current questions and order before saving.",
      );
      reloaded = true;
    } catch (_error: unknown) {
      setMessage(
        "The latest assignment could not be loaded. Your local question changes remain here.",
      );
    } finally {
      setBusy(false);
      if (reloaded) {
        if (targetItemId() !== undefined && questionLookupController.selected() !== undefined) {
          queueMicrotask(() => replaceSelectedQuestionButton?.focus());
        } else {
          queueMicrotask(() => saveQuestionsButton?.focus());
        }
      }
    }
  }

  async function previewPool(assignmentEntryId: string): Promise<void> {
    const failure = validateAssignmentEditorDraft(draft());
    if (failure !== null) {
      setValidationMessage(
        `${failure} Correct the questions, then preview a Question Pool selection.`,
      );
      return;
    }
    if (hasUnsavedContent()) {
      setMessage(
        "Save questions and order before previewing a Question Pool selection. Your local question changes remain here.",
      );
      queueMicrotask(() => saveQuestionsButton?.focus());
      return;
    }
    setMessage("Generating a server sample from the saved Assignment Questions.");
    const saved = workspace.assignment();
    setBusy(true);
    try {
      const preview = await workspace.client.previewQuestionPool(
        workspace.courseReference,
        saved.reference,
        previewRevision(saved.revision),
        assignmentEntryId,
      );
      setPoolPreview(preview);
      setMessage(
        `${preview.questionPoolLabel} server sample is ready. It does not create Student work.`,
      );
    } catch (_error: unknown) {
      setMessage(
        "The pool sample could not be generated. The saved Assignment Questions remain available.",
      );
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
      questionLookupController.setSelected(row);
      setTargetItemId(itemId);
      setMessage(`${row.questionId} is ready to replace the selected question.`);
      queueMicrotask(() => replaceSelectedQuestionButton?.focus());
    },
    onMessage: setMessage,
    onError: (_error, fallback) => setMessage(fallback),
  });

  onMount(() => {
    void pickerController.loadSources();
    void reuseController.load();
  });

  createEffect(() => {
    if (!needsReload() || busy()) return;
    queueMicrotask(() => reloadLatestButton?.focus());
  });

  return (
    <section class="assignment-workspace-questions" aria-labelledby="assignment-questions-heading">
      <header class="assignment-workspace-header">
        <p class="eyebrow">Assignment workspace</p>
        <h1 id="assignment-questions-heading">Questions</h1>
        <p class="page-lede">
          Set the assignment title, ordered fixed questions, and question pools.
        </p>
      </header>
      <p role="status" aria-live="polite" aria-busy={busy()}>
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
            ref={(element) => {
              reloadLatestButton = element;
            }}
            disabled={busy()}
            aria-label="Reload latest assignment and replace local changes"
            onClick={() => void reloadLatest()}
          >
            Reload latest assignment
          </button>
        </section>
      </Show>
      <Show when={successorRevisionRequired()}>
        <section class="inline-error" role="alert">
          <p>
            Student work already pins this Assignment Revision. Your local question changes remain
            here, and a successor Draft Assignment Revision is required for structural changes.
          </p>
          <button
            class="quiet-action"
            type="button"
            ref={(element) => {
              reloadLatestButton = element;
            }}
            disabled={busy()}
            onClick={() => void reloadLatest()}
          >
            Reload latest assignment
          </button>
        </section>
      </Show>
      <Show when={validationMessage()}>
        {(value) => (
          <section class="inline-error" role="alert">
            <p>{value()}</p>
          </section>
        )}
      </Show>
      <fieldset class="assignment-editor-operation-boundary" disabled={busy()} aria-busy={busy()}>
        <div class="assignment-editor-actions">
          <button
            class="primary-action"
            type="button"
            ref={(element) => {
              saveQuestionsButton = element;
            }}
            disabled={busy() || needsReload() || successorRevisionRequired()}
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
              A Question Pool selects its configured number of candidates with the server&apos;s
              current selection implementation.
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
                    (entry) => entry.kind === "fixedQuestion" && entry.id === itemId,
                  );
                  if (index >= 0) removeEntry(index);
                }}
                onPoolChange={replacePool}
                onRemovePool={removeEntry}
                onMessage={setMessage}
                onPreviewPool={(assignmentEntryId) => void previewPool(assignmentEntryId)}
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
                Replace this fixed question immediately for future runs. Issued work remains with
                its original question.
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
                disabled={
                  targetItemId() !== undefined && (needsReload() || successorRevisionRequired())
                }
                onInput={(event) => {
                  setDirectQuestionId(event.currentTarget.value);
                  setDirectMessage("");
                  questionLookupController.setSelected(undefined);
                }}
              />
            </label>
            <p id="assignment-question-id-help" class="assignment-editor-note">
              Add one or more canonical Question IDs separated by commas or lines. Use one ID to
              choose a replacement.
            </p>
            <div class="assignment-editor-actions">
              <button
                class="quiet-action"
                type="button"
                disabled={
                  targetItemId() !== undefined && (needsReload() || successorRevisionRequired())
                }
                onClick={() => void chooseQuestionId()}
              >
                Check Question ID
              </button>
              <button
                class="primary-action"
                type="button"
                ref={(element) => {
                  replaceSelectedQuestionButton = element;
                }}
                disabled={
                  (targetItemId() !== undefined &&
                    questionLookupController.selected() === undefined) ||
                  needsReload() ||
                  successorRevisionRequired()
                }
                aria-label={targetItemId() === undefined ? undefined : replacementActionLabel()}
                onClick={() => {
                  const itemId = targetItemId();
                  const selected = questionLookupController.selected();
                  if (itemId !== undefined && selected !== undefined)
                    void replaceFixedQuestion(selected, itemId);
                  else void addQuestionIds();
                }}
              >
                {targetItemId() === undefined ? "Add Question IDs" : "Replace selected question"}
              </button>
              <button
                class="quiet-action"
                type="button"
                disabled={
                  pickerController.sources().length === 0 ||
                  needsReload() ||
                  successorRevisionRequired()
                }
                onClick={(event) =>
                  pickerController.open(
                    targetItemId() === undefined
                      ? { kind: "fixedQuestion" }
                      : { kind: "replacement", itemId: targetItemId()! },
                    event.currentTarget,
                  )
                }
              >
                {targetItemId() === undefined ? "Search question library" : "Choose replacement"}
              </button>
            </div>
            <Show when={questionLookupController.selected()}>
              {(row) => (
                <p class="success-state">
                  {targetItemId() === undefined ? (
                    <>
                      Selected: {row().questionId} {row().title}
                    </>
                  ) : (
                    <>
                      Replacing {replacementTargetTitle() ?? "the current question"} with{" "}
                      {row().title}
                    </>
                  )}{" "}
                  ({questionBackendLabel(row().backend)})
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
                        "Selected saved Assignment Questions added. Save Questions and order when ready.",
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
      <AssignmentEditorQuestionPicker
        repository={workspace.repository}
        controller={pickerController}
      />
    </section>
  );
}
