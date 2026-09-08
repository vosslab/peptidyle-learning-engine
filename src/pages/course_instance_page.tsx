// course_instance_page.tsx - Course Instance Teaching Team workspace.

import { A, useParams } from "@solidjs/router";
import { createResource, Show, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceReference } from "../navigation/public_route";

/** Minimal live Course Instance workspace; Assignment and roster tasks remain unavailable. */
export function CourseInstancePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const [course] = createResource(
    () => parseCourseInstanceReference(params["courseRef"] ?? ""),
    async (reference) => {
      if (reference === null) throw new Error("Course Instance reference is invalid");
      return applicationApi.client.getCourseInstance(reference);
    },
  );

  return (
    <section class="page" data-route-surface="courseInstance">
      <Show when={course.loading}>
        <p class="loading-state">Loading Course Instance...</p>
      </Show>
      <Show
        when={course()}
        fallback={
          <Show when={course.error !== undefined}>
            <section class="route-error" role="alert">
              <h1>Course Instance unavailable</h1>
              <p>
                This Course Instance is not available through your current Teaching Team access.
              </p>
              <A class="primary-link" href="/">
                Return to Course Instances
              </A>
            </section>
          </Show>
        }
      >
        {(view) => (
          <>
            <p class="eyebrow">Live Course Instance · {view().course.reference}</p>
            <h1>{view().course.title}</h1>
            <p class="page-lede">
              Course Term: {view().course.term.startDate} through {view().course.term.endDate} in{" "}
              {view().course.term.timeZone}.
            </p>
            <section class="course-card" aria-labelledby="teaching-team-heading">
              <p class="card-kicker">Teaching Team</p>
              <h2 id="teaching-team-heading">Initial Teaching Team</h2>
              <p>
                {view().isAssignedInstructor
                  ? "You are the Assigned Instructor for this Course Instance."
                  : "You are an active Teaching Team Member for this Course Instance."}
              </p>
              <p>
                {view().activeInstructorCount} active Instructor
                {view().activeInstructorCount === 1 ? " is" : "s are"} currently recorded.
              </p>
            </section>
            <section class="empty-state" aria-label="Course delivery work">
              <h2>Course delivery begins with the roster</h2>
              <p>
                Import Student roster entries before creating Assignment content, deadlines, or
                release state.
              </p>
              <A
                class="primary-link"
                href={`/instructor/courses/${view().course.reference}/students`}
              >
                Open Students
              </A>
              <A
                class="primary-link"
                href={`/instructor/courses/${view().course.reference}/assignments/new`}
              >
                Open Assignments
              </A>
            </section>
          </>
        )}
      </Show>
    </section>
  );
}
