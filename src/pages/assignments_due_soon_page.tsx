// assignments_due_soon_page.tsx - Instructor cross-Course Assignment deadline index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { DueSoonAssignmentSummary, LiveAssignmentStatus } from "../api/assignment_release";
import { useApplicationApi } from "../api/application_api";
import { assignmentRouteReference, courseInstanceRouteReference } from "../navigation/public_route";

import "./assignments_due_soon_page.css";

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

function formatDueTime(dueAtMillis: number, displayTimeZone: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: displayTimeZone,
  }).format(new Date(dueAtMillis));
}

function DueSoonAssignmentRow(props: {
  readonly assignment: DueSoonAssignmentSummary;
  readonly displayTimeZone: string;
}): JSX.Element {
  const courseReference = courseInstanceRouteReference(props.assignment.courseReference);
  const assignmentReference = assignmentRouteReference(props.assignment.assignmentReference);
  const assignmentPath = `/instructor/courses/${courseReference}/assignments/${assignmentReference}`;

  return (
    <li class="instructor-list__row assignments-due-soon__row">
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">
          {assignmentStatusLabel(props.assignment.assignmentStatus)} Assignment
        </p>
        <h2>
          <A href={assignmentPath}>{props.assignment.assignmentTitle}</A>
        </h2>
        <p class="instructor-list__metadata">
          Course: <A href={`/courses/${courseReference}`}>{props.assignment.courseTitle}</A>
        </p>
      </div>
      <p class="assignments-due-soon__due">
        <span>Due</span>
        {formatDueTime(props.assignment.dueAtMillis, props.displayTimeZone)}
      </p>
      <div class="instructor-list__actions">
        <A class="primary-link" href={assignmentPath}>
          Open Assignment
        </A>
      </div>
    </li>
  );
}

/** Shows the signed-in Instructor's server-bounded upcoming Assignment deadlines. */
export function AssignmentsDueSoonPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assignments, { refetch }] = createResource(() =>
    applicationApi.client.listAssignmentsDueSoon(),
  );
  const items = (): ReadonlyArray<DueSoonAssignmentSummary> => assignments()?.items ?? [];

  return (
    <section class="page assignments-due-soon" data-route-surface="assignmentsDueSoon">
      <p class="eyebrow">Assignments</p>
      <h1>Assignments Due Soon</h1>
      <p class="page-lede">
        Review upcoming Assignment deadlines across the Courses you currently teach.
      </p>
      <Show when={assignments.loading}>
        <p class="loading-state" role="status">
          Loading Assignments due soon...
        </p>
      </Show>
      <Show when={assignments.error !== undefined}>
        <section class="route-error" role="alert">
          <p>Assignments due soon could not be loaded.</p>
          <button class="primary-action" type="button" onClick={() => void refetch()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={assignments.error === undefined && assignments()}>
        {(dueSoon) => (
          <>
            <p class="assignments-due-soon__window">
              Showing the next 7 days in your Account time zone: {dueSoon().displayTimeZone}.
            </p>
            <Show
              when={items().length > 0}
              fallback={
                <section class="empty-state">
                  <h2>No Assignments are due in the next 7 days.</h2>
                  <p>Set a due date on an Assignment in a Course you teach to see it here.</p>
                </section>
              }
            >
              <ul
                class="instructor-list assignments-due-soon__list"
                aria-label="Assignments due in the next 7 days"
              >
                <For each={items()}>
                  {(assignment) => (
                    <DueSoonAssignmentRow
                      assignment={assignment}
                      displayTimeZone={dueSoon().displayTimeZone}
                    />
                  )}
                </For>
              </ul>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
