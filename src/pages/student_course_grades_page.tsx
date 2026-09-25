// Student-owned released Scores across every current Course.

import { createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { formatPointScore } from "../score_format";

function CourseScores(props: { readonly course: LiveStudentCourseLandingSummary }): JSX.Element {
  const api = useApplicationApi();
  const [assessments] = createResource(
    () => props.course,
    (course) => api.client.listLiveStudentAssessments(course.id),
  );
  const scores = (): ReadonlyArray<LiveStudentAssessmentLandingSummary> =>
    (assessments() ?? []).filter((assessment) => assessment.assessmentScore !== undefined);

  return (
    <section aria-label={`${props.course.shortName} Scores`}>
      <h2>
        {props.course.shortName}: {props.course.longName}
      </h2>
      <Show when={assessments.loading}>
        <p class="loading-state">Loading Scores...</p>
      </Show>
      <Show when={assessments.error !== undefined}>
        <p class="route-error" role="alert">
          Scores could not be loaded for this Course.
        </p>
      </Show>
      <Show when={!assessments.loading && assessments.error === undefined && scores().length === 0}>
        <p class="empty-state">No released Scores are available for this Course yet.</p>
      </Show>
      <Show when={scores().length > 0}>
        <dl>
          <For each={scores()}>
            {(assessment) => (
              <div>
                <dt>{assessment.title}</dt>
                <dd>
                  {formatPointScore(
                    assessment.assessmentScore!.pointsEarned,
                    assessment.assessmentScore!.pointsPossible,
                  )}
                </dd>
              </div>
            )}
          </For>
        </dl>
      </Show>
    </section>
  );
}

/** Shows only released Student scores and identifies the Course for each group. */
export function StudentScoresPage(): JSX.Element {
  const api = useApplicationApi();
  const [courses] = createResource(() => api.client.listLiveStudentCourses());

  return (
    <PageFrame routeSurface="studentScores" title="Scores">
      <p>Released Coursework scores from all of your current Courses.</p>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Scores unavailable</h2>
          <p>Scores could not be loaded right now.</p>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">You do not have any current Courses.</p>
      </Show>
      <For each={courses()}>{(course) => <CourseScores course={course} />}</For>
    </PageFrame>
  );
}
