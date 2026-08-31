// client.ts - the only API shape consumed by browser routes and components.

import type { AssetId } from "../../generated/api/AssetId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentEntryId } from "../../generated/api/AssignmentEntryId";
import type { AssignmentAttempt } from "../../generated/api/AssignmentAttempt";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../generated/api/CourseAppearanceUpdate";
import type { CourseGradeSchemeView } from "../../generated/api/CourseGradeSchemeView";
import type { CourseGradeSchemeUpdateView } from "../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseGradebookTotalsView } from "../../generated/api/CourseGradebookTotalsView";
import type { CourseBannerCandidateReceipt } from "../../generated/api/CourseBannerCandidateReceipt";
import type { CourseBannerId } from "../../generated/api/CourseBannerId";
import type { StudentRecordId } from "../../generated/api/StudentRecordId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { AccountApprovalView } from "../../generated/api/AccountApprovalView";
import type { AccountReference } from "../../generated/api/AccountReference";
import type { SysadminInstructorCandidateSearchPage } from "../../generated/api/SysadminInstructorCandidateSearchPage";
import type { SysadminInstructorCandidateSearchRequest } from "../../generated/api/SysadminInstructorCandidateSearchRequest";
import type { InstructorCourseInvitationCreateRequest } from "../../generated/api/InstructorCourseInvitationCreateRequest";
import type { CourseInvitationReference } from "../../generated/api/CourseInvitationReference";
import type { CourseInvitationTerminalActionRequest } from "../../generated/api/CourseInvitationTerminalActionRequest";
import type { CourseInvitationTargetSearchPage } from "../../generated/api/CourseInvitationTargetSearchPage";
import type { CourseInvitationTargetSearchQuery } from "../../generated/api/CourseInvitationTargetSearchQuery";
import type { InstructorCourseInvitationsPage } from "../../generated/api/InstructorCourseInvitationsPage";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { CourseStudentMembershipsPage } from "../../generated/api/CourseStudentMembershipsPage";
import type { AccommodationAdjustmentUpdateRequest } from "../../generated/api/AccommodationAdjustmentUpdateRequest";
import type { InstructorMembershipRemovalRequest } from "../../generated/api/InstructorMembershipRemovalRequest";
import type { InstructorMembershipsPage } from "../../generated/api/InstructorMembershipsPage";
import type { PendingCourseInvitationsPage } from "../../generated/api/PendingCourseInvitationsPage";
import type { RetentionActionResponse } from "../../generated/api/RetentionActionResponse";
import type { RetentionArchiveRequest } from "../../generated/api/RetentionArchiveRequest";
import type { RetentionExtendRequest } from "../../generated/api/RetentionExtendRequest";
import type { RetentionReadView } from "../../generated/api/RetentionReadView";
import type { TeachingOperationRevision } from "../../generated/api/TeachingOperationRevision";
import type { TeachingOperationRevisionResponse } from "../../generated/api/TeachingOperationRevisionResponse";
import type { TeachingPreviewView } from "../../generated/api/TeachingPreviewView";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { DerivedPreviewSubjectRequest } from "../../generated/api/DerivedPreviewSubjectRequest";
import type { InstructorPreviewSchedulePage } from "../../generated/api/InstructorPreviewSchedulePage";
import type { PreviewPlaneResponse } from "../../generated/api/PreviewPlaneResponse";
import type { StudentViewScenarioRequest } from "../../generated/api/StudentViewScenarioRequest";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type { CourseRosterClient } from "./enrollment";
import type {
  AssignmentEditorDetail,
  AssignmentDraftInput,
  AssignmentContentInput,
  AssignmentPoliciesInput,
  InstructorStudentView,
  StudentAssignmentLandingSummary,
  StudentAssignmentDetail,
  StudentQuestionAttempt,
  QuestionSubmissionAcknowledgement,
  AuthenticatedSession,
  CourseCreateInput,
  CourseSummary,
  CursorPage,
  ExternalToolLaunch,
  FeedbackReleaseResponse,
  AssignmentAttemptScreenData,
  AssignmentAttemptSummaryResponse,
  WorkspaceDraftDetail,
  WorkspaceDraftPage,
  PublicationDiff,
  PublicationResult,
  PublicationRequest,
  PublicationValidationResponse,
  PrefetchedNextQuestion,
  PoolDrawPreview,
} from "./contracts";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { PublicRouteReference } from "../navigation/public_route";
import type { LiveDemoClient } from "./live_demo";
import type { QuestionCurationClient } from "./question_curation";
import type { BlueprintCourseClient } from "./blueprint_course";
import type { CurriculumAdoptionClient } from "./curriculum_adoption";
import type {
  GradingOperationActionId,
  GradingOperationActionReceipt,
  GradingOperationFocus,
  GradingOperationStrongEtag,
  InstructorGradingOperationsPage,
} from "./decoders/grading_operations";
import type { InstructorGradingOperationReference } from "../../generated/api/InstructorGradingOperationReference";
import type { AssignmentAttemptReference } from "../../generated/api/AssignmentAttemptReference";
import type {
  CalculatedGradebookQuery,
  CalculatedGradebookResult,
  InspectedStudentWorkDetail,
} from "./decoders/calculated_gradebook";
import type {
  GradebookSelectionQuery,
  GradebookSelectionResult,
  SubmittedRunChoicesPage,
  SubmittedRunChoicesQuery,
} from "./decoders/gradebook_selection";

