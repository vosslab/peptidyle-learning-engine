import type { DueSoonAssessmentSummary, LiveAssessmentStatus } from "../api/assessment_release";
import type { RecordField } from "../components/data_sheet/record_fields";
import { assessmentTypePresentation } from "../assessment_type_presentation";

export type DueSoonSortKey = "title" | "course" | "dueAt" | "assessmentType" | "status";
export type DueSoonFilterKey = "course" | "assessmentType" | "status";

export function assessmentStatusLabel(status: LiveAssessmentStatus): string {
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

export function assessmentsDueSoonSearchFields(
  formatDueTime: (dueAtMillis: number) => string,
): ReadonlyArray<RecordField<DueSoonAssessmentSummary, DueSoonSortKey, DueSoonFilterKey>> {
  return [
    {
      id: "title",
      label: "Assessment",
      role: "identity",
      sortKey: "title",
      value: (assessment) => assessment.assessmentTitle,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "course",
      label: "Course",
      sortKey: "course",
      filterKey: "course",
      value: (assessment) => assessment.courseLongName,
      defaultColumn: { priority: 1, width: "16rem" },
    },
    {
      id: "dueAt",
      label: "Due",
      sortKey: "dueAt",
      value: (assessment) => assessment.dueAtMillis,
      render: (assessment) => formatDueTime(assessment.dueAtMillis),
      defaultColumn: { priority: 2, width: "11rem" },
    },
    {
      id: "assessmentType",
      label: "Assessment Type",
      sortKey: "assessmentType",
      filterKey: "assessmentType",
      value: (assessment) => assessmentTypePresentation(assessment.assessmentType).label,
      defaultColumn: { priority: 3, width: "12rem" },
    },
    {
      id: "status",
      label: "Status",
      sortKey: "status",
      filterKey: "status",
      value: (assessment) => assessmentStatusLabel(assessment.assessmentStatus),
      defaultColumn: { priority: 4, width: "8rem" },
    },
  ];
}
