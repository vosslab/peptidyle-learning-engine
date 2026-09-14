// client.ts - the only API shape consumed by browser routes and components.

import type { QuestionAssetId } from "../../generated/api/QuestionAssetId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentAttempt } from "../../generated/api/AssignmentAttempt";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { CourseThemeUpdate } from "../../generated/api/CourseThemeUpdate";
import type { CourseBannerReference } from "../../generated/api/CourseBannerReference";
import type { CourseBannerUpdate } from "../../generated/api/CourseBannerUpdate";
import type { CourseBannerUploadReceipt } from "../../generated/api/CourseBannerUploadReceipt";
import type {
  InstructorProfile,
  InstructorProfileThumbnail,
  UpdateInstructorProfileInput,
} from "./instructor_profile";
import type { StudentRecordId } from "../../generated/api/StudentRecordId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { StudentAssignmentProgress } from "../../generated/api/StudentAssignmentProgress";
import type { CourseInvitationReference } from "../../generated/api/CourseInvitationReference";
import type { CourseInvitationTerminalActionRequest } from "../../generated/api/CourseInvitationTerminalActionRequest";
import type { PendingCourseInvitationsPage } from "../../generated/api/PendingCourseInvitationsPage";
import type { CourseInvitationStatePrecondition } from "../../generated/api/CourseInvitationStatePrecondition";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type { CourseRosterClient } from "./enrollment";
import type {
  AssignmentEditorDetail,
  AssignmentCreateInput,
  AssignmentContentInput,
  InstructorStudentView,
  StudentAssignmentLandingSummary,
  StudentAssignmentDetail,
  StudentQuestionAttempt,
  AuthenticatedSession,
  CourseSummary,
  CursorPage,
  ImathasQuestionBackendLaunch,
  StudentFeedbackReleaseResponse,
} from "./contracts";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { PublicRouteReference } from "../navigation/public_route";
import type { LiveDemoClient } from "./live_demo";
import type { BlueprintCourseClient } from "./blueprint_course";
import type { CourseInstanceClient } from "./course_instance";
import type { LiveCourseRosterClient } from "./course_roster";
import type { LiveInvitationExportClient } from "./invitation_export";
import type { LiveAssignmentReleaseClient } from "./assignment_release";
import type { LiveAssignmentAttemptIssuanceClient } from "./assignment_attempt_issuance";
import type { StudentAssignmentAttemptHistoryClient } from "./assignment_attempt_history";
import type { StudentAssignmentAttemptNavigationClient } from "./assignment_attempt_navigation";
import type { InstructorAccountClient } from "./instructor_account";
import type { SupportCapabilityClient } from "./support_roster";
import type { CourseGradebookClient } from "./live_gradebook";
import type { LiveStudentCourseLandingClient } from "./live_student_course_landing";
import type { QuestionAvailabilityClient } from "./question_availability";
/** Browser-safe client contract implemented by the current same-origin HTTP transport. */
export interface ApiClient
  extends
    CourseRosterClient,
    BlueprintCourseClient,
    CourseInstanceClient,
    LiveCourseRosterClient,
    LiveInvitationExportClient,
    LiveAssignmentReleaseClient,
    LiveAssignmentAttemptIssuanceClient,
    StudentAssignmentAttemptHistoryClient,
    StudentAssignmentAttemptNavigationClient,
    InstructorAccountClient,
    SupportCapabilityClient,
    CourseGradebookClient,
    LiveStudentCourseLandingClient,
    QuestionAvailabilityClient {
  /** Reads only the authenticated Instructor's account-owned display zone. */
  readonly getInstructorProfile: () => Promise<InstructorProfile>;
  /** Replaces only the authenticated Instructor's account-owned display zone. */
  readonly updateInstructorProfile: (
    input: UpdateInstructorProfileInput,
  ) => Promise<InstructorProfile>;
  readonly getInstructorProfileThumbnail: () => Promise<InstructorProfileThumbnail>;
  readonly replaceInstructorProfileThumbnail: (image: Blob) => Promise<InstructorProfileThumbnail>;
  readonly fetchInstructorProfileThumbnail: (reference: string) => Promise<Blob>;
  readonly listPendingCourseInvitations: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<PendingCourseInvitationsPage>;
  readonly respondToCourseInvitation: (
    invitation: CourseInvitationReference,
    request: CourseInvitationTerminalActionRequest,
    statePrecondition: CourseInvitationStatePrecondition,
  ) => Promise<void>;
  readonly getSession: () => Promise<AuthenticatedSession>;
  /** Resolves a compact visible reference inside the current authorization boundary. */
  readonly resolveNavigation: (reference: PublicRouteReference) => Promise<NavigationResolution>;
  /** Revokes the account credential for this browser. */
  readonly logout: () => Promise<void>;
  readonly listQuestions: (cursor?: string) => Promise<CursorPage<QuestionSummary>>;
  /** Searches Question Library metadata with server-computed facets. */
  readonly searchQuestionLibrary: (query: QuestionSearchRequest) => Promise<QuestionSearchPage>;
  /** Resolves one copyable Instructor-facing ID to its exact answer-free Question Summary. */
  readonly resolveQuestion: (displayReference: string) => Promise<QuestionSummary>;
  /** Gets the safe immutable Question Details View, never a complete Question Revision. */
  readonly getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>;
  readonly listCourses: (cursor?: string) => Promise<CursorPage<CourseSummary>>;
  readonly getCourse: (courseId: CourseId) => Promise<CourseSummary>;
  /** Gets only the authorized current Course Appearance View. */
  readonly getCourseAppearanceView: (courseId: CourseId) => Promise<CourseAppearanceView>;
  /** Saves one independent Course Theme and returns the current aggregate appearance. */
  readonly updateCourseTheme: (
    courseId: CourseId,
    update: CourseThemeUpdate,
  ) => Promise<CourseAppearanceView>;
  /** Stages raw verified banner bytes for this exact Instructor and Course. */
  readonly uploadCourseBanner: (
    courseId: CourseId,
    image: Blob,
  ) => Promise<CourseBannerUploadReceipt>;
  /** Promotes one staged banner independently of the Course Theme. */
  readonly setCourseBanner: (
    courseId: CourseId,
    update: CourseBannerUpdate,
  ) => Promise<CourseAppearanceView>;
  /** Removes only the current Course Banner. */
  readonly removeCourseBanner: (courseId: CourseId) => Promise<CourseAppearanceView>;
  readonly listAssignments: (
    courseId: CourseId,
    cursor?: string,
  ) => Promise<CursorPage<StudentAssignmentLandingSummary>>;
  /** Student-safe detail; Instructor workspace reads require an exact course identity. */
  readonly getAssignment: (assignmentId: AssignmentId) => Promise<StudentAssignmentDetail>;
  /** Current key-free student progress; the server omits withheld score totals. */
  readonly getAssignmentSummary: (assignmentId: AssignmentId) => Promise<StudentAssignmentProgress>;
  /** Reads the course-bound Instructor assignment workspace. */
  readonly getAssignmentWorkspace: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<AssignmentEditorDetail>;
  /** Creates a persisted empty Assignment with server-owned defaults. */
  readonly createAssignment: (
    courseId: CourseId,
    input: AssignmentCreateInput,
  ) => Promise<AssignmentEditorDetail>;
  /** Replaces only Questions-owned title and ordered content. */
  readonly saveAssignmentContent: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    assignmentReference: AssignmentReference,
    input: AssignmentContentInput,
    assignmentEtag: string,
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
   * The browser supplies no student-work authority or Answer Key.
   */
  readonly startAssignmentAttempt: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<AssignmentAttempt>;
  readonly getAssignmentAttempt: (
    assignmentAttemptId: AssignmentAttemptId,
  ) => Promise<AssignmentAttempt>;
  readonly listQuestionAttempts: (
    assignmentAttemptId: AssignmentAttemptId,
    cursor?: string,
  ) => Promise<CursorPage<StudentQuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<StudentQuestionAttempt>;
  /** Returns the regenerated, answer-free Question Presentation; grading stays server-side. */
  readonly getIssuedQuestion: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionPresentation>;
  /** Creates an iMathAS Question Backend launch by same-origin POST, then returns its inert shell route. */
  readonly beginImathasQuestionBackendLaunch: (
    courseId: CourseId,
    assignmentId: AssignmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<ImathasQuestionBackendLaunch>;
  /** Instructor command only; current Student Feedback is read through a later summary GET. */
  readonly releaseStudentFeedback: (
    attemptId: QuestionAttemptId,
  ) => Promise<StudentFeedbackReleaseResponse>;
  readonly getAssignmentActivitySummary: (
    studentRecordId: StudentRecordId,
  ) => Promise<StudentAssignmentProgress>;
  /** Same-origin POST that authorizes, audits, and returns one normalized course banner. */
  readonly fetchCourseBanner: (bannerReference: CourseBannerReference) => Promise<Blob>;
  /** Fetches the fixed 5:2 course-card WebP rendition. */
  readonly fetchCourseBannerCard: (bannerReference: CourseBannerReference) => Promise<Blob>;
  /** Exact immutable Question Revision asset redirect path; it never issues a capability. */
  readonly assetUrl: (
    questionRevision: QuestionRevisionReference,
    assetId: QuestionAssetId,
  ) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly questionAttemptTimingDecisionOnServer: TimerEvaluator;
  readonly validateAssignmentConfigOnServer: CapabilityValidator;
}

/** The ordinary deployed browser composes HTTP capabilities without test-double transport. */
export interface OrdinaryBrowserApiClient extends ApiClient, LiveDemoClient {}
