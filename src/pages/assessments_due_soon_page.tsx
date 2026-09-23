// assessments_due_soon_page.tsx - Instructor cross-Course Assessment deadline index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { DueSoonAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { assessmentRouteId, courseInstanceRouteId } from "../navigation/public_route";

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

function DueSoonAssessmentRow(props: {
  readonly assessment: DueSoonAssessmentSummary;
  readonly formatDueTime: (dueAtMillis: number) => string;
}): JSX.Element {
  const courseInstanceId = courseInstanceRouteId(props.assessment.courseInstanceId);
  const assessmentId = assessmentRouteId(props.assessment.assessmentId);
  const assessmentPath = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}`;

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
          Course: <A href={`/courses/${courseInstanceId}`}>{props.assessment.courseLongName}</A>
        </p>
      </div>
      <p class="assessments-due-soon__due">
        <span>Due</span>
        {props.formatDueTime(props.assessment.dueAtMillis)}
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
    <PageFrame
      contentClass="assessments-due-soon"
      routeSurface="assessmentsDueSoon"
      eyebrow="Assessments"
      title="Assessments Due Soon"
      lede="Review upcoming Assessment deadlines across the Courses you currently teach."
    >
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
        {(dueSoon) => {
          const formatDueTime = createDisplayDateTimeFormatter(dueSoon().displayTimeZone);
          return (
            <>
              <p class="assessments-due-soon__window">
                Showing the next 7 days in your Account time zone: {dueSoon().displayTimeZone}.
              </p>
              <Show
                when={items().length > 0}
                fallback={
                  <section class="empty-state">
                    <h2>No Assessments are due in the next 7 days.</h2>
                    <p>Manage Coursework to review or set due dates in the Courses you teach.</p>
                    <A class="primary-link" href="/instructor">
                      Manage Coursework
                    </A>
                  </section>
                }
              >
                <ul
                  class="instructor-list assessments-due-soon__list"
                  aria-label="Assessments due in the next 7 days"
                >
                  <For each={items()}>
                    {(assessment) => (
                      <DueSoonAssessmentRow assessment={assessment} formatDueTime={formatDueTime} />
                    )}
                  </For>
                </ul>
              </Show>
            </>
          );
        }}
      </Show>
    </PageFrame>
  );
}
