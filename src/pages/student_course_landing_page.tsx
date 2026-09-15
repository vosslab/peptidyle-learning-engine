// Student-owned answer-free course and available assessment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { StudentAssessmentDecisionDetails } from "../components/student_assessment_presentation";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { parseCourseInstanceReference } from "../navigation/public_route";
import { formatPointScore } from "../score_format";

function progressLabel(assessment: LiveStudentAssessmentLandingSummary): string {
  if (assessment.assessmentAttemptCompletion === "completed") return "Completed and scored";
  if (assessment.assessmentAttemptCompletion === "inProgress") return "In progress";
  return "Not started";
}

function AssessmentCard(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessment: LiveStudentAssessmentLandingSummary;
}): JSX.Element {
  return (
    <article class="course-card student-assessment-card">
      <h2>{props.assessment.title}</h2>
      <p class="student-assessment-card__progress">
        <strong>{progressLabel(props.assessment)}</strong>
      </p>
      <Show when={props.assessment.assessmentAttemptCompletion !== null}>
        <p class="student-assessment-card__grade">
          {props.assessment.gradedQuestionCount} of {props.assessment.questionCount} questions
          graded
          <Show when={props.assessment.score}>
            {(score) => (
              <>
                {" · "}
                {props.assessment.assessmentAttemptCompletion === "completed"
                  ? "Score"
                  : "Score so far"}{" "}
                {formatPointScore(score().pointsEarned, score().pointsPossible)}
              </>
            )}
          </Show>
        </p>
      </Show>
      <section class="student-assessment-card__decision" aria-label="Assessment access and timing">
        <h3>Before you start</h3>
        <StudentAssessmentDecisionDetails decision={props.assessment.decision} />
      </section>
      <A
        class="primary-link"
        href={`/courses/${props.course.reference}/assessments/${props.assessment.reference}`}
      >
        Open Assessment
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
  async function loadAssessments(
    current: LiveStudentCourseLandingSummary,
  ): Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>> {
    return applicationApi.client.listLiveStudentAssessments(current.reference);
  }
  const [assessments] = createResource(course, loadAssessments);
  function unavailable(): boolean {
    return (
      courseReference() === null || courses.error !== undefined || assessments.error !== undefined
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
            <h2>Assessments</h2>
            <Show when={assessments.loading}>
              <p class="loading-state">Loading assessments...</p>
            </Show>
            <Show when={!assessments.loading && (assessments()?.length ?? 0) === 0}>
              <p class="empty-state">No assessments are available right now.</p>
            </Show>
            <Show when={(assessments()?.length ?? 0) > 0}>
              <div class="card-grid">
                <For each={assessments()}>
                  {(assessment) => <AssessmentCard course={current()} assessment={assessment} />}
                </For>
              </div>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
