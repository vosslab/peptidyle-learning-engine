// M10 direct-Instructor Assignment Workspace and answer-free Assignment Preview.

import { A, useNavigate, useParams } from "@solidjs/router";
import { For, Show, createEffect, createResource, createSignal, type JSX } from "solid-js";

import type { AssignmentPreview, LiveAssignmentWorkspace } from "../api/assignment_release";
import type { CourseLocalDateAndTime } from "../../generated/api/CourseLocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { QuestionId } from "../../generated/api/QuestionId";
import { useApplicationApi } from "../api/application_api";
import { LiveAssignmentWorkspaceConflictError } from "../api/http_client/assignment_release";
import { parseAssignmentReference, parseCourseInstanceReference } from "../navigation/public_route";

function issueCopy(issue: "noPublishedQuestions" | "questionUnavailable"): string {
  return issue === "noPublishedQuestions"
    ? "Choose at least one Available Published Question before release."
    : "One or more selected Questions are no longer Available. Update the selection before release.";
}

/** Keeps the native local wall-clock value intact; the server owns zone resolution. */
function canonicalDueAt(value: string): CourseLocalDateAndTime | null {
  if (value === "") return null;
  const normalized = value.length === 16 ? `${value}:00.000` : value;
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(normalized)) {
    throw new Error("Due date must be a course-local date and time.");
  }
  return normalized;
}

