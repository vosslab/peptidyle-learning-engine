// client.ts - the only API shape consumed by browser routes and components.

import type { AssetId } from "../../generated/api/AssetId";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CatalogProblemDetail } from "../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseGroupCreateRequest } from "../../generated/api/CourseGroupCreateRequest";
import type { CourseGroupDetailView } from "../../generated/api/CourseGroupDetailView";
import type { CourseGroupListPage } from "../../generated/api/CourseGroupListPage";
import type { CourseGroupMembershipWarningView } from "../../generated/api/CourseGroupMembershipWarningView";
import type { CourseGroupPurpose } from "../../generated/api/CourseGroupPurpose";
import type { CourseGroupPurposePolicyUpdateRequest } from "../../generated/api/CourseGroupPurposePolicyUpdateRequest";
import type { CourseGroupPurposePolicyView } from "../../generated/api/CourseGroupPurposePolicyView";
import type { CourseGroupReference } from "../../generated/api/CourseGroupReference";
import type { CourseGroupSummaryView } from "../../generated/api/CourseGroupSummaryView";
import type { CourseGroupUpdateRequest } from "../../generated/api/CourseGroupUpdateRequest";
import type { CourseAppearance } from "../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../generated/api/CourseAppearanceUpdate";
import type { CourseGradeSchemeView } from "../../generated/api/CourseGradeSchemeView";
import type { CourseGradeSchemeUpdateView } from "../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseGradebookTotalsView } from "../../generated/api/CourseGradebookTotalsView";
import type { CourseBannerCandidateReceipt } from "../../generated/api/CourseBannerCandidateReceipt";
import type { CourseBannerId } from "../../generated/api/CourseBannerId";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { RunId } from "../../generated/api/RunId";
import type { LearnerAssignmentProgress } from "../../generated/api/LearnerAssignmentProgress";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { AccountApprovalView } from "../../generated/api/AccountApprovalView";
import type { AccountReference } from "../../generated/api/AccountReference";
import type { SysadminInstructorCandidateSearchPage } from "../../generated/api/SysadminInstructorCandidateSearchPage";
import type { SysadminInstructorCandidateSearchRequest } from "../../generated/api/SysadminInstructorCandidateSearchRequest";
import type { AssignmentPolicyPatchUpdateRequest } from "../../generated/api/AssignmentPolicyPatchUpdateRequest";
import type { CoInstructorInvitationCreateRequest } from "../../generated/api/CoInstructorInvitationCreateRequest";
import type { CoInstructorInvitationReference } from "../../generated/api/CoInstructorInvitationReference";
import type { CoInstructorInvitationTerminalActionRequest } from "../../generated/api/CoInstructorInvitationTerminalActionRequest";
import type { CoInstructorTargetSearchPage } from "../../generated/api/CoInstructorTargetSearchPage";
import type { CoInstructorTargetSearchQuery } from "../../generated/api/CoInstructorTargetSearchQuery";
import type { CourseCoInstructorInvitationsPage } from "../../generated/api/CourseCoInstructorInvitationsPage";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { CourseStudentMembershipsPage } from "../../generated/api/CourseStudentMembershipsPage";
import type { GroupScheduleOffsetUpdateRequest } from "../../generated/api/GroupScheduleOffsetUpdateRequest";
import type { IndividualPolicyPatchUpdateRequest } from "../../generated/api/IndividualPolicyPatchUpdateRequest";
import type { InstructorMembershipRemovalRequest } from "../../generated/api/InstructorMembershipRemovalRequest";
import type { InstructorMembershipsPage } from "../../generated/api/InstructorMembershipsPage";
import type { PendingCoInstructorInvitationsPage } from "../../generated/api/PendingCoInstructorInvitationsPage";
import type { RetentionActionResponse } from "../../generated/api/RetentionActionResponse";
import type { RetentionArchiveRequest } from "../../generated/api/RetentionArchiveRequest";
import type { RetentionExtendRequest } from "../../generated/api/RetentionExtendRequest";
import type { RetentionReadView } from "../../generated/api/RetentionReadView";
import type { TeachingOperationRevision } from "../../generated/api/TeachingOperationRevision";
import type { TeachingOperationRevisionResponse } from "../../generated/api/TeachingOperationRevisionResponse";
import type { TeachingPreviewView } from "../../generated/api/TeachingPreviewView";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { DerivedPreviewSubjectRequest } from "../../generated/api/DerivedPreviewSubjectRequest";
import type { InstructorPreviewSchedulePage } from "../../generated/api/InstructorPreviewSchedulePage";
import type { PreviewPlaneResponse } from "../../generated/api/PreviewPlaneResponse";
import type { SyntheticPreviewSubjectRequest } from "../../generated/api/SyntheticPreviewSubjectRequest";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type { CourseRosterClient } from "./enrollment";
import type {
  AssignmentEditorDetail,
  AssignmentCreateInput,
  AssignmentEditorInput,
  AddAssignmentItemInput,
  ReplaceAssignmentItemQuestionInput,
  LearnerAssignmentSummary,
  LearnerAssignmentDetail,
  LearnerQuestionAttempt,
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
  PoolDrawPreview,
} from "./contracts";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { PublicRouteReference } from "../navigation/public_route";
import type { LiveDemoClient } from "./live_demo";
import type { ProblemCurationClient } from "./problem_curation";
import type { ReusableCurriculumClient } from "./reusable_curriculum";
import type { CurriculumAdoptionClient } from "./curriculum_adoption";

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
    ProblemCurationClient,
    ReusableCurriculumClient,
    CurriculumAdoptionClient {
  readonly listCourseGroups: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseGroupListPage>;
  readonly getCourseGroup: (
    courseId: CourseId,
    group: CourseGroupReference,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseGroupDetailView>;
  readonly createCourseGroup: (
    courseId: CourseId,
    request: CourseGroupCreateRequest,
  ) => Promise<CourseGroupSummaryView>;
  readonly updateCourseGroup: (
    courseId: CourseId,
    group: CourseGroupReference,
    request: CourseGroupUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<CourseGroupSummaryView>;
  readonly deleteCourseGroup: (
    courseId: CourseId,
    group: CourseGroupReference,
    revision: TeachingOperationRevision,
  ) => Promise<void>;
  readonly getCourseGroupPurposePolicy: (
    courseId: CourseId,
    purpose: CourseGroupPurpose,
  ) => Promise<CourseGroupPurposePolicyView>;
  readonly updateCourseGroupPurposePolicy: (
    courseId: CourseId,
    purpose: CourseGroupPurpose,
    request: CourseGroupPurposePolicyUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<CourseGroupPurposePolicyView>;
  readonly getCourseGroupMembershipWarnings: (
    courseId: CourseId,
  ) => Promise<CourseGroupMembershipWarningView>;
  readonly listCourseStudentTargets: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseStudentMembershipsPage>;
  readonly putGroupScheduleOffset: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    group: CourseGroupReference,
    request: GroupScheduleOffsetUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly deleteGroupScheduleOffset: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    group: CourseGroupReference,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly putGroupAccommodation: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    group: CourseGroupReference,
    request: AssignmentPolicyPatchUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly deleteGroupAccommodation: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    group: CourseGroupReference,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly putIndividualPolicyException: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    student: CourseMembershipReference,
    request: IndividualPolicyPatchUpdateRequest,
    revision: TeachingOperationRevision,
  ) => Promise<TeachingOperationRevisionResponse>;
  readonly deleteIndividualPolicyException: (
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
    course: CourseReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    cursor?: string,
    pageSize?: number,
  ) => Promise<InstructorPreviewSchedulePage>;
  /** Builds one identity-free synthetic subject and returns its server-resolved projection. */
  readonly constructSyntheticPreview: (
    course: CourseReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    request: Omit<SyntheticPreviewSubjectRequest, "assignment" | "revision">,
  ) => Promise<PreviewPlaneResponse>;
  /** Resolves one authorized M- request locator into an identity-free derived projection. */
  readonly constructDerivedPreview: (
    course: CourseReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    request: Omit<DerivedPreviewSubjectRequest, "assignment" | "revision">,
  ) => Promise<PreviewPlaneResponse>;
  /** Samples one saved item pool with server-owned entropy and no learner activity. */
  readonly previewPoolDraw: (
    course: CourseReference,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    groupPosition: number,
  ) => Promise<PoolDrawPreview>;
  readonly approveInstructorAccount: (
    account: AccountReference,
    revision?: TeachingOperationRevision,
  ) => Promise<AccountApprovalView>;
  readonly revokeInstructorApproval: (
    account: AccountReference,
    revision: TeachingOperationRevision,
  ) => Promise<AccountApprovalView>;
  readonly listCourseCoInstructorInvitations: (
    courseId: CourseId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CourseCoInstructorInvitationsPage>;
  readonly searchCourseCoInstructorTargets: (
    courseId: CourseId,
    query: CoInstructorTargetSearchQuery,
    cursor?: string,
    pageSize?: number,
  ) => Promise<CoInstructorTargetSearchPage>;
  readonly createCourseCoInstructorInvitation: (
    courseId: CourseId,
    request: CoInstructorInvitationCreateRequest,
  ) => Promise<CoInstructorInvitationReference>;
  readonly revokeCourseCoInstructorInvitation: (
    courseId: CourseId,
    invitation: CoInstructorInvitationReference,
    revision: TeachingOperationRevision,
  ) => Promise<void>;
  readonly listPendingCoInstructorInvitations: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<PendingCoInstructorInvitationsPage>;
  readonly respondToCoInstructorInvitation: (
    invitation: CoInstructorInvitationReference,
    request: CoInstructorInvitationTerminalActionRequest,
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
  ) => Promise<CursorPage<LearnerAssignmentSummary>>;
  /** Learner-safe detail; the editor uses getAssignmentEditor instead. */
  readonly getAssignment: (assignmentId: AssignmentId) => Promise<LearnerAssignmentDetail>;
  /** Current key-free learner progress; the server omits withheld score totals. */
  readonly getAssignmentSummary: (assignmentId: AssignmentId) => Promise<LearnerAssignmentProgress>;
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
  /** Atomically saves lifecycle, instructions, and course-local delivery policy. */
  readonly saveAssignmentTeachingSettings: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    settings: InstructorAssignmentTeachingSettingsLocal,
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
  /**
   * Starts or resumes learner work within the course route that authorizes the assignment.
   * The browser supplies no learner-work authority or answer material.
   */
  readonly startRun: (courseId: CourseId, assignmentId: AssignmentId) => Promise<AssignmentRun>;
  readonly getRun: (runId: RunId) => Promise<AssignmentRun>;
  readonly getRunSummary: (
    runId: RunId,
    cursor?: string,
    pageSize?: number,
  ) => Promise<RunSummaryResponse>;
  readonly listAttempts: (
    runId: RunId,
    cursor?: string,
  ) => Promise<CursorPage<LearnerQuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<LearnerQuestionAttempt>;
  /** Returns only the regenerated renderable variant; grading stays server-side. */
  readonly getIssuedQuestion: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionEnvelope>;
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
  ) => Promise<SubmissionReceipt>;
  /** Instructor command only; current feedback is read through a later summary GET. */
  readonly releaseAttemptFeedback: (
    attemptId: QuestionAttemptId,
  ) => Promise<FeedbackReleaseResponse>;
  readonly getSummary: (enrollmentId: EnrollmentId) => Promise<LearnerAssignmentProgress>;
  readonly getRunScreen: (runId: RunId) => Promise<RunScreenData>;
  /** Same-origin POST that authorizes, audits, and returns one normalized course banner. */
  readonly fetchCourseBanner: (bannerId: CourseBannerId) => Promise<Blob>;
  /** Public immutable catalog-asset redirect path; it never issues a capability. */
  readonly assetUrl: (assetId: AssetId) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly timerVerdictOnServer: TimerEvaluator;
  readonly validateAssignmentConfigOnServer: CapabilityValidator;
}

/** The ordinary deployed browser composes HTTP capabilities without test-double transport. */
export interface OrdinaryBrowserApiClient
  extends ApiClient, LiveDemoClient, SysadminInstructorCandidateClient {}
