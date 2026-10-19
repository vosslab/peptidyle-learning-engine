import type { CourseInstanceSummary } from "../../api/course_instance";
import type { CourseAssessmentSummary } from "../../api/assessment_release";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { formatLocalWallClockDateTime } from "../../format_datetime";
import { assessmentStatusLabel } from "../student_coursework/assessments_due_soon_search_fields";

export type CourseSortKey = "name" | "shortName" | "startDate" | "endDate";
export type CourseFilterKey = "tag";

export const courseCollectionFields: ReadonlyArray<
  RecordField<CourseInstanceSummary, CourseSortKey, CourseFilterKey>
> = [
  {
    id: "name",
    label: "Course Instance",
    role: "identity",
    sortKey: "name",
    value: (course) => course.longName,
    defaultColumn: { priority: 0, width: "19rem" },
  },
  {
    id: "short_name",
    label: "Short name",
    sortKey: "shortName",
    value: (course) => course.shortName,
    defaultColumn: { priority: 1, width: "11rem" },
  },
  {
    id: "start_date",
    label: "Term starts",
    sortKey: "startDate",
    value: (course) => course.term.startDate,
    defaultColumn: { priority: 2, width: "9rem" },
  },
  {
    id: "end_date",
    label: "Term ends",
    sortKey: "endDate",
    value: (course) => course.term.endDate,
    defaultColumn: { priority: 3, width: "9rem" },
  },
  {
    id: "tags",
    label: "Tags",
    filterKey: "tag",
    value: (course) => course.classification.tags,
    defaultColumn: { priority: 4, width: "10rem", visible: false },
  },
];

export type AssessmentSortKey = "title" | "position" | "dueAt";
export type AssessmentFilterKey = "status" | "type";

export type AssessmentCollectionRow = CourseAssessmentSummary & {
  readonly courseInstanceId: string;
  readonly authoredPosition: number;
};

export const assessmentCollectionFields: ReadonlyArray<
  RecordField<AssessmentCollectionRow, AssessmentSortKey, AssessmentFilterKey>
> = [
  {
    id: "title",
    label: "Assessment",
    role: "identity",
    sortKey: "title",
    value: (assessment) => assessment.title,
    defaultColumn: { priority: 0, width: "19rem" },
  },
  {
    id: "position",
    label: "Course order",
    sortKey: "position",
    value: (assessment) => assessment.authoredPosition,
    defaultColumn: { priority: 1, width: "8rem" },
  },
  {
    id: "status",
    label: "Status",
    filterKey: "status",
    value: (assessment) => assessment.status,
    render: (assessment) => assessmentStatusLabel(assessment.status),
    defaultColumn: { priority: 2, width: "9rem" },
  },
  {
    id: "type",
    label: "Type",
    filterKey: "type",
    value: (assessment) => assessment.assessmentType,
    render: (assessment) => assessmentTypePresentation(assessment.assessmentType).label,
    defaultColumn: { priority: 3, width: "10rem" },
  },
  {
    id: "due_at",
    label: "Due",
    sortKey: "dueAt",
    value: (assessment) => assessment.dueAt,
    render: (assessment) =>
      assessment.dueAt === null ? "No due date" : formatLocalWallClockDateTime(assessment.dueAt),
    defaultColumn: { priority: 4, width: "12rem" },
  },
];
