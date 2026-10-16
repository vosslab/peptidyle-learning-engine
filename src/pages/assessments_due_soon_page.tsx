// assessments_due_soon_page.tsx - Instructor cross-Course Assessment deadline index.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { DueSoonAssessmentSummary } from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import type { RecordContent } from "../components/record_list/record_list";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createLocalCompleteSearch } from "../features/search/local_complete_search_state";
import { assessmentRouteId, courseInstanceRouteId } from "../navigation/public_route";
import { useSelectedDisplayDateTimeFormatter } from "../selected_display_zone";
import {
  assessmentsDueSoonSearchFields,
  assessmentStatusLabel,
  type DueSoonSortKey,
} from "./assessments_due_soon_search_fields";

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
  const formatInAccountZone = useSelectedDisplayDateTimeFormatter();
  const formatDueTime = (dueAtMillis: number): string => formatInAccountZone()(dueAtMillis);
  const fields = assessmentsDueSoonSearchFields(formatDueTime);
  const [assessments, { refetch }] = createResource(
    async () => (await applicationApi.client.listAssessmentsDueSoon()).items,
  );
  const recordContent = (assessment: DueSoonAssessmentSummary): RecordContent =>
    dueSoonContent(formatDueTime)(assessment);
  const dueSoonSearch = createLocalCompleteSearch(
    { resource: assessments, refetchSource: refetch, rows: (value) => value },
    {
      fields,
      rowId: (assessment) => `${assessment.courseInstanceId}:${assessment.assessmentId}`,
      keyword: (assessment) =>
        `${assessment.assessmentTitle} ${assessment.courseLongName} ${assessment.assessmentType}`,
      compare: (key: DueSoonSortKey, left, right) =>
        key === "dueAt" ? left.dueAtMillis - right.dueAtMillis : undefined,
      content: recordContent,
    },
  );

  return (
    <PageFrame routeSurface="assessmentsDueSoon">
      <SearchBlock
        layout="above"
        controls={
          <SearchControls state={dueSoonSearch.state} textLabel="Search Assessments due soon" />
        }
        state={dueSoonSearch.state}
        collection={{
          ariaLabel: "Assessments due in the next 7 days",
          fields,
          query: dueSoonSearch.query,
          get emptyState() {
            return {
              title:
                dueSoonSearch.state.query().text.length > 0 ||
                dueSoonSearch.state.query().filters.size > 0
                  ? "No Assessments match these filters"
                  : "No Assessments are due in the next 7 days.",
            };
          },
        }}
      />
      <Show
        when={
          !assessments.loading && assessments.error === undefined && assessments()?.length === 0
        }
      >
        <A class="primary-link assessments-due-soon__empty-action" href="/instructor">
          Manage Coursework
        </A>
      </Show>
    </PageFrame>
  );
}
