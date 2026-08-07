// course_list_page.tsx - mock-backed first-success route.

import { A, createAsync } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";

export function CourseListPage(): JSX.Element {
  const runtime = useApiRuntime();
  const courses = createAsync(() => runtime.queries.courses());

  return (
    <section class="page" data-route-surface="courses">
      <p class="eyebrow">Your courses</p>
      <h1>Pick up where you left off</h1>
      <p class="page-lede">
        Practice is open-book. Choose a course, explain your reasoning, and learn from each attempt.
      </p>
      <Suspense fallback={<p class="loading-state">Loading your courses...</p>}>
        <Show
          when={courses()}
          fallback={<p class="empty-state">No courses are available for this account yet.</p>}
        >
          {(page) => (
            <div class="card-grid">
              <For each={page().items}>
                {(course) => (
                  <article class="course-card">
                    <p class="card-kicker">Active course</p>
                    <h2>{course.title}</h2>
                    <p>Review the current assignment or resume an in-progress practice run.</p>
                    <A class="primary-link" href={`/courses/${course.id}`}>
                      Open course
                    </A>
                  </article>
                )}
              </For>
            </div>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
