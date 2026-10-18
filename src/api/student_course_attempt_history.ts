// Browser contract for the Student's global, cursor-paged Attempt History.

import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { Timestamp } from "../../generated/api/Timestamp";

export type StudentAttemptHistoryState = "all" | "submitted" | "inProgress";
export type StudentAttemptHistorySort =
  | "startedNewest"
  | "startedOldest"
  | "titleAscending"
  | "titleDescending"
  | "courseAscending"
  | "courseDescending";
export type StudentAttemptHistoryFacetField = "course" | "state";

export interface StudentAttemptHistoryQuery {
  readonly search: string;
  readonly state: StudentAttemptHistoryState;
  readonly courseIds: ReadonlyArray<CourseInstanceId>;
  readonly sort: StudentAttemptHistorySort;
}

export interface StudentAttemptHistoryEntry {
  readonly courseInstanceId: CourseInstanceId;
  readonly courseShortName: string;
  readonly courseLongName: string;
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly assessmentId: AssessmentId;
  readonly assessmentTitle: string;
  readonly assessmentAttemptNumber: number;
  readonly startedAt: Timestamp;
  readonly submittedAt?: Timestamp;
  readonly assessmentScore?: { readonly pointsEarned: number; readonly pointsPossible: number };
}

export interface StudentAttemptHistoryPage {
  readonly items: ReadonlyArray<StudentAttemptHistoryEntry>;
  readonly pageNumber: number;
  readonly pageSize: number;
  readonly totalCount: number;
}

export interface StudentAttemptHistoryFacetPage {
  readonly choices: ReadonlyArray<{
    readonly key: string;
    readonly label: string;
    readonly count: number;
  }>;
  readonly nextCursor?: string;
}

export interface StudentAttemptHistoryClient {
  readonly listStudentAttemptHistory: (
    query: StudentAttemptHistoryQuery,
    pageNumber: number,
    pageSize: number,
  ) => Promise<StudentAttemptHistoryPage>;
  readonly listStudentAttemptHistoryFacetChoices: (
    query: StudentAttemptHistoryQuery,
    field: StudentAttemptHistoryFacetField,
    choiceSearch: string,
    cursor?: string,
  ) => Promise<StudentAttemptHistoryFacetPage>;
}
