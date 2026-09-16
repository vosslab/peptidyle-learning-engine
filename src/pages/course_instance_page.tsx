// course_instance_page.tsx - Course Instance Teaching Team workspace.

import { A, useParams } from "@solidjs/router";
import { createResource, createSignal, For, Show, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { CourseClassificationEditor } from "../components/course_classification_editor";
import type { CourseAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { LiveAssessmentWorkspaceConflictError } from "../api/http_client/assessment_release";
import { parseCourseInstanceReference } from "../navigation/public_route";
import { CourseBlueprintUpdateReviewList } from "./course_blueprint_update_review";
import {
  canonicalLocalDateAndTime,
  dueDateDraft,
  dueTimeDraft,
  localDueDateAndTime,
} from "./assessment_workspace/assessment_workspace_policy_model";

import "./course_instance_page.css";

function assessmentStatusLabel(status: LiveAssessmentStatus): string {
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

function assessmentQuestionsPath(courseReference: string, assessmentReference: string): string {
  return `/instructor/courses/${courseReference}/assessments/${assessmentReference}/questions`;
}

function formatLocalDueDateAndTime(value: string | null): string {
  if (value === null) return "No due date";
  // UTC carries the server-local wall-clock fields without converting them to the browser zone.
  const neutralCarrier = new Date(`${value}Z`);
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "UTC",
  }).format(neutralCarrier);
}

function AssessmentRow(props: {
  readonly courseReference: string;
  readonly assessment: CourseAssessmentSummary;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assessment, setAssessment] = createSignal(props.assessment);
  const [state, setState] = createSignal<"idle" | "editing" | "saving" | "failed" | "read-only">(
    "idle",
  );
  const [title, setTitle] = createSignal(assessment().title);
  const [dueDate, setDueDate] = createSignal(dueDateDraft(assessment().dueAt));
  const [dueTime, setDueTime] = createSignal(dueTimeDraft(assessment().dueAt));
  const [message, setMessage] = createSignal("");
  const [needsRefresh, setNeedsRefresh] = createSignal(false);
  let titleInput: HTMLInputElement | undefined;
  let editButton: HTMLButtonElement | undefined;
  let assessmentQuestionsLink: HTMLAnchorElement | undefined;
  let readOnlyNotice: HTMLParagraphElement | undefined;

  function mayEdit(): boolean {
    return assessment().status === "unreleased" || assessment().status === "released";
  }

  function beginEditing(): void {
    const current = assessment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("editing");
    queueMicrotask(() => titleInput?.focus());
  }

  function cancelEditing(): void {
    const current = assessment();
    setTitle(current.title);
    setDueDate(dueDateDraft(current.dueAt));
    setDueTime(dueTimeDraft(current.dueAt));
    setMessage("");
    setNeedsRefresh(false);
    setState("idle");
    queueMicrotask(focusAssessmentAction);
  }

  function focusAssessmentAction(): void {
    const focusTarget = mayEdit() ? editButton : assessmentQuestionsLink;
    focusTarget?.focus();
  }

  function closeReadOnlyEditor(): void {
    setMessage("");
    setState("idle");
    queueMicrotask(focusAssessmentAction);
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
      const saved = await applicationApi.client.saveLiveAssessmentInline(
        props.courseReference,
        assessment().reference,
        { title: title(), dueAt },
        assessment().editNumber,
      );
      setAssessment(saved);
      setState("idle");
      setMessage("Assessment title and due date saved.");
      queueMicrotask(() => editButton?.focus());
    } catch (error: unknown) {
      setState("failed");
      setNeedsRefresh(error instanceof LiveAssessmentWorkspaceConflictError);
      setMessage(
        error instanceof LiveAssessmentWorkspaceConflictError
          ? "This Assessment changed elsewhere. Load the current Assessment, then review it before retrying. Your typed values remain here."
          : "The Assessment was not saved. Check the title and local due date and time, then try again. Your typed values remain here.",
      );
    }
  }

  async function refreshCurrentAssessment(): Promise<void> {
    setState("saving");
    setMessage("");
    try {
      const latest = (
        await applicationApi.client.listCourseAssessments(props.courseReference)
      ).find((candidate) => candidate.reference === assessment().reference);
      if (latest === undefined) throw new Error("Current Assessment was not returned");
      setAssessment(latest);
      setNeedsRefresh(false);
      if (!mayEdit()) {
        setState("read-only");
        setMessage(
          `This Assessment is ${assessmentStatusLabel(latest.status)} and cannot save title and due date edits. Your typed values remain available until you close this editor.`,
        );
        queueMicrotask(() => readOnlyNotice?.focus());
        return;
      }
      setState("failed");
      setMessage("Latest Assessment loaded. Review it, then save your typed values again.");
    } catch {
      setState("failed");
      setMessage(
        "The current Assessment could not load. Your typed values remain here. Try again.",
      );
    }
  }

  return (
    <article class="instructor-list__row instructor-list__row--assessment">
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">{assessmentStatusLabel(assessment().status)} Assessment</p>
        <h3>{assessment().title}</h3>
        <p class="instructor-list__metadata">Assessment {assessment().reference}</p>
        <p class="instructor-list__metadata">
          Due: {formatLocalDueDateAndTime(assessment().dueAt)} ({assessment().displayTimeZone})
        </p>
      </div>
      <div class="instructor-list__actions">
        <A
          ref={(element) => (assessmentQuestionsLink = element)}
          class="primary-link"
          href={assessmentQuestionsPath(props.courseReference, assessment().reference)}
        >
          Edit Assessment
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
          class="assessment-inline-editor"
          aria-label={`Edit ${assessment().title}`}
          aria-busy={state() === "saving"}
          onSubmit={(event) => void save(event)}
        >
          <fieldset disabled={state() === "saving"}>
            <div class="assessment-inline-editor__fields">
              <label class="assessment-inline-editor__field">
                Title
                <input
                  ref={(element) => (titleInput = element)}
                  value={title()}
                  onInput={(event) => setTitle(event.currentTarget.value)}
                  required
                />
              </label>
              <div
                class="assessment-inline-editor__schedule"
                role="group"
                aria-label="Due date and time"
              >
                <label class="assessment-inline-editor__field">
                  Due date ({assessment().displayTimeZone})
                  <input
                    type="date"
                    value={dueDate()}
                    onInput={(event) => setDueDate(event.currentTarget.value)}
                  />
                </label>
                <label class="assessment-inline-editor__field">
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
            <div class="assessment-inline-editor__actions">
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
                  onClick={() => void refreshCurrentAssessment()}
                >
                  Load current Assessment
                </button>
              </Show>
            </div>
          </fieldset>
          <Show when={message()}>
            {(value) => (
              <p
                class="assessment-inline-editor__message"
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
          class="assessment-inline-editor"
          aria-label={`Editing unavailable for ${assessment().title}`}
        >
          <p
            ref={(element) => (readOnlyNotice = element)}
            class="assessment-inline-editor__message"
            role="alert"
            tabindex="-1"
          >
            {message()}
          </p>
          <p class="assessment-inline-editor__message">Unsaved title: {title()}</p>
          <p class="assessment-inline-editor__message">
            Unsaved due date and time: {typedDueDateAndTime()}
          </p>
          <button class="quiet-action" type="button" onClick={closeReadOnlyEditor}>
            Close editor
          </button>
        </section>
      </Show>
      <Show when={state() === "idle" && message()}>
        {(value) => (
          <p class="assessment-inline-editor__message" role="status">
            {value()}
          </p>
        )}
      </Show>
    </article>
  );
}

/** Instructor Course Instance workspace for Teaching Team, roster, and Assessment delivery. */
export function CourseInstancePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseReference(): ReturnType<typeof parseCourseInstanceReference> {
    return parseCourseInstanceReference(params["courseRef"] ?? "");
  }
  const [course, { mutate: mutateCourse }] = createResource(courseReference, async (reference) =>
    applicationApi.client.getCourseInstance(reference),
  );
  const [assessments] = createResource(courseReference, async (reference) =>
    applicationApi.client.listCourseAssessments(reference),
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
            <CourseClassificationEditor
              value={view().course.classification}
              metadataEtag={view().course.metadataEtag}
              canEdit
              save={async (classification, etag) => {
                const saved = await applicationApi.client.updateCourseInstanceClassification(
                  view().course.reference,
                  classification,
                  etag,
                );
                mutateCourse({
                  ...view(),
                  course: {
                    ...view().course,
                    classification: saved.classification,
                    metadataEtag: saved.metadataEtag,
                  },
                });
              }}
              reload={async () => {
                const current = await applicationApi.client.getCourseInstance(
                  view().course.reference,
                );
                mutateCourse(current);
                return {
                  classification: current.course.classification,
                  metadataEtag: current.course.metadataEtag,
                };
              }}
            />
            <Show when={view().blueprintOrigin}>
              {(origin) => (
                <>
                  <p class="page-lede" data-blueprint-origin>
                    Adopted from Blueprint{" "}
                    <A href={`/blueprint-courses/${origin().reference}`}>{origin().reference}</A>,
                    Revision {origin().adoptedRevision}; source now Revision{" "}
                    {origin().currentRevision}.
                  </p>
                  <Show when={BigInt(origin().currentRevision) > BigInt(origin().adoptedRevision)}>
                    <p class="page-lede" data-blueprint-revision-notice>
                      Newer Blueprint Revision available
                    </p>
                    <CourseBlueprintUpdateReviewList courseReference={view().course.reference} />
                  </Show>
                </>
              )}
            </Show>
            <p class="page-lede">
              Course Term: {view().course.term.startDate} through {view().course.term.endDate}.
            </p>
            <section class="course-card" aria-labelledby="teaching-team-heading">
              <p class="card-kicker">Teaching Team</p>
              <h2 id="teaching-team-heading">Initial Teaching Team</h2>
              <p>You are an active co-Instructor for this Course Instance.</p>
              <p>
                {view().activeInstructorCount} active Instructor
                {view().activeInstructorCount === 1 ? " is" : "s are"} currently recorded.
              </p>
            </section>
            <section aria-labelledby="course-assessments-heading">
              <h2 id="course-assessments-heading">Assessments</h2>
              <Show when={assessments.loading}>
                <p class="loading-state">Loading Assessments...</p>
              </Show>
              <Show when={assessments.error !== undefined}>
                <p class="route-error" role="alert">
                  Assessments could not be loaded.
                </p>
              </Show>
              <Show
                when={(assessments()?.length ?? 0) > 0}
                fallback={
                  <Show when={!assessments.loading && assessments.error === undefined}>
                    <p class="empty-state">No Assessments have been created for this course.</p>
                  </Show>
                }
              >
                <div class="instructor-list" aria-label="Assessments">
                  <For each={assessments()}>
                    {(assessment) => (
                      <AssessmentRow
                        courseReference={view().course.reference}
                        assessment={assessment}
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
                href={`/instructor/courses/${view().course.reference}/assessments/new`}
              >
                Create Assessment
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
