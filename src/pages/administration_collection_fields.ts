import type { CourseGradebookStudentWork } from "../api/live_gradebook";
import type { CourseRosterEntry } from "../api/course_roster";
import type { RecordField } from "../components/data_sheet/data_sheet_fields";
import { formatPointScore } from "../score_format";

export function rosterStateLabel(state: CourseRosterEntry["state"]): string {
  return state === "activeStudent" ? "Active Student" : "Invitation pending";
}

export type RosterSort = "student" | "rosterId" | "state";
export type RosterFilter = "state";
export const rosterFields: ReadonlyArray<RecordField<CourseRosterEntry, RosterSort, RosterFilter>> =
  [
    {
      id: "student",
      label: "Student",
      role: "identity",
      sortKey: "student",
      value: (entry) => entry.rosterName,
      defaultColumn: { priority: 0, width: "20rem" },
    },
    {
      id: "roster_id",
      label: "Roster ID",
      sortKey: "rosterId",
      value: (entry) => entry.rosterId,
      defaultColumn: { priority: 2, width: "12rem" },
    },
    {
      id: "state",
      label: "State",
      sortKey: "state",
      filterKey: "state",
      value: (entry) => rosterStateLabel(entry.state),
      defaultColumn: { priority: 1, width: "13rem" },
    },
  ];

export type GradebookSort = "student" | "coursework" | "progress" | "score";
export type GradebookFilter = "coursework" | "progress";

export function gradebookProgressLabel(work: CourseGradebookStudentWork): string {
  if (work.assessmentAttemptCompletion === "completed") return "Completed and scored";
  if (work.assessmentAttemptCompletion === "inProgress") return "In progress";
  return "Not started";
}

export function gradebookScoreLabel(work: CourseGradebookStudentWork): string {
  if (work.score !== null)
    return formatPointScore(work.score.pointsEarned, work.score.pointsPossible);
  return work.expiredSubmitting ? "Expired, submitting" : "-";
}

/** Tuple encoding avoids collisions between Course roster and Assessment identifiers. */
export function gradebookWorkId(work: CourseGradebookStudentWork): string {
  return JSON.stringify([work.rosterId, work.assessmentId]);
}

export function compareGradebookWork(
  key: GradebookSort,
  left: CourseGradebookStudentWork,
  right: CourseGradebookStudentWork,
): number | undefined {
  if (key !== "score") return undefined;
  const leftScore = left.score?.pointsEarned ?? Number.NEGATIVE_INFINITY;
  const rightScore = right.score?.pointsEarned ?? Number.NEGATIVE_INFINITY;
  return leftScore === rightScore ? 0 : leftScore - rightScore;
}

export const gradebookFields: ReadonlyArray<
  RecordField<CourseGradebookStudentWork, GradebookSort, GradebookFilter>
> = [
  {
    id: "student",
    label: "Student",
    role: "identity",
    sortKey: "student",
    value: (work) => work.rosterName,
    defaultColumn: { priority: 0, width: "16rem" },
  },
  {
    id: "coursework",
    label: "Coursework",
    sortKey: "coursework",
    filterKey: "coursework",
    value: (work) => work.assessmentTitle,
    defaultColumn: { priority: 1, width: "20rem" },
  },
  {
    id: "progress",
    label: "Progress status",
    sortKey: "progress",
    filterKey: "progress",
    value: gradebookProgressLabel,
    defaultColumn: { priority: 2, width: "13rem" },
  },
  {
    id: "score",
    label: "Current score",
    sortKey: "score",
    value: gradebookScoreLabel,
    defaultColumn: { priority: 3, width: "10rem" },
  },
  {
    id: "roster_id",
    label: "Roster ID",
    value: (work) => work.rosterId,
    defaultColumn: { priority: 4, width: "12rem" },
  },
  {
    id: "assessment_id",
    label: "Assessment ID",
    value: (work) => work.assessmentId,
    defaultColumn: { priority: 5, width: "14rem", visible: false },
  },
];
