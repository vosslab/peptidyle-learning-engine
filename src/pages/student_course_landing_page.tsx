// Student-owned answer-free course and available assessment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { useApplicationApi } from "../api/application_api";
import { StudentAssessmentDecisionDetails } from "../components/student_assessment_presentation";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { parseCourseInstanceId } from "../navigation/public_route";
import { RibbonIcon } from "../ribbon/ribbon_icon";
import { formatPointScore } from "../score_format";
import { studentCourseworkDisplay } from "./student_coursework_presentation";
import "./student_course_landing_page.css";

function AssessmentCard(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessment: LiveStudentAssessmentLandingSummary;
}): JSX.Element {
  function display(): ReturnType<typeof studentCourseworkDisplay> {
    return studentCourseworkDisplay(
      props.assessment.decision.startDecision,
      props.assessment.assessmentAttemptCompletion,
      props.assessment.canResumeAssessmentAttempt,
    );
  }
  function typePresentation(): ReturnType<typeof assessmentTypePresentation> {
    return assessmentTypePresentation(props.assessment.assessmentType);
  }
  return (
    <article
      class={`course-card student-coursework-card student-coursework-card--${display().state}`}
      data-coursework-state={display().state}
    >
      <p class="student-coursework-card__state">
        <strong>{display().stateLabel}</strong>
      </p>
      <h3>{props.assessment.title}</h3>
      <dl class="student-coursework-card__facts">
        <div>
          <dt>Type</dt>
          <dd
            class="student-coursework-card__type"
            style={`--student-coursework-type-color: var(${typePresentation().colorToken})`}
          >
            <RibbonIcon glyph={typePresentation().icon} />
            <span>{typePresentation().label}</span>
          </dd>
        </div>
        <div>
          <dt>Completion</dt>
          <dd>{display().completionLabel}</dd>
        </div>
      </dl>
      <Show when={props.assessment.assessmentAttemptCompletion === "inProgress"}>
        <p class="student-coursework-card__progress">
          {props.assessment.savedQuestionCount} of {props.assessment.questionCount} responses saved
        </p>
      </Show>
      <Show when={props.assessment.assessmentAttemptCompletion === "completed"}>
        <p class="student-coursework-card__grade">
          {props.assessment.gradedQuestionCount} of {props.assessment.questionCount} questions
          graded
          <Show when={props.assessment.assessmentScore}>
            {(score) => (
              <>
                {" · "}
                Assessment score {formatPointScore(score().pointsEarned, score().pointsPossible)}
              </>
            )}
          </Show>
        </p>
      </Show>
      <section class="student-coursework-card__decision" aria-label="Coursework access and timing">
        <StudentAssessmentDecisionDetails decision={props.assessment.decision} />
      </section>
      <A
        class="primary-link"
        href={`/courses/${props.course.id}/assessments/${props.assessment.id}`}
      >
        {display().actionVerb} {typePresentation().label}
      </A>
    </article>
  );
}

/** Student-owned entry point for answer-free current Course work. */
export function StudentCourseLandingPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseReference(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseRef"] ?? "");
  }
  async function loadCourses(): Promise<ReadonlyArray<LiveStudentCourseLandingSummary>> {
    return applicationApi.client.listLiveStudentCourses();
  }
  const [courses] = createResource(loadCourses);
  const course = createMemo(() => {
    const reference = courseReference();
    if (reference === null) return undefined;
    return courses()?.find((candidate) => candidate.id === reference);
  });
  // ASVS V2.2.2/2.3.1: the server projects Student membership; this view makes no access decision.
  async function loadAssessments(
    current: LiveStudentCourseLandingSummary,
  ): Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>> {
    return applicationApi.client.listLiveStudentAssessments(current.id);
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
            <A class="quiet-link" href="/student?choose=1">
              Your courses
            </A>
            <h2>Coursework</h2>
            <Show when={assessments.loading}>
              <p class="loading-state">Loading Coursework...</p>
            </Show>
            <Show when={!assessments.loading && (assessments()?.length ?? 0) === 0}>
              <p class="empty-state">No Coursework is available right now.</p>
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
