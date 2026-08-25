import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseReference } from "../../generated/api/CourseReference";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import type { TenantId } from "../../generated/api/TenantId";
import { CourseManagementNav } from "../components/course_management_nav";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { ApiRequestError, PreviewPlaneConflictError } from "../api/http_client";
import { ASSIGNMENT_EDITOR_STYLES } from "./assignment_editor_styles";
import {
  assignmentCreateInput,
  assignmentEditorDraftFrom,
  appendFixedEntries,
  appendSelectionGroup,
  assignmentInput,
  capabilityLabel,
  createMasteryAssignmentDraft,
  fixedEntries,
  fixedEntry,
  moveAssignmentEntry,
  parseExactProblemDisplayReferences,
  questionBackendLabel,
  validateAssignmentEditorDraft,
  type AssignmentCatalogRow,
  type AssignmentEditorDraft,
  type AssignmentEditorSelectionGroupEntry,
} from "./assignment_editor_model";
import type { AssignmentEditorDetail, PoolDrawPreview } from "../api/contracts";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
import { AssignmentEditorPolicyPanel } from "./assignment_editor_policy_panel";
import { AssignmentEditorContentList } from "./assignment_editor_content_list";
import { saveThenPreviewPoolDraw } from "./assignment_pool_preview_action";
import { createAssignmentEditorReuseController } from "./assignment_editor_reuse_controller";
import { createAssignmentEditorCatalogController } from "./assignment_editor_catalog_controller";
import { resolveAssignmentEditorError } from "./assignment_editor_error";
import { saveAssignmentEditorTeachingSettings } from "./assignment_editor_teaching_save";
import {
  AssignmentTeachingOperationsPanel,
  assignmentCurrentStateCopy,
} from "./assignment_teaching_operations_panel";
import { assignmentRouteReference, courseRouteReference } from "../navigation/public_route";
import { AssignmentEditorSavedLinks } from "./assignment_editor_saved_links";
import { createAssignmentEditorPickerController } from "./assignment_editor_picker_controller";
import { AssignmentEditorProblemPicker } from "./assignment_editor_problem_picker";
export type AssignmentEditorMode =
  { readonly kind: "edit"; readonly assignmentId: AssignmentId } | { readonly kind: "create" };
