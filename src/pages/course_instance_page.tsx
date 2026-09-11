// course_instance_page.tsx - Course Instance Teaching Team workspace.

import { A, useParams } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { CourseAssignmentSummary, LiveAssignmentStatus } from "../api/assignment_release";
import { parseCourseInstanceReference } from "../navigation/public_route";

import "./course_instance_page.css";

function assignmentStatusLabel(status: LiveAssignmentStatus): string {
  switch (status) {
    case "unreleased":
      return "Unreleased";
    case "released":
      return "Released";
    case "closed":
      return "Closed";
    case "archived":
      return "Archived";
  }
}

function assignmentQuestionsPath(courseReference: string, assignmentReference: string): string {
  return `/instructor/courses/${courseReference}/assignments/${assignmentReference}/questions`;
}

function AssignmentRow(props: {
  readonly courseReference: string;
  readonly assignment: CourseAssignmentSummary;
}): JSX.Element {
  return (
    <article class="instructor-list__row instructor-list__row--assignment">
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">
          {assignmentStatusLabel(props.assignment.status)} Assignment
        </p>
        <h3>{props.assignment.title}</h3>
        <p class="instructor-list__metadata">Assignment {props.assignment.reference}</p>
      </div>
      <div class="instructor-list__actions">
        <A
          class="primary-link"
          href={assignmentQuestionsPath(props.courseReference, props.assignment.reference)}
        >
          Edit Assignment
        </A>
      </div>
    </article>
  );
}

/** Instructor Course Instance workspace for Teaching Team, roster, and Assignment delivery. */
export function CourseInstancePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseReference(): ReturnType<typeof parseCourseInstanceReference> {
    return parseCourseInstanceReference(params["courseRef"] ?? "");
  }
  const [course] = createResource(courseReference, async (reference) =>
    applicationApi.client.getCourseInstance(reference),
  );
  const [assignments] = createResource(courseReference, async (reference) =>
    applicationApi.client.listCourseAssignments(reference),
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
            <p class="eyebrow">Course Instance · {view().course.reference}</p>
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
            <section aria-labelledby="course-assignments-heading">
              <h2 id="course-assignments-heading">Assignments</h2>
              <Show when={assignments.loading}>
                <p class="loading-state">Loading Assignments...</p>
              </Show>
              <Show when={assignments.error !== undefined}>
                <p class="route-error" role="alert">
                  Assignments could not be loaded.
                </p>
              </Show>
              <Show
                when={(assignments()?.length ?? 0) > 0}
                fallback={
                  <Show when={!assignments.loading && assignments.error === undefined}>
                    <p class="empty-state">No Assignments have been created for this course.</p>
                  </Show>
                }
              >
                <div class="instructor-list" aria-label="Assignments">
                  <For each={assignments()}>
                    {(assignment) => (
                      <AssignmentRow
                        courseReference={view().course.reference}
                        assignment={assignment}
                      />
                    )}
                  </For>
                </div>
              </Show>
            </section>
            <nav
              class="course-card-actions course-instance-page__actions"
              aria-label="Course actions"
            >
              <A
                class="primary-link"
                href={`/instructor/courses/${view().course.reference}/students`}
              >
                Open Students
              </A>
              <A
                class="quiet-link"
                href={`/instructor/courses/${view().course.reference}/assignments/new`}
              >
                Create Assignment
              </A>
              <A
                class="quiet-link"
                href={`/instructor/courses/${view().course.reference}/appearance`}
              >
                Appearance
              </A>
            </nav>
          </>
        )}
      </Show>
    </section>
  );
}
