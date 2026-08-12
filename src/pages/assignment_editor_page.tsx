// assignment_editor_page.tsx - role-gated instructor controls for one revisioned assignment.

import { A } from "@solidjs/router";
import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { TenantId } from "../../generated/api/TenantId";
import type { AssignmentCapabilityViolation, AssignmentEditorDetail } from "../api/contracts";
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
  moveCatalogReference,
  questionBackendLabel,
  removeCatalogReference,
  sameReference,
  violationMatchesReference,
  type AssignmentCatalogRow,
  type AssignmentEditorDraft,
} from "./assignment_editor_model";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
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

function editorDraft(detail: AssignmentEditorDetail): AssignmentEditorDraft {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    problems: [...detail.problems],
    policies: detail.policies,
    revision: detail.revision,
  };
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

function gradePolicy(value: string): RunPolicies["grade"] {
  switch (value) {
    case "first":
    case "latest":
    case "highest":
    case "instructorSelected":
      return value;
    default:
      throw new Error("Grade policy selection is invalid");
  }
}

function variationPolicy(value: string): RunPolicies["variation"] {
  switch (value) {
    case "newSeeds":
    case "selectedProblemVariants":
    case "fullRegeneration":
      return value;
    default:
      throw new Error("Variation policy selection is invalid");
  }
}

export type AssignmentEditorMode =
  { readonly kind: "edit"; readonly assignmentId: AssignmentId } | { readonly kind: "create" };

export interface AssignmentEditorPageProps {
  readonly repository: AssignmentEditorRepository;
  readonly courseId: CourseId;
  readonly mode: AssignmentEditorMode;
  /** Session-derived tenant used only to reject a hostile cross-tenant response. */
  readonly tenant: TenantId;
}

/**
 * This component owns only editable assignment state. Its route parent has already
 * established both session and course-manager access before constructing the repository.
 */
