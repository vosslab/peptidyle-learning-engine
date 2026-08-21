// assignment_editor_page.tsx - focused instructor editing with QID-only replacement.

import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { TenantId } from "../../generated/api/TenantId";
import { CourseManagementNav } from "../components/course_management_nav";
import { CopyableQuestionId } from "../components/copyable_question_id";
import {
  AssignmentConflictError,
  AssignmentTeachingSettingsValidationError,
  AssignmentValidationError,
  ApiRequestError,
} from "../api/http_client";
import { ASSIGNMENT_EDITOR_STYLES } from "./assignment_editor_styles";
import {
  assignmentCreateInput,
  assignmentInput,
  capabilityLabel,
  createMasteryAssignmentDraft,
  moveAssignmentItem,
  parseExactProblemDisplayReferences,
  questionBackendLabel,
  type AssignmentCatalogRow,
  type AssignmentEditorDraft,
} from "./assignment_editor_model";
import type { AssignmentEditorDetail } from "../api/contracts";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
import type { ReusableAssignment } from "./assignment_editor_repository";
import { AssignmentEditorPolicyPanel } from "./assignment_editor_policy_panel";
import { AssignmentTeachingOperationsPanel } from "./assignment_teaching_operations_panel";
import { assignmentRouteReference, courseRouteReference } from "../navigation/public_route";

export type AssignmentEditorMode =
  { readonly kind: "edit"; readonly assignmentId: AssignmentId } | { readonly kind: "create" };
export interface AssignmentEditorPageProps {
  readonly repository: AssignmentEditorRepository;
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
  readonly mode: AssignmentEditorMode;
  readonly tenant: TenantId;
}
type EditorState = { readonly message?: string } & (
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly draft: AssignmentEditorDraft }
  | { readonly kind: "error"; readonly message: string }
);

function draftFrom(detail: AssignmentEditorDetail): AssignmentEditorDraft {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    items: detail.items,
    policies: detail.policies,
    disclosurePolicy: detail.disclosurePolicy,
    revision: detail.revision,
  };
}

