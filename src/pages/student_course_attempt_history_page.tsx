// Student-owned, cursor-paginated history across every enrolled Course.

import { A } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { StudentCourseAttemptHistoryEntry } from "../api/student_course_attempt_history";
import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
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

function attemptHistoryRegions(
  formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined,
): ReadonlyArray<RecordRegion<StudentCourseAttemptHistoryEntry>> {
  return [
    {
      id: "attempt",
      role: "identity",
      priority: "required",
      width: "minmax(0, 1fr)",
      align: "start",
      content: (attempt): JSX.Element => (
        <div class="student-course-attempt-history__identity">
          <h3>{attempt.assessmentTitle}</h3>
          <p>
            Attempt {attempt.assessmentAttemptNumber} ·{" "}
            {attempt.submittedAt === undefined ? "In progress" : "Submitted"}
          </p>
          <Show when={formatDateTime()}>
            {(format) => (
              <p>
                Started{" "}
                <time dateTime={new Date(attempt.startedAt).toISOString()}>
                  {format()(attempt.startedAt)}
                </time>
              </p>
            )}
          </Show>
          <Show when={attempt.submittedAt !== undefined && formatDateTime()}>
            {(format) => (
              <p>
                Submitted{" "}
                <time dateTime={new Date(attempt.submittedAt!).toISOString()}>
                  {format()(attempt.submittedAt!)}
                </time>
              </p>
            )}
          </Show>
        </div>
      ),
    },
    {
      id: "score",
      role: "status",
      priority: "high",
      width: "minmax(9rem, auto)",
      align: "start",
      content: (attempt): JSX.Element => (
        <div class="student-course-attempt-history__score">
          <Show when={attempt.submittedAt !== undefined} fallback={<span>No score yet</span>}>
            <Show when={scoreLabel(attempt)} fallback={<span>Score not released</span>}>
              {(label) => <span>{label()}</span>}
            </Show>
          </Show>
        </div>
      ),
    },
    {
      id: "action",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (attempt): JSX.Element => (
        <A class="quiet-link" href={attemptPath(attempt)}>
          {attempt.submittedAt === undefined ? "Open Attempt" : "Review Attempt"}
        </A>
      ),
    },
  ];
}

function attemptHistoryListState(loading: boolean, unavailable: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading Attempts..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Attempt History unavailable",
      message: "Attempts for this Course could not be loaded right now.",
    };
  }
  return { kind: "ready" };
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
      <RecordList
        ariaLabel={`${props.course.shortName} Attempts`}
        emptyState={{ title: "No Attempts have been started in this Course." }}
        recordId={(attempt) => attempt.assessmentAttemptId}
        regions={attemptHistoryRegions(props.formatDateTime)}
        rows={page()?.items ?? []}
        state={attemptHistoryListState(page.loading, page.error !== undefined)}
      />
      <Show when={!page.loading && page.error === undefined && page()?.items.length !== 0}>
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
