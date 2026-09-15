// assessments_due_soon_page.tsx - Instructor cross-Course Assessment deadline index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { DueSoonAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { assessmentRouteReference, courseInstanceRouteReference } from "../navigation/public_route";

import "./assessments_due_soon_page.css";

function assessmentStatusLabel(status: LiveAssessmentStatus): string {
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

function DueSoonAssessmentRow(props: {
  readonly assessment: DueSoonAssessmentSummary;
  readonly displayTimeZone: string;
}): JSX.Element {
  const courseReference = courseInstanceRouteReference(props.assessment.courseReference);
  const assessmentReference = assessmentRouteReference(props.assessment.assessmentReference);
  const assessmentPath = `/instructor/courses/${courseReference}/assessments/${assessmentReference}`;

  return (
    <li class="instructor-list__row assessments-due-soon__row">
      <div class="instructor-list__identity">
        <p class="instructor-list__kind">
          {assessmentStatusLabel(props.assessment.assessmentStatus)} Assessment
        </p>
        <h2>
          <A href={assessmentPath}>{props.assessment.assessmentTitle}</A>
        </h2>
        <p class="instructor-list__metadata">
          Course: <A href={`/courses/${courseReference}`}>{props.assessment.courseLongName}</A>
        </p>
      </div>
      <p class="assessments-due-soon__due">
        <span>Due</span>
        {formatDueTime(props.assessment.dueAtMillis, props.displayTimeZone)}
      </p>
      <div class="instructor-list__actions">
        <A class="primary-link" href={assessmentPath}>
          Open Assessment
        </A>
      </div>
    </li>
  );
}

/** Shows the signed-in Instructor's server-bounded upcoming Assessment deadlines. */
export function AssessmentsDueSoonPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assessments, { refetch }] = createResource(() =>
    applicationApi.client.listAssessmentsDueSoon(),
  );
  const items = (): ReadonlyArray<DueSoonAssessmentSummary> => assessments()?.items ?? [];

  return (
    <section class="page assessments-due-soon" data-route-surface="assessmentsDueSoon">
      <p class="eyebrow">Assessments</p>
      <h1>Assessments Due Soon</h1>
      <p class="page-lede">
        Review upcoming Assessment deadlines across the Courses you currently teach.
      </p>
      <Show when={assessments.loading}>
        <p class="loading-state" role="status">
          Loading Assessments due soon...
        </p>
      </Show>
      <Show when={assessments.error !== undefined}>
        <section class="route-error" role="alert">
          <p>Assessments due soon could not be loaded.</p>
          <button class="primary-action" type="button" onClick={() => void refetch()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={assessments.error === undefined && assessments()}>
        {(dueSoon) => (
          <>
            <p class="assessments-due-soon__window">
              Showing the next 7 days in your Account time zone: {dueSoon().displayTimeZone}.
            </p>
            <Show
              when={items().length > 0}
              fallback={
                <section class="empty-state">
                  <h2>No Assessments are due in the next 7 days.</h2>
                  <p>Set a due date on an Assessment in a Course you teach to see it here.</p>
                </section>
              }
            >
              <ul
                class="instructor-list assessments-due-soon__list"
                aria-label="Assessments due in the next 7 days"
              >
                <For each={items()}>
                  {(assessment) => (
                    <DueSoonAssessmentRow
                      assessment={assessment}
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
