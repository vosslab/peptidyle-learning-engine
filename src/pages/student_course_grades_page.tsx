// Student-owned self-only Course grade summary.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { PageFrame } from "../components/page_frame";
import { parseCourseInstanceId } from "../navigation/public_route";
import { formatPointScore } from "../score_format";

/** Student-owned score and graded-question view for one current Course. */
export function StudentCourseGradesPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseInstanceId(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseInstanceId"] ?? "");
  }
  async function loadCourses(): Promise<ReadonlyArray<LiveStudentCourseLandingSummary>> {
    return applicationApi.client.listLiveStudentCourses();
  }
  const [courses] = createResource(loadCourses);
  const course = createMemo(() => {
    const id = courseInstanceId();
    if (id === null) return undefined;
    return courses()?.find((candidate) => candidate.id === id);
  });
  async function loadAssessments(
    current: LiveStudentCourseLandingSummary,
  ): Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>> {
    return applicationApi.client.listLiveStudentAssessments(current.id);
  }
  const [assessments] = createResource(course, loadAssessments);
  function unavailable(): boolean {
    return (
      courseInstanceId() === null || courses.error !== undefined || assessments.error !== undefined
    );
  }

  return (
    <PageFrame routeSurface="studentCourseGrades" title={course()?.longName ?? "Grades"}>
      <Show when={!courses.loading && !unavailable() && course() !== undefined}>
        <>
          <CourseEntryBanner />
          <A class="quiet-link" href={`/student/courses/${course()!.id}`}>
            Coursework
          </A>
          <h2>Grades</h2>
          <Show when={assessments.loading}>
            <p class="loading-state">Loading grades...</p>
          </Show>
          <Show when={!assessments.loading && (assessments()?.length ?? 0) === 0}>
            <p class="empty-state">No Coursework has been graded yet.</p>
          </Show>
          <Show when={(assessments()?.length ?? 0) > 0}>
            <dl>
              <For each={assessments()}>
                {(assessment) => (
                  <div>
                    <dt>{assessment.title}</dt>
                    <dd>
                      {assessment.gradedQuestionCount} of {assessment.questionCount} questions
                      graded
                      <Show when={assessment.assessmentScore}>
                        {(score) => (
                          <>
                            {" · "}
                            {formatPointScore(score().pointsEarned, score().pointsPossible)}
                          </>
                        )}
                      </Show>
                    </dd>
                  </div>
                )}
              </For>
            </dl>
          </Show>
        </>
      </Show>
      <Show when={courses.loading}>
        <p class="loading-state">Loading grades...</p>
      </Show>
      <Show when={unavailable() || (!courses.loading && course() === undefined)}>
        <section class="route-error" role="alert">
          <h2>Grades unavailable</h2>
          <p>This Course is not available.</p>
          <A class="primary-link" href="/student">
            Return to courses
          </A>
        </section>
      </Show>
    </PageFrame>
  );
}
