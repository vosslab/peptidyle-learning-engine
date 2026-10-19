import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../../api/live_student_course_landing";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import { createDisplayDateTimeFormatter } from "../../format_datetime";
import { formatPointScore } from "../../score_format";
import { studentAssessmentScoreStateLabel } from "../../student_course_progress_presentation";
import { studentCourseworkDisplay } from "./student_coursework_presentation";

export function byStudentTitleThenId<T extends { readonly title: string; readonly id: string }>(
  rows: ReadonlyArray<T>,
): T[] {
  return [...rows].sort(
    (left, right) => left.title.localeCompare(right.title) || left.id.localeCompare(right.id),
  );
}

export function byStudentCourseNameThenId<
  T extends { readonly longName: string; readonly id: string },
>(rows: ReadonlyArray<T>): T[] {
  return [...rows].sort(
    (left, right) => left.longName.localeCompare(right.longName) || left.id.localeCompare(right.id),
  );
}

export type StudentCourseSort = "name" | "shortName";
export type StudentCourseFilter = "shortName";
export const studentCourseFields: ReadonlyArray<
  RecordField<LiveStudentCourseLandingSummary, StudentCourseSort, StudentCourseFilter>
> = [
  {
    id: "name",
    label: "Course",
    role: "identity",
    sortKey: "name",
    value: (row) => row.longName,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "short_name",
    label: "Short name",
    sortKey: "shortName",
    filterKey: "shortName",
    value: (row) => row.shortName,
    defaultColumn: { priority: 1, width: "12rem" },
  },
];

export type StudentInvitationSort = "name" | "instructor" | "startDate";
export type StudentInvitationFilter = "instructor";
export const studentInvitationFields: ReadonlyArray<
  RecordField<LiveStudentCourseInvitationSummary, StudentInvitationSort, StudentInvitationFilter>
> = [
  {
    id: "name",
    label: "Course",
    role: "identity",
    sortKey: "name",
    value: (row) => row.longName,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "instructor",
    label: "Instructor",
    sortKey: "instructor",
    filterKey: "instructor",
    value: (row) => row.instructorDisplayName,
    defaultColumn: { priority: 1, width: "14rem" },
  },
  {
    id: "start_date",
    label: "Term starts",
    sortKey: "startDate",
    value: (row) => row.term.startDate,
    defaultColumn: { priority: 2, width: "10rem" },
  },
];

export type StudentAssessmentSort = "title" | "type" | "dueAt" | "state";
export type StudentAssessmentFilter = "type" | "state";
export const studentAssessmentFields: ReadonlyArray<
  RecordField<LiveStudentAssessmentLandingSummary, StudentAssessmentSort, StudentAssessmentFilter>
> = [
  {
    id: "title",
    label: "Coursework",
    role: "identity",
    sortKey: "title",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessmentType).label,
    defaultColumn: { priority: 1, width: "11rem" },
  },
  {
    id: "state",
    label: "Status",
    sortKey: "state",
    filterKey: "state",
    value: (row) =>
      studentCourseworkDisplay(
        row.decision.startDecision,
        row.assessmentAttemptCompletion,
        row.canResumeAssessmentAttempt,
      ).stateLabel,
    defaultColumn: { priority: 2, width: "11rem" },
  },
  {
    id: "due_at",
    label: "Due",
    sortKey: "dueAt",
    value: (row) => row.decision.dueAt,
    render: (row) =>
      row.decision.dueAt === null
        ? "No due time"
        : createDisplayDateTimeFormatter(row.decision.displayTimeZone)(row.decision.dueAt),
    defaultColumn: { priority: 3, width: "12rem" },
  },
];

/** One Assessment retains its Course identity in global Student collections. */
export interface StudentCrossCourseAssessment {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessment: LiveStudentAssessmentLandingSummary;
}

export function studentCrossCourseLabel(course: LiveStudentCourseLandingSummary): string {
  const name =
    course.shortName === course.longName
      ? course.longName
      : `${course.shortName}: ${course.longName}`;
  return `${name} (Course ID: ${course.id})`;
}

export function studentCrossCourseAssessmentId(row: StudentCrossCourseAssessment): string {
  return `${row.course.id}:${row.assessment.id}`;
}

export function byStudentCrossCourseTitleThenId(
  rows: ReadonlyArray<StudentCrossCourseAssessment>,
): StudentCrossCourseAssessment[] {
  return [...rows].sort(
    (left, right) =>
      left.assessment.title.localeCompare(right.assessment.title) ||
      studentCrossCourseAssessmentId(left).localeCompare(studentCrossCourseAssessmentId(right)),
  );
}

