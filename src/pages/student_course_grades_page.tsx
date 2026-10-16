// Student-owned released Scores across every current Course.

import { createResource, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createLocalCompleteSearch } from "../features/search/local_complete_search_state";
import {
  byStudentCrossCourseTitleThenId,
  studentCrossCourseAssessmentId,
  studentCrossCourseLabel,
  studentCrossCourseScoreFields,
  type StudentCrossCourseAssessment,
} from "./student_collection_fields";
import {
  loadStudentCrossCourseCollection,
  visibleStudentScores,
} from "./student_cross_course_collection";
import { type RecordContent } from "../components/record_list/record_list";
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

function CrossCourseScores(props: {
  readonly courses: ReadonlyArray<LiveStudentCourseLandingSummary>;
}): JSX.Element {
  const api = useApplicationApi();
  const [scores, { refetch }] = createResource(
    () => props.courses,
    (courses) =>
      loadStudentCrossCourseCollection<StudentCrossCourseAssessment>(courses, async (course) =>
        visibleStudentScores(await api.client.listLiveStudentAssessments(course.id)).map(
          (assessment) => ({ course, assessment }),
        ),
      ),
  );
  const scoreSearch = createLocalCompleteSearch(
    {
      resource: scores,
      refetchSource: refetch,
      rows: (source) => byStudentCrossCourseTitleThenId(source.rows),
    },
    {
      fields: studentCrossCourseScoreFields,
      rowId: studentCrossCourseAssessmentId,
      keyword: (row) =>
        `${studentCrossCourseLabel(row.course)} ${row.assessment.title} ${row.assessment.assessmentType}`,
      content: (row) => {
        const content = scoreContent(row.assessment);
        return {
          ...content,
          details: [
            { kind: "text", label: "Course:", value: studentCrossCourseLabel(row.course) },
            ...content.details,
          ],
        };
      },
    },
  );
  const partial = (): boolean => (scores()?.failedCourses.length ?? 0) > 0;
  return (
    <section aria-label="Scores across Courses">
      <Show when={partial()}>
        <p class="route-error" role="alert">
          Some Scores could not be loaded. Results are partial. Try again for{" "}
          {scores()?.failedCourses.map(studentCrossCourseLabel).join(", ")}.
          <button type="button" onClick={() => void refetch()}>
            Retry Scores
          </button>
        </p>
      </Show>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={scoreSearch.state} textLabel="Search Scores" />}
        state={scoreSearch.state}
        collection={{
          ariaLabel: "Scores across Courses",
          fields: studentCrossCourseScoreFields,
          query: scoreSearch.query,
          get emptyState() {
            return {
              title: partial()
                ? "No matching Scores in the Courses that loaded. Results are partial."
                : scoreSearch.state.query().text || scoreSearch.state.query().filters.size
                  ? "No released Scores match your search."
                  : "No released Scores are available yet.",
            };
          },
        }}
      />
    </section>
  );
}

/** Shows only released Student scores and identifies the Course for each group. */
export function StudentScoresPage(): JSX.Element {
  const api = useApplicationApi();
  const [courses, { refetch: refetchCourses }] = createResource(() =>
    api.client.listLiveStudentCourses(),
  );

  return (
    <PageFrame routeSurface="studentScores">
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Scores unavailable</h2>
          <p>Scores could not be loaded right now.</p>
          <button type="button" onClick={() => void refetchCourses()}>
            Retry Scores
          </button>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">You do not have any current Courses.</p>
      </Show>
      <Show
        keyed
        when={
          !courses.loading && courses.error === undefined && (courses()?.length ?? 0) > 0
            ? courses()
            : undefined
        }
      >
        {(current) => <CrossCourseScores courses={current} />}
      </Show>
    </PageFrame>
  );
}
