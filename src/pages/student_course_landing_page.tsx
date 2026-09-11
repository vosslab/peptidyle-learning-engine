// Student-owned answer-free course and available assignment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssignmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { parseCourseInstanceReference } from "../navigation/public_route";
import { formatPointScore } from "../score_format";

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
    <article class="course-card">
      <h2>{props.assignment.title}</h2>
      <p>
        <strong>{progressLabel(props.assignment)}</strong>
      </p>
      <Show when={props.assignment.assignmentAttemptCompletion !== null}>
        <p>
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
  const [assignments] = createResource(course, loadAssignments);
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
