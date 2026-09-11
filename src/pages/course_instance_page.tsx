// course_instance_page.tsx - Course Instance Teaching Team workspace.

import { A, useParams } from "@solidjs/router";
import { createResource, createSignal, For, Show, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { CourseAssignmentSummary, LiveAssignmentStatus } from "../api/assignment_release";
import { LiveAssignmentWorkspaceConflictError } from "../api/http_client/assignment_release";
import { parseCourseInstanceReference } from "../navigation/public_route";
import {
  canonicalLocalDateAndTime,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
} from "./assignment_workspace/assignment_workspace_policy_model";

import "./course_instance_page.css";

function assignmentStatusLabel(status: LiveAssignmentStatus): string {
  switch (status) {
    case "unreleased":
      return "Unreleased";
    case "released":
      return "Released";
    case "closed":
      return "Closed";
    case "archived":
      return "Archived";
  }
}

function assignmentQuestionsPath(courseReference: string, assignmentReference: string): string {
  return `/instructor/courses/${courseReference}/assignments/${assignmentReference}/questions`;
}

function AssignmentRow(props: {
  readonly courseReference: string;
  readonly assignment: CourseAssignmentSummary;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assignment, setAssignment] = createSignal(props.assignment);
  const [state, setState] = createSignal<"idle" | "editing" | "saving" | "failed" | "read-only">(
    "idle",
  );
  const [title, setTitle] = createSignal(assignment().title);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(assignment().dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(assignment().dueAt));
  const [message, setMessage] = createSignal("");
  const [needsRefresh, setNeedsRefresh] = createSignal(false);
  let titleInput: HTMLInputElement | undefined;
  let editButton: HTMLButtonElement | undefined;
  let assignmentQuestionsLink: HTMLAnchorElement | undefined;
  let readOnlyNotice: HTMLParagraphElement | undefined;

  function mayEdit(): boolean {
    return assignment().status === "unreleased" || assignment().status === "released";
  }

  function beginEditing(): void {
    const current = assignment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("editing");
    queueMicrotask(() => titleInput?.focus());
  }

  function cancelEditing(): void {
    const current = assignment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("idle");
    queueMicrotask(focusAssignmentAction);
  }

  function focusAssignmentAction(): void {
    const focusTarget = mayEdit() ? editButton : assignmentQuestionsLink;
    focusTarget?.focus();
  }

  function closeReadOnlyEditor(): void {
    setMessage("");
    setState("idle");
    queueMicrotask(focusAssignmentAction);
  }

  function typedDueDateAndTime(): string {
    if (dueDate() === "") return "No due date";
    return `${dueDate()} ${dueTime()}`;
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const dueAt = canonicalLocalDateAndTime(localDueDateAndTime(dueDate(), dueTime()));
    if (dueDate() !== "" && dueAt === null) {
      setState("failed");
      setMessage("Enter a complete local due date and time, or clear the due date.");
      return;
    }
    setState("saving");
    setMessage("");
    try {
      const saved = await applicationApi.client.saveLiveAssignmentInline(
        props.courseReference,
        assignment().reference,
        { title: title(), dueAt },
        assignment().editNumber,
      );
      setAssignment(saved);
      setState("idle");
      setMessage("Assignment title and due date saved.");
      queueMicrotask(() => editButton?.focus());
    } catch (error: unknown) {
      setState("failed");
      setNeedsRefresh(error instanceof LiveAssignmentWorkspaceConflictError);
      setMessage(
        error instanceof LiveAssignmentWorkspaceConflictError
          ? "This Assignment changed elsewhere. Load the current Assignment, then review it before retrying. Your typed values remain here."
          : "The Assignment was not saved. Check the title and local due date and time, then try again. Your typed values remain here.",
      );
    }
  }

  async function refreshCurrentAssignment(): Promise<void> {
    setState("saving");
    setMessage("");
    try {
      const latest = (
        await applicationApi.client.listCourseAssignments(props.courseReference)
      ).find((candidate) => candidate.reference === assignment().reference);
      if (latest === undefined) throw new Error("Current Assignment was not returned");
      setAssignment(latest);
      setNeedsRefresh(false);
      if (!mayEdit()) {
        setState("read-only");
        setMessage(
          `This Assignment is ${assignmentStatusLabel(latest.status)} and cannot save title and due date edits. Your typed values remain available until you close this editor.`,
        );
        queueMicrotask(() => readOnlyNotice?.focus());
        return;
      }
      setState("failed");
      setMessage("Latest Assignment loaded. Review it, then save your typed values again.");
    } catch {
      setState("failed");
      setMessage(
        "The current Assignment could not load. Your typed values remain here. Try again.",
      );
    }
  }

  return (
    <article class="instructor-list__row instructor-list__row--assignment">
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">{assignmentStatusLabel(assignment().status)} Assignment</p>
        <h3>{assignment().title}</h3>
        <p class="instructor-list__metadata">Assignment {assignment().reference}</p>
        <p class="instructor-list__metadata">
          Due: {assignment().dueAt ?? "No due date"} ({assignment().displayTimeZone})
        </p>
      </div>
      <div class="instructor-list__actions">
        <A
          ref={(element) => (assignmentQuestionsLink = element)}
          class="primary-link"
          href={assignmentQuestionsPath(props.courseReference, assignment().reference)}
        >
          Edit Assignment
        </A>
        <Show when={mayEdit() && state() === "idle"}>
          <button
            ref={(element) => (editButton = element)}
            class="quiet-action"
            type="button"
            disabled={state() === "saving"}
            onClick={beginEditing}
          >
            Edit title and due date
          </button>
        </Show>
      </div>
      <Show when={state() !== "idle" && state() !== "read-only"}>
        <form
          class="assignment-inline-editor"
          aria-label={`Edit ${assignment().title}`}
          aria-busy={state() === "saving"}
          onSubmit={(event) => void save(event)}
        >
          <fieldset disabled={state() === "saving"}>
            <div class="assignment-inline-editor__fields">
              <label class="assignment-inline-editor__field">
                Title
                <input
                  ref={(element) => (titleInput = element)}
                  value={title()}
                  onInput={(event) => setTitle(event.currentTarget.value)}
                  required
                />
              </label>
              <div
                class="assignment-inline-editor__schedule"
                role="group"
                aria-label="Due date and time"
              >
                <label class="assignment-inline-editor__field">
                  Due date ({assignment().displayTimeZone})
                  <input
                    type="date"
                    value={dueDate()}
                    onInput={(event) => setDueDate(event.currentTarget.value)}
                  />
                </label>
                <label class="assignment-inline-editor__field">
                  Due time
                  <input
                    type="time"
                    step="0.001"
                    value={dueTime()}
                    onInput={(event) => setDueTime(event.currentTarget.value)}
                  />
                </label>
              </div>
            </div>
            <div class="assignment-inline-editor__actions">
              <button class="primary-action" type="submit">
                {state() === "saving" ? "Saving title and due date..." : "Save title and due date"}
              </button>
              <button class="quiet-action" type="button" onClick={cancelEditing}>
                Cancel
              </button>
              <Show when={needsRefresh()}>
                <button
                  class="quiet-action"
                  type="button"
                  onClick={() => void refreshCurrentAssignment()}
                >
                  Load current Assignment
                </button>
              </Show>
            </div>
          </fieldset>
          <Show when={message()}>
            {(value) => (
              <p
                class="assignment-inline-editor__message"
                role={state() === "failed" ? "alert" : "status"}
              >
                {value()}
              </p>
            )}
          </Show>
        </form>
      </Show>
      <Show when={state() === "read-only"}>
        <section
          class="assignment-inline-editor"
          aria-label={`Editing unavailable for ${assignment().title}`}
        >
          <p
            ref={(element) => (readOnlyNotice = element)}
            class="assignment-inline-editor__message"
            role="alert"
            tabindex="-1"
          >
            {message()}
          </p>
          <p class="assignment-inline-editor__message">Unsaved title: {title()}</p>
          <p class="assignment-inline-editor__message">
            Unsaved due date and time: {typedDueDateAndTime()}
          </p>
          <button class="quiet-action" type="button" onClick={closeReadOnlyEditor}>
            Close editor
          </button>
        </section>
      </Show>
      <Show when={state() === "idle" && message()}>
        {(value) => (
          <p class="assignment-inline-editor__message" role="status">
            {value()}
          </p>
        )}
      </Show>
    </article>
  );
}

/** Instructor Course Instance workspace for Teaching Team, roster, and Assignment delivery. */
export function CourseInstancePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseReference(): ReturnType<typeof parseCourseInstanceReference> {
    return parseCourseInstanceReference(params["courseRef"] ?? "");
  }
  const [course] = createResource(courseReference, async (reference) =>
    applicationApi.client.getCourseInstance(reference),
  );
  const [assignments] = createResource(courseReference, async (reference) =>
    applicationApi.client.listCourseAssignments(reference),
  );

  return (
    <section class="page" data-route-surface="courseInstance">
      <Show when={course.loading}>
        <p class="loading-state">Loading Course Instance...</p>
      </Show>
      <Show
        when={course()}
        fallback={
          <Show when={course.error !== undefined}>
            <section class="route-error" role="alert">
              <h1>Course Instance unavailable</h1>
              <p>
                This Course Instance is not available through your current Teaching Team access.
              </p>
              <A class="primary-link" href="/">
                Return to Course Instances
              </A>
            </section>
          </Show>
        }
      >
        {(view) => (
          <>
            <p class="eyebrow">Course Instance · {view().course.reference}</p>
            <h1>{view().course.longName}</h1>
            <p class="page-lede">
              Course Term: {view().course.term.startDate} through {view().course.term.endDate}.
            </p>
            <section class="course-card" aria-labelledby="teaching-team-heading">
              <p class="card-kicker">Teaching Team</p>
              <h2 id="teaching-team-heading">Initial Teaching Team</h2>
              <p>
                {view().isAssignedInstructor
                  ? "You are the Assigned Instructor for this Course Instance."
                  : "You are an active Teaching Team Member for this Course Instance."}
              </p>
              <p>
                {view().activeInstructorCount} active Instructor
                {view().activeInstructorCount === 1 ? " is" : "s are"} currently recorded.
              </p>
            </section>
            <section aria-labelledby="course-assignments-heading">
              <h2 id="course-assignments-heading">Assignments</h2>
              <Show when={assignments.loading}>
                <p class="loading-state">Loading Assignments...</p>
              </Show>
              <Show when={assignments.error !== undefined}>
                <p class="route-error" role="alert">
                  Assignments could not be loaded.
                </p>
              </Show>
              <Show
                when={(assignments()?.length ?? 0) > 0}
                fallback={
                  <Show when={!assignments.loading && assignments.error === undefined}>
                    <p class="empty-state">No Assignments have been created for this course.</p>
                  </Show>
                }
              >
                <div class="instructor-list" aria-label="Assignments">
                  <For each={assignments()}>
                    {(assignment) => (
                      <AssignmentRow
                        courseReference={view().course.reference}
                        assignment={assignment}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </section>
            <nav
              class="course-card-actions course-instance-page__actions"
              aria-label="Course actions"
            >
              <A
                class="primary-link"
                href={`/instructor/courses/${view().course.reference}/students`}
              >
                Open Students
              </A>
              <A
                class="quiet-link"
                href={`/instructor/courses/${view().course.reference}/assignments/new`}
              >
                Create Assignment
              </A>
              <A
                class="quiet-link"
                href={`/instructor/courses/${view().course.reference}/appearance`}
              >
                Appearance
              </A>
            </nav>
          </>
        )}
      </Show>
    </section>
  );
}
