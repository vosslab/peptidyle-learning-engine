// Student-owned, disclosed Question outcomes across every enrolled Course.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { StudentCourseResponseStats } from "../api/student_course_response_stats";
import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { PageFrame } from "../components/page_frame";
import { RecordDetailList } from "../components/record_list/record_detail_list";
import type { RecordCollectionState } from "../components/record_list/record_collection_state";
import "./student_course_response_stats_page.css";

function reviewPath(stats: StudentCourseResponseStats["questions"][number]): string {
  const path = buildRoutePath("assessmentAttemptSummary", {
    assessmentAttemptId: stats.relevantAssessmentAttemptId,
  });
  if (path === undefined) throw new Error("Response Stats requires a canonical Attempt ID.");
  return path;
}

function durationLabel(stats: StudentCourseResponseStats["questions"][number]): string {
  const average = stats.averageDisplayDurationMs;
  if (average === null || stats.displayDurationSampleCount === 0) return "Not recorded";
  const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(
    Math.round(average / 1_000),
  );
  return `About ${seconds} sec · ${stats.displayDurationSampleCount} measured Attempt${stats.displayDurationSampleCount === 1 ? "" : "s"}`;
}

function ResponseStatsRecord(props: {
  readonly question: StudentCourseResponseStats["questions"][number];
}): JSX.Element {
  const question = (): StudentCourseResponseStats["questions"][number] => props.question;
  return (
    <div class="student-course-response-stats__row">
      <div class="student-course-response-stats__identity">
        <h3>Question {question().publishedQuestionRevisionTuple.publishedQuestionId}</h3>
        <p>Version {question().publishedQuestionRevisionTuple.revisionNumber}</p>
      </div>
      <dl class="student-course-response-stats__counts">
        <div>
          <dt>Full credit</dt>
          <dd>{question().fullCreditAttemptCount}</dd>
        </div>
        <div>
          <dt>Partial credit</dt>
          <dd>{question().partialCreditAttemptCount}</dd>
        </div>
        <div>
          <dt>Incorrect</dt>
          <dd>{question().incorrectAttemptCount}</dd>
        </div>
        <div>
          <dt>Unanswered</dt>
          <dd>{question().unansweredAttemptCount}</dd>
        </div>
        <div>
          <dt>Not full credit</dt>
          <dd>
            {question().notFullCreditCount} / {question().disclosedAttemptCount}
          </dd>
        </div>
        <div>
          <dt>Approx. average time shown with a Question</dt>
          <dd>{durationLabel(question())}</dd>
        </div>
      </dl>
      <A class="quiet-link student-course-response-stats__action" href={reviewPath(question())}>
        Review an Attempt
      </A>
    </div>
  );
}

function responseStatsListState(loading: boolean, unavailable: boolean): RecordCollectionState {
  if (loading) return { kind: "loading", label: "Loading Response Stats..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Response Stats unavailable",
      message: "Results for this Course could not be loaded right now.",
    };
  }
  return { kind: "ready" };
}

function CourseResponseStats(props: {
  readonly course: LiveStudentCourseLandingSummary;
}): JSX.Element {
  const api = useApplicationApi();
  const [stats] = createResource(props.course, (course) =>
    api.client.getStudentCourseResponseStats(course.id),
  );

  return (
    <section
      class="student-course-response-stats"
      aria-label={`${props.course.shortName} Response Stats`}
    >
      <h2>
        {props.course.shortName}: {props.course.longName}
      </h2>
      <RecordDetailList
        ariaLabel={`${props.course.shortName} Question outcomes`}
        emptyState={{ title: "No Question outcomes are available for this Course yet." }}
        recordId={(question) =>
          `${question.publishedQuestionRevisionTuple.publishedQuestionId}:${question.publishedQuestionRevisionTuple.revisionNumber}`
        }
        renderRecord={(question) => <ResponseStatsRecord question={question} />}
        rows={stats()?.questions ?? []}
        state={responseStatsListState(stats.loading, stats.error !== undefined)}
      />
    </section>
  );
}

/** Shows only this Student's disclosed saved outcomes, grouped by Course. */
export function StudentResponseStatsPage(): JSX.Element {
  const api = useApplicationApi();
  const [courses] = createResource(() => api.client.listLiveStudentCourses());

  return (
    <PageFrame routeSurface="studentResponseStats" title="Response Stats">
      <p>
        These stats summarize your submitted Question outcomes from eligible Coursework. Counts
        appear only when their scores and Question feedback are released. Time shown with a Question
        is approximate, is recorded only while the Question is current and the browser page is
        visible, and does not measure attention.
      </p>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Response Stats unavailable</h2>
          <p>Your Course list could not be loaded right now.</p>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">You are not enrolled in any Courses.</p>
      </Show>
      <For each={courses()}>{(course) => <CourseResponseStats course={course} />}</For>
    </PageFrame>
  );
}