export interface AssignmentEditorPageProps {
  readonly repository: AssignmentEditorRepository;
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
  readonly mode: AssignmentEditorMode;
  readonly tenant: TenantId;
  readonly refreshCourseAssignmentList: () => Promise<void>;
}
type EditorState = { readonly message?: string } & (
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly draft: AssignmentEditorDraft }
  | { readonly kind: "error"; readonly message: string }
);
export function AssignmentEditorPage(props: AssignmentEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<EditorState>({ kind: "loading" });
  const [targetItemId, setTargetItemId] = createSignal<string>();
  const [message, setMessage] = createSignal("");
  const [definitionValidationMessage, setDefinitionValidationMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [poolPreview, setPoolPreview] = createSignal<PoolDrawPreview>();
  const [poolPreviewNeedsReload, setPoolPreviewNeedsReload] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [directImportText, setDirectImportText] = createSignal("");
  const [directImportMessage, setDirectImportMessage] = createSignal("");
  const reuseController = createAssignmentEditorReuseController(props.repository, props.courseId);
  const catalogController = createAssignmentEditorCatalogController(props.repository);
  const [violations, setViolations] = createSignal<
    ReadonlyArray<import("../api/contracts").AssignmentCapabilityViolation>
  >([]);
  const [created, setCreated] = createSignal<AssignmentEditorDetail>();
  const [savedAssignmentReference, setSavedAssignmentReference] =
    createSignal<AssignmentReference>();
  const [teachingSettings, setTeachingSettings] =
    createSignal<AssignmentEditorDetail["teachingSettings"]>();
  const [teachingCurrentState, setTeachingCurrentState] =
    createSignal<AssignmentEditorDetail["currentState"]>();
  const [teachingMessage, setTeachingMessage] = createSignal("");
  const [teachingSaveResult, setTeachingSaveResult] = createSignal<string>();
  const [teachingFailureField, setTeachingFailureField] = createSignal<string>();
  const [latestTeachingSettings, setLatestTeachingSettings] =
    createSignal<AssignmentEditorDetail["teachingSettings"]>();
  const [teachingBusy, setTeachingBusy] = createSignal(false);
  const editorBusy = (): boolean => busy() || teachingBusy();
  let violationHeading: HTMLHeadingElement | undefined;
  let replacementQuestionInput: HTMLInputElement | undefined;
  const currentDraft = createMemo<AssignmentEditorDraft | undefined>(() => {
    const current = state();
    return current.kind === "ready" ? current.draft : undefined;
  });
  const ready = (): Extract<EditorState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };
  const pickerController = createAssignmentEditorPickerController({
    repository: props.repository,
    courseId: props.courseId,
    mode: props.mode,
    currentDraft: () => ready()?.draft,
    editorBusy,
    setBusy,
    onDraftChange: update,
    onSaved: (saved) => {
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(saved) });
      setSavedAssignmentReference(saved.reference);
      setPoolPreview(undefined);
    },
    onReplacementPrepared: (row, itemId) => {
      catalogController.setSelected(row);
      setTargetItemId(itemId);
      setMessage(`${row.questionId} is ready to replace the selected assignment question.`);
    },
    onMessage: setMessage,
    onError: handleError,
  });
  function beginReplacement(itemId: string): void {
    setTargetItemId(itemId);
    queueMicrotask(() => replacementQuestionInput?.focus());
  }
  async function load(): Promise<void> {
    setState({ kind: "loading" });
    setConflict(false);
    setPoolPreview(undefined);
    setPoolPreviewNeedsReload(false);
    try {
      if (props.mode.kind === "create") {
        const draft = createMasteryAssignmentDraft(props.courseId);
        setState({ kind: "ready", draft });
        setCreated(undefined);
        setSavedAssignmentReference(undefined);
        setMessage("Choose a title and Question IDs.");
        return;
      }
      const detail = await props.repository.load(props.mode.assignmentId);
      if (
        detail.id !== props.mode.assignmentId ||
        detail.courseId !== props.courseId ||
        detail.tenant !== props.tenant
      )
        throw new Error("The editor received an unrelated assignment.");
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(detail) });
      setSavedAssignmentReference(detail.reference);
      setTeachingSettings(detail.teachingSettings);
      setTeachingCurrentState(detail.currentState);
      setMessage("Assignment loaded.");
    } catch (error: unknown) {
      setState({
        kind: "error",
        message: error instanceof Error ? error.message : "Assignment could not load.",
      });
    }
  }
  function update(next: AssignmentEditorDraft): void {
    setState({ kind: "ready", draft: next });
    const validationError = validateAssignmentEditorDraft(next);
    setDefinitionValidationMessage(
      validationError === null ? "" : `${validationError} Correct the assignment, then save.`,
    );
    setPoolPreview(undefined);
    setPoolPreviewNeedsReload(false);
    setMessage("Change recorded. Review the complete assignment definition, then save.");
  }
  function replaceEntry(index: number, nextEntry: AssignmentEditorSelectionGroupEntry): void {
    const current = ready();
    if (current === undefined) return;
    const entries = [...current.draft.entries];
    entries[index] = nextEntry;
    update({ ...current.draft, entries });
  }
  function removeEntry(index: number): void {
    const current = ready();
    if (current === undefined) return;
    const entries = current.draft.entries
      .filter((_entry, entryIndex) => entryIndex !== index)
      .map((entry, position) => ({ ...entry, position }));
    update({ ...current.draft, entries });
    setMessage("Pool removed. Review the remaining ordered assignment definition, then save.");
  }
  function addSelectionGroup(): void {
    const current = ready();
    if (current === undefined) return;
    if (current.draft.entries.length >= MAX_ASSIGNMENT_ORDERED_ENTRIES) {
      setDefinitionValidationMessage(
        `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer, then save.`,
      );
      return;
    }
    update(appendSelectionGroup(current.draft));
    setMessage(
      "Question pool added. Add candidate Question IDs, then set its draw count and save.",
    );
  }
  async function saveTeachingSettings(
    settings: AssignmentEditorDetail["teachingSettings"],
  ): Promise<void> {
    const current = ready();
    if (current === undefined || props.mode.kind !== "edit" || editorBusy()) return;
    setTeachingBusy(true);
    setTeachingMessage("");
    setTeachingFailureField(undefined);
    try {
      await saveAssignmentEditorTeachingSettings(
        props.repository,
        props.courseId,
        props.mode.assignmentId,
        settings,
        current.draft.revision,
        props.refreshCourseAssignmentList,
        {
          onSaved: (saved) => {
            setState({ kind: "ready", draft: { ...current.draft, revision: saved.revision } });
            setTeachingSettings(saved.teachingSettings);
            setTeachingCurrentState(saved.currentState);
            setLatestTeachingSettings(undefined);
            setTeachingSaveResult(
              `${current.draft.title} is saved. ${assignmentCurrentStateCopy(
                saved.teachingSettings.lifecycle,
                saved.currentState,
                saved.teachingSettings.timeZone,
              )}`,
            );
            setTeachingMessage("Teaching operations saved.");
          },
          onValidation: (field, message) => {
            setTeachingFailureField(field);
            setTeachingMessage(message);
          },
          onConflictLatest: (latest) => {
            setLatestTeachingSettings(latest.teachingSettings);
            setTeachingCurrentState(latest.currentState);
            setState({ kind: "ready", draft: { ...current.draft, revision: latest.revision } });
          },
          onMessage: setTeachingMessage,
        },
      );
    } finally {
      setTeachingBusy(false);
    }
  }
  function addRows(rowsToAdd: ReadonlyArray<AssignmentCatalogRow>, success: string): void {
    if (props.mode.kind !== "create") return;
    const current = ready();
    if (current === undefined) return;
    const next = appendFixedEntries(current.draft, rowsToAdd);
    if (next === current.draft) {
      setMessage("Every selected Question ID is already in this assignment.");
      return;
    }
    update(next);
    setMessage(success);
  }
  async function addByQuestionIds(): Promise<void> {
    let questionIds: ReadonlyArray<string>;
    try {
      questionIds = parseExactProblemDisplayReferences(directImportText());
    } catch (error: unknown) {
      setDirectImportMessage(error instanceof Error ? error.message : "Question IDs are invalid.");
      return;
    }
    if (props.mode.kind === "edit") {
      if (questionIds.length !== 1) {
        setDirectImportMessage("Add one Question ID at a time to this existing assignment.");
        return;
      }
      await addQuestion(questionIds[0]);
      return;
    }
    const current = ready();
    if (
      current !== undefined &&
      current.draft.entries.length + questionIds.length > MAX_ASSIGNMENT_ORDERED_ENTRIES
    ) {
      setDirectImportMessage(
        `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer, then add Question IDs.`,
      );
      return;
    }
    try {
      const resolved = await Promise.all(
        questionIds.map((questionId) => props.repository.resolvePublished(questionId)),
      );
      addRows(resolved, `Added ${questionIds.join(", ")} to the unsaved selection.`);
      setDirectImportText("");
      setDirectImportMessage("");
    } catch (error: unknown) {
      setDirectImportMessage(
        error instanceof Error
          ? `${error.message} Your pasted IDs are still here.`
          : "A Question ID could not be found. Your pasted IDs are still here.",
      );
    }
  }
  async function addQuestion(questionId?: string): Promise<void> {
    const current = ready();
    if (current === undefined || editorBusy()) return;
    try {
      const row =
        questionId === undefined
          ? await catalogController.lookup(catalogController.replacementText())
          : await props.repository.resolvePublished(questionId);
      if (props.mode.kind === "create") {
        const id = row?.questionId;
        if (id === undefined) return;
        if (fixedEntries(current.draft).some((item) => item.questionId === id))
          throw new Error(`${id} is already selected.`);
        update({
          ...current.draft,
          entries: [
            ...current.draft.entries,
            fixedEntry({
              id: `new-${id}`,
              questionId: id,
              title: row.title,
              backend: row.backend,
              capabilities: [],
              position: current.draft.entries.length,
              pointsPossible: "1",
              deliveryState: "active",
              scoringMode: "normal",
            }),
          ],
        });
        catalogController.setReplacementText("");
        return;
      }
      setBusy(true);
      const saved = await props.repository.add(
        props.courseId,
        props.mode.assignmentId,
        { questionId: row?.questionId ?? "", position: current.draft.entries.length },
        current.draft.revision,
      );
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(saved) });
      setSavedAssignmentReference(saved.reference);
      setPoolPreview(undefined);
      setMessage("Question added. Add and remove are available before student work begins.");
    } catch (error: unknown) {
      handleError(error, "The question was not added. Your typed Question ID is still here.");
    } finally {
      setBusy(false);
    }
  }
  async function replaceQuestion(): Promise<void> {
    const current = ready();
    const itemId = targetItemId();
    const row = catalogController.selected();
    if (
      current === undefined ||
      itemId === undefined ||
      row === undefined ||
      props.mode.kind !== "edit" ||
      editorBusy()
    )
      return;
    setBusy(true);
    try {
      const saved = await props.repository.replace(
        props.courseId,
        props.mode.assignmentId,
        itemId,
        { questionId: row.questionId },
        current.draft.revision,
      );
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(saved) });
      setSavedAssignmentReference(saved.reference);
      setPoolPreview(undefined);
      catalogController.setSelected(undefined);
      setTargetItemId(undefined);
      setMessage(
        "Replacement saved. Future runs use the replacement; issued work stays with its original question.",
      );
    } catch (error: unknown) {
      handleError(error, "The replacement was not saved. Your selected Question ID is still here.");
    } finally {
      setBusy(false);
    }
  }
  async function removeQuestion(itemId: string): Promise<void> {
    const current = ready();
    if (current === undefined || props.mode.kind !== "edit" || editorBusy()) return;
    setBusy(true);
    try {
      const saved = await props.repository.remove(
        props.courseId,
        props.mode.assignmentId,
        itemId,
        current.draft.revision,
      );
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(saved) });
      setSavedAssignmentReference(saved.reference);
      setPoolPreview(undefined);
      setMessage("Question removed before student work began.");
    } catch (error: unknown) {
      handleError(error, "The question was not removed. Reload to review the current assignment.");
    } finally {
      setBusy(false);
    }
  }
  async function save(): Promise<void> {
    const current = ready();
    if (current === undefined || editorBusy()) return;
    const validationError = validateAssignmentEditorDraft(current.draft);
    if (validationError !== null) {
      setDefinitionValidationMessage(`${validationError} Correct the assignment, then save.`);
      return;
    }
    setBusy(true);
    try {
      const saved =
        props.mode.kind === "create"
          ? await props.repository.create(props.courseId, assignmentCreateInput(current.draft))
          : await props.repository.save(
              props.courseId,
              props.mode.assignmentId,
              assignmentInput(current.draft),
              current.draft.revision,
            );
      await props.refreshCourseAssignmentList();
      setState({ kind: "ready", draft: assignmentEditorDraftFrom(saved) });
      setSavedAssignmentReference(saved.reference);
      setPoolPreview(undefined);
      if (props.mode.kind === "create") {
        setCreated(saved);
        setMessage("Assignment created. Open it to review the student-facing course link.");
      } else setMessage("Assignment title, order, and settings saved.");
    } catch (error: unknown) {
      handleError(error, "The assignment was not saved. Your edits are still here.");
    } finally {
      setBusy(false);
    }
  }
  async function previewPoolDraw(groupPosition: number): Promise<void> {
    const current = ready();
    const reference = savedAssignmentReference();
    if (
      current === undefined ||
      props.mode.kind !== "edit" ||
      reference === undefined ||
      editorBusy()
    ) {
      setMessage("Save this new assignment first, then preview a server-generated pool draw.");
      return;
    }
    const validationError = validateAssignmentEditorDraft(current.draft);
    if (validationError !== null) {
      setDefinitionValidationMessage(
        `${validationError} Correct the assignment, then preview its draw.`,
      );
      return;
    }
    setBusy(true);
    setPoolPreviewNeedsReload(false);
    try {
      const result = await saveThenPreviewPoolDraw(
        props.repository,
        props.courseId,
        props.mode.assignmentId,
        props.courseReference,
        current.draft,
        groupPosition,
      );
      await props.refreshCourseAssignmentList();
      setState({ kind: "ready", draft: result.draft });
      setPoolPreview(result.preview);
      setMessage(
        `${result.preview.groupLabel} preview is ready. Review the server-sampled draw or preview another draw.`,
      );
    } catch (error: unknown) {
      if (error instanceof PreviewPlaneConflictError) {
        setPoolPreviewNeedsReload(true);
        setMessage(
          "This pool changed before the preview could be generated. Your local edits remain here; reload the assignment before previewing again.",
        );
      } else if (error instanceof ApiRequestError && error.status === 404) {
        setMessage(
          "This saved pool is unavailable for preview. Review the assignment and try again.",
        );
      } else {
        setMessage(
          error instanceof Error
            ? `${error.message} Your local assignment edits remain here.`
            : "The pool preview was not generated. Your local assignment edits remain here.",
        );
      }
    } finally {
      setBusy(false);
    }
  }
  function handleError(error: unknown, fallback: string): void {
    const resolution = resolveAssignmentEditorError(error, fallback);
    if (resolution.kind === "conflict") {
      setConflict(true);
      setMessage(resolution.message);
      return;
    }
    if (resolution.kind === "validation") {
      setViolations(resolution.violations);
      setMessage(resolution.message);
      queueMicrotask(() => violationHeading?.focus());
      return;
    }
    setMessage(resolution.message);
  }
  async function initialize(): Promise<void> {
    await load();
    if (props.mode.kind === "create") await reuseController.load();
    await pickerController.loadSources();
  }
  onMount(() => void initialize());
  return (
    <section
      class="page"
      data-route-surface="assignmentEditor"
      aria-busy={state().kind === "loading" || editorBusy()}
    >
      <style>{ASSIGNMENT_EDITOR_STYLES}</style>
      <p class="eyebrow">Instructor course design</p>
      <h1>{props.mode.kind === "create" ? "Create assignment" : "Assignment editor"}</h1>
      <p class="page-lede">
        Use assigned Question IDs to build a practice set and deliberately replace a question when
        your course needs a different one.
      </p>
      <CourseManagementNav
        courseReference={props.courseReference}
        active={props.mode.kind === "create" ? "newAssignment" : "assignments"}
      />
      <Show when={teachingSaveResult()}>
        {(result) => (
          <section class="success-state assignment-editor-save-result" role="status">
            <h2>Teaching operations saved</h2>
            {/* ASVS 1.2.1: the server-derived saved state remains a text node. */}
            <p>{result()}</p>
          </section>
        )}
      </Show>
      <AssignmentEditorSavedLinks
        assignmentReference={savedAssignmentReference}
        courseReference={props.courseReference}
      />
      <p role="status" aria-live="polite">
        {message()}
      </p>
      <Show when={poolPreviewNeedsReload()}>
        <section class="inline-error" role="alert">
          <p>Reload the assignment to use its latest saved pool definition.</p>
          <button class="quiet-action" type="button" onClick={() => void load()}>
            Reload assignment
          </button>
        </section>
      </Show>
      <Show when={state().kind === "loading"}>
        <p class="loading-state">Loading assignment editor...</p>
      </Show>
      <Show when={state().kind === "error" ? state().message : undefined}>
        {(failure) => (
          <section class="route-error" role="alert">
            <h2>This assignment could not be opened</h2>
            <p>{failure()}</p>
            <button class="primary-action" onClick={() => void load()}>
              Try again
            </button>
          </section>
        )}
      </Show>
      <Show when={currentDraft()}>
        {(draft) => (
          <>
            <Show when={created()}>
              {(assignment) => (
                <section class="success-state" role="status">
                  <h2>Assignment created</h2>
                  <p>{assignment().title} now appears in this course.</p>
                  <A
                    class="primary-link"
                    href={`/instructor/courses/${courseRouteReference(props.courseReference)}/assignments/${assignmentRouteReference(assignment().reference)}/edit`}
                  >
                    Open {assignment().title}
                  </A>
                </section>
              )}
            </Show>
            <Show when={violations().length > 0}>
              <section
                class="assignment-editor-violations"
                role="alert"
                aria-labelledby="assignment-violations-heading"
              >
                <h2
                  id="assignment-violations-heading"
                  tabindex="-1"
                  ref={(element: HTMLHeadingElement) => {
                    violationHeading = element;
                  }}
                >
                  Fix these assignment settings
                </h2>
                <p>The server found unsupported capabilities in this saved selection.</p>
                <ul>
                  <For each={violations()}>
                    {(violation) => (
                      <li>
                        {violation.title} cannot provide {capabilityLabel(violation.capability)}.
                      </li>
                    )}
                  </For>
                </ul>
              </section>
            </Show>
            <Show when={conflict()}>
              <section class="inline-error" role="alert">
                <p>
                  A newer assignment revision is available. Your typed Question ID and selected
                  replacement remain available. If learner work has begun, create a new assignment
                  or use the supported future-run replacement workflow.
                </p>
                <button class="quiet-action" onClick={() => void load()}>
                  Reload assignment
                </button>
              </section>
            </Show>
            <Show when={definitionValidationMessage()}>
              {(validationError) => (
                <section class="inline-error" role="alert">
                  <p>{validationError()}</p>
                </section>
              )}
            </Show>
            <Show when={pickerController.pendingSelection()}>
              {(pending) => (
                <section class="inline-error assignment-editor-picker-retry" role="alert">
                  <p>
                    {pending().questionIds.length} selected Question ID
                    {pending().questionIds.length === 1 ? " remains" : "s remain"} ready to add.
                    Review the current assignment, then retry the server-backed update.
                  </p>
                  <button
                    class="quiet-action"
                    type="button"
                    onClick={() => void pickerController.retryPendingSelection()}
                  >
                    Retry selected questions
                  </button>
                </section>
              )}
            </Show>
            <fieldset class="assignment-editor-operation-boundary" disabled={editorBusy()}>
              <div class="assignment-editor-actions">
                <button
                  class="primary-action"
                  disabled={busy() || draft().title.trim() === "" || draft().entries.length === 0}
                  onClick={() => void save()}
                >
                  {props.mode.kind === "create"
                    ? "Create assignment"
                    : "Save title, order, and settings"}
                </button>
              </div>
              <div class="assignment-editor-grid">
                <section class="assignment-editor-panel">
                  <h2>Assignment content</h2>
                  <label class="assignment-editor-field">
                    Assignment title
                    <input
                      value={draft().title}
                      onInput={(event) => update({ ...draft(), title: event.currentTarget.value })}
                    />
                  </label>
                  <button class="quiet-action" type="button" onClick={addSelectionGroup}>
                    Add question pool
                  </button>
                  <p class="assignment-editor-note">
                    A pool draws a configured number of candidates with the fixed Draw algorithm v1.
                    Its position is shared with fixed questions.
                  </p>
                  <Show when={props.mode.kind === "create"}>
                    <details class="assignment-editor-reuse">
                      <summary>Reuse questions from an existing assignment</summary>
                      <div>
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
                              No other assignments are available in this course yet.
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
                                {(assignment, index) => (
                                  <option value={index()}>{assignment.title}</option>
                                )}
                              </For>
                            </select>
                          </label>
                          <p class="assignment-editor-note">
                            Copy all questions or choose a subset. Your title and policies stay
                            unchanged.
                          </p>
                          <div class="assignment-editor-reuse-checklist">
                            <For each={reuseController.selectedSource()?.questions ?? []}>
                              {(question, index) => (
                                <label>
                                  <input
                                    type="checkbox"
                                    checked={reuseController.questionIndexes().has(index())}
                                    onChange={(event) =>
                                      reuseController.toggleQuestion(
                                        index(),
                                        event.currentTarget.checked,
                                      )
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
                                    (_item, index) => reuseController.questionIndexes().has(index),
                                  ),
                                  "Selected questions copied to the unsaved assignment.",
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
                                  "Assignment questions copied to the unsaved assignment.",
                                )
                              }
                            >
                              Add entire assignment
                            </button>
                          </div>
                        </Show>
                      </div>
                    </details>
                  </Show>
                  <Show
                    when={draft().entries.length > 0}
                    fallback={
                      <p class="empty-state">
                        No questions yet. Search the library or paste one Question ID.
                      </p>
                    }
                  >
                    <AssignmentEditorContentList
                      entries={draft().entries}
                      createMode={props.mode.kind === "create"}
                      busy={busy()}
                      preview={poolPreview()}
                      resolveCandidates={async (questionIds) =>
                        await Promise.all(
                          questionIds.map(
                            async (questionId) =>
                              await props.repository.resolvePublished(questionId),
                          ),
                        )
                      }
                      onMove={(entryIndex, direction) =>
                        update(moveAssignmentEntry(draft(), entryIndex, direction))
                      }
                      onReplace={beginReplacement}
                      onRemoveFixed={(itemId) => void removeQuestion(itemId)}
                      onPoolChange={replaceEntry}
                      onRemovePool={removeEntry}
                      onMessage={setMessage}
                      onPreviewPool={(groupPosition) => void previewPoolDraw(groupPosition)}
                      onChoosePoolCandidates={(entryIndex, trigger) =>
                        pickerController.open({ kind: "pool", entryIndex }, trigger)
                      }
                    />
                  </Show>
                  <details class="assignment-editor-direct-import">
                    <summary>
                      {props.mode.kind === "create"
                        ? "Add several Question IDs"
                        : "Add by Question ID"}
                    </summary>
                    <div>
                      <p id="add-by-id-help" class="assignment-editor-note">
                        {props.mode.kind === "create"
                          ? "Paste canonical Question IDs from the library, separated by commas or lines. These become the initial selection before you create the assignment."
                          : "Add one canonical Question ID at a time. The server assigns its item identity before ordinary assignment settings can be saved."}
                      </p>
                      <label class="assignment-editor-field">
                        {props.mode.kind === "create"
                          ? "Question IDs"
                          : "Direct import Question ID"}
                        <textarea
                          rows="2"
                          value={directImportText()}
                          placeholder="7K3-M9QP"
                          aria-describedby="add-by-id-help"
                          aria-invalid={directImportMessage() !== ""}
                          onInput={(event) => {
                            setDirectImportText(event.currentTarget.value);
                            setDirectImportMessage("");
                          }}
                        />
                      </label>
                      <button
                        class="primary-action"
                        type="button"
                        onClick={() => void addByQuestionIds()}
                      >
                        {props.mode.kind === "create" ? "Add questions by ID" : "Add Question ID"}
                      </button>
                      <Show when={directImportMessage()}>
                        {(value) => (
                          <p class="inline-error" role="alert">
                            {value()}
                          </p>
                        )}
                      </Show>
                    </div>
                  </details>
                </section>
                <section class="assignment-editor-panel">
                  <AssignmentEditorPolicyPanel
                    policies={() => draft().policies}
                    disclosurePolicy={() => draft().disclosurePolicy}
                    onPoliciesChange={(policies) => update({ ...draft(), policies })}
                    onDisclosurePolicyChange={(disclosurePolicy) =>
                      update({ ...draft(), disclosurePolicy })
                    }
                  />
                  <Show when={props.mode.kind === "edit"}>
                    <AssignmentTeachingOperationsPanel
                      settings={teachingSettings}
                      currentState={teachingCurrentState}
                      busy={teachingBusy}
                      message={teachingMessage}
                      failureField={teachingFailureField}
                      latestSettings={latestTeachingSettings}
                      onAdoptLatest={() => {
                        const latest = latestTeachingSettings();
                        if (latest !== undefined) {
                          setTeachingSettings(latest);
                          setLatestTeachingSettings(undefined);
                          setTeachingFailureField(undefined);
                          setTeachingMessage("Latest teaching operations adopted.");
                        }
                      }}
                      onSave={saveTeachingSettings}
                    />
                  </Show>
                  <h2>
                    {targetItemId() === undefined ? "Add questions" : "Replace assigned question"}
                  </h2>
                  <Show when={props.mode.kind === "edit" && targetItemId() === undefined}>
                    <p class="assignment-editor-note">
                      Choose one or more published questions. The server assigns each item identity
                      and confirms every addition before student work begins.
                    </p>
                  </Show>
                  <Show when={targetItemId() !== undefined}>
                    <p class="assignment-editor-note">
                      Future runs use the replacement. Already issued work stays with its original
                      question.
                    </p>
                    <p>
                      Current:{" "}
                      <CopyableQuestionId
                        displayId={
                          fixedEntries(draft()).find((item) => item.id === targetItemId())
                            ?.questionId ?? ""
                        }
                      />{" "}
                      {fixedEntries(draft()).find((item) => item.id === targetItemId())?.title} (
                      {questionBackendLabel(
                        fixedEntries(draft()).find((item) => item.id === targetItemId())?.backend ??
                          "native",
                      )}
                      )
                    </p>
                  </Show>
                  <label class="assignment-editor-field">
                    Question ID
                    <input
                      ref={(element) => {
                        replacementQuestionInput = element;
                      }}
                      aria-label={
                        targetItemId() === undefined ? "Add Question ID" : "Replacement Question ID"
                      }
                      value={catalogController.replacementText()}
                      onInput={(event) =>
                        catalogController.setReplacementText(event.currentTarget.value)
                      }
                      placeholder="7K3-M9QP"
                    />
                  </label>
                  <div class="assignment-editor-actions">
                    <button
                      class="quiet-action"
                      disabled={busy()}
                      onClick={() => void catalogController.chooseReplacement(setMessage)}
                    >
                      Check Question ID
                    </button>
                    <button
                      class="primary-action"
                      disabled={
                        busy() ||
                        (targetItemId() !== undefined && catalogController.selected() === undefined)
                      }
                      onClick={() =>
                        targetItemId() === undefined ? void addQuestion() : void replaceQuestion()
                      }
                    >
                      {targetItemId() === undefined
                        ? "Add question"
                        : "Replace with selected question"}
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={pickerController.sources().length === 0 || busy()}
                      onClick={(event) =>
                        pickerController.open(
                          targetItemId() === undefined
                            ? { kind: "fixed" }
                            : { kind: "replacement", itemId: targetItemId()! },
                          event.currentTarget,
                        )
                      }
                    >
                      {targetItemId() === undefined ? "Choose questions" : "Choose replacement"}
                    </button>
                  </div>
                  <Show when={catalogController.selected()}>
                    {(row) => (
                      <p class="success-state">
                        Selected: <CopyableQuestionId displayId={row().questionId} /> {row().title}{" "}
                        ({questionBackendLabel(row().backend)})
                      </p>
                    )}
                  </Show>
                </section>
              </div>
            </fieldset>
            <AssignmentEditorProblemPicker
              repository={props.repository}
              controller={pickerController}
            />
          </>
        )}
      </Show>
    </section>
  );
}
