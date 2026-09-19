import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { StudentQuestionPresentation } from "./decoders/presentation_delivery";

export type StudentAssessmentAttemptResponseState = "unanswered" | "saved" | "submitted" | "closed";

/** Answer-free durable navigation projection for one authorized Student Attempt. */
export interface StudentAssessmentAttemptProgress {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly questionCount: number;
  readonly recommendedPosition: number | null;
  readonly positions: ReadonlyArray<{
    readonly position: number;
    readonly responseState: StudentAssessmentAttemptResponseState;
  }>;
}

/** UUID-free display context authorized with an active Student Assessment Attempt. */
export interface StudentAssessmentAttemptContext {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly attemptNumber: number;
  readonly displayTimeZone: AccountTimeZone;
  readonly expiresAt: number | null;
  readonly timerRemainingMilliseconds: number | null;
  readonly course: {
    readonly id: CourseInstanceId;
    readonly shortName: string;
    readonly longName: string;
    readonly theme: CourseTheme;
  };
  readonly assessment: {
    readonly id: AssessmentId;
    readonly title: string;
  };
}

/** One server-authorized immutable presentation selected by a 1-based position. */
export interface StudentAssessmentAttemptPresentation {
  readonly position: number;
  readonly presentation: StudentQuestionPresentation;
  /** The authenticated Student's current working response, never answer or grading data. */
  readonly savedResponse: StudentResponse | null;
}

/** Durable-save receipt for one owned Question position in an Assessment Attempt. */
export interface StudentAssessmentAttemptResponseSaveAcknowledgement {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly position: number;
  readonly responseState: "saved";
}

/** Immutable final-submission result for the whole Assessment Attempt. */
export interface StudentAssessmentAttemptSubmissionResult {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly submissionState: "submitted";
}

export interface StudentAssessmentAttemptNavigationClient {
  readonly getStudentAssessmentAttemptContext: (
    assessmentAttempt: AssessmentAttemptId,
  ) => Promise<StudentAssessmentAttemptContext>;
  readonly getStudentAssessmentAttemptProgress: (
    assessmentAttempt: AssessmentAttemptId,
  ) => Promise<StudentAssessmentAttemptProgress>;
  readonly getStudentAssessmentAttemptPresentation: (
    assessmentAttempt: AssessmentAttemptId,
    position: number,
  ) => Promise<StudentAssessmentAttemptPresentation>;
  readonly studentAuthorContentDocumentUrl: (
    assessmentAttempt: AssessmentAttemptId,
    position: number,
  ) => string;
  readonly saveStudentAssessmentAttemptResponse: (
    assessmentAttempt: AssessmentAttemptId,
    position: number,
    response: StudentResponse,
  ) => Promise<StudentAssessmentAttemptResponseSaveAcknowledgement>;
  readonly submitStudentAssessmentAttempt: (
    assessmentAttempt: AssessmentAttemptId,
  ) => Promise<StudentAssessmentAttemptSubmissionResult>;
}
