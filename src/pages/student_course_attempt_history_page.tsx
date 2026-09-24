// Student-owned, cursor-paginated history for every Attempt in one Course.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { StudentCourseAttemptHistoryEntry } from "../api/student_course_attempt_history";
import { useApplicationApi } from "../api/application_api";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { parseCourseInstanceId, type CourseInstanceRouteId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { PageFrame } from "../components/page_frame";
import "./student_course_attempt_history_page.css";

const localDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

function attemptPath(attempt: StudentCourseAttemptHistoryEntry): string {
  const route =
    attempt.submittedAt === undefined ? "assessmentAttempt" : "assessmentAttemptSummary";
  const path = buildRoutePath(route, { assessmentAttemptId: attempt.assessmentAttemptId });
  if (path === undefined) throw new Error("Attempt History requires a canonical Attempt ID.");
  return path;
}

function scoreLabel(attempt: StudentCourseAttemptHistoryEntry): string | undefined {
  const score = attempt.assessmentScore;
  if (score === undefined) return undefined;
  const earned = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(
    score.pointsEarned,
  );
  const possible = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(
    score.pointsPossible,
  );
  return `Score: ${earned} of ${possible} points`;
}

function HistoryRow(props: { readonly attempt: StudentCourseAttemptHistoryEntry }): JSX.Element {
  const path = (): string => attemptPath(props.attempt);
  return (
    <li class="student-course-attempt-history__row">
      <div class="student-course-attempt-history__identity">
        <h3>{props.attempt.assessmentTitle}</h3>
        <p>
          Attempt {props.attempt.assessmentAttemptNumber} ·{" "}
          {props.attempt.submittedAt === undefined ? "In progress" : "Submitted"}
        </p>
        <p>
          Started{" "}
          <time dateTime={new Date(props.attempt.startedAt).toISOString()}>
            {localDateTimeFormatter.format(props.attempt.startedAt)}
          </time>
        </p>
        <Show when={props.attempt.submittedAt !== undefined}>
          <p>
            Submitted{" "}
            <time dateTime={new Date(props.attempt.submittedAt!).toISOString()}>
              {localDateTimeFormatter.format(props.attempt.submittedAt)}
            </time>
          </p>
        </Show>
      </div>
      <div class="student-course-attempt-history__score">
        <Show when={props.attempt.submittedAt !== undefined} fallback={<span>No score yet</span>}>
          <Show when={scoreLabel(props.attempt)} fallback={<span>Score not released</span>}>
            {(label) => <span>{label()}</span>}
          </Show>
        </Show>
      </div>
      <A class="quiet-link" href={path()}>
        {props.attempt.submittedAt === undefined ? "Open Attempt" : "Review Attempt"}
      </A>
    </li>
  );
}

/** Lists every self-owned Course Attempt without truncating older history. */
export function StudentCourseAttemptHistoryPage(): JSX.Element {
  const api = useApplicationApi();
  const params = useParams();
  const courseId = (): CourseInstanceRouteId | null =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  const [courses] = createResource(() => api.client.listLiveStudentCourses());
  const course = createMemo(() => {
    const id = courseId();
    return id === null ? undefined : courses()?.find((item) => item.id === id);
  });
  const [cursor, setCursor] = createSignal<string | undefined>(undefined);
  const [priorCursors, setPriorCursors] = createSignal<Array<string | undefined>>([]);
  const [page] = createResource(
    () => {
      const selected = course();
      return selected === undefined ? undefined : { id: selected.id, cursor: cursor() };
    },
    ({ id, cursor: after }) => api.client.listStudentCourseAttemptHistory(id, 50, after),
  );
  const unavailable = (): boolean =>
    courseId() === null || courses.error !== undefined || page.error !== undefined;

  return (
    <PageFrame
      routeSurface="studentCourseAttemptHistory"
      title={course()?.longName ?? "Attempt History"}
    >
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={unavailable() || (!courses.loading && course() === undefined)}>
        <section class="route-error" role="alert">
          <h2>Attempt History unavailable</h2>
          <p>This Course Attempt History is not available.</p>
          <A class="primary-link" href="/student">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!unavailable() && course() !== undefined}>
        <>
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student?choose=1">
              Your courses
            </A>
            <section
              class="student-course-attempt-history"
              aria-labelledby="student-attempt-history-heading"
            >
              <h2 id="student-attempt-history-heading">Attempt History</h2>
              <p>Attempts are listed newest first. Scores appear when they have been released.</p>
              <Show when={page.loading}>
                <p class="loading-state">Loading Attempt History...</p>
              </Show>
              <Show when={!page.loading && page.error === undefined && page()?.items.length === 0}>
                <p class="empty-state">No Attempts have been started in this Course.</p>
              </Show>
              <Show when={!page.loading && page.error === undefined && page()?.items.length !== 0}>
                <ul class="student-course-attempt-history__list" aria-label="Course Attempts">
                  <For each={page()?.items}>{(attempt) => <HistoryRow attempt={attempt} />}</For>
                </ul>
                <nav
                  class="student-course-attempt-history__pagination"
                  aria-label="Attempt History pages"
                >
                  <button
                    type="button"
                    class="quiet-button"
                    disabled={priorCursors().length === 0 || page.loading}
                    onClick={() => {
                      const trail = [...priorCursors()];
                      const previous = trail.pop();
                      setPriorCursors(trail);
                      setCursor(previous);
                    }}
                  >
                    Newer Attempts
                  </button>
                  <button
                    type="button"
                    class="quiet-button"
                    disabled={page()?.nextCursor === undefined || page.loading}
                    onClick={() => {
                      const next = page()?.nextCursor;
                      if (next === undefined) return;
                      setPriorCursors([...priorCursors(), cursor()]);
                      setCursor(next);
                    }}
                  >
                    Older Attempts
                  </button>
                </nav>
              </Show>
              <Show when={page.error !== undefined}>
                <section class="route-error" role="alert">
                  <h3>Attempt History unavailable</h3>
                  <p>Attempt History could not be loaded right now.</p>
                </section>
              </Show>
            </section>
          </>
        </>
      </Show>
    </PageFrame>
  );
}
