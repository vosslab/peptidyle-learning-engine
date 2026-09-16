import type { AssessmentAttemptReference } from "../../generated/api/AssessmentAttemptReference";
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { StudentQuestionPresentation } from "./decoders/presentation_delivery";

export type StudentAssessmentAttemptResponseState = "unanswered" | "saved" | "submitted" | "closed";

/** Answer-free durable navigation projection for one authorized Student Attempt. */
export interface StudentAssessmentAttemptProgress {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly questionCount: number;
  readonly recommendedPosition: number | null;
  readonly positions: ReadonlyArray<{
    readonly position: number;
    readonly responseState: StudentAssessmentAttemptResponseState;
  }>;
}

/** UUID-free display context authorized with an active Student Assessment Attempt. */
export interface StudentAssessmentAttemptContext {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly attemptNumber: number;
  readonly displayTimeZone: AccountTimeZone;
  readonly expiresAt: number | null;
  readonly timerRemainingMilliseconds: number | null;
  readonly course: {
    readonly reference: CourseInstanceReference;
    readonly shortName: string;
    readonly longName: string;
    readonly theme: CourseTheme;
  };
  readonly assessment: {
    readonly reference: AssessmentReference;
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
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly position: number;
  readonly responseState: "saved";
}

/** Immutable final-submission result for the whole Assessment Attempt. */
export interface StudentAssessmentAttemptSubmissionResult {
  readonly assessmentAttempt: AssessmentAttemptReference;
  readonly submissionState: "submitted";
}

export interface StudentAssessmentAttemptNavigationClient {
  readonly getStudentAssessmentAttemptContext: (
    assessmentAttempt: AssessmentAttemptReference,
  ) => Promise<StudentAssessmentAttemptContext>;
  readonly getStudentAssessmentAttemptProgress: (
    assessmentAttempt: AssessmentAttemptReference,
  ) => Promise<StudentAssessmentAttemptProgress>;
  readonly getStudentAssessmentAttemptPresentation: (
    assessmentAttempt: AssessmentAttemptReference,
    position: number,
  ) => Promise<StudentAssessmentAttemptPresentation>;
  readonly saveStudentAssessmentAttemptResponse: (
    assessmentAttempt: AssessmentAttemptReference,
    position: number,
    response: StudentResponse,
  ) => Promise<StudentAssessmentAttemptResponseSaveAcknowledgement>;
  readonly submitStudentAssessmentAttempt: (
    assessmentAttempt: AssessmentAttemptReference,
  ) => Promise<StudentAssessmentAttemptSubmissionResult>;
}
