// Student-owned answer-free course and available assignment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssignmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { StudentAssignmentDecisionDetails } from "../components/student_assignment_presentation";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { parseCourseInstanceReference } from "../navigation/public_route";
import { formatPointScore } from "../score_format";
import { applyStudentDisplayTimeZone } from "./student_time_zone_model";

function availableTimeZones(current: string): readonly string[] {
  const intl = Intl as typeof Intl & {
    readonly supportedValuesOf?: (key: "timeZone") => readonly string[];
  };
  const supported = intl.supportedValuesOf?.("timeZone") ?? [];
  return [...new Set([...supported, "UTC", current])].sort((left, right) =>
    left.localeCompare(right),
  );
}

function progressLabel(assignment: LiveStudentAssignmentLandingSummary): string {
  if (assignment.assignmentAttemptCompletion === "completed") return "Completed and scored";
  if (assignment.assignmentAttemptCompletion === "inProgress") return "In progress";
  return "Not started";
}

function AssignmentCard(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assignment: LiveStudentAssignmentLandingSummary;
}): JSX.Element {
  return (
    <article class="course-card student-assignment-card">
      <h2>{props.assignment.title}</h2>
      <p class="student-assignment-card__progress">
        <strong>{progressLabel(props.assignment)}</strong>
      </p>
      <Show when={props.assignment.assignmentAttemptCompletion !== null}>
        <p class="student-assignment-card__grade">
          {props.assignment.gradedQuestionCount} of {props.assignment.questionCount} questions
          graded
          <Show when={props.assignment.score}>
            {(score) => (
              <>
                {" · "}
                {props.assignment.assignmentAttemptCompletion === "completed"
                  ? "Score"
                  : "Score so far"}{" "}
                {formatPointScore(score().pointsEarned, score().pointsPossible)}
              </>
            )}
          </Show>
        </p>
      </Show>
      <section class="student-assignment-card__decision" aria-label="Assignment access and timing">
        <h3>Before you start</h3>
        <StudentAssignmentDecisionDetails decision={props.assignment.decision} />
      </section>
      <A
        class="primary-link"
        href={`/courses/${props.course.reference}/assignments/${props.assignment.reference}`}
      >
        Open Assignment
      </A>
    </article>
  );
}

/** Student-owned entry point for answer-free current Course work. */
export function StudentCourseLandingPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseReference(): ReturnType<typeof parseCourseInstanceReference> {
    return parseCourseInstanceReference(params["courseRef"] ?? "");
  }
  async function loadCourses(): Promise<ReadonlyArray<LiveStudentCourseLandingSummary>> {
    return applicationApi.client.listLiveStudentCourses();
  }
  const [courses] = createResource(loadCourses);
  const course = createMemo(() => {
    const reference = courseReference();
    if (reference === null) return undefined;
    return courses()?.find((candidate) => candidate.reference === reference);
  });
  // ASVS V2.2.2/2.3.1: the server projects Student membership; this view makes no access decision.
  async function loadAssignments(
    current: LiveStudentCourseLandingSummary,
  ): Promise<ReadonlyArray<LiveStudentAssignmentLandingSummary>> {
    return applicationApi.client.listLiveStudentAssignments(current.reference);
  }
  const [assignments, { mutate: mutateAssignments }] = createResource(course, loadAssignments);
  const [timeZoneProfile, { mutate: mutateTimeZoneProfile }] = createResource(course, async () =>
    applicationApi.client.getStudentTimeZoneProfile(),
  );
  const [timeZoneDraft, setTimeZoneDraft] = createSignal<string>();
  const [timeZoneSaving, setTimeZoneSaving] = createSignal(false);
  const [timeZoneMessage, setTimeZoneMessage] = createSignal("");
  const selectedTimeZone = (): string => timeZoneDraft() ?? timeZoneProfile()?.timeZone ?? "UTC";

  async function saveTimeZone(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (timeZoneSaving()) return;
    setTimeZoneSaving(true);
    setTimeZoneMessage("");
    try {
      const saved = await applicationApi.client.updateStudentTimeZone({
        timeZone: selectedTimeZone(),
      });
      mutateTimeZoneProfile(saved);
      setTimeZoneDraft(saved.timeZone);
      mutateAssignments((current) =>
        current === undefined ? current : applyStudentDisplayTimeZone(current, saved.timeZone),
      );
      setTimeZoneMessage("Your time zone was saved.");
    } catch (_error: unknown) {
      setTimeZoneMessage("Your time zone could not be saved. Try again.");
    } finally {
      setTimeZoneSaving(false);
    }
  }
  function unavailable(): boolean {
    return (
      courseReference() === null || courses.error !== undefined || assignments.error !== undefined
    );
  }

  return (
    <section class="page" data-route-surface="studentCourseLanding">
      <Show when={courses.loading}>
        <p class="loading-state">Loading assigned work...</p>
      </Show>
      <Show when={unavailable()}>
        <section class="route-error" role="alert">
          <h1>Assigned work unavailable</h1>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!courses.loading && !unavailable() && course() === undefined}>
        <section class="route-error" role="alert">
          <h1>Assigned work unavailable</h1>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryIdentity />
            <A class="quiet-link" href="/?choose=1">
              Your courses
            </A>
            <section class="student-time-zone-control" aria-labelledby="student-time-zone-heading">
              <div>
                <h2 id="student-time-zone-heading">Your time zone</h2>
                <p>
                  This changes how dates and times are shown. It does not change Assignment
                  deadlines.
                </p>
              </div>
              <Show when={timeZoneProfile.loading}>
                <p class="calm-status" role="status">
                  Loading your time zone...
                </p>
              </Show>
              <Show when={timeZoneProfile.error !== undefined}>
                <p role="alert">Your time zone is unavailable. Refresh to try again.</p>
              </Show>
              <Show when={timeZoneProfile()}>
                <form aria-busy={timeZoneSaving()} onSubmit={(event) => void saveTimeZone(event)}>
                  <label for="student-time-zone">
                    Time zone
                    <select
                      id="student-time-zone"
                      value={selectedTimeZone()}
                      disabled={timeZoneSaving()}
                      onInput={(event) => setTimeZoneDraft(event.currentTarget.value)}
                    >
                      <For each={availableTimeZones(selectedTimeZone())}>
                        {(timeZone) => <option value={timeZone}>{timeZone}</option>}
                      </For>
                    </select>
                  </label>
                  <button class="primary-action" type="submit" disabled={timeZoneSaving()}>
                    {timeZoneSaving() ? "Saving..." : "Save time zone"}
                  </button>
                </form>
              </Show>
              <Show when={timeZoneMessage()}>
                {(message) => (
                  <p role="status" aria-live="polite">
                    {message()}
                  </p>
                )}
              </Show>
            </section>
            <h2>Assignments</h2>
            <Show when={assignments.loading}>
              <p class="loading-state">Loading assignments...</p>
            </Show>
            <Show when={!assignments.loading && (assignments()?.length ?? 0) === 0}>
              <p class="empty-state">No assignments are available right now.</p>
            </Show>
            <Show when={(assignments()?.length ?? 0) > 0}>
              <div class="card-grid">
                <For each={assignments()}>
                  {(assignment) => <AssignmentCard course={current()} assignment={assignment} />}
                </For>
              </div>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