/** Bounded M10 workspace: no Student identity, records, work, or answers cross this surface. */
export function AssignmentReleasePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const params = useParams();
  const course = () => parseCourseInstanceReference(params["courseRef"] ?? "");
  const assignment = () => parseAssignmentReference(params["assignmentRef"] ?? "");
  const isCreate = () => assignment() === null;
  const [workspace, { refetch }] = createResource(assignment, async (reference) => {
    const currentCourse = course();
    if (currentCourse === null || reference === null)
      throw new Error("Assignment workspace reference is invalid");
    return applicationApi.client.getLiveAssignmentWorkspace(currentCourse, reference);
  });
  const [picker] = createResource(course, async (reference) => {
    if (reference === null) throw new Error("Course Instance reference is invalid");
    return applicationApi.client.listLiveAssignmentQuestionPicker(reference);
  });
  const [title, setTitle] = createSignal("");
  const [instructions, setInstructions] = createSignal("");
  const [dueAt, setDueAt] = createSignal("");
  const [lateWorkRule, setLateWorkRule] = createSignal<LateWorkRule>("accept");
  const [selectedQuestionIds, setSelectedQuestionIds] = createSignal<ReadonlyArray<string>>([]);
  const [message, setMessage] = createSignal("");
  const [previewOpen, setPreviewOpen] = createSignal(false);
  const [preview, setPreview] = createSignal<AssignmentPreview>();
  const [busy, setBusy] = createSignal(false);

  createEffect(() => {
    const loaded = workspace();
    if (loaded === undefined) return;
    setTitle(loaded.workspace.title);
    setInstructions(loaded.workspace.instructions);
    setDueAt(loaded.workspace.dueAt ?? "");
    setLateWorkRule(loaded.workspace.lateWorkRule);
    setSelectedQuestionIds(loaded.workspace.questions.map((question) => question.questionId));
  });

  function currentWorkspace(): LiveAssignmentWorkspace | undefined {
    return workspace()?.workspace;
  }

  async function createAssignment(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const reference = course();
    if (reference === null) return;
    setBusy(true);
    setMessage("");
    try {
      const created = await applicationApi.client.createLiveAssignment(reference, {
        title: title(),
        instructions: instructions(),
      });
      await navigate(
        `/instructor/courses/${reference}/assignments/${created.workspace.reference}/release`,
      );
    } catch {
      setMessage("The Assignment could not be created. Check the title and try again.");
    } finally {
      setBusy(false);
    }
  }

  function toggleQuestion(questionId: string, checked: boolean): void {
    setSelectedQuestionIds((current) =>
      checked ? [...current, questionId] : current.filter((candidate) => candidate !== questionId),
    );
  }

  async function saveAssignment(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const currentCourse = course();
    const current = workspace();
    if (currentCourse === null || current === undefined) return;
    setBusy(true);
    setMessage("");
    try {
      await applicationApi.client.saveLiveAssignment(
        currentCourse,
        current.workspace.reference,
        {
          title: title(),
          instructions: instructions(),
          questionIds: selectedQuestionIds() as ReadonlyArray<QuestionId>,
          dueAt: canonicalDueAt(dueAt()),
          lateWorkRule: lateWorkRule(),
        },
        current.etag,
      );
      setMessage("Assignment saved. Validate it before release.");
      await refetch();
    } catch (error) {
      setMessage(
        error instanceof LiveAssignmentWorkspaceConflictError
          ? "This Assignment changed elsewhere. Reload the workspace, review your edits, and save again."
          : "The Assignment could not be saved. Check the selected Available Published Questions and try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function validate(): Promise<void> {
    const currentCourse = course();
    const current = currentWorkspace();
    if (currentCourse === null || current === undefined) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await applicationApi.client.validateLiveAssignmentRelease(
        currentCourse,
        current.reference,
      );
      setMessage(
        result.canRelease
          ? "Assignment validation passed. You can review the answer-free Assignment Preview or release it."
          : result.issues.map(issueCopy).join(" "),
      );
    } catch {
      setMessage("Assignment validation is unavailable. Reload and try again.");
    } finally {
      setBusy(false);
    }
  }

  async function openPreview(): Promise<void> {
    const currentCourse = course();
    const current = currentWorkspace();
    if (currentCourse === null || current === undefined) return;
    setBusy(true);
    setMessage("");
    try {
      setPreview(
        await applicationApi.client.getLiveAssignmentPreview(currentCourse, current.reference),
      );
      setPreviewOpen(true);
    } catch {
      setMessage("Assignment Preview is unavailable. Save the Assignment and try again.");
    } finally {
      setBusy(false);
    }
  }

  async function release(): Promise<void> {
    const currentCourse = course();
    const current = workspace();
    if (currentCourse === null || current === undefined) return;
    setBusy(true);
    setMessage("");
    try {
      const released = await applicationApi.client.releaseLiveAssignment(
        currentCourse,
        current.workspace.reference,
        current.etag,
      );
      setMessage(`Released Assignment Revision ${released.revisionNumber}.`);
      await refetch();
    } catch (error) {
      setMessage(
        error instanceof LiveAssignmentWorkspaceConflictError
          ? "This Assignment changed elsewhere. Reload the workspace before releasing it."
          : "The Assignment could not be released. Validate the saved Assignment and try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="page" data-route-surface="assignmentReleaseWorkspace">
      <p class="eyebrow">Course Instance · Assignment Workspace</p>
      <h1>{isCreate() ? "Create Assignment" : "Assignment Workspace"}</h1>
      <p class="page-lede">
        Select only Available Published Questions. Assignment Preview is answer-free and does not
        create Student delivery.
      </p>
      <Show when={message()}>
        {(text) => (
          <p class="inline-error" role="status" aria-live="polite">
            {text()}
          </p>
        )}
      </Show>
      <Show when={isCreate()}>
        <form class="auth-panel auth-form" onSubmit={(event) => void createAssignment(event)}>
          <label for="assignment-title">Assignment title</label>
          <input
            id="assignment-title"
            required
            value={title()}
            onInput={(event) => setTitle(event.currentTarget.value)}
          />
          <label for="assignment-instructions">Instructions</label>
          <textarea
            id="assignment-instructions"
            rows={5}
            value={instructions()}
            onInput={(event) => setInstructions(event.currentTarget.value)}
          />
          <button class="primary-action" type="submit" disabled={busy()}>
            Create Assignment
          </button>
        </form>
      </Show>
      <Show when={!isCreate() && workspace.loading}>
        <p class="loading-state">Loading Assignment Workspace...</p>
      </Show>
      <Show when={!isCreate() && workspace.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Assignment unavailable</h2>
          <p>Your current Teaching Team access cannot load this Assignment.</p>
          <button class="primary-action" type="button" onClick={() => void refetch()}>
            Reload workspace
          </button>
        </section>
      </Show>
      <Show when={!isCreate() && workspace() && picker()}>
        <form class="auth-panel auth-form" onSubmit={(event) => void saveAssignment(event)}>
          <label for="assignment-title">Assignment title</label>
          <input
            id="assignment-title"
            required
            value={title()}
            onInput={(event) => setTitle(event.currentTarget.value)}
          />
          <label for="assignment-instructions">Instructions</label>
          <textarea
            id="assignment-instructions"
            rows={5}
            value={instructions()}
            onInput={(event) => setInstructions(event.currentTarget.value)}
          />
          <fieldset>
            <legend>Available Published Questions</legend>
            <For each={picker()}>
              {(question) => (
                <label>
                  <input
                    type="checkbox"
                    checked={selectedQuestionIds().includes(question.questionId)}
                    onChange={(event) =>
                      toggleQuestion(question.questionId, event.currentTarget.checked)
                    }
                  />{" "}
                  {question.description}
                </label>
              )}
            </For>
          </fieldset>
          <label for="assignment-due-at">Due date</label>
          <input
            id="assignment-due-at"
            type="datetime-local"
            step="0.001"
            value={dueAt()}
            onInput={(event) => setDueAt(event.currentTarget.value)}
            aria-describedby="assignment-due-at-help"
          />
          <p id="assignment-due-at-help" class="field-help">
            Course-local wall-clock time. The server resolves this Due date using the Course Term.
          </p>
          <label for="assignment-late-work-rule">Late-work rule</label>
          <select
            id="assignment-late-work-rule"
            value={lateWorkRule()}
            onChange={(event) => setLateWorkRule(event.currentTarget.value as LateWorkRule)}
          >
            <option value="accept">Accept</option>
            <option value="mark_late">Mark late</option>
            <option value="reject">Reject</option>
          </select>
          <button class="primary-action" type="submit" disabled={busy()}>
            Save Assignment
          </button>
          <button
            class="quiet-action"
            type="button"
            disabled={busy()}
            onClick={() => void validate()}
          >
            Validate Assignment
          </button>
          <button
            class="quiet-action"
            type="button"
            disabled={busy()}
            onClick={() => void openPreview()}
          >
            Open Assignment Preview
          </button>
          <Show when={currentWorkspace()?.status === "unreleased"}>
            <button
              class="primary-action"
              type="button"
              disabled={busy()}
              onClick={() => void release()}
            >
              Release Assignment
            </button>
          </Show>
          <Show when={currentWorkspace()?.status === "released"}>
            <p role="status">Released state · current edit {currentWorkspace()?.editNumber}</p>
          </Show>
        </form>
      </Show>
      <Show when={previewOpen() && preview()}>
        {(loadedPreview) => (
          <section class="auth-panel">
            <h2>Assignment Preview</h2>
            <p>This answer-free preview does not create a Student attempt or access.</p>
            <h3>{loadedPreview().title}</h3>
            <p>{loadedPreview().instructions}</p>
            <For each={loadedPreview().questions}>
              {(question) => <p>{question.description}</p>}
            </For>
            <button class="quiet-action" type="button" onClick={() => setPreviewOpen(false)}>
              Return to Assignment Workspace
            </button>
          </section>
        )}
      </Show>
      <p>
        <A href={`/courses/${params["courseRef"] ?? ""}`}>Return to Course Instance</A>
      </p>
    </section>
  );
}
