// contracts.ts - browser-safe DTOs at the transport boundary (MOD-CLIENT).

import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { ProblemId } from "../../generated/api/ProblemId";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { RunId } from "../../generated/api/RunId";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentId } from "../../generated/api/StudentId";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { TenantId } from "../../generated/api/TenantId";
import type { VersionId } from "../../generated/api/VersionId";

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
    readonly id: StudentId;
    readonly displayName: string;
  };
}

/** Course information sufficient for the signed-in landing page. */
export interface CourseSummary {
  readonly id: string;
  readonly tenant: TenantId;
  readonly title: string;
}

/** Immutable problem/version pair selected by an assignment. */
export interface PublishedVersionReference {
  readonly problem: ProblemId;
  readonly version: VersionId;
}

/** Assignment projection used before the full course routes are implemented. */
export interface AssignmentSummary {
  readonly id: AssignmentId;
  readonly tenant: TenantId;
  readonly courseId: string;
  readonly title: string;
  readonly problems: ReadonlyArray<PublishedVersionReference>;
  readonly policies: RunPolicies;
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

/** Catalog taxonomy response used by the typed client surface. */
export interface TaxonomyPage {
  readonly items: ReadonlyArray<TaxonomyTerm>;
}

/** Parameters needed to issue a fresh run. */
export interface StartRunRequest {
  readonly assignmentId: AssignmentId;
}

/** Run identity alias used where a return value is clearer than a full DTO. */
export type StartedRunId = RunId;
