import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseRosterTuple } from "../../generated/api/CourseRosterTuple";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";

export interface RecoverySummary {
  readonly courseInstanceId: CourseInstanceId;
  readonly courseRosterTuple: CourseRosterTuple | null;
  readonly assessmentId: AssessmentId;
  readonly assessmentTitle: string;
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly assessmentAttemptNumber: number;
  readonly startedAt: string;
  readonly submittedAt: string | null;
  readonly studentDataArchivedAt: string;
  readonly deleteDueAt: string;
}
export interface RecoveredQuestion {
  readonly issuedPosition: number;
  readonly publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple;
  readonly deliveryText: string;
  readonly poolText: string | null;
  readonly attemptText: string | null;
  readonly presentationText: string | null;
  readonly reproductionText: string | null;
  readonly backendDocumentText: string | null;
  readonly savedResponseText: string | null;
  readonly finalizedResponseText: string | null;
  readonly gradingText: string | null;
  readonly unavailableEvidence: ReadonlyArray<string>;
}
export interface RecoveredAttempt extends Omit<RecoverySummary, "submittedAt"> {
  readonly expiresAt: string | null;
  readonly attemptFactsText: string;
  readonly submissionText: string | null;
  readonly questions: ReadonlyArray<RecoveredQuestion>;
}
export interface RecoverySelection {
  readonly action: "select";
  readonly courseInstanceId: CourseInstanceId;
  readonly attempts: ReadonlyArray<RecoverySummary>;
  readonly pageNumber: number;
  readonly pageSize: number;
  readonly totalCount: number;
  readonly assessmentChoices: ReadonlyArray<{
    readonly key: AssessmentId;
    readonly label: string;
    readonly count: number;
  }>;
}
export interface RecoveryBrowse {
  readonly search: string;
  readonly assessmentIds: ReadonlyArray<AssessmentId>;
  readonly sort: "attemptAscending" | "attemptDescending";
  readonly pageNumber: number;
  readonly pageSize: 50 | 100;
}
export interface CourseStudentWorkRecoveryClient {
  readonly selectArchivedStudentWork: (
    courseInstanceId: CourseInstanceId,
    browse: RecoveryBrowse,
  ) => Promise<RecoverySelection>;
  readonly recoverArchivedStudentWork: (
    courseInstanceId: CourseInstanceId,
    assessmentAttemptId: AssessmentAttemptId,
  ) => Promise<RecoveredAttempt>;
}
