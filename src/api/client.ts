// client.ts - the only API shape consumed by browser routes and components.

import type { AssetId } from "../../generated/api/AssetId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CourseId } from "../../generated/api/CourseId";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { ProblemId } from "../../generated/api/ProblemId";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { RunId } from "../../generated/api/RunId";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { VersionId } from "../../generated/api/VersionId";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type {
  AssignmentSummary,
  AuthSession,
  CourseSummary,
  CursorPage,
  EnrollmentView,
  RunScreenData,
  SubmissionReceipt,
} from "./contracts";

/** Browser-safe client contract. A future HTTP transport implements this interface. */
export interface ApiClient {
  readonly getSession: () => Promise<AuthSession>;
  readonly listProblems: (cursor?: string) => Promise<CursorPage<CatalogProblemSummary>>;
  readonly getProblemVersion: (
    problemId: ProblemId,
    versionId: VersionId,
  ) => Promise<QuestionDefinition>;
  readonly listTaxonomy: (cursor?: string) => Promise<CursorPage<TaxonomyTerm>>;
  readonly listCourses: (cursor?: string) => Promise<CursorPage<CourseSummary>>;
  readonly getCourse: (courseId: CourseId) => Promise<CourseSummary>;
  readonly listAssignments: (
    courseId: CourseId,
    cursor?: string,
  ) => Promise<CursorPage<AssignmentSummary>>;
  readonly getAssignment: (assignmentId: AssignmentId) => Promise<AssignmentSummary>;
  readonly getEnrollment: (enrollmentId: EnrollmentId) => Promise<EnrollmentView>;
  readonly listRuns: (
    enrollmentId: EnrollmentId,
    cursor?: string,
  ) => Promise<CursorPage<AssignmentRun>>;
  readonly startRun: (assignmentId: AssignmentId) => Promise<AssignmentRun>;
  readonly getRun: (runId: RunId) => Promise<AssignmentRun>;
  readonly listAttempts: (runId: RunId, cursor?: string) => Promise<CursorPage<QuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<QuestionAttempt>;
  readonly submitResponse: (
    attemptId: QuestionAttemptId,
    response: StudentResponse,
    idempotencyKey: string,
  ) => Promise<SubmissionReceipt>;
  readonly getSummary: (enrollmentId: EnrollmentId) => Promise<StudentAssignmentSummary>;
  readonly getRunScreen: (runId: RunId) => Promise<RunScreenData>;
  readonly assetUrl: (assetId: AssetId) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly timerVerdictOnServer: TimerEvaluator;
  readonly validateAssignmentConfigOnServer: CapabilityValidator;
}
