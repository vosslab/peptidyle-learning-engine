// Student-owned released Scores across every current Course.

import { createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame, PageSection } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { formatPointScore } from "../score_format";

function scoreContent(assessment: LiveStudentAssessmentLandingSummary): RecordContent {
  const score = assessment.assessmentScore;
  if (score === undefined) {
    throw new Error("Student Scores only renders released Assessment scores.");
  }
  return {
    title: assessment.title,
    details: [
      { kind: "assessmentType", value: assessment.assessmentType },
      {
        kind: "text",
        label: "Score",
        value: formatPointScore(score.pointsEarned, score.pointsPossible),
      },
    ],
    actions: [],
  };
}

function scoreListState(loading: boolean, unavailable: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading Scores..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Scores unavailable",
      message: "Scores could not be loaded for this Course.",
    };
  }
  return { kind: "ready" };
}

function CourseScores(props: { readonly course: LiveStudentCourseLandingSummary }): JSX.Element {
  const api = useApplicationApi();
  const [assessments] = createResource(
    () => props.course,
    (course) => api.client.listLiveStudentAssessments(course.id),
  );
  const scores = (): ReadonlyArray<LiveStudentAssessmentLandingSummary> =>
    (assessments() ?? []).filter((assessment) => assessment.assessmentScore !== undefined);

  const headingId = `course-scores-${props.course.id}`;
  return (
    <PageSection
      headingId={headingId}
      heading={
        <>
          {props.course.shortName}: {props.course.longName}
        </>
      }
    >
      <RecordList
        ariaLabel={`${props.course.shortName} Scores`}
        emptyState={{ title: "No released Scores are available for this Course yet." }}
        recordId={(assessment) => assessment.id}
        content={scoreContent}
        rows={scores()}
        state={scoreListState(assessments.loading, assessments.error !== undefined)}
      />
    </PageSection>
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
