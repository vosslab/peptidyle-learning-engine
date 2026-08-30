// course_list_page.tsx - runtime-backed course list and creation page.

import { A, createAsync, revalidate } from "@solidjs/router";
import { createMemo, createSignal, For, Show, Suspense, type JSX } from "solid-js";

import type { CourseSummary, CursorPage } from "../api/contracts";
import { useApiRuntime } from "../api/runtime";
import { CourseTermValidationError } from "../api/http_client/error";
import { useSessionBootstrap } from "../auth/session_context";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";
import { courseRouteReference } from "../navigation/public_route";
import type { CourseTermField } from "../../generated/api/CourseTermField";

type CourseCreateField = "title" | CourseTermField;

interface CourseCardProps {
  readonly course: CourseSummary;
  readonly registerLink: (course: CourseSummary, element: HTMLAnchorElement) => void;
}

export interface CourseListProps {
  readonly initialPage: CursorPage<CourseSummary>;
  readonly createdCourses: () => ReadonlyArray<CourseSummary>;
  readonly reloadCourses: () => Promise<void>;
  readonly registerLink: (course: CourseSummary, element: HTMLAnchorElement) => void;
}

function CourseCard(props: CourseCardProps): JSX.Element {
  return (
    <article class="course-card">
      <p class="card-kicker">Active course</p>
      <h2>{props.course.title}</h2>
      <p>Review the current assignment or resume an in-progress practice run.</p>
      <A
        class="primary-link"
        href={`/courses/${courseRouteReference(props.course.reference)}`}
        id={`course-open-${courseRouteReference(props.course.reference)}`}
        ref={(element: HTMLAnchorElement) => props.registerLink(props.course, element)}
      >
        Open course
      </A>
    </article>
  );
}

function pluralize(count: number, singular: string, plural: string): string {
  return count === 1 ? singular : plural;
}

