// Student-owned, disclosed Question outcomes grouped by immutable revision.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type { StudentCoursePracticeStats } from "../api/student_course_practice_stats";
import { useApplicationApi } from "../api/application_api";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { parseCourseInstanceId, type CourseInstanceRouteId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { PageFrame } from "../components/page_frame";
import "./student_course_practice_stats_page.css";

function reviewPath(stats: StudentCoursePracticeStats["questions"][number]): string {
  const path = buildRoutePath("assessmentAttemptSummary", {
    assessmentAttemptId: stats.relevantAssessmentAttemptId,
  });
  if (path === undefined) throw new Error("Practice Stats requires a canonical Attempt ID.");
  return path;
}

function durationLabel(stats: StudentCoursePracticeStats["questions"][number]): string {
  const average = stats.averageDisplayDurationMs;
  if (average === null || stats.displayDurationSampleCount === 0) return "Not recorded";
  const seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(
    Math.round(average / 1_000),
  );
  return `About ${seconds} sec · ${stats.displayDurationSampleCount} measured Attempt${stats.displayDurationSampleCount === 1 ? "" : "s"}`;
}

function PracticeStatsRow(props: {
  readonly question: StudentCoursePracticeStats["questions"][number];
}): JSX.Element {
  const question = (): StudentCoursePracticeStats["questions"][number] => props.question;
  return (
    <li class="student-course-practice-stats__row">
      <div class="student-course-practice-stats__identity">
        <h3>Question {question().publishedQuestionRevisionTuple.publishedQuestionId}</h3>
        <p>Revision {question().publishedQuestionRevisionTuple.revisionNumber}</p>
      </div>
      <dl class="student-course-practice-stats__counts">
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
          <dt>Approx. average time shown</dt>
          <dd>{durationLabel(question())}</dd>
        </div>
      </dl>
      <A class="quiet-link student-course-practice-stats__action" href={reviewPath(question())}>
        Review an Attempt
      </A>
    </li>
  );
}

/** Shows only this Student's outcomes disclosed for the selected Course. */
export function StudentCoursePracticeStatsPage(): JSX.Element {
  const api = useApplicationApi();
  const params = useParams();
  const courseId = (): CourseInstanceRouteId | null =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  const [courses] = createResource(() => api.client.listLiveStudentCourses());
  const course = createMemo(() => {
    const id = courseId();
    return id === null ? undefined : courses()?.find((item) => item.id === id);
  });
  const [stats] = createResource(course, (current) =>
    api.client.getStudentCoursePracticeStats(current.id),
  );
  const unavailable = (): boolean =>
    courseId() === null || courses.error !== undefined || stats.error !== undefined;

  return (
    <PageFrame
      routeSurface="studentCoursePracticeStats"
      title={course()?.longName ?? "Practice Stats"}
    >
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={unavailable() || (!courses.loading && course() === undefined)}>
        <section class="route-error" role="alert">
          <h2>Practice Stats unavailable</h2>
          <p>Practice Stats are not available for this Course.</p>
          <A class="primary-link" href="/student">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!unavailable() && course() !== undefined}>
        <>
          <CourseEntryBanner />
          <A class="quiet-link" href="/student?choose=1">
            Your courses
          </A>
          <section
            class="student-course-practice-stats"
            aria-labelledby="student-practice-stats-heading"
          >
            <h2 id="student-practice-stats-heading">Practice Stats</h2>
            <p>
              These stats summarize your actual submitted Question outcomes across this Course's
              eligible Coursework: Regular and Practice Question Assignments, Bonus Assignments,
              Quizzes, and Exams. Outcomes appear only after both scores and per-Question
              correctness are released. Questions are grouped by the exact Published revision used
              in each Attempt. Time shown with a Question is approximate. It is recorded only while
              the Question is current in your Attempt and that browser page is visible; it does not
              measure attention.
            </p>
            <Show when={stats.loading}>
              <p class="loading-state">Loading Practice Stats...</p>
            </Show>
            <Show
              when={!stats.loading && stats.error === undefined && stats()?.questions.length === 0}
            >
              <p class="empty-state">No Question outcomes are available for this Course yet.</p>
            </Show>
            <ul class="student-course-practice-stats__list" aria-label="Question Practice Stats">
              <For each={stats()?.questions}>
                {(question) => <PracticeStatsRow question={question} />}
              </For>
            </ul>
            <Show when={stats.error !== undefined}>
              <section class="route-error" role="alert">
                <h3>Practice Stats unavailable</h3>
                <p>Practice Stats could not be loaded right now.</p>
              </section>
            </Show>
          </section>
        </>
      </Show>
    </PageFrame>
  );
}
