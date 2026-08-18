// course_list_page.tsx - mock-backed first-success route.

import { A, createAsync, revalidate } from "@solidjs/router";
import { createMemo, createSignal, For, Show, Suspense, type JSX } from "solid-js";

import type { CourseSummary, CursorPage } from "../api/contracts";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";
import { courseRouteReference } from "../navigation/public_route";

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
        href={`/courses/${courseRouteReference(props.course.publicId)}`}
        id={`course-open-${courseRouteReference(props.course.publicId)}`}
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
      document.getElementById(`course-open-${courseRouteReference(first.publicId)}`)?.focus(),
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
  const [createdCourses, setCreatedCourses] = createSignal<ReadonlyArray<CourseSummary>>([]);
  const [isCreating, setIsCreating] = createSignal(false);
  const [creationError, setCreationError] = createSignal<string | null>(null);
  const courseLinks = new Map<string, HTMLAnchorElement>();

  async function reloadCourses(): Promise<void> {
    await revalidate(runtime.queries.courses.key);
  }

  const mayCreateCourse = createMemo((): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" &&
      state.session.user.roles.some((role) => role === "instructor" || role === "sysadmin")
    );
  });

  function registerCourseLink(course: CourseSummary, element: HTMLAnchorElement): void {
    courseLinks.set(course.id, element);
  }

  async function createCourse(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (isCreating()) return;
    if (title().trim().length === 0) {
      setCreationError("Enter a course title before creating the course.");
      return;
    }
    setCreationError(null);
    setIsCreating(true);
    try {
      const course = await runtime.client.createCourse({ title: title() });
      setCreatedCourses((current) => [course, ...current]);
      setTitle("");
      queueMicrotask(() => courseLinks.get(course.id)?.focus());
    } catch (_error: unknown) {
      setCreationError("We could not create that course. Check your connection and try again.");
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
              aria-describedby="course-create-status"
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