export type StudentCrossCourseAssessmentSort = StudentAssessmentSort | "course";
export type StudentCrossCourseAssessmentFilter = StudentAssessmentFilter | "course";
export const studentCrossCourseAssessmentFields: ReadonlyArray<
  RecordField<
    StudentCrossCourseAssessment,
    StudentCrossCourseAssessmentSort,
    StudentCrossCourseAssessmentFilter
  >
> = [
  {
    id: "title",
    label: "Coursework",
    role: "identity",
    sortKey: "title",
    value: (row) => row.assessment.title,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "course",
    label: "Course",
    sortKey: "course",
    filterKey: "course",
    value: (row) => studentCrossCourseLabel(row.course),
    defaultColumn: { priority: 1, width: "12rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessment.assessmentType).label,
    defaultColumn: { priority: 2, width: "11rem" },
  },
  {
    id: "state",
    label: "Status",
    sortKey: "state",
    filterKey: "state",
    value: (row) =>
      studentCourseworkDisplay(
        row.assessment.decision.startDecision,
        row.assessment.assessmentAttemptCompletion,
        row.assessment.canResumeAssessmentAttempt,
      ).stateLabel,
    defaultColumn: { priority: 3, width: "11rem" },
  },
  {
    id: "due_at",
    label: "Due",
    sortKey: "dueAt",
    value: (row) => row.assessment.decision.dueAt,
    render: (row) =>
      row.assessment.decision.dueAt === null
        ? "No due time"
        : createDisplayDateTimeFormatter(row.assessment.decision.displayTimeZone)(
            row.assessment.decision.dueAt,
          ),
    defaultColumn: { priority: 4, width: "12rem" },
  },
];

export type StudentProgressSort = "title" | "type" | "attempts" | "scoreState";
export type StudentProgressFilter = "type" | "scoreState";
export const studentProgressFields: ReadonlyArray<
  RecordField<StudentCourseProgressAssessment, StudentProgressSort, StudentProgressFilter>
> = [
  {
    id: "title",
    label: "Coursework",
    role: "identity",
    sortKey: "title",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessmentType).label,
    defaultColumn: { priority: 1, width: "11rem" },
  },
  {
    id: "attempts",
    label: "Attempts",
    sortKey: "attempts",
    value: (row) => row.assessmentAttemptCount,
    defaultColumn: { priority: 2, width: "8rem" },
  },
  {
    id: "score_state",
    label: "Score state",
    sortKey: "scoreState",
    filterKey: "scoreState",
    value: studentAssessmentScoreStateLabel,
    defaultColumn: { priority: 3, width: "12rem" },
  },
];

export const studentScoreFields: ReadonlyArray<
  RecordField<LiveStudentAssessmentLandingSummary, "title" | "type" | "score", "type">
> = [
  {
    id: "title",
    label: "Coursework",
    role: "identity",
    sortKey: "title",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "12rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessmentType).label,
    defaultColumn: { priority: 2, width: "11rem" },
  },
  {
    id: "score",
    label: "Score",
    sortKey: "score",
    value: (row) => (row.assessmentScore === undefined ? null : row.assessmentScore.pointsEarned),
    render: (row) =>
      row.assessmentScore === undefined
        ? null
        : formatPointScore(row.assessmentScore.pointsEarned, row.assessmentScore.pointsPossible),
    defaultColumn: { priority: 1, width: "5rem" },
  },
];

export const studentCrossCourseScoreFields: ReadonlyArray<
  RecordField<
    StudentCrossCourseAssessment,
    "title" | "course" | "type" | "score",
    "course" | "type"
  >
> = [
  {
    id: "title",
    label: "Coursework",
    role: "identity",
    sortKey: "title",
    value: (row) => row.assessment.title,
    defaultColumn: { priority: 0, width: "12rem" },
  },
  {
    id: "course",
    label: "Course",
    sortKey: "course",
    filterKey: "course",
    value: (row) => studentCrossCourseLabel(row.course),
    defaultColumn: { priority: 2, width: "12rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (row) => assessmentTypePresentation(row.assessment.assessmentType).label,
    defaultColumn: { priority: 3, width: "11rem" },
  },
  {
    id: "score",
    label: "Score",
    sortKey: "score",
    value: (row) => row.assessment.assessmentScore?.pointsEarned ?? null,
    render: (row) =>
      row.assessment.assessmentScore === undefined
        ? null
        : formatPointScore(
            row.assessment.assessmentScore.pointsEarned,
            row.assessment.assessmentScore.pointsPossible,
          ),
    defaultColumn: { priority: 1, width: "5rem" },
  },
];
