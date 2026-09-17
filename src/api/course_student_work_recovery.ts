import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";

export interface RecoverySummary {
  readonly course: string;
  readonly rosterId: string | null;
  readonly assessment: string;
  readonly assessmentTitle: string;
  readonly assessmentAttempt: string;
  readonly assessmentAttemptNumber: number;
  readonly startedAt: string;
  readonly submittedAt: string | null;
  readonly studentDataArchivedAt: string;
  readonly deleteDueAt: string;
}
export interface RecoveredQuestion {
  readonly issuedPosition: number;
  readonly questionId: string;
  readonly revisionNumber: number;
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
  readonly course: string;
  readonly attempts: ReadonlyArray<RecoverySummary>;
  readonly nextCursor: string | null;
}
export interface CourseStudentWorkRecoveryClient {
  readonly selectArchivedStudentWork: (
    course: CourseInstanceReference,
    cursor: string | null,
  ) => Promise<RecoverySelection>;
  readonly recoverArchivedStudentWork: (
    course: CourseInstanceReference,
    assessmentAttempt: string,
  ) => Promise<RecoveredAttempt>;
}