/** Visible, append-only course paging keeps a large learner roster reachable without a route shortcut. */
export function CourseList(props: CourseListProps): JSX.Element {
  const runtime = useApiRuntime();
  const [state, setState] = createSignal<CursorPageSessionState<CourseSummary>>({
    items: [],
    nextCursor: null,
    loading: false,
    error: null,
  });
  const [announcement, setAnnouncement] = createSignal("");
  let retryButton: HTMLButtonElement | undefined;
  let reloadButton: HTMLButtonElement | undefined;
  const session = new CursorPageSession(
    props.initialPage,
    (cursor) => runtime.client.listCourses(cursor),
    (course) => course.id,
    setState,
  );
  setState(session.state);

  const visibleCourses = createMemo((): ReadonlyArray<CourseSummary> => {
    const seen = new Set<string>();
    const courses: CourseSummary[] = [];
    for (const course of [...props.createdCourses(), ...state().items]) {
      if (!seen.has(course.id)) {
        seen.add(course.id);
        courses.push(course);
      }
    }
    return courses;
  });

  function focusFirstAppended(appended: ReadonlyArray<CourseSummary>): void {
    const first = appended[0];
    if (first === undefined) return;
    requestAnimationFrame(() =>
      document.getElementById(`course-open-${courseRouteReference(first.reference)}`)?.focus(),
    );
  }

  function focusRecoveryButton(kind: "protocol" | "transport"): void {
    requestAnimationFrame(() => {
      if (kind === "transport") retryButton?.focus();
      else reloadButton?.focus();
    });
  }

  function statusMessage(): string {
    const current = state();
    if (current.loading) return "Loading more courses...";
    const shownCount = visibleCourses().length;
    if (current.error !== null || shownCount === 0) return "";
    if (current.nextCursor === null) {
      return `Loaded ${shownCount} ${pluralize(shownCount, "course", "courses")}.`;
    }
    return announcement();
  }

  async function loadMore(): Promise<void> {
    const appended = await session.loadMore();
    const current = state();
    if (current.error !== null) {
      focusRecoveryButton(current.error.kind);
      return;
    }
    if (appended.length > 0) {
      setAnnouncement(
        `Loaded ${appended.length} more ${pluralize(appended.length, "course", "courses")}. ${visibleCourses().length} ${pluralize(visibleCourses().length, "course", "courses")} visible.`,
      );
      focusFirstAppended(appended);
    }
  }

  async function retryLoadMore(): Promise<void> {
    const appended = await session.retry();
    const current = state();
    if (current.error !== null) {
      focusRecoveryButton(current.error.kind);
      return;
    }
    if (appended.length > 0) {
      setAnnouncement(
        `Loaded ${appended.length} more ${pluralize(appended.length, "course", "courses")}. ${visibleCourses().length} ${pluralize(visibleCourses().length, "course", "courses")} visible.`,
      );
      focusFirstAppended(appended);
    }
  }

  return (
    <>
      <Show
        when={
          (state().error === null && state().nextCursor !== null) ||
          state().error?.kind === "transport" ||
          state().error?.kind === "protocol"
        }
      >
        <a class="skip-link" href="#course-pagination" target="_self">
          Skip to load more courses
        </a>
      </Show>
      <Show
        when={visibleCourses().length > 0 || state().nextCursor !== null}
        fallback={<p class="empty-state">No courses are available for this account yet.</p>}
      >
        <div class="card-grid" aria-busy={state().loading}>
          <For each={visibleCourses()}>
            {(course) => <CourseCard course={course} registerLink={props.registerLink} />}
          </For>
        </div>
      </Show>
      <Show when={statusMessage().length > 0}>
        <p role="status" aria-live="polite" aria-atomic="true">
          {statusMessage()}
        </p>
      </Show>
      <Show
        when={
          state().error?.kind === "transport" ||
          state().error?.kind === "protocol" ||
          state().nextCursor !== null
        }
      >
        <section
          id="course-pagination"
          class="course-pagination"
          aria-label="Course pagination"
          tabindex="-1"
        >
          <Show when={state().error?.kind === "transport"}>
            <section class="route-error" role="alert">
              <p>
                Could not load more courses. The {visibleCourses().length}{" "}
                {pluralize(visibleCourses().length, "course", "courses")} already visible{" "}
                {visibleCourses().length === 1 ? "is" : "are"} still available.
              </p>
              <button
                class="primary-action"
                type="button"
                ref={(element) => (retryButton = element)}
                onClick={() => void retryLoadMore()}
              >
                Try loading more courses again
              </button>
            </section>
          </Show>
          <Show when={state().error?.kind === "protocol"}>
            <section class="route-error" role="alert">
              <p>{state().error?.message}</p>
              <button
                class="primary-action"
                type="button"
                ref={(element) => (reloadButton = element)}
                onClick={() => void props.reloadCourses()}
              >
                Reload courses
              </button>
            </section>
          </Show>
          <Show when={state().error === null && state().nextCursor !== null}>
            <button
              class="primary-action"
              type="button"
              disabled={state().loading}
              onClick={() => void loadMore()}
            >
              {state().loading ? "Loading more courses..." : "Load more courses"}
            </button>
          </Show>
        </section>
      </Show>
    </>
  );
}

