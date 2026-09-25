// Student-owned, cursor-paginated history across every enrolled Course.

import { A } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { StudentCourseAttemptHistoryEntry } from "../api/student_course_attempt_history";
import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { PageFrame } from "../components/page_frame";
import "./student_course_attempt_history_page.css";

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

function HistoryRow(props: {
  readonly attempt: StudentCourseAttemptHistoryEntry;
  readonly formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined;
}): JSX.Element {
  const path = (): string => attemptPath(props.attempt);
  return (
    <li class="student-course-attempt-history__row">
      <div class="student-course-attempt-history__identity">
        <h3>{props.attempt.assessmentTitle}</h3>
        <p>
          Attempt {props.attempt.assessmentAttemptNumber} ·{" "}
          {props.attempt.submittedAt === undefined ? "In progress" : "Submitted"}
        </p>
        <Show when={props.formatDateTime()}>
          {(formatDateTime) => (
            <p>
              Started{" "}
              <time dateTime={new Date(props.attempt.startedAt).toISOString()}>
                {formatDateTime()(props.attempt.startedAt)}
              </time>
            </p>
          )}
        </Show>
        <Show when={props.attempt.submittedAt !== undefined}>
          <Show when={props.formatDateTime()}>
            {(formatDateTime) => (
              <p>
                Submitted{" "}
                <time dateTime={new Date(props.attempt.submittedAt!).toISOString()}>
                  {formatDateTime()(props.attempt.submittedAt!)}
                </time>
              </p>
            )}
          </Show>
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

function CourseAttemptHistory(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined;
}): JSX.Element {
  const api = useApplicationApi();
  const [cursor, setCursor] = createSignal<string | undefined>(undefined);
  const [priorCursors, setPriorCursors] = createSignal<Array<string | undefined>>([]);
  const [page] = createResource(
    () => ({ courseInstanceId: props.course.id, cursor: cursor() }),
    ({ courseInstanceId, cursor: after }) =>
      api.client.listStudentCourseAttemptHistory(courseInstanceId, 50, after),
  );

  return (
    <section
      class="student-course-attempt-history"
      aria-label={`${props.course.shortName} Attempt History`}
    >
      <h2>
        {props.course.shortName}: {props.course.longName}
      </h2>
      <p>
        Attempts in this Course are listed newest first. Scores appear when they have been released.
      </p>
      <Show when={page.loading}>
        <p class="loading-state">Loading Attempts...</p>
      </Show>
      <Show when={!page.loading && page.error === undefined && page()?.items.length === 0}>
        <p class="empty-state">No Attempts have been started in this Course.</p>
      </Show>
      <Show when={!page.loading && page.error === undefined && page()?.items.length !== 0}>
        <ul
          class="student-course-attempt-history__list"
          aria-label={`${props.course.shortName} Attempts`}
        >
          <For each={page()?.items}>
            {(attempt) => <HistoryRow attempt={attempt} formatDateTime={props.formatDateTime} />}
          </For>
        </ul>
        <nav
          class="student-course-attempt-history__pagination"
          aria-label={`${props.course.shortName} Attempt History pages`}
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
          <p>Attempts for this Course could not be loaded right now.</p>
        </section>
      </Show>
    </section>
  );
}

/** Lists the authenticated Student's history with independent pagination per Course. */
export function StudentAttemptHistoryPage(): JSX.Element {
  const api = useApplicationApi();
  const [courses] = createResource(() => api.client.listLiveStudentCourses());
  const [accountSettings] = createResource(() => api.client.getAccountSettings());
  const formatDateTime = createMemo(() => {
    const timeZone = accountSettings()?.timeZone;
    return timeZone === undefined ? undefined : createDisplayDateTimeFormatter(timeZone);
  });

  return (
    <PageFrame routeSurface="studentAttemptHistory" title="Attempt History">
      <p>
        Attempts are grouped by Course. Each Course has its own newest-first list and page controls.
      </p>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Courses...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Attempt History unavailable</h2>
          <p>Your Course list could not be loaded right now.</p>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">You are not enrolled in any Courses.</p>
      </Show>
      <For each={courses()}>
        {(course) => <CourseAttemptHistory course={course} formatDateTime={formatDateTime} />}
      </For>
    </PageFrame>
  );
}
