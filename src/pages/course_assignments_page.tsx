// course_assignments_page.tsx - cursor-ready assignment list for one course.

import { A, createAsync, useParams } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";

export function CourseAssignmentsPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const assignments = createAsync(() => {
    const courseId = params["courseId"];
    if (courseId === undefined) {
      return Promise.reject(new Error("Course route is missing courseId"));
    }
    return runtime.queries.assignments(courseId);
  });

  return (
    <section class="page" data-route-surface="courseAssignments">
      <CourseEntryIdentity />
      <h2>Assignments</h2>
      <Suspense fallback={<p class="loading-state">Loading assignments...</p>}>
        <Show
          when={assignments()}
          fallback={<p class="empty-state">No assignments are available in this course.</p>}
        >
          {(page) => (
            <div class="card-grid">
              <For each={page().items}>
                {(assignment) => (
                  <article class="course-card">
                    <p class="card-kicker">Mastery practice</p>
                    <h2>{assignment.title}</h2>
                    <p>
                      {assignment.items.filter((item) => item.deliveryState === "active").length +
                        assignment.selectionGroups.reduce(
                          (count, group) => count + group.drawCount,
                          0,
                        )}{" "}
                      questions in each new run.
                    </p>
                    <A
                      class="primary-link"
                      href={`/courses/${assignment.courseId}/assignments/${assignment.id}`}
                    >
                      Review assignment
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