/** Instructor-only browser capability for answer-free automated-grading recovery metadata. */
export interface GradingOperationsClient {
  readonly listInstructorGradingOperations: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    focus?: GradingOperationFocus,
    cursor?: string,
    pageSize?: number,
  ) => Promise<InstructorGradingOperationsPage>;
  readonly retryInstructorGradingOperation: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    operation: InstructorGradingOperationReference,
    expectedRevision: GradingOperationStrongEtag,
    idempotencyKey: GradingOperationActionId,
  ) => Promise<GradingOperationActionReceipt>;
  readonly recalculateInstructorAssignment: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    expectedRevision: GradingOperationStrongEtag,
    idempotencyKey: GradingOperationActionId,
  ) => Promise<GradingOperationActionReceipt>;
}

/** Instructor-only calculated Gradebook and audited Student-work capability. */
export interface CalculatedGradebookClient {
  readonly getCalculatedGradebook: (
    courseId: CourseId,
    query?: CalculatedGradebookQuery,
  ) => Promise<CalculatedGradebookResult>;
  readonly getGradebookSelection: (
    courseId: CourseId,
    query: GradebookSelectionQuery,
  ) => Promise<GradebookSelectionResult>;
  readonly getSubmittedRunChoices: (
    courseId: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    query?: SubmittedRunChoicesQuery,
  ) => Promise<SubmittedRunChoicesPage>;
  readonly getInspectedStudentWork: (
    courseId: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    run: AssignmentAttemptReference,
    operationRef?: InstructorGradingOperationReference,
  ) => Promise<InspectedStudentWorkDetail>;
}

/** Sysadmin-only discovery capability over generated public account references. */
export interface SysadminInstructorCandidateClient {
  readonly searchSysadminInstructorCandidates: (
    request: SysadminInstructorCandidateSearchRequest,
  ) => Promise<SysadminInstructorCandidateSearchPage>;
}