export function AssignmentEditorPage(props: AssignmentEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<EditorState>({ kind: "loading" });
  const [search, setSearch] = createSignal("");
  const [rows, setRows] = createSignal<ReadonlyArray<AssignmentCatalogRow>>([]);
  const [replacementText, setReplacementText] = createSignal("");
  const [selected, setSelected] = createSignal<AssignmentCatalogRow>();
  const [targetItemId, setTargetItemId] = createSignal<string>();
  const [message, setMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [directImportText, setDirectImportText] = createSignal("");
  const [directImportMessage, setDirectImportMessage] = createSignal("");
  const [reuse, setReuse] = createSignal<ReadonlyArray<ReusableAssignment>>([]);
  const [reuseMessage, setReuseMessage] = createSignal("");
  const [reuseSourceIndex, setReuseSourceIndex] = createSignal<number>();
  const [reuseQuestionIndexes, setReuseQuestionIndexes] = createSignal<ReadonlySet<number>>(
    new Set(),
  );
  const [violations, setViolations] = createSignal<
    ReadonlyArray<import("../api/contracts").AssignmentCapabilityViolation>
  >([]);
  const [created, setCreated] = createSignal<AssignmentEditorDetail>();
  const [teachingSettings, setTeachingSettings] =
    createSignal<AssignmentEditorDetail["teachingSettings"]>();
  const [teachingCurrentState, setTeachingCurrentState] =
    createSignal<AssignmentEditorDetail["currentState"]>();
  const [teachingMessage, setTeachingMessage] = createSignal("");
  const [teachingFailureField, setTeachingFailureField] = createSignal<string>();
  const [latestTeachingSettings, setLatestTeachingSettings] =
    createSignal<AssignmentEditorDetail["teachingSettings"]>();
  const [teachingBusy, setTeachingBusy] = createSignal(false);
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

  function beginReplacement(itemId: string): void {
    setTargetItemId(itemId);
    queueMicrotask(() => replacementQuestionInput?.focus());
  }

  async function load(): Promise<void> {
    setState({ kind: "loading" });
    setConflict(false);
    try {
      if (props.mode.kind === "create") {
        const draft = createMasteryAssignmentDraft(props.courseId);
        setState({ kind: "ready", draft });
        setCreated(undefined);
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
      setState({ kind: "ready", draft: draftFrom(detail) });
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
    setMessage("Unsaved assignment changes.");
  }
  async function saveTeachingSettings(
    settings: AssignmentEditorDetail["teachingSettings"],
  ): Promise<void> {
    const current = ready();
    if (current === undefined || props.mode.kind !== "edit" || teachingBusy()) return;
    setTeachingBusy(true);
    setTeachingMessage("");
    setTeachingFailureField(undefined);
    try {
      const saved = await props.repository.saveTeachingSettings(
        props.courseId,
        props.mode.assignmentId,
        settings,
        current.draft.revision,
      );
      // Teaching operations are a separate transaction; keep ordinary content
      // edits in the browser and only advance their shared revision.
      setState({ kind: "ready", draft: { ...current.draft, revision: saved.revision } });
      setTeachingSettings(saved.teachingSettings);
      setTeachingCurrentState(saved.currentState);
      setLatestTeachingSettings(undefined);
      setTeachingMessage("Teaching operations saved.");
    } catch (error: unknown) {
      if (error instanceof AssignmentTeachingSettingsValidationError) {
        setTeachingFailureField(error.failure.field);
        setTeachingMessage(error.failure.message);
      } else {
        if (error instanceof AssignmentConflictError) {
          try {
            const latest = await props.repository.load(props.mode.assignmentId);
            setLatestTeachingSettings(latest.teachingSettings);
            setTeachingCurrentState(latest.currentState);
            setState({ kind: "ready", draft: { ...current.draft, revision: latest.revision } });
          } catch {
            // Keep the local draft even if the latest-version fetch is unavailable.
          }
        }
        setTeachingMessage(
          error instanceof AssignmentConflictError
            ? "A newer teaching-settings revision was fetched. Your edits are still here; retry to save them, or adopt the latest teaching operations."
            : "Teaching operations were not saved. Correct the schedule or try again.",
        );
      }
    } finally {
      setTeachingBusy(false);
    }
  }
  async function loadReusableAssignments(): Promise<void> {
    if (props.mode.kind !== "create") return;
    try {
      const values = await props.repository.listReusableAssignments(props.courseId, undefined);
      setReuse(values);
      setReuseSourceIndex(values.length > 0 ? 0 : undefined);
      setReuseQuestionIndexes(new Set(values[0]?.questions.map((_item, index) => index) ?? []));
    } catch {
      setReuseMessage("Existing assignments could not be loaded. Your current work is unchanged.");
    }
  }
  function selectedReuseSource(): ReusableAssignment | undefined {
    const index = reuseSourceIndex();
    return index === undefined ? undefined : reuse()[index];
  }
  function chooseReuseSource(index: number): void {
    const source = reuse()[index];
    if (source === undefined) return;
    setReuseSourceIndex(index);
    setReuseQuestionIndexes(new Set(source.questions.map((_item, itemIndex) => itemIndex)));
  }
  function toggleReuseQuestion(index: number, checked: boolean): void {
    setReuseQuestionIndexes((previous) => {
      const next = new Set(previous);
      if (checked) next.add(index);
      else next.delete(index);
      return next;
    });
  }
  function addRows(rowsToAdd: ReadonlyArray<AssignmentCatalogRow>, success: string): void {
    if (props.mode.kind !== "create") return;
    const current = ready();
    if (current === undefined) return;
    const known = new Set(current.draft.items.map((item) => item.questionId));
    const fresh = rowsToAdd.filter((row) => !known.has(row.questionId));
    if (fresh.length === 0) {
      setMessage("Every selected Question ID is already in this assignment.");
      return;
    }
    update({
      ...current.draft,
      items: [
        ...current.draft.items,
        ...fresh.map((row, index) => ({
          id: `new-${row.questionId}`,
          questionId: row.questionId,
          title: row.title,
          backend: row.backend,
          capabilities: [],
          position: current.draft.items.length + index,
          pointsPossible: "1",
          deliveryState: "active" as const,
          scoringMode: "normal" as const,
        })),
      ],
    });
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
  async function lookup(value: string): Promise<AssignmentCatalogRow> {
    const ids = parseExactProblemDisplayReferences(value);
    if (ids.length !== 1) throw new Error("Choose one Question ID for this action.");
    const id = ids[0];
    if (id === undefined) throw new Error("Choose one Question ID for this action.");
    return await props.repository.resolvePublished(id);
  }
  async function searchCatalog(): Promise<void> {
    try {
      setRows(await props.repository.searchPublished(search()));
      setMessage("Library results updated.");
    } catch {
      setMessage("The library could not be searched. Keep your Question ID and try again.");
    }
  }
  async function chooseReplacement(): Promise<void> {
    try {
      const row = await lookup(replacementText());
      setSelected(row);
      setMessage(
        `${row?.questionId ?? "Question"} is ready to replace the selected assignment question.`,
      );
    } catch (error: unknown) {
      setMessage(error instanceof Error ? error.message : "That Question ID could not be found.");
    }
  }
  async function addQuestion(questionId?: string): Promise<void> {
    const current = ready();
    if (current === undefined || busy()) return;
    try {
      const row =
        questionId === undefined
          ? await lookup(replacementText())
          : await props.repository.resolvePublished(questionId);
      if (props.mode.kind === "create") {
        const id = row?.questionId;
        if (id === undefined) return;
        if (current.draft.items.some((item) => item.questionId === id))
          throw new Error(`${id} is already selected.`);
        update({
          ...current.draft,
          items: [
            ...current.draft.items,
            {
              id: `new-${id}`,
              questionId: id,
              title: row.title,
              backend: row.backend,
              capabilities: [],
              position: current.draft.items.length,
              pointsPossible: "1",
              deliveryState: "active",
              scoringMode: "normal",
            },
          ],
        });
        setReplacementText("");
        return;
      }
      setBusy(true);
      const saved = await props.repository.add(
        props.courseId,
        props.mode.assignmentId,
        { questionId: row?.questionId ?? "", position: current.draft.items.length },
        current.draft.revision,
      );
      setState({ kind: "ready", draft: draftFrom(saved) });
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
    const row = selected();
    if (
      current === undefined ||
      itemId === undefined ||
      row === undefined ||
      props.mode.kind !== "edit" ||
      busy()
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
      setState({ kind: "ready", draft: draftFrom(saved) });
      setSelected(undefined);
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
    if (current === undefined || props.mode.kind !== "edit" || busy()) return;
    setBusy(true);
    try {
      const saved = await props.repository.remove(
        props.courseId,
        props.mode.assignmentId,
        itemId,
        current.draft.revision,
      );
      setState({ kind: "ready", draft: draftFrom(saved) });
      setMessage("Question removed before student work began.");
    } catch (error: unknown) {
      handleError(error, "The question was not removed. Reload to review the current assignment.");
    } finally {
      setBusy(false);
    }
  }
  async function save(): Promise<void> {
    const current = ready();
    if (current === undefined || busy()) return;
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
      setState({ kind: "ready", draft: draftFrom(saved) });
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
  function handleError(error: unknown, fallback: string): void {
    if (
      error instanceof AssignmentConflictError ||
      (error instanceof ApiRequestError && error.status === 409)
    ) {
      setConflict(true);
      setMessage(
        "A newer assignment revision exists. Your typed Question ID and selected replacement are still here.",
      );
      return;
    }
    if (error instanceof AssignmentValidationError) {
      setViolations(error.violations);
      setMessage("The assignment settings need adjustment before they can be saved.");
      queueMicrotask(() => violationHeading?.focus());
      return;
    }
    setMessage(error instanceof Error ? `${error.message} ${fallback}` : fallback);
  }
  async function initialize(): Promise<void> {
    await load();
    await loadReusableAssignments();
  }
  onMount(() => void initialize());
  return (
    <section
      class="page"
      data-route-surface="assignmentEditor"
      aria-busy={state().kind === "loading"}
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
      <p role="status" aria-live="polite">
        {message()}
      </p>
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
                  replacement remain available.
                </p>
                <button class="quiet-action" onClick={() => void load()}>
                  Reload assignment
                </button>
              </section>
            </Show>
            <div class="assignment-editor-actions">
              <button
                class="primary-action"
                disabled={busy() || draft().title.trim() === "" || draft().items.length === 0}
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
                <Show when={props.mode.kind === "create"}>
                  <details class="assignment-editor-reuse">
                    <summary>Reuse questions from an existing assignment</summary>
                    <div>
                      <Show when={reuseMessage()}>
                        {(value) => (
                          <p class="inline-error" role="alert">
                            {value()}
                          </p>
                        )}
                      </Show>
                      <Show
                        when={reuse().length > 0}
                        fallback={
                          <p class="assignment-editor-note">
                            No other assignments are available in this course yet.
                          </p>
                        }
                      >
                        <label class="assignment-editor-field">
                          Source assignment
                          <select
                            value={reuseSourceIndex() ?? ""}
                            onChange={(event) =>
                              chooseReuseSource(Number(event.currentTarget.value))
                            }
                          >
                            <For each={reuse()}>
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
                          <For each={selectedReuseSource()?.questions ?? []}>
                            {(question, index) => (
                              <label>
                                <input
                                  type="checkbox"
                                  checked={reuseQuestionIndexes().has(index())}
                                  onChange={(event) =>
                                    toggleReuseQuestion(index(), event.currentTarget.checked)
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
                                (selectedReuseSource()?.questions ?? []).filter((_item, index) =>
                                  reuseQuestionIndexes().has(index),
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
                                selectedReuseSource()?.questions ?? [],
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
                  when={draft().items.length > 0}
                  fallback={
                    <p class="empty-state">
                      No questions yet. Search the library or paste one Question ID.
                    </p>
                  }
                >
                  <ol class="assignment-editor-list">
                    <For each={draft().items}>
                      {(item) => (
                        <li class="assignment-editor-row">
                          <h3>{item.title}</h3>
                          <p>
                            <CopyableQuestionId displayId={item.questionId} />{" "}
                            {questionBackendLabel(item.backend)}
                          </p>
                          <div class="assignment-editor-row-actions">
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={item.position === 0}
                              aria-label={`Move ${item.title} earlier`}
                              onClick={() => update(moveAssignmentItem(draft(), item.id, -1))}
                            >
                              &uarr;
                            </button>
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={item.position === draft().items.length - 1}
                              aria-label={`Move ${item.title} later`}
                              onClick={() => update(moveAssignmentItem(draft(), item.id, 1))}
                            >
                              &darr;
                            </button>
                            <button
                              class="quiet-action"
                              disabled={props.mode.kind === "create"}
                              onClick={() => beginReplacement(item.id)}
                            >
                              Replace
                            </button>
                            <button
                              class="quiet-action"
                              disabled={props.mode.kind === "create" || busy()}
                              onClick={() => void removeQuestion(item.id)}
                            >
                              Remove
                            </button>
                          </div>
                        </li>
                      )}
                    </For>
                  </ol>
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
                      {props.mode.kind === "create" ? "Question IDs" : "Direct import Question ID"}
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
                  {targetItemId() === undefined ? "Add from library" : "Replace assigned question"}
                </h2>
                <Show when={props.mode.kind === "edit" && targetItemId() === undefined}>
                  <p class="assignment-editor-note">
                    Add one published Question ID at a time. The server assigns its item identity.
                    Add and remove are available before student work begins.
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
                        draft().items.find((item) => item.id === targetItemId())?.questionId ?? ""
                      }
                    />{" "}
                    {draft().items.find((item) => item.id === targetItemId())?.title} (
                    {questionBackendLabel(
                      draft().items.find((item) => item.id === targetItemId())?.backend ?? "native",
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
                    value={replacementText()}
                    onInput={(event) => setReplacementText(event.currentTarget.value)}
                    placeholder="7K3-M9QP"
                  />
                </label>
                <div class="assignment-editor-actions">
                  <button
                    class="quiet-action"
                    disabled={busy()}
                    onClick={() => void chooseReplacement()}
                  >
                    Check Question ID
                  </button>
                  <button
                    class="primary-action"
                    disabled={busy() || (targetItemId() !== undefined && selected() === undefined)}
                    onClick={() =>
                      targetItemId() === undefined ? void addQuestion() : void replaceQuestion()
                    }
                  >
                    {targetItemId() === undefined
                      ? "Add question"
                      : "Replace with selected question"}
                  </button>
                </div>
                <Show when={selected()}>
                  {(row) => (
                    <p class="success-state">
                      Selected: <CopyableQuestionId displayId={row().questionId} /> {row().title} (
                      {questionBackendLabel(row().backend)})
                    </p>
                  )}
                </Show>
                <label class="assignment-editor-field">
                  Search published questions
                  <input
                    value={search()}
                    onInput={(event) => setSearch(event.currentTarget.value)}
                  />
                </label>
                <button class="quiet-action" onClick={() => void searchCatalog()}>
                  Search library
                </button>
                <div class="assignment-editor-catalog-results">
                  <For each={rows()}>
                    {(row) => (
                      <article class="assignment-editor-row">
                        <h3>{row.title}</h3>
                        <p>
                          <CopyableQuestionId displayId={row.questionId} />{" "}
                          {questionBackendLabel(row.backend)}
                        </p>
                        <button
                          class="quiet-action"
                          onClick={() => {
                            setReplacementText(row.questionId);
                            setSelected(row);
                          }}
                        >
                          Use this Question ID
                        </button>
                      </article>
                    )}
                  </For>
                </div>
              </section>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}
