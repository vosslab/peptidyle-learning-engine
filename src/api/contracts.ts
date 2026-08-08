// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { RunId } from "../../generated/api/RunId";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { TenantId } from "../../generated/api/TenantId";
import type { UserId } from "../../generated/api/UserId";
import type { UserRole } from "../../generated/api/UserRole";

export type { AssignmentSummary, CourseSummary };

/** Cursor-paged API result. Offset pagination is intentionally absent. */
export interface CursorPage<T> {
  readonly items: ReadonlyArray<T>;
  readonly nextCursor: string | null;
}

/** Signed-in identity projection. Session credentials remain in an HttpOnly cookie. */
export interface AuthSession {
  readonly authenticated: true;
  readonly tenant: TenantId;
  readonly user: {
    readonly id: UserId;
    readonly displayName: string;
    readonly roles: ReadonlyArray<UserRole>;
  };
}

/** Enrollment and its transactionally maintained student summary. */
export interface EnrollmentView {
  readonly enrollment: AssignmentEnrollment;
  readonly summary: StudentAssignmentSummary;
}

/** Explicit acknowledgement of an idempotent response submission. */
export interface SubmissionReceipt {
  readonly accepted: true;
  readonly attempt: QuestionAttempt;
}

/** Everything the reference run screen needs from one cached query. */
export interface RunScreenData {
  readonly course: CourseSummary;
  readonly assignment: AssignmentSummary;
  readonly run: AssignmentRun;
  readonly attempt: QuestionAttempt;
  readonly question: QuestionDefinition;
}

/** Parameters needed to issue a fresh run. */
export interface StartRunRequest {
  readonly assignmentId: AssignmentId;
}

/** Run identity alias used where a return value is clearer than a full DTO. */
export type StartedRunId = RunId;
