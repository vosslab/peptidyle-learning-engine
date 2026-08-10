// course_assignments_page.tsx - cursor-ready assignment list for one course.

import { A, createAsync, useParams } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { useSessionBootstrap } from "../auth/session_context";

export function CourseAssignmentsPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const params = useParams();
  const assignments = createAsync(() => {
    const courseId = params["courseId"];
    if (courseId === undefined) {
      return Promise.reject(new Error("Course route is missing courseId"));
    }
    return runtime.queries.assignments(courseId);
  });
  const canManageCourse = (): boolean => {
    const current = session.state();
    return (
      current.kind === "authenticated" &&
      current.session.user.roles.some((role) => role === "instructor" || role === "administrator")
    );
  };

  return (
    <section class="page" data-route-surface="courseAssignments">
      <CourseEntryIdentity />
      <h2>Assignments</h2>
      <Show when={canManageCourse() && params["courseId"] !== undefined}>
        <nav class="course-management-nav" aria-label="Course management">
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/students`}>Students</A>
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/gradebook`}>Gradebook</A>
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/appearance`}>Appearance</A>
        </nav>
      </Show>
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
