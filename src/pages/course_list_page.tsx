// course_list_page.tsx - mock-backed first-success route.

import { A, createAsync } from "@solidjs/router";
import { createMemo, createSignal, For, Show, Suspense, type JSX } from "solid-js";

import type { CourseSummary } from "../api/contracts";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";

interface CourseCardProps {
  readonly course: CourseSummary;
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
        href={`/courses/${props.course.id}`}
        ref={(element: HTMLAnchorElement) => props.registerLink(props.course, element)}
      >
        Open course
      </A>
    </article>
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

  const mayCreateCourse = createMemo((): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" &&
      state.session.user.roles.some((role) => role === "instructor" || role === "administrator")
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
      <p class="eyebrow">Your courses</p>
      <h1>Pick up where you left off</h1>
      <p class="page-lede">
        Practice is open-book. Choose a course, explain your reasoning, and learn from each attempt.
      </p>
      <Show when={mayCreateCourse()}>
        <form
          class="course-create-form"
          aria-busy={isCreating()}
          onSubmit={(event) => void createCourse(event)}
        >
          <h2>Create a course</h2>
          <label for="course-title">Course title</label>
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
          fallback={<p class="empty-state">No courses are available for this account yet.</p>}
        >
          {(page) => (
            <div class="card-grid">
              <For each={createdCourses()}>
                {(course) => <CourseCard course={course} registerLink={registerCourseLink} />}
              </For>
              <For each={page().items}>
                {(course) => <CourseCard course={course} registerLink={registerCourseLink} />}
              </For>
            </div>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
