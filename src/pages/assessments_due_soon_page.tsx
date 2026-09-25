// assessments_due_soon_page.tsx - Instructor cross-Course Assessment deadline index.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { DueSoonAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
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

function dueSoonRegions(
  formatDueTime: (dueAtMillis: number) => string,
): ReadonlyArray<RecordRegion<DueSoonAssessmentSummary>> {
  return [
    {
      id: "identity",
      role: "identity",
      priority: "required",
      width: "minmax(14rem, 1fr)",
      align: "stretch",
      content: (assessment): JSX.Element => {
        const courseInstanceId = courseInstanceRouteId(assessment.courseInstanceId);
        const assessmentId = assessmentRouteId(assessment.assessmentId);
        const assessmentPath = `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}`;
        return (
          <div>
            <p class="assessments-due-soon__kind">
              {assessmentStatusLabel(assessment.assessmentStatus)} Assessment
            </p>
            <h2>
              <A href={assessmentPath}>{assessment.assessmentTitle}</A>
            </h2>
            <p>
              Course: <A href={`/courses/${courseInstanceId}`}>{assessment.courseLongName}</A>
            </p>
          </div>
        );
      },
    },
    {
      id: "due",
      role: "metadata",
      priority: "high",
      width: "minmax(13rem, auto)",
      align: "center",
      content: (assessment) => (
        <p class="assessments-due-soon__due">
          <span>Due</span>
          {formatDueTime(assessment.dueAtMillis)}
        </p>
      ),
    },
    {
      id: "actions",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "center",
      content: (assessment): JSX.Element => {
        const courseInstanceId = courseInstanceRouteId(assessment.courseInstanceId);
        const assessmentId = assessmentRouteId(assessment.assessmentId);
        return (
          <A
            class="primary-link"
            href={`/instructor/courses/${courseInstanceId}/assessments/${assessmentId}`}
          >
            Open Assessment
          </A>
        );
      },
    },
  ];
}

/** Shows the signed-in Instructor's server-bounded upcoming Assessment deadlines. */
export function AssessmentsDueSoonPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assessments, { refetch }] = createResource(() =>
    applicationApi.client.listAssessmentsDueSoon(),
  );
  const items = (): ReadonlyArray<DueSoonAssessmentSummary> => assessments()?.items ?? [];
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
      contentClass="assessments-due-soon"
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
        regions={dueSoonRegions(
          createDisplayDateTimeFormatter(assessments()?.displayTimeZone ?? "UTC"),
        )}
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
