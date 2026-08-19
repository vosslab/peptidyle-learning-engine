// client.ts - the only API shape consumed by browser routes and components.

import type { AssetId } from "../../generated/api/AssetId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CatalogProblemDetail } from "../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerCandidateReceipt } from "../../generated/api/CourseBannerCandidateReceipt";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { RunId } from "../../generated/api/RunId";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type { CourseRosterClient } from "./enrollment";
import type {
  AssignmentEditorDetail,
  AssignmentCreateInput,
  AssignmentEditorInput,
  AssignmentSummaryWithTiming,
  AddAssignmentItemInput,
  ReplaceAssignmentItemQuestionInput,
  AssignmentSummary,
  AuthSession,
  CourseCreateInput,
  CourseSummary,
  CursorPage,
  EnrollmentView,
  ExternalToolLaunch,
  FeedbackReleaseResponse,
  RunScreenData,
  RunSummaryResponse,
  SubmissionReceipt,
  WorkspaceDraftDetail,
  WorkspaceDraftPage,
  PublicationDiff,
  PublicationResult,
  PublicationRequest,
  PublicationValidationResponse,
  PrefetchedNextQuestion,
} from "./contracts";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { PublicRouteReference } from "../navigation/public_route";

/** Browser-safe client contract. A future HTTP transport implements this interface. */
export interface ApiClient extends CourseRosterClient {
  readonly getSession: () => Promise<AuthSession>;
  /** Resolves a compact visible reference inside the current authorization boundary. */
  readonly resolveNavigation: (reference: PublicRouteReference) => Promise<NavigationResolution>;
  /** Revokes both account and tenant credentials for this browser. */
  readonly logout: () => Promise<void>;
  readonly listWorkspaceDrafts: (cursor?: string) => Promise<WorkspaceDraftPage>;
  readonly getWorkspaceDraft: (workspace: WorkspaceId) => Promise<WorkspaceDraftDetail>;
  readonly saveWorkspaceDraft: (
    workspace: WorkspaceId,
    draft: DraftQuestionDefinition,
    revision?: string,
  ) => Promise<WorkspaceDraftDetail>;
  readonly deleteWorkspaceDraft: (workspace: WorkspaceId, revision: string) => Promise<void>;
  readonly validateWorkspacePublication: (
    workspace: WorkspaceId,
  ) => Promise<PublicationValidationResponse>;
  readonly getWorkspacePublicationDiff: (workspace: WorkspaceId) => Promise<PublicationDiff>;
  readonly publishWorkspace: (
    workspace: WorkspaceId,
    request: PublicationRequest,
    revision: string,
  ) => Promise<PublicationResult>;
  readonly listProblems: (cursor?: string) => Promise<CursorPage<CatalogProblemSummary>>;
  /** Searches immutable hot catalog metadata with server-computed facets. */
  readonly searchCatalog: (query: CatalogSearchQuery) => Promise<CatalogSearchPage>;
  /** Resolves one copyable instructor-facing ID to its exact safe catalog summary. */
  readonly resolveCatalogProblem: (displayReference: string) => Promise<CatalogProblemSummary>;
  /** Gets the safe immutable library projection, never a question definition. */
  readonly getCatalogProblemDetail: (questionId: QuestionId) => Promise<CatalogProblemDetail>;
  readonly listTaxonomy: (cursor?: string) => Promise<CursorPage<TaxonomyTerm>>;
  readonly listCourses: (cursor?: string) => Promise<CursorPage<CourseSummary>>;
  /** Creates one course for an authenticated instructor or sysadmin. */
  readonly createCourse: (input: CourseCreateInput) => Promise<CourseSummary>;
  readonly getCourse: (courseId: CourseId) => Promise<CourseSummary>;
  /** Gets only the current authorized, browser-safe course appearance. */
  readonly getCourseAppearance: (courseId: CourseId) => Promise<CourseAppearance>;
  /** Uploads opaque image bytes and returns only a course-bound temporary candidate identity. */
  readonly uploadCourseBannerCandidate: (
    courseId: CourseId,
    image: Blob,
  ) => Promise<CourseBannerCandidateReceipt>;
  /** Atomically saves the complete appearance using the last observed strong revision. */
  readonly saveCourseAppearance: (
    courseId: CourseId,
    update: CourseAppearanceUpdate,
    revision: string,
  ) => Promise<CourseAppearance>;
  /**
   * Instructor gradebook projection. This cursor-paged route never loads
   * historical runs or question attempts.
   */
  readonly listGradebook: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<GradebookSummaryRow>>;
  readonly listAssignments: (
    courseId: CourseId,
    cursor?: string,
  ) => Promise<CursorPage<AssignmentSummary>>;
  readonly getAssignment: (assignmentId: AssignmentId) => Promise<AssignmentSummaryWithTiming>;
  readonly getAssignmentSummary: (assignmentId: AssignmentId) => Promise<StudentAssignmentSummary>;
  /** Instructor-only revisioned assignment projection for the policy editor. */
  readonly getAssignmentEditor: (assignmentId: AssignmentId) => Promise<AssignmentEditorDetail>;
  /** Creates a tenant-owned assignment in the course named only by the path. */
  readonly createAssignment: (
    courseId: CourseId,
    input: AssignmentCreateInput,
  ) => Promise<AssignmentEditorDetail>;
  /** Replaces an assignment using its most recently observed strong ETag. */
  readonly saveAssignment: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    input: AssignmentEditorInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly addAssignmentItem: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    input: AddAssignmentItemInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly removeAssignmentItem: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    itemId: string,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly replaceAssignmentItemQuestion: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    itemId: string,
    input: ReplaceAssignmentItemQuestionInput,
    revision: string,
  ) => Promise<AssignmentEditorDetail>;
  readonly getEnrollment: (enrollmentId: EnrollmentId) => Promise<EnrollmentView>;
  readonly listRuns: (
    enrollmentId: EnrollmentId,
    cursor?: string,
  ) => Promise<CursorPage<AssignmentRun>>;
  readonly startRun: (assignmentId: AssignmentId) => Promise<AssignmentRun>;
  readonly getRun: (runId: RunId) => Promise<AssignmentRun>;
  readonly getRunSummary: (
    runId: RunId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<RunSummaryResponse>;
  readonly listAttempts: (runId: RunId, cursor?: string) => Promise<CursorPage<QuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<QuestionAttempt>;
  /** Returns only the regenerated renderable variant; grading stays server-side. */
  readonly getIssuedQuestion: (attemptId: QuestionAttemptId) => Promise<QuestionEnvelope>;
  /** Best-effort key-free preparation; null means no deterministic successor. */
  readonly prefetchNextQuestion: (
    attemptId: QuestionAttemptId,
    signal?: AbortSignal,
  ) => Promise<PrefetchedNextQuestion | null>;
  /** Creates a broker launch by same-origin POST, then returns its inert shell route. */
  readonly beginExternalToolLaunch: (attemptId: QuestionAttemptId) => Promise<ExternalToolLaunch>;
  readonly submitResponse: (
    attemptId: QuestionAttemptId,
    response: StudentResponse,
    idempotencyKey: string,
  ) => Promise<SubmissionReceipt>;
  /** Instructor command only; current feedback is read through a later summary GET. */
  readonly releaseAttemptFeedback: (
    attemptId: QuestionAttemptId,
  ) => Promise<FeedbackReleaseResponse>;
  readonly getSummary: (enrollmentId: EnrollmentId) => Promise<StudentAssignmentSummary>;
  readonly getRunScreen: (runId: RunId) => Promise<RunScreenData>;
  /** Same-origin POST that authorizes one protected asset and returns its short-lived URL. */
  readonly issueProtectedAssetDelivery: (assetId: AssetId) => Promise<string>;
  /** Public immutable catalog-asset redirect path; it never issues a capability. */
  readonly assetUrl: (assetId: AssetId) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly timerVerdictOnServer: TimerEvaluator;
  readonly validateAssignmentConfigOnServer: CapabilityValidator;
}