export function AssignmentEditorPage(props: AssignmentEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<EditorState>({ kind: "loading" });
  const [catalog, setCatalog] = createSignal<CatalogState>({ kind: "idle", rows: [] });
  const [knownProblems, setKnownProblems] = createSignal<ReadonlyArray<AssignmentCatalogRow>>([]);
  const [searchText, setSearchText] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [saveMessage, setSaveMessage] = createSignal("");
  const [conflict, setConflict] = createSignal(false);
  const [created, setCreated] = createSignal<AssignmentEditorDetail | null>(null);
  const [violations, setViolations] = createSignal<ReadonlyArray<AssignmentCapabilityViolation>>(
    [],
  );
  let titleInput: HTMLInputElement | undefined;
  let violationHeading: HTMLHeadingElement | undefined;

  const ready = (): Extract<EditorState, { readonly kind: "ready" }> | undefined => {
    const current = state();
    return current.kind === "ready" ? current : undefined;
  };
  const loadError = (): Extract<EditorState, { readonly kind: "error" }> | undefined => {
    const current = state();
    return current.kind === "error" ? current : undefined;
  };

  function completionFraction(policies: RunPolicies): number {
    return policies.completion.kind === "scoreAtLeast" ? policies.completion.fraction : 0.8;
  }

  function additionalRunLimit(policies: RunPolicies): number {
    return policies.continuedPractice.kind === "capped"
      ? policies.continuedPractice.maxAdditionalRuns
      : 3;
  }

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
      setKnownProblems([]);
      setState({ kind: "ready", draft: createMasteryAssignmentDraft(props.courseId) });
      setSaveMessage("Choose a title and published problem versions.");
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

  function updatePolicies(policies: RunPolicies): void {
    const current = ready();
    if (current === undefined) return;
    replaceDraft({ ...current.draft, policies });
  }

  async function save(): Promise<void> {
    const current = ready();
    if (current === undefined || saving()) return;
    setSaving(true);
    setSaveMessage("Saving assignment...");
    setConflict(false);
    setViolations([]);
    try {
      const saved =
        props.mode.kind === "create"
          ? await props.repository.create(props.courseId, assignmentInput(current.draft))
          : await props.repository.save(
              props.courseId,
              props.mode.assignmentId,
              assignmentInput(current.draft),
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
      if (props.mode.kind === "create") {
        setCreated(saved);
        setSaveMessage("Assignment created. Open it to review the student-facing course link.");
      } else {
        setSaveMessage("Assignment saved.");
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
      current.draft.problems.length === 0
    );
  }

  function titleFor(reference: ProblemVersionRef): string {
    return problemFor(reference)?.title ?? "Published problem";
  }

  function problemFor(reference: ProblemVersionRef): AssignmentCatalogRow | undefined {
    return knownProblems().find((candidate) => sameReference(candidate.reference, reference));
  }

  onMount(() => void load());

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
        Choose published, immutable question versions and set the four run policies. Workspace
        drafts must be published first before they can be added here.
      </p>
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
                    href={`/courses/${assignment.courseId}/assignments/${assignment.id}`}
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
                <h3>Selected published versions</h3>
                <Show
                  when={current().draft.problems.length > 0}
                  fallback={
                    <section class="empty-state" aria-label="No published questions selected">
                      <p>No published question versions are selected yet.</p>
                      <p>Add a catalog result below. A workspace draft must be published first.</p>
                    </section>
                  }
                >
                  <ol class="assignment-editor-list">
                    <For each={current().draft.problems}>
                      {(reference, index) => (
                        <li class="assignment-editor-row">
                          <h3>{titleFor(reference)}</h3>
                          <p
                            data-problem-id={reference.problem}
                            data-version-id={reference.version}
                          >
                            <Show
                              when={problemFor(reference)}
                              fallback="Published problem details unavailable"
                            >
                              {(row) =>
                                `${assignmentProblemLabel(row())} · ${questionBackendLabel(
                                  row().backend,
                                )}`
                              }
                            </Show>
                          </p>
                          <Show
                            when={violations().some((violation) =>
                              violationMatchesReference(violation, reference),
                            )}
                          >
                            <p class="inline-error">
                              This version has a server-reported capability conflict.
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
                              Move earlier
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
                              Move later
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
              </div>

              <aside class="assignment-editor-panel" aria-label="Assignment policies and catalog">
                <h2>Run policies</h2>
                <fieldset class="assignment-editor-policy-set">
                  <legend>Completion requirement</legend>
                  <label class="assignment-editor-field">
                    Completion
                    <select
                      aria-label="Completion requirement"
                      value={current().draft.policies.completion.kind}
                      onChange={(event) => {
                        const kind = event.currentTarget.value;
                        const completion =
                          kind === "answerAll"
                            ? { kind: "answerAll" as const }
                            : kind === "scoreAtLeast"
                              ? { kind: "scoreAtLeast" as const, fraction: 0.8 }
                              : { kind: "allCorrect" as const };
                        updatePolicies({ ...current().draft.policies, completion });
                      }}
                    >
                      <option value="allCorrect">All questions correct</option>
                      <option value="answerAll">Answer every question</option>
                      <option value="scoreAtLeast">Reach a score threshold</option>
                    </select>
                  </label>
                  <Show when={current().draft.policies.completion.kind === "scoreAtLeast"}>
                    <label class="assignment-editor-field">
                      Required score fraction
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.05"
                        value={completionFraction(current().draft.policies)}
                        onInput={(event) => {
                          const fraction = Number(event.currentTarget.value);
                          if (!Number.isFinite(fraction)) return;
                          updatePolicies({
                            ...current().draft.policies,
                            completion: { kind: "scoreAtLeast", fraction },
                          });
                        }}
                      />
                    </label>
                  </Show>
                </fieldset>
                <fieldset class="assignment-editor-policy-set">
                  <legend>Grade policy</legend>
                  <label class="assignment-editor-field">
                    Record
                    <select
                      aria-label="Grade policy"
                      value={current().draft.policies.grade}
                      onChange={(event) =>
                        updatePolicies({
                          ...current().draft.policies,
                          grade: gradePolicy(event.currentTarget.value),
                        })
                      }
                    >
                      <option value="highest">Highest run score</option>
                      <option value="latest">Latest run score</option>
                      <option value="first">First run score</option>
                      <option value="instructorSelected">Instructor-selected run</option>
                    </select>
                  </label>
                </fieldset>
                <fieldset class="assignment-editor-policy-set">
                  <legend>Continued practice</legend>
                  <label class="assignment-editor-field">
                    After completion
                    <select
                      aria-label="Continued practice"
                      value={current().draft.policies.continuedPractice.kind}
                      onChange={(event) => {
                        const kind = event.currentTarget.value;
                        const continuedPractice =
                          kind === "closed"
                            ? { kind: "closed" as const }
                            : kind === "capped"
                              ? { kind: "capped" as const, maxAdditionalRuns: 3 }
                              : { kind: "unlimited" as const };
                        updatePolicies({ ...current().draft.policies, continuedPractice });
                      }}
                    >
                      <option value="unlimited">Allow unlimited practice</option>
                      <option value="capped">Limit additional runs</option>
                      <option value="closed">Close after completion</option>
                    </select>
                  </label>
                  <Show when={current().draft.policies.continuedPractice.kind === "capped"}>
                    <label class="assignment-editor-field">
                      Additional runs
                      <input
                        type="number"
                        min="0"
                        step="1"
                        value={additionalRunLimit(current().draft.policies)}
                        onInput={(event) => {
                          const maxAdditionalRuns = Number(event.currentTarget.value);
                          if (!Number.isSafeInteger(maxAdditionalRuns) || maxAdditionalRuns < 0)
                            return;
                          updatePolicies({
                            ...current().draft.policies,
                            continuedPractice: { kind: "capped", maxAdditionalRuns },
                          });
                        }}
                      />
                    </label>
                  </Show>
                </fieldset>
                <fieldset class="assignment-editor-policy-set">
                  <legend>Variation policy</legend>
                  <label class="assignment-editor-field">
                    Next practice run
                    <select
                      aria-label="Variation policy"
                      value={current().draft.policies.variation}
                      onChange={(event) =>
                        updatePolicies({
                          ...current().draft.policies,
                          variation: variationPolicy(event.currentTarget.value),
                        })
                      }
                    >
                      <option value="newSeeds">Use new seeds</option>
                      <option value="selectedProblemVariants">Use selected problem variants</option>
                      <option value="fullRegeneration">Fully regenerate</option>
                    </select>
                  </label>
                </fieldset>
                <p class="assignment-editor-note">
                  Attempt and timing rules remain immutable properties of each published question
                  version; this assignment does not override them.
                </p>

                <h2>Published problem catalog</h2>
                <label class="assignment-editor-field">
                  Search published problems
                  <input
                    value={searchText()}
                    placeholder="Title or concept"
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
                    ? "Searching published problems..."
                    : "Search catalog"}
                </button>
                <Show when={catalog().kind === "error"}>
                  <p class="inline-error" role="alert">
                    The catalog could not load. Your assignment is unchanged; try again.
                  </p>
                </Show>
                <Show when={catalog().kind === "ready" && catalog().rows.length === 0}>
                  <p class="empty-state">No published problems match this search.</p>
                </Show>
                <div class="assignment-editor-catalog-results">
                  <For each={catalog().rows}>
                    {(row) => (
                      <article class="assignment-editor-row">
                        <h3>{row.title}</h3>
                        <p
                          data-problem-id={row.reference.problem}
                          data-version-id={row.reference.version}
                        >
                          {assignmentProblemLabel(row)} · {questionBackendLabel(row.backend)}
                        </p>
                        <button
                          class="quiet-action"
                          type="button"
                          disabled={selected(row.reference)}
                          onClick={() => replaceDraft(addCatalogReference(current().draft, row))}
                        >
                          {selected(row.reference) ? "Already selected" : "Add published version"}
                        </button>
                      </article>
                    )}
                  </For>
                </div>
              </aside>
            </div>
            <div class="assignment-editor-actions">
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
                  Add a title and at least one published version before saving. Publish workspace
                  drafts first.
                </p>
              </Show>
              <Show when={saveMessage()}>{(message) => <span>{message()}</span>}</Show>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}
