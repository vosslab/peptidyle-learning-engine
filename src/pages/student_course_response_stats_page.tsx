// Student-owned, disclosed Question outcomes across every enrolled Course.

import { useNavigate } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { StudentCourseResponseStats } from "../api/student_course_response_stats";
import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import {
  assessmentAttemptPath,
  assessmentAttemptRouteState,
} from "../navigation/assessment_attempt_route";
import { assessmentAttemptRouteId } from "../navigation/public_route";
import { PageFrame } from "../components/page_frame/page_frame";
import type { RecordField } from "../components/data_sheet/record_fields";
import type { RecordContent } from "../components/data_sheet/record_content";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import { recognitionTitle } from "../features/recognition_label";
import { studentCrossCourseLabel } from "./student_collection_fields";
import { loadStudentCrossCourseCollection } from "./student_cross_course_collection";

type ResponseQuestion = StudentCourseResponseStats["questions"][number];
type ResponseRow = {
  readonly course: LiveStudentCourseLandingSummary;
  readonly question: ResponseQuestion;
};

function reviewPath(courseInstanceId: LiveStudentCourseLandingSummary["id"]): string {
  return assessmentAttemptPath("review", courseInstanceId);
}

function durationLabel(question: ResponseQuestion): string {
  const average = question.averageDisplayDurationMs;
  if (average === null || question.displayDurationSampleCount === 0) return "Not recorded";
  const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(
    Math.round(average / 1_000),
  );
  return `About ${seconds} sec · ${question.displayDurationSampleCount} measured Attempt${question.displayDurationSampleCount === 1 ? "" : "s"}`;
}

function rowId(row: ResponseRow): string {
  const revision = row.question.publishedQuestionRevisionTuple;
  return `${row.course.id}:${revision.publishedQuestionId}:${revision.revisionNumber}`;
}

const responseFields: ReadonlyArray<RecordField<ResponseRow, "question" | "course", "course">> = [
  {
    id: "question",
    label: "Question",
    role: "identity",
    value: (row) => recognitionTitle(row.question.questionTitle, "Question"),
    sortKey: "question",
    defaultColumn: { priority: 1 },
  },
  {
    id: "course",
    label: "Course",
    value: (row) => studentCrossCourseLabel(row.course),
    sortKey: "course",
    filterKey: "course",
    defaultColumn: { priority: 2 },
  },
  {
    id: "full_credit",
    label: "Full credit",
    value: (row) => row.question.fullCreditAttemptCount,
    defaultColumn: { priority: 3 },
  },
  {
    id: "not_full_credit",
    label: "Not full credit",
    value: (row) => row.question.notFullCreditCount,
    defaultColumn: { priority: 4 },
  },
];

function responseContent(
  row: ResponseRow,
  navigate: ReturnType<typeof useNavigate>,
): RecordContent {
  const question = row.question;
  return {
    title: recognitionTitle(question.questionTitle, "Question"),
    description: studentCrossCourseLabel(row.course),
    details: [
      {
        kind: "text",
        label: "Question ID and Revision",
        value: `${question.publishedQuestionRevisionTuple.publishedQuestionId}, Revision ${question.publishedQuestionRevisionTuple.revisionNumber}`,
      },
      { kind: "text", label: "Full credit", value: String(question.fullCreditAttemptCount) },
      { kind: "text", label: "Partial credit", value: String(question.partialCreditAttemptCount) },
      { kind: "text", label: "Incorrect", value: String(question.incorrectAttemptCount) },
      { kind: "text", label: "Unanswered", value: String(question.unansweredAttemptCount) },
      {
        kind: "text",
        label: "Not full credit",
        value: `${question.notFullCreditCount} / ${question.disclosedAttemptCount}`,
      },
      {
        kind: "text",
        label: "Approximate time shown with the Question",
        value: durationLabel(question),
      },
    ],
    actions: [
      {
        id: "review-attempt",
        kind: "command",
        label: "Review an Attempt",
        onClick: () =>
          navigate(reviewPath(row.course.id), {
            state: assessmentAttemptRouteState(
              assessmentAttemptRouteId(question.relevantAssessmentAttemptId),
            ),
          }),
      },
    ],
  };
}

/** Shows only this Student's disclosed saved outcomes across enrolled Courses. */
export function StudentResponseStatsPage(): JSX.Element {
  const api = useApplicationApi();
  const navigate = useNavigate();
  const [responses, { refetch }] = createResource(async () => {
    // ASVS 8.2.2: use only Courses and disclosed Question outcomes returned for this session.
    const currentCourses = await api.client.listLiveStudentCourses();
    const collection = await loadStudentCrossCourseCollection<ResponseRow>(
      currentCourses,
      async (course) => {
        const stats = await api.client.getStudentCourseResponseStats(course.id);
        return stats.questions.map((question) => ({ course, question }));
      },
    );
    return {
      ...collection,
      courseCount: currentCourses.length,
    };
  });
  const search = createLocalCompleteSearch(
    {
      rows: () => {
        const value = responses();
        return value === undefined ? undefined : value.rows;
      },
      loading: () => responses.loading,
      error: (): unknown => responses.error,
      refetchSource: refetch,
    },
    {
      fields: responseFields,
      rowId,
      keyword: (row) =>
        `${studentCrossCourseLabel(row.course)} ${row.question.questionTitle} ${row.question.publishedQuestionRevisionTuple.publishedQuestionId}`,
      content: (row) => responseContent(row, navigate),
    },
  );

  return (
    <PageFrame routeSurface="studentResponseStats">
      <Show when={(responses()?.failedCourses.length ?? 0) > 0}>
        <p class="route-error" role="alert">
          Some Response Stats could not be loaded. Results are partial. Try again for{" "}
          {responses()?.failedCourses.map(studentCrossCourseLabel).join(", ")}.
          <button type="button" onClick={() => void refetch()}>
            Retry Response Stats
          </button>
        </p>
      </Show>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={search.state} textLabel="Search Response Stats" />}
        state={search.state}
        collection={{
          ariaLabel: "Response Stats across Courses",
          get emptyState() {
            return {
              title:
                responses()?.courseCount === 0
                  ? "You are not enrolled in any Courses."
                  : (responses()?.failedCourses.length ?? 0) > 0
                    ? "No matching Question outcomes in the Courses that loaded. Results are partial."
                    : "No Question outcomes match your search.",
            };
          },
          fields: responseFields,
          query: search.query,
        }}
      />
      <p>Time shown is approximate; it does not measure attention or effort or affect grades.</p>
    </PageFrame>
  );
}
