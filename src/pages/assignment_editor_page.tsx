// assignment_editor_page.tsx - role-gated instructor controls for one revisioned assignment.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CoursePublicId } from "../../generated/api/CoursePublicId";
import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { TenantId } from "../../generated/api/TenantId";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../api/contracts";
import { CopyableProblemId } from "../components/copyable_problem_id";
import { CourseManagementNav } from "../components/course_management_nav";
import {
  ApiRequestError,
  AssignmentConflictError,
  AssignmentValidationError,
} from "../api/http_client";
import {
  addCatalogReference,
  assignmentProblemLabel,
  assignmentInput,
  capabilityLabel,
  createMasteryAssignmentDraft,
  minutesToRunTimeLimit,
  moveCatalogReference,
  questionBackendLabel,
  parseExactProblemDisplayReferences,
  removeCatalogReference,
  sameReference,
  runTimeLimitMinutes,
  violationMatchesReference,
  type AssignmentCatalogRow,
  type AssignmentEditorDraft,
  type TimeLimitValidation,
} from "./assignment_editor_model";
import type {
  AssignmentEditorRepository,
  ReusableAssignment,
} from "./assignment_editor_repository";
import { assignmentRouteReference, courseRouteReference } from "../navigation/public_route";
import { AssignmentEditorPolicyPanel } from "./assignment_editor_policy_panel";
import { ASSIGNMENT_EDITOR_STYLES } from "./assignment_editor_styles";

type EditorState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly draft: AssignmentEditorDraft }
  | { readonly kind: "error"; readonly message: string };

type CatalogState =
  | { readonly kind: "idle"; readonly rows: ReadonlyArray<AssignmentCatalogRow> }
  | { readonly kind: "loading"; readonly rows: ReadonlyArray<AssignmentCatalogRow> }
  | { readonly kind: "ready"; readonly rows: ReadonlyArray<AssignmentCatalogRow> }
  | { readonly kind: "error"; readonly rows: ReadonlyArray<AssignmentCatalogRow> };

type DirectImportState =
  | { readonly kind: "idle" }
  | { readonly kind: "loading"; readonly count: number }
  | { readonly kind: "success"; readonly message: string }
  | { readonly kind: "error"; readonly message: string };

type ReuseState =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly assignments: ReadonlyArray<ReusableAssignment> }
  | { readonly kind: "error" };

class DirectImportLookupError extends Error {
  public readonly reference: string;
  public readonly lookupCause: unknown;

  public constructor(reference: string, cause: unknown) {
    super(`Catalog lookup failed for ${reference}.`);
    this.name = "DirectImportLookupError";
    this.reference = reference;
    this.lookupCause = cause;
  }
}

function editorDraft(detail: AssignmentEditorDetail): AssignmentEditorDraft {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    problems: [...detail.problems],
    policies: detail.policies,
    assignmentTiming: detail.assignmentTiming,
    revision: detail.revision,
  };
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

export type AssignmentEditorMode =
  { readonly kind: "edit"; readonly assignmentId: AssignmentId } | { readonly kind: "create" };

export interface AssignmentEditorPageProps {
  readonly repository: AssignmentEditorRepository;
  readonly courseId: CourseId;
  readonly coursePublicId: CoursePublicId;
  readonly mode: AssignmentEditorMode;
  /** Session-derived tenant used only to reject a hostile cross-tenant response. */
  readonly tenant: TenantId;
}

/**
 * This component owns only editable assignment state. Its route parent has already
 * established both session and direct course-instructor access before constructing the repository.
 */
