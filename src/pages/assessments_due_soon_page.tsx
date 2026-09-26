// assessments_due_soon_page.tsx - Instructor cross-Course Assessment deadline index.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { DueSoonAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
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

function dueSoonContent(
  formatDueTime: (dueAtMillis: number) => string,
): (assessment: DueSoonAssessmentSummary) => RecordContent {
  return (assessment): RecordContent => {
    const courseInstanceId = courseInstanceRouteId(assessment.courseInstanceId);
    const assessmentId = assessmentRouteId(assessment.assessmentId);
    const assessmentPath = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}`;
    return {
      title: assessment.assessmentTitle,
      details: [
        { kind: "assessmentType", value: assessment.assessmentType },
        {
          kind: "text",
          label: "Status:",
          value: assessmentStatusLabel(assessment.assessmentStatus),
        },
        {
          kind: "time",
          label: "Due:",
          value: formatDueTime(assessment.dueAtMillis),
          dateTime: new Date(assessment.dueAtMillis).toISOString(),
        },
        {
          kind: "link",
          label: assessment.courseLongName,
          href: `/courses/${courseInstanceId}`,
        },
      ],
      actions: [
        {
          id: "open-assessment",
          kind: "link",
          label: "Open Assessment",
          href: assessmentPath,
          primary: true,
        },
      ],
    };
  };
}

/** Shows the signed-in Instructor's server-bounded upcoming Assessment deadlines. */
export function AssessmentsDueSoonPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assessments, { refetch }] = createResource(() =>
    applicationApi.client.listAssessmentsDueSoon(),
  );
  const items = (): ReadonlyArray<DueSoonAssessmentSummary> => assessments()?.items ?? [];
  const recordContent = (assessment: DueSoonAssessmentSummary): RecordContent =>
    dueSoonContent(createDisplayDateTimeFormatter(assessments()?.displayTimeZone ?? "UTC"))(
      assessment,
    );
  const listState = (): RecordListState => {
    if (assessments.loading) return { kind: "loading", label: "Loading Assessments due soon..." };
    if (assessments.error !== undefined) {
      return {
        kind: "error",
        title: "Assessments due soon unavailable",
        message: "Assessments due soon could not be loaded.",
        retry: () => void refetch(),
        retryLabel: "Try again",
      };
    }
    return { kind: "ready" };
  };

  return (
    <PageFrame
      routeSurface="assessmentsDueSoon"
      eyebrow="Assessments"
      title="Assessments Due Soon"
      lede="Review upcoming Assessment deadlines across the Courses you currently teach."
    >
      <p class="assessments-due-soon__window">Showing Assessments due in the next 7 days.</p>
      <RecordList
        ariaLabel="Assessments due in the next 7 days"
        emptyState={{
          title: "No Assessments are due in the next 7 days.",
          message: "Manage Coursework to review or set due dates in the Courses you teach.",
        }}
        recordId={(assessment) => assessment.assessmentId}
        content={recordContent}
        rows={items()}
        state={listState()}
      />
      <Show when={!assessments.loading && assessments.error === undefined && items().length === 0}>
        <A class="primary-link assessments-due-soon__empty-action" href="/instructor">
          Manage Coursework
        </A>
      </Show>
    </PageFrame>
  );
}
