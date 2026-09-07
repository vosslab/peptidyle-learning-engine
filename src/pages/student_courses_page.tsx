// Student-owned current Course Instance index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";

function CourseCard(props: { readonly course: LiveStudentCourseLandingSummary }): JSX.Element {
  return (
    <article class="course-card">
      <p class="card-kicker">Course Instance {props.course.reference}</p>
      <h2>{props.course.title}</h2>
      <A class="primary-link" href={`/student/courses/${props.course.reference}`}>
        Open assigned work
      </A>
    </article>
  );
}

/** Lists only the signed-in Student's current Course Instances. */
export function StudentCoursesPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [courses] = createResource(() => applicationApi.client.listLiveStudentCourses());

  return (
    <section class="page" data-route-surface="studentCourses">
      <p class="eyebrow">Your learning</p>
      <h1>Your courses</h1>
      <p class="page-lede">Open assigned work in one of your current Course Instances.</p>
      <A class="quiet-link" href="/student/course-invitations">
        Course invitations
      </A>
      <Show when={courses.loading}>
        <p class="loading-state">Loading your courses...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Your courses are unavailable</h2>
          <p>Your courses are not available right now.</p>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">You do not have any current courses.</p>
      </Show>
      <Show when={(courses()?.length ?? 0) > 0}>
        <div class="card-grid">
          <For each={courses()}>{(course) => <CourseCard course={course} />}</For>
        </div>
      </Show>
    </section>
  );
}