export function AssignmentEditorPage(props: AssignmentEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<EditorState>({ kind: "loading" });
  const [catalog, setCatalog] = createSignal<CatalogState>({ kind: "idle", rows: [] });
  const [knownProblems, setKnownProblems] = createSignal<ReadonlyArray<AssignmentCatalogRow>>([]);
  const [searchText, setSearchText] = createSignal("");
  const [directImportText, setDirectImportText] = createSignal("");
  const [directImport, setDirectImport] = createSignal<DirectImportState>({ kind: "idle" });
  const [reuse, setReuse] = createSignal<ReuseState>({ kind: "loading" });
  const [reuseSourceIndex, setReuseSourceIndex] = createSignal<number | null>(null);
  const [reuseQuestionIndexes, setReuseQuestionIndexes] = createSignal<ReadonlySet<number>>(
    new Set(),
  );
  const [saving, setSaving] = createSignal(false);
  const [saveMessage, setSaveMessage] = createSignal("");
  const [conflict, setConflict] = createSignal(false);
  const [runTimed, setRunTimed] = createSignal(true);
  const [runMinutesText, setRunMinutesText] = createSignal("15");
  // A database-valid integer second value can have a recurring minute form.
  // Keep that canonical value for an unrelated save until the instructor edits
  // the displayed approximation.
  const [preservedRunSeconds, setPreservedRunSeconds] = createSignal<number | null>(null);
  const [runMinutesEdited, setRunMinutesEdited] = createSignal(false);
  const [created, setCreated] = createSignal<AssignmentEditorDetail | null>(null);
  const [violations, setViolations] = createSignal<ReadonlyArray<AssignmentCapabilityViolation>>(
    [],
  );
  let titleInput: HTMLInputElement | undefined;
  let violationHeading: HTMLHeadingElement | undefined;
  let runMinutesInput: HTMLInputElement | undefined;

  function adoptTiming(seconds: number | null): void {
    setRunTimed(seconds !== null);
    setRunMinutesText(runTimeLimitMinutes(seconds));
    setPreservedRunSeconds(seconds);
    setRunMinutesEdited(false);
  }

  function runTimingValidation(): TimeLimitValidation {
    if (!runMinutesEdited()) {
      return { seconds: runTimed() ? preservedRunSeconds() : null, error: null };
    }
    return minutesToRunTimeLimit(runMinutesText(), runTimed());
  }

  const ready = (): Extract<EditorState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };
  const loadError = (): Extract<EditorState, { readonly kind: "error" }> | undefined => {
    const current = state();
    return current.kind === "error" ? current : undefined;
  };

  function replaceDraft(next: AssignmentEditorDraft): void {
    setState({ kind: "ready", draft: next });
    setSaveMessage("Unsaved assignment changes.");
    setConflict(false);
    setViolations([]);
  }

  function rememberProblems(rows: ReadonlyArray<AssignmentCatalogRow>): void {
    setKnownProblems((previous) => {
      const remembered = [...previous];
      for (const row of rows) {
        const index = remembered.findIndex((candidate) =>
          sameReference(candidate.reference, row.reference),
        );
        if (index < 0) remembered.push(row);
        else remembered[index] = row;
      }
      return remembered;
    });
  }

  async function load(): Promise<void> {
    if (props.mode.kind === "create") {
      const draft = createMasteryAssignmentDraft(props.courseId);
      setKnownProblems([]);
      setState({ kind: "ready", draft });
      adoptTiming(draft.assignmentTiming.timeLimitSeconds);
      setSaveMessage("Choose a title and at least one published question.");
      setConflict(false);
      setViolations([]);
      setCreated(null);
      queueMicrotask(() => titleInput?.focus());
      return;
    }
    setState({ kind: "loading" });
    setSaveMessage("");
    setConflict(false);
    setViolations([]);
    try {
      const detail = await props.repository.load(props.mode.assignmentId);
      if (
        detail.id !== props.mode.assignmentId ||
        detail.courseId !== props.courseId ||
        detail.tenant !== props.tenant
      ) {
        throw new Error("The assignment editor received an unrelated record.");
      }
      const described = await props.repository.describePublished(detail.problems);
      if (
        described.length !== detail.problems.length ||
        described.some((row, index) => {
          const reference = detail.problems[index];
          return reference === undefined || !sameReference(row.reference, reference);
        })
      ) {
        throw new Error("The assignment editor received unrelated published problem details.");
      }
      setKnownProblems(described);
      setState({ kind: "ready", draft: editorDraft(detail) });
      adoptTiming(detail.assignmentTiming.timeLimitSeconds);
      setSaveMessage("Assignment loaded.");
      queueMicrotask(() => titleInput?.focus());
    } catch (error: unknown) {
      setState({ kind: "error", message: errorMessage(error, "Assignment could not load.") });
    }
  }

  async function searchCatalog(): Promise<void> {
    const previous = catalog().rows;
    setCatalog({ kind: "loading", rows: previous });
    try {
      const rows = await props.repository.searchPublished(searchText());
      rememberProblems(rows);
      setCatalog({ kind: "ready", rows });
    } catch {
      setCatalog({ kind: "error", rows: previous });
    }
  }

  async function loadReusableAssignments(): Promise<void> {
    setReuse({ kind: "loading" });
    try {
      const assignments = await props.repository.listReusableAssignments(
        props.courseId,
        props.mode.kind === "edit" ? props.mode.assignmentId : undefined,
      );
      setReuse({ kind: "ready", assignments });
      setReuseSourceIndex(assignments.length > 0 ? 0 : null);
      setReuseQuestionIndexes(
        new Set(assignments[0]?.questions.map((_question, index) => index) ?? []),
      );
    } catch {
      setReuse({ kind: "error" });
    }
  }

  function reusableAssignments(): ReadonlyArray<ReusableAssignment> {
    const current = reuse();
    return current.kind === "ready" ? current.assignments : [];
  }

  function selectedReuseSource(): ReusableAssignment | undefined {
    const index = reuseSourceIndex();
    return index === null ? undefined : reusableAssignments()[index];
  }

  function chooseReuseSource(index: number): void {
    const source = reusableAssignments()[index];
    setReuseSourceIndex(source === undefined ? null : index);
    setReuseQuestionIndexes(
      new Set(source?.questions.map((_question, questionIndex) => questionIndex) ?? []),
    );
  }

  function toggleReuseQuestion(index: number, selected: boolean): void {
    setReuseQuestionIndexes((current) => {
      const next = new Set(current);
      if (selected) next.add(index);
      else next.delete(index);
      return next;
    });
  }

  function addReusedQuestions(allQuestions: boolean): void {
    const current = ready();
    const source = selectedReuseSource();
    if (current === undefined || source === undefined) return;
    const selectedRows = source.questions.filter(
      (_question, index) => allQuestions || reuseQuestionIndexes().has(index),
    );
    if (selectedRows.length === 0) {
      setSaveMessage("Choose at least one question to copy.");
      return;
    }
    rememberProblems(selectedRows);
    const next = selectedRows.reduce(addCatalogReference, current.draft);
    replaceDraft(next);
    const added = next.problems.length - current.draft.problems.length;
    setSaveMessage(
      added === 0
        ? `Every question from ${source.title} is already selected.`
        : `Copied ${added} question${added === 1 ? "" : "s"} from ${source.title}.`,
    );
  }

  function directImportError(reference: string, error: unknown): string {
    if (error instanceof ApiRequestError && error.status === 404) {
      return `${reference} is not an available published question. Check the Question ID.`;
    }
    if (error instanceof ApiRequestError && error.status === 403) {
      return `You do not have access to ${reference}. Ask its owner to publish or share it.`;
    }
    if (error instanceof ApiRequestError && error.status === 400) {
      return `${reference} is not a valid Question ID. Use the form 7K3-M9QP.`;
    }
    return `Could not look up ${reference}. Your pasted IDs and assignment are unchanged. Try again.`;
  }

  async function addByQuestionId(): Promise<void> {
    const current = ready();
    if (current === undefined || directImport().kind === "loading") return;
    let references: ReadonlyArray<string>;
    try {
      references = parseExactProblemDisplayReferences(directImportText());
    } catch (error: unknown) {
      setDirectImport({ kind: "error", message: errorMessage(error, "Question IDs are invalid.") });
      return;
    }
    const alreadySelected = references.find((reference) =>
      current.draft.problems.some((candidate) => {
        const row = problemFor(candidate);
        return row !== undefined && assignmentProblemLabel(row) === reference;
      }),
    );
    if (alreadySelected !== undefined) {
      setDirectImport({
        kind: "error",
        message: `${alreadySelected} is already in this assignment. Remove it from the pasted list or paste another ID.`,
      });
      return;
    }
    setDirectImport({ kind: "loading", count: references.length });
    try {
      const rows = await Promise.all(
        references.map(async (reference) => {
          try {
            return await props.repository.resolvePublished(reference);
          } catch (error: unknown) {
            throw new DirectImportLookupError(reference, error);
          }
        }),
      );
      const currentAfterLookup = ready();
      if (currentAfterLookup === undefined) return;
      const becameSelected = rows.find((row) => selected(row.reference));
      if (becameSelected !== undefined) {
        setDirectImport({
          kind: "error",
          message: `${assignmentProblemLabel(becameSelected)} is already in this assignment. Your pasted IDs and assignment are unchanged.`,
        });
        return;
      }
      rememberProblems(rows);
      const draft = rows.reduce(addCatalogReference, currentAfterLookup.draft);
      replaceDraft(draft);
      const labels = rows.map(assignmentProblemLabel);
      setDirectImport({
        kind: "success",
        message: `Added ${labels.join(", ")} to the unsaved selection.`,
      });
      setDirectImportText("");
    } catch (error: unknown) {
      const failure = error instanceof DirectImportLookupError ? error : undefined;
      setDirectImport({
        kind: "error",
        message: directImportError(
          failure?.reference ?? references[0] ?? "that ID",
          failure?.lookupCause ?? error,
        ),
      });
    }
  }

  function updatePolicies(policies: RunPolicies): void {
    const current = ready();
    if (current === undefined) return;
    replaceDraft({ ...current.draft, policies });
  }

  async function save(): Promise<void> {
    const current = ready();
    if (current === undefined || saving()) return;
    const timing = runTimingValidation();
    if (timing.error !== null) {
      setSaveMessage(timing.error);
      queueMicrotask(() => runMinutesInput?.focus());
      return;
    }
    setSaving(true);
    setSaveMessage("Saving assignment...");
    setConflict(false);
    setViolations([]);
    try {
      const saved =
        props.mode.kind === "create"
          ? await props.repository.create(props.courseId, {
              ...assignmentInput(current.draft),
              assignmentTiming: { timeLimitSeconds: timing.seconds },
            })
          : await props.repository.save(
              props.courseId,
              props.mode.assignmentId,
              {
                ...assignmentInput(current.draft),
                assignmentTiming: { timeLimitSeconds: timing.seconds },
              },
              current.draft.revision,
            );
      if (
        (props.mode.kind === "edit" && saved.id !== props.mode.assignmentId) ||
        saved.courseId !== props.courseId ||
        saved.tenant !== props.tenant
      ) {
        throw new Error("The assignment editor received an unrelated saved record.");
      }
      setState({ kind: "ready", draft: editorDraft(saved) });
      adoptTiming(saved.assignmentTiming.timeLimitSeconds);
      if (props.mode.kind === "create") {
        setCreated(saved);
        setSaveMessage("Assignment created. Open it to review the student-facing course link.");
      } else {
        setSaveMessage(
          saved.assignmentTiming.timeLimitSeconds === null
            ? "Assignment saved. This assignment is untimed."
            : `Assignment saved. Students have a ${runTimeLimitMinutes(saved.assignmentTiming.timeLimitSeconds)}-minute limit per practice run.`,
        );
      }
    } catch (error: unknown) {
      if (error instanceof AssignmentValidationError) {
        setViolations(error.violations);
        setSaveMessage("Fix the listed assignment settings, then save again.");
        queueMicrotask(() => violationHeading?.focus());
      } else if (error instanceof AssignmentConflictError) {
        setConflict(true);
        setSaveMessage(
          props.mode.kind === "create"
            ? "The course changed before this assignment was created. Your work is still here."
            : "A newer assignment revision exists. Your edits are still here.",
        );
      } else if (error instanceof ApiRequestError && error.status === 403) {
        setSaveMessage(
          "You no longer have permission to create this assignment. Your work is still here.",
        );
      } else if (error instanceof ApiRequestError && error.status === 409) {
        setSaveMessage(
          "The course changed before this assignment was created. Your work is still here.",
        );
      } else {
        setSaveMessage(
          `${errorMessage(error, "Assignment could not be saved.")} Your work is still here. Try again.`,
        );
      }
    } finally {
      setSaving(false);
    }
  }

  function selected(reference: ProblemVersionRef): boolean {
    return (
      ready()?.draft.problems.some((candidate) => sameReference(candidate, reference)) ?? false
    );
  }

  function saveDisabled(): boolean {
    const current = ready();
    return (
      saving() ||
      current === undefined ||
      (props.mode.kind === "create" && created() !== null) ||
      current.draft.title.trim().length === 0 ||
      current.draft.problems.length === 0 ||
      runTimingValidation().error !== null
    );
  }

  function titleFor(reference: ProblemVersionRef): string {
    return problemFor(reference)?.title ?? "Published problem";
  }

  function problemFor(reference: ProblemVersionRef): AssignmentCatalogRow | undefined {
    return knownProblems().find((candidate) => sameReference(candidate.reference, reference));
  }

  function directImportButtonLabel(): string {
    const current = directImport();
    if (current.kind !== "loading") return "Add questions by ID";
    return `Adding ${current.count} question${current.count === 1 ? "" : "s"}...`;
  }

  function directImportMessage(kind: "error" | "success"): string {
    const current = directImport();
    return current.kind === kind ? current.message : "";
  }

  onMount((): void => {
    void (async (): Promise<void> => {
      await load();
      if (ready() !== undefined) await loadReusableAssignments();
    })();
  });

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
        Choose published questions, arrange their order, and set how each practice run behaves.
      </p>
      <CourseManagementNav
        coursePublicId={props.coursePublicId}
        active={props.mode.kind === "create" ? "newAssignment" : "assignments"}
      />
      <p class="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {saveMessage()}
      </p>

      <Show when={state().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading assignment editor...
        </p>
      </Show>
      <Show when={loadError()}>
        {(current) => (
          <section class="route-error" role="alert">
            <p class="eyebrow">Assignment unavailable</p>
            <h2>This assignment could not be opened</h2>
            <p>{current().message}</p>
            <button class="primary-action" type="button" onClick={() => void load()}>
              Try again
            </button>
          </section>
        )}
      </Show>
      <Show when={ready()}>
        {(current) => (
          <>
            <Show when={created()} keyed>
              {(assignment) => (
                <section class="success-state" role="status">
                  <h2>Assignment created</h2>
                  <p>{assignment.title} now appears in this course.</p>
                  <A
                    class="primary-link"
                    href={`/courses/${courseRouteReference(props.coursePublicId)}/assignments/${assignmentRouteReference(assignment.publicId)}`}
                  >
                    Open {assignment.title}
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
                <p>The server found every unsupported capability in this saved selection.</p>
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
                  {props.mode.kind === "create"
                    ? "The course changed before this assignment was created. Your work is still visible."
                    : "A newer assignment revision exists. Your unsaved changes are still visible."}
                </p>
                <Show when={props.mode.kind === "edit"}>
                  <button class="quiet-action" type="button" onClick={() => void load()}>
                    Reload newest assignment
                  </button>
                </Show>
              </section>
            </Show>
            <div class="assignment-editor-actions" aria-label="Assignment save actions">
              <button
                class="primary-action"
                type="button"
                disabled={saveDisabled()}
                onClick={() => void save()}
              >
                {saving()
                  ? props.mode.kind === "create"
                    ? "Creating assignment..."
                    : "Saving assignment..."
                  : props.mode.kind === "create"
                    ? "Create assignment"
                    : "Save assignment"}
              </button>
              <Show
                when={
                  current().draft.title.trim().length === 0 || current().draft.problems.length === 0
                }
              >
                <p class="assignment-editor-note">
                  Add a title and at least one published question before saving. Publish workspace
                  drafts first.
                </p>
              </Show>
              <Show when={saveMessage()}>{(message) => <span>{message()}</span>}</Show>
            </div>
            <div class="assignment-editor-grid">
              <div class="assignment-editor-panel">
                <h2>Assignment content</h2>
                <label class="assignment-editor-field">
                  Assignment title
                  <input
                    ref={(element: HTMLInputElement) => {
                      titleInput = element;
                    }}
                    value={current().draft.title}
                    onInput={(event) =>
                      replaceDraft({ ...current().draft, title: event.currentTarget.value })
                    }
                  />
                </label>
                <details class="assignment-editor-reuse">
                  <summary>Reuse questions from an existing assignment</summary>
                  <div>
                    <Show when={reuse().kind === "loading"}>
                      <p class="assignment-editor-note" role="status">
                        Loading assignment question sets...
                      </p>
                    </Show>
                    <Show when={reuse().kind === "error"}>
                      <p class="inline-error">
                        Existing assignments could not be loaded. Your current work is unchanged.
                      </p>
                    </Show>
                    <Show
                      when={reusableAssignments().length > 0}
                      fallback={
                        <Show when={reuse().kind === "ready"}>
                          <p class="assignment-editor-note">
                            No other assignments are available in this course yet.
                          </p>
                        </Show>
                      }
                    >
                      <label class="assignment-editor-field">
                        Source assignment
                        <select
                          value={reuseSourceIndex() ?? ""}
                          onChange={(event) => chooseReuseSource(Number(event.currentTarget.value))}
                        >
                          <For each={reusableAssignments()}>
                            {(assignment, index) => (
                              <option value={index()}>{assignment.title}</option>
                            )}
                          </For>
                        </select>
                      </label>
                      <p class="assignment-editor-note">
                        Copy its whole question set or choose individual questions. Your title and
                        policies stay unchanged.
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
                                <small>{assignmentProblemLabel(question)}</small>
                              </span>
                            </label>
                          )}
                        </For>
                      </div>
                      <div class="assignment-editor-reuse-actions">
                        <button
                          class="primary-action"
                          type="button"
                          onClick={() => addReusedQuestions(false)}
                        >
                          Add selected questions
                        </button>
                        <button
                          class="quiet-action"
                          type="button"
                          onClick={() => addReusedQuestions(true)}
                        >
                          Add entire assignment
                        </button>
                      </div>
                    </Show>
                  </div>
                </details>
                <h3>Questions ({current().draft.problems.length})</h3>
                <Show
                  when={current().draft.problems.length > 0}
                  fallback={
                    <section class="empty-state" aria-label="No published questions selected">
                      <p>No published questions are selected yet.</p>
                      <p>Add a catalog result below. A workspace draft must be published first.</p>
                    </section>
                  }
                >
                  <ol class="assignment-editor-list">
                    <For each={current().draft.problems}>
                      {(reference, index) => (
                        <li class="assignment-editor-row">
                          <h3>{titleFor(reference)}</h3>
                          <div class="assignment-editor-problem-identity">
                            <Show
                              when={problemFor(reference)}
                              fallback="Published problem details unavailable"
                            >
                              {(row) => (
                                <>
                                  <code>{assignmentProblemLabel(row())}</code>
                                  <span>{questionBackendLabel(row().backend)}</span>
                                </>
                              )}
                            </Show>
                          </div>
                          <Show
                            when={violations().some((violation) =>
                              violationMatchesReference(violation, reference),
                            )}
                          >
                            <p class="inline-error">
                              This question has a server-reported capability conflict.
                            </p>
                          </Show>
                          <div class="assignment-editor-row-actions">
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={index() === 0}
                              aria-label={`Move ${titleFor(reference)} earlier`}
                              onClick={() =>
                                replaceDraft(moveCatalogReference(current().draft, reference, -1))
                              }
                            >
                              <span aria-hidden="true">&uarr;</span>
                            </button>
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={index() === current().draft.problems.length - 1}
                              aria-label={`Move ${titleFor(reference)} later`}
                              onClick={() =>
                                replaceDraft(moveCatalogReference(current().draft, reference, 1))
                              }
                            >
                              <span aria-hidden="true">&darr;</span>
                            </button>
                            <button
                              class="quiet-action"
                              type="button"
                              aria-label={`Remove ${titleFor(reference)} from assignment`}
                              onClick={() =>
                                replaceDraft(removeCatalogReference(current().draft, reference))
                              }
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
                  <summary id="add-by-id-heading">Add by question ID</summary>
                  <div>
                    <p id="add-by-id-help" class="assignment-editor-note">
                      Paste IDs from the library, separated by commas or lines-for example,
                      <code>7K3-M9QP, ABC-123T</code>.
                    </p>
                    <label class="assignment-editor-field">
                      Question IDs
                      <textarea
                        rows="2"
                        value={directImportText()}
                        placeholder="7K3-M9QP"
                        aria-describedby={
                          directImport().kind === "error" || directImport().kind === "success"
                            ? "add-by-id-help add-by-id-feedback"
                            : "add-by-id-help"
                        }
                        aria-invalid={directImport().kind === "error"}
                        onInput={(event) => {
                          setDirectImportText(event.currentTarget.value);
                          if (directImport().kind !== "loading") setDirectImport({ kind: "idle" });
                        }}
                      />
                    </label>
                    <button
                      class="primary-action"
                      type="button"
                      disabled={directImport().kind === "loading"}
                      onClick={() => void addByQuestionId()}
                    >
                      {directImportButtonLabel()}
                    </button>
                    <Show when={directImport().kind === "error"}>
                      <p id="add-by-id-feedback" class="inline-error" role="alert">
                        {directImportMessage("error")}
                      </p>
                    </Show>
                    <Show when={directImport().kind === "success"}>
                      <p
                        id="add-by-id-feedback"
                        class="assignment-editor-import-success"
                        role="status"
                      >
                        {directImportMessage("success")}
                      </p>
                    </Show>
                  </div>
                </details>
              </div>

              <aside class="assignment-editor-panel" aria-label="Assignment policies and catalog">
                <AssignmentEditorPolicyPanel
                  policies={() => current().draft.policies}
                  runTimed={runTimed}
                  runMinutesText={runMinutesText}
                  runTimingError={() => runTimingValidation().error}
                  onPoliciesChange={updatePolicies}
                  onRunTimedChange={(timed) => {
                    setRunTimed(timed);
                    if (timed && preservedRunSeconds() === null) setRunMinutesEdited(true);
                  }}
                  onRunMinutesInput={(value) => {
                    setRunMinutesEdited(true);
                    setRunMinutesText(value);
                  }}
                  onRunMinutesInputRef={(element) => {
                    runMinutesInput = element;
                  }}
                />

                <h2>Question catalog</h2>
                <label class="assignment-editor-field">
                  Search published questions
                  <input
                    value={searchText()}
                    placeholder="Title, concept, or Question ID"
                    onInput={(event) => setSearchText(event.currentTarget.value)}
                  />
                </label>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={catalog().kind === "loading"}
                  onClick={() => void searchCatalog()}
                >
                  {catalog().kind === "loading"
                    ? "Searching published questions..."
                    : "Search catalog"}
                </button>
                <Show when={catalog().kind === "error"}>
                  <p class="inline-error" role="alert">
                    The catalog could not load. Your assignment is unchanged; try again.
                  </p>
                </Show>
                <Show when={catalog().kind === "ready" && catalog().rows.length === 0}>
                  <p class="empty-state">No published questions match this search.</p>
                </Show>
                <div class="assignment-editor-catalog-results">
                  <For each={catalog().rows}>
                    {(row) => (
                      <article class="assignment-editor-row">
                        <h3>{row.title}</h3>
                        <div class="assignment-editor-problem-identity">
                          <CopyableProblemId displayId={assignmentProblemLabel(row)} />
                          <span>{questionBackendLabel(row.backend)}</span>
                        </div>
                        <button
                          class="quiet-action"
                          type="button"
                          disabled={selected(row.reference)}
                          onClick={() => replaceDraft(addCatalogReference(current().draft, row))}
                        >
                          {selected(row.reference) ? "Already selected" : "Add question"}
                        </button>
                      </article>
                    )}
                  </For>
                </div>
              </aside>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}