export function CourseListPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const courses = createAsync(() => runtime.queries.courses());
  const [title, setTitle] = createSignal("");
  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const [timeZone, setTimeZone] = createSignal("");
  const [createdCourses, setCreatedCourses] = createSignal<ReadonlyArray<CourseSummary>>([]);
  const [isCreating, setIsCreating] = createSignal(false);
  const [creationError, setCreationError] = createSignal<string | null>(null);
  const [creationErrorField, setCreationErrorField] = createSignal<CourseCreateField | null>(null);
  const courseLinks = new Map<string, HTMLAnchorElement>();
  let titleInput: HTMLInputElement | undefined;
  let startDateInput: HTMLInputElement | undefined;
  let endDateInput: HTMLInputElement | undefined;
  let timeZoneInput: HTMLInputElement | undefined;

  async function reloadCourses(): Promise<void> {
    await revalidate(runtime.queries.courses.key);
  }

  const mayCreateCourse = createMemo((): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" &&
      (state.session.account.role === "instructor" || state.session.account.role === "sysadmin")
    );
  });

  function registerCourseLink(course: CourseSummary, element: HTMLAnchorElement): void {
    courseLinks.set(course.id, element);
  }

  function focusCourseField(field: CourseCreateField): void {
    const input = {
      title: titleInput,
      term: startDateInput,
      startDate: startDateInput,
      endDate: endDateInput,
      timeZone: timeZoneInput,
    }[field];
    queueMicrotask(() => input?.focus());
  }

  function rejectCourseInput(field: CourseCreateField, message: string): void {
    setCreationErrorField(field);
    setCreationError(message);
    focusCourseField(field);
  }

  async function createCourse(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isCreating()) return;
    if (title().trim().length === 0) {
      rejectCourseInput("title", "Enter a course title before creating the course.");
      return;
    }
    if (startDate() === "") {
      rejectCourseInput("startDate", "Enter a course start date.");
      return;
    }
    if (endDate() === "") {
      rejectCourseInput("endDate", "Enter a course end date.");
      return;
    }
    if (endDate() < startDate()) {
      rejectCourseInput("endDate", "Choose an end date on or after the start date.");
      return;
    }
    if (timeZone().trim().length === 0) {
      rejectCourseInput("timeZone", "Enter an IANA time zone.");
      return;
    }
    setCreationError(null);
    setCreationErrorField(null);
    setIsCreating(true);
    try {
      const course = await runtime.client.createCourse({
        title: title(),
        term: {
          startDate: startDate(),
          endDate: endDate(),
          timeZone: timeZone(),
        },
      });
      setCreatedCourses((current) => [course, ...current]);
      setTitle("");
      setStartDate("");
      setEndDate("");
      setTimeZone("");
      queueMicrotask(() => courseLinks.get(course.id)?.focus());
    } catch (error: unknown) {
      if (error instanceof CourseTermValidationError) {
        rejectCourseInput(error.failure.field, error.failure.message);
      } else {
        setCreationErrorField(null);
        setCreationError("We could not create that course. Check your connection and try again.");
      }
    } finally {
      setIsCreating(false);
    }
  }

  return (
    <section class="page" data-route-surface="courses">
      <p class="eyebrow">{mayCreateCourse() ? "Instructor workspace" : "Your courses"}</p>
      <h1>{mayCreateCourse() ? "Courses you teach" : "Pick up where you left off"}</h1>
      <p class="page-lede">
        {mayCreateCourse()
          ? "Open a course to manage assignments, learners, progress, and its visual identity."
          : "Practice is open-book. Choose a course, explain your reasoning, and learn from each attempt."}
      </p>
      <Show when={mayCreateCourse()}>
        <form
          class="course-create-form"
          aria-busy={isCreating()}
          novalidate
          onSubmit={(event) => void createCourse(event)}
        >
          <h2>Start another course</h2>
          <label for="course-title">
            Course title
            <input
              id="course-title"
              name="title"
              type="text"
              value={title()}
              onInput={(event) => setTitle(event.currentTarget.value)}
              autocomplete="off"
              required
              aria-invalid={creationErrorField() === "title"}
              aria-describedby="course-create-status"
              ref={(element) => (titleInput = element)}
            />
          </label>
          <label for="course-start-date">
            Start date
            <input
              id="course-start-date"
              name="startDate"
              type="date"
              value={startDate()}
              onInput={(event) => setStartDate(event.currentTarget.value)}
              required
              aria-invalid={creationErrorField() === "startDate"}
              aria-describedby="course-create-status"
              ref={(element) => (startDateInput = element)}
            />
          </label>
          <label for="course-end-date">
            End date
            <input
              id="course-end-date"
              name="endDate"
              type="date"
              value={endDate()}
              onInput={(event) => setEndDate(event.currentTarget.value)}
              required
              aria-invalid={creationErrorField() === "endDate"}
              aria-describedby="course-create-status"
              ref={(element) => (endDateInput = element)}
            />
          </label>
          <label for="course-time-zone">
            Time zone (IANA)
            <input
              id="course-time-zone"
              name="timeZone"
              type="text"
              value={timeZone()}
              onInput={(event) => setTimeZone(event.currentTarget.value)}
              autocomplete="off"
              autocapitalize="none"
              spellcheck={false}
              placeholder="America/Chicago"
              required
              aria-invalid={creationErrorField() === "timeZone"}
              aria-describedby="course-create-status"
              ref={(element) => (timeZoneInput = element)}
            />
          </label>
          <button class="primary-action" type="submit" disabled={isCreating()}>
            {isCreating() ? "Creating course..." : "Create course"}
          </button>
          <p id="course-create-status" role="status" aria-live="polite" aria-atomic="true">
            {creationError() ?? (isCreating() ? "Creating your course..." : "")}
          </p>
        </form>
      </Show>
      <Suspense fallback={<p class="loading-state">Loading your courses...</p>}>
        <Show
          when={courses()}
          keyed
          fallback={<p class="empty-state">No courses are available for this account yet.</p>}
        >
          {(page) => (
            <CourseList
              initialPage={page}
              createdCourses={createdCourses}
              reloadCourses={reloadCourses}
              registerLink={registerCourseLink}
            />
          )}
        </Show>
      </Suspense>
    </section>
  );
}