/** Browser-safe client contract implemented by the current same-origin HTTP transport. */
export interface ApiClient
  extends
    CourseRosterClient,
    QuestionCurationClient,
    BlueprintCourseClient,
    CurriculumAdoptionClient,
    GradingOperationsClient,
    CalculatedGradebookClient {
  readonly listCourseStudentTargets: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseStudentMembershipsPage>;
  readonly putAccommodation: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    student: CourseMembershipReference,
    request: AccommodationAdjustmentUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly deleteAccommodation: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    student: CourseMembershipReference,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly getTeachingPreview: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    student: CourseMembershipReference,
  ) => Promise<TeachingPreviewView>;
  /** Instructor-only T3 schedule projection using public C-/A- route references. */
  readonly listPreviewSchedule: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    cursor?: string,
    pageSize?: number,
  ) => Promise<InstructorPreviewSchedulePage>;
  /** Builds one identity-free synthetic subject and returns its server-resolved projection. */
  readonly constructSyntheticPreview: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    request: Omit<StudentViewScenarioRequest, "assignment" | "revision">,
  ) => Promise<PreviewPlaneResponse>;
  /** Resolves one authorized M- request locator into an identity-free derived projection. */
  readonly constructDerivedPreview: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    request: Omit<DerivedPreviewSubjectRequest, "assignment" | "revision">,
  ) => Promise<PreviewPlaneResponse>;
  /** Samples one saved Question Pool with server-owned entropy and no student activity. */
  readonly previewPoolDraw: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    assignmentEntryId: string,
  ) => Promise<PoolDrawPreview>;
  readonly approveInstructorAccount: (
    account: AccountReference,
    revision?: TeachingOperationRevision,
  ) => Promise<AccountApprovalView>;
  readonly revokeInstructorApproval: (
    account: AccountReference,
    revision: TeachingOperationRevision,
  ) => Promise<AccountApprovalView>;
  readonly listInstructorCourseInvitations: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<InstructorCourseInvitationsPage>;
  readonly searchInstructorCourseInvitationTargets: (
    courseId: CourseId,
    query: CourseInvitationTargetSearchQuery,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseInvitationTargetSearchPage>;
  readonly createInstructorCourseInvitation: (
    courseId: CourseId,
    request: InstructorCourseInvitationCreateRequest,
  ) => Promise<CourseInvitationReference>;
  readonly revokeInstructorCourseInvitation: (
    courseId: CourseId,
    invitation: CourseInvitationReference,
    revision: TeachingOperationRevision,
  ) => Promise<void>;
  readonly listPendingCourseInvitations: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<PendingCourseInvitationsPage>;
  readonly respondToCourseInvitation: (
    invitation: CourseInvitationReference,
    request: CourseInvitationTerminalActionRequest,
    revision: TeachingOperationRevision,
  ) => Promise<void>;
  readonly listCourseInstructors: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<InstructorMembershipsPage>;
  readonly removeCourseInstructor: (
    courseId: CourseId,
    membership: CourseMembershipReference,
    request: InstructorMembershipRemovalRequest,
    revision: TeachingOperationRevision,
  ) => Promise<void>;
  readonly getCourseRetention: (courseId: CourseId) => Promise<RetentionReadView>;
  readonly endCourseRetention: (courseId: CourseId) => Promise<RetentionReadView>;
  readonly archiveCourseRetention: (
    courseId: CourseId,
    request: RetentionArchiveRequest,
    revision: TeachingOperationRevision,
  ) => Promise<RetentionActionResponse>;
  readonly deleteCourseRetention: (
    courseId: CourseId,
    revision: TeachingOperationRevision,
  ) => Promise<RetentionActionResponse>;
  readonly extendCourseRetention: (
    courseId: CourseId,
    request: RetentionExtendRequest,
    revision: TeachingOperationRevision,
  ) => Promise<RetentionReadView>;
  readonly getCourseGradeScheme: (
    courseId: CourseId,
  ) => Promise<CourseGradeSchemeView & { readonly revision: string }>;
  readonly saveCourseGradeScheme: (
    courseId: CourseId,
    update: CourseGradeSchemeUpdateView,
    revision: string,
  ) => Promise<CourseGradeSchemeView & { readonly revision: string }>;
  readonly getCourseGradebookTotals: (courseId: CourseId) => Promise<CourseGradebookTotalsView>;
  readonly createCourseGradeExport: (
    courseId: CourseId,
  ) => Promise<{ readonly exportId: string; readonly filename: string; readonly csv: Blob }>;
  readonly getSession: () => Promise<AuthenticatedSession>;
  /** Resolves a compact visible reference inside the current authorization boundary. */
  readonly resolveNavigation: (reference: PublicRouteReference) => Promise<NavigationResolution>;
  /** Revokes the account credential for this browser. */
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
  readonly listQuestions: (cursor?: string) => Promise<CursorPage<QuestionSummary>>;
  /** Searches Question Library metadata with server-computed facets. */
  readonly searchQuestionLibrary: (query: QuestionSearchRequest) => Promise<QuestionSearchPage>;
  /** Resolves one copyable Instructor-facing ID to its exact answer-free Question Summary. */
  readonly resolveQuestion: (displayReference: string) => Promise<QuestionSummary>;
  /** Gets the safe immutable library projection, never a question definition. */
  readonly getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>;
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
  readonly listAssignments: (
    courseId: CourseId,
    cursor?: string,
  ) => Promise<CursorPage<StudentAssignmentLandingSummary>>;
  /** Student-safe detail; Instructor workspace reads require an exact course identity. */
  readonly getAssignment: (assignmentId: AssignmentId) => Promise<StudentAssignmentDetail>;
  /** Current key-free student progress; the server omits withheld score totals. */
  readonly getAssignmentSummary: (assignmentId: AssignmentId) => Promise<AssignmentProgress>;
  /** Reads the course-bound Instructor assignment workspace. */
  readonly getAssignmentWorkspace: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<AssignmentEditorDetail>;
  /** Creates a persisted empty Draft with server-owned defaults. */
  readonly createAssignmentDraft: (
    courseId: CourseId,
    input: AssignmentDraftInput,
  ) => Promise<AssignmentEditorDetail>;
  /** Replaces only Questions-owned title and ordered content. */
  readonly saveAssignmentContent: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    assignmentReference: AssignmentReference,
    input: AssignmentContentInput,
    assignmentRevisionEtag: string,
  ) => Promise<AssignmentEditorDetail>;
  /** Replaces one existing fixed slot for future runs without changing issued student work. */
  readonly replaceAssignmentFixedItem: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    assignmentReference: AssignmentReference,
    itemId: AssignmentEntryId,
    questionId: QuestionId,
    assignmentRevisionEtag: string,
  ) => Promise<AssignmentEditorDetail>;
  /** Replaces only Policies-owned audience, disclosure, run, and teaching settings. */
  readonly saveAssignmentPolicies: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    assignmentReference: AssignmentReference,
    input: AssignmentPoliciesInput,
    assignmentRevisionEtag: string,
  ) => Promise<AssignmentEditorDetail>;
  /** Reads the non-mutating, answer-free Instructor Student view. */
  readonly getInstructorStudentView: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<InstructorStudentView>;
  readonly listAssignmentAttempts: (
    studentRecordId: StudentRecordId,
    cursor?: string,
  ) => Promise<CursorPage<AssignmentAttempt>>;
  /**
   * Starts or resumes student work within the course route that authorizes the assignment.
   * The browser supplies no student-work authority or answer material.
   */
  readonly startAssignmentAttempt: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<AssignmentAttempt>;
  readonly getAssignmentAttempt: (
    assignmentAttemptId: AssignmentAttemptId,
  ) => Promise<AssignmentAttempt>;
  readonly getAssignmentAttemptSummary: (
    assignmentAttemptId: AssignmentAttemptId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<AssignmentAttemptSummaryResponse>;
  readonly listQuestionAttempts: (
    assignmentAttemptId: AssignmentAttemptId,
    cursor?: string,
  ) => Promise<CursorPage<StudentQuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<StudentQuestionAttempt>;
  /** Returns only the regenerated renderable variant; grading stays server-side. */
  readonly getIssuedQuestion: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionPresentation>;
  /** Best-effort key-free preparation; null means no deterministic successor. */
  readonly prefetchNextQuestion: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
    signal?: AbortSignal,
  ) => Promise<PrefetchedNextQuestion | null>;
  /** Creates a broker launch by same-origin POST, then returns its inert shell route. */
  readonly beginExternalToolLaunch: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<ExternalToolLaunch>;
  readonly submitResponse: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
    response: StudentResponse,
    idempotencyKey: string,
  ) => Promise<QuestionSubmissionAcknowledgement>;
  /** Reads a previously acknowledged student submission without resending answer material. */
  readonly getSubmissionStatus: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionSubmissionAcknowledgement>;
  /** Instructor command only; current feedback is read through a later summary GET. */
  readonly releaseAttemptFeedback: (
    attemptId: QuestionAttemptId,
  ) => Promise<FeedbackReleaseResponse>;
  readonly getAssignmentActivitySummary: (
    studentRecordId: StudentRecordId,
  ) => Promise<AssignmentProgress>;
  readonly getAssignmentAttemptScreen: (
    assignmentAttemptId: AssignmentAttemptId,
  ) => Promise<AssignmentAttemptScreenData>;
  /** Same-origin POST that authorizes, audits, and returns one normalized course banner. */
  readonly fetchCourseBanner: (bannerId: CourseBannerId) => Promise<Blob>;
  /** Public immutable Question Library asset redirect path; it never issues a capability. */
  readonly assetUrl: (assetId: AssetId) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly questionAttemptTimingDecisionOnServer: TimerEvaluator;
  readonly validateAssignmentConfigOnServer: CapabilityValidator;
}

/** The ordinary deployed browser composes HTTP capabilities without test-double transport. */
export interface OrdinaryBrowserApiClient
  extends ApiClient, LiveDemoClient, SysadminInstructorCandidateClient {}
