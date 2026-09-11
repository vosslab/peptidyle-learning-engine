import type { AssignmentAttemptReference } from "../../generated/api/AssignmentAttemptReference";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { StudentQuestionPresentation } from "./decoders/presentation_delivery";

export type StudentAssignmentAttemptResponseState = "unanswered" | "saved" | "submitted" | "closed";

/** Answer-free durable navigation projection for one authorized Student Attempt. */
export interface StudentAssignmentAttemptProgress {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly questionCount: number;
  readonly recommendedPosition: number | null;
  readonly positions: ReadonlyArray<{
    readonly position: number;
    readonly responseState: StudentAssignmentAttemptResponseState;
  }>;
}

/** UUID-free display context authorized with an active Student Assignment Attempt. */
export interface StudentAssignmentAttemptContext {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly attemptNumber: number;
  readonly timerRemainingMilliseconds: number | null;
  readonly course: {
    readonly reference: CourseInstanceReference;
    readonly title: string;
    readonly theme: CourseTheme;
  };
  readonly assignment: {
    readonly reference: AssignmentReference;
    readonly title: string;
  };
}

/** One server-authorized immutable presentation selected by a 1-based position. */
export interface StudentAssignmentAttemptPresentation {
  readonly position: number;
  readonly presentation: StudentQuestionPresentation;
  /** The authenticated Student's current working response, never answer or grading data. */
  readonly savedResponse: StudentResponse | null;
}

/** Durable-save receipt for one owned Question position in an Assignment Attempt. */
export interface StudentAssignmentAttemptResponseSaveAcknowledgement {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly position: number;
  readonly responseState: "saved";
}

/** Immutable final-submission receipt for the whole Assignment Attempt. */
export interface StudentAssignmentAttemptSubmissionAcknowledgement {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly submissionState: "submitted";
}

export interface StudentAssignmentAttemptNavigationClient {
  readonly getStudentAssignmentAttemptContext: (
    assignmentAttempt: AssignmentAttemptReference,
  ) => Promise<StudentAssignmentAttemptContext>;
  readonly getStudentAssignmentAttemptProgress: (
    assignmentAttempt: AssignmentAttemptReference,
  ) => Promise<StudentAssignmentAttemptProgress>;
  readonly getStudentAssignmentAttemptPresentation: (
    assignmentAttempt: AssignmentAttemptReference,
    position: number,
  ) => Promise<StudentAssignmentAttemptPresentation>;
  readonly saveStudentAssignmentAttemptResponse: (
    assignmentAttempt: AssignmentAttemptReference,
    position: number,
    response: StudentResponse,
  ) => Promise<StudentAssignmentAttemptResponseSaveAcknowledgement>;
  readonly submitStudentAssignmentAttempt: (
    assignmentAttempt: AssignmentAttemptReference,
  ) => Promise<StudentAssignmentAttemptSubmissionAcknowledgement>;
}
