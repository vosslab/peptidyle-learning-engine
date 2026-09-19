// client.ts - the only API shape consumed by browser routes and components.

import type { QuestionAssetId } from "../../generated/api/QuestionAssetId";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { AssessmentAttempt } from "../../generated/api/AssessmentAttempt";
import type { QuestionSummary } from "../../generated/api/QuestionSummary";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { CourseThemeUpdate } from "../../generated/api/CourseThemeUpdate";
import type { CourseBannerId } from "../../generated/api/CourseBannerId";
import type { CourseBannerUpdate } from "../../generated/api/CourseBannerUpdate";
import type { CourseBannerUploadReceipt } from "../../generated/api/CourseBannerUploadReceipt";
import type {
  ProfileAvatarView,
  ProfileImageCropInput,
  SelectProvidedProfileAvatarInput,
} from "./profile_avatar";
import type { ProfileSettings, UpdateAccountSettingsInput } from "./profile_settings";
import type { StudentRecordId } from "../../generated/api/StudentRecordId";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { StudentAssessmentProgress } from "../../generated/api/StudentAssessmentProgress";
import type { CourseInvitationId } from "../../generated/api/CourseInvitationId";
import type { CourseInvitationTerminalActionRequest } from "../../generated/api/CourseInvitationTerminalActionRequest";
import type { PendingCourseInvitationsPage } from "../../generated/api/PendingCourseInvitationsPage";
import type { CourseInvitationStatePrecondition } from "../../generated/api/CourseInvitationStatePrecondition";
import type { CapabilityValidator, FormatValidator, TimerEvaluator } from "../wasm/index";
import type {
  AssessmentEditorDetail,
  AssessmentCreateInput,
  AssessmentContentInput,
  StudentAssessmentLandingSummary,
  StudentAssessmentDetail,
  StudentQuestionAttempt,
  AuthenticatedSession,
  CourseSummary,
  CursorPage,
  ImathasQuestionBackendLaunch,
  StudentFeedbackReleaseResponse,
} from "./contracts";
import type { NavigationResolution } from "../../generated/api/NavigationResolution";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { NavigationRouteId } from "../navigation/public_route";
import type { LiveDemoClient } from "./live_demo";
import type { BlueprintCourseClient } from "./blueprint_course";
import type { BlueprintChangeProposalClient } from "./blueprint_change_proposal";
import type { CourseInstanceClient } from "./course_instance";
import type { LiveCourseRosterClient } from "./course_roster";
import type { LiveInvitationExportClient } from "./invitation_export";
import type { LiveAssessmentReleaseClient } from "./assessment_release";
import type { AssessmentPoolForkClient } from "./assessment_pool_fork";
import type { AssessmentStudentTimeAccommodationClient } from "./assessment_student_time_accommodation";
import type { LiveAssessmentAttemptIssuanceClient } from "./assessment_attempt_issuance";
import type { StudentAssessmentAttemptHistoryClient } from "./assessment_attempt_history";
import type { StudentAssessmentAttemptNavigationClient } from "./assessment_attempt_navigation";
import type { InstructorAccountClient } from "./instructor_account";
import type { CourseGradebookClient } from "./live_gradebook";
import type { LiveStudentCourseLandingClient } from "./live_student_course_landing";
import type { QuestionAvailabilityClient } from "./question_availability";
import type { QuestionWatchClient } from "./question_watch";
import type { QuestionStarClient } from "./question_star";
import type { QuestionPoolLibraryClient } from "./question_pool_library";
import type { QuestionPoolCreationClient } from "./question_pool_creation";
import type { QuestionPoolStewardshipClient } from "./question_pool_stewardship";
import type { QuestionForkClient } from "./question_fork";
import type { AssessmentStudentViewClient } from "./assessment_student_view";
import type { AssessmentTemplateClient } from "./assessment_template";
import type { QuestionBulkMetadataClient } from "./question_bulk_metadata";
import type {
  ContentClassificationClient,
  ContentDisciplineAdministrationClient,
} from "./content_classification";
import type { CourseStudentWorkRecoveryClient } from "./course_student_work_recovery";
import type { LibraryDiscussionClient } from "./library_discussion";
import type { LibraryWatchNotificationClient } from "./library_watch_notification";
import type { BloomClassificationCorrectionClient } from "./bloom_classification";
/** Browser-safe client contract implemented by the current same-origin HTTP transport. */
export interface ApiClient
  extends
    BlueprintCourseClient,
    BlueprintChangeProposalClient,
    CourseInstanceClient,
    LiveCourseRosterClient,
    LiveInvitationExportClient,
    LiveAssessmentReleaseClient,
    AssessmentPoolForkClient,
    AssessmentStudentTimeAccommodationClient,
    LiveAssessmentAttemptIssuanceClient,
    StudentAssessmentAttemptHistoryClient,
    StudentAssessmentAttemptNavigationClient,
    InstructorAccountClient,
    CourseGradebookClient,
    LiveStudentCourseLandingClient,
    QuestionAvailabilityClient,
    QuestionWatchClient,
    QuestionStarClient,
    QuestionForkClient,
    QuestionPoolLibraryClient,
    QuestionPoolCreationClient,
    QuestionPoolStewardshipClient,
    BloomClassificationCorrectionClient,
    LibraryDiscussionClient,
    LibraryWatchNotificationClient,
    AssessmentStudentViewClient,
    AssessmentTemplateClient,
    QuestionBulkMetadataClient,
    ContentClassificationClient,
    ContentDisciplineAdministrationClient,
    CourseStudentWorkRecoveryClient {
  /** Reads only the authenticated Account's role-neutral Profile settings. */
  readonly getProfile: () => Promise<ProfileSettings>;
  /** Reads only the authenticated Account's Account Settings preference. */
  readonly getAccountSettings: () => Promise<ProfileSettings>;
  /** Replaces only the authenticated Account's exact IANA display zone. */
  readonly updateAccountSettings: (input: UpdateAccountSettingsInput) => Promise<ProfileSettings>;
  /** Reads only the authenticated Account's currently selected avatar. */
  readonly getProfileAvatar: () => Promise<ProfileAvatarView>;
  /** Selects one validated PLE-provided avatar for the authenticated Account. */
  readonly selectProvidedProfileAvatar: (input: SelectProvidedProfileAvatarInput) => Promise<void>;
  /** Replaces the authenticated Instructor or Sysadmin Account's profile image. */
  readonly replaceProfileAvatarImage: (
    image: Blob,
    crop: ProfileImageCropInput,
  ) => Promise<ProfileAvatarView>;
  /** Fetches the authenticated Account's current protected profile-image rendition. */
  readonly fetchProfileAvatarImage: (reference: string) => Promise<Blob>;
  readonly listPendingCourseInvitations: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<PendingCourseInvitationsPage>;
  readonly respondToCourseInvitation: (
    invitation: CourseInvitationId,
    request: CourseInvitationTerminalActionRequest,
    statePrecondition: CourseInvitationStatePrecondition,
  ) => Promise<void>;
  readonly getSession: () => Promise<AuthenticatedSession>;
  /** Resolves a compact visible reference inside the current authorization boundary. */
  readonly resolveNavigation: (id: NavigationRouteId) => Promise<NavigationResolution>;
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
  /** Gets only the authorized current Course Appearance View. */
  readonly getCourseAppearanceView: (
    courseInstanceId: CourseInstanceId,
  ) => Promise<CourseAppearanceView>;
  /** Saves one independent Course Theme and returns the current aggregate appearance. */
  readonly updateCourseTheme: (
    courseInstanceId: CourseInstanceId,
    update: CourseThemeUpdate,
  ) => Promise<CourseAppearanceView>;
  /** Stages raw verified banner bytes for this exact Instructor and Course. */
  readonly uploadCourseBanner: (
    courseInstanceId: CourseInstanceId,
    image: Blob,
  ) => Promise<CourseBannerUploadReceipt>;
  /** Promotes one staged banner independently of the Course Theme. */
  readonly setCourseBanner: (
    courseInstanceId: CourseInstanceId,
    update: CourseBannerUpdate,
  ) => Promise<CourseAppearanceView>;
  /** Removes only the current Course Banner. */
  readonly removeCourseBanner: (
    courseInstanceId: CourseInstanceId,
  ) => Promise<CourseAppearanceView>;
  readonly listAssessments: (
    courseId: CourseInstanceId,
    cursor?: string,
  ) => Promise<CursorPage<StudentAssessmentLandingSummary>>;
  /** Student-safe detail; Instructor workspace reads require an exact course identity. */
  readonly getAssessment: (assessmentId: AssessmentId) => Promise<StudentAssessmentDetail>;
  /** Current key-free student progress; the server omits withheld score totals. */
  readonly getAssessmentSummary: (assessmentId: AssessmentId) => Promise<StudentAssessmentProgress>;
  /** Reads the course-bound Instructor assessment workspace. */
  readonly getAssessmentWorkspace: (
    courseId: CourseInstanceId,
    assessmentId: AssessmentId,
  ) => Promise<AssessmentEditorDetail>;
  /** Creates a persisted empty Assessment with server-owned defaults. */
  readonly createAssessment: (
    courseId: CourseInstanceId,
    input: AssessmentCreateInput,
  ) => Promise<AssessmentEditorDetail>;
  /** Replaces only Questions-owned title and ordered content. */
  readonly saveAssessmentContent: (
    courseId: CourseInstanceId,
    assessmentId: AssessmentId,
    input: AssessmentContentInput,
    assessmentEtag: string,
  ) => Promise<AssessmentEditorDetail>;
  readonly listAssessmentAttempts: (
    studentRecordId: StudentRecordId,
    cursor?: string,
  ) => Promise<CursorPage<AssessmentAttempt>>;
  /**
   * Starts or resumes student work within the course route that authorizes the assessment.
   * The browser supplies no student-work authority or Answer Key.
   */
  readonly startAssessmentAttempt: (
    courseId: CourseInstanceId,
    assessmentId: AssessmentId,
  ) => Promise<AssessmentAttempt>;
  readonly getAssessmentAttempt: (
    assessmentAttemptId: AssessmentAttemptId,
  ) => Promise<AssessmentAttempt>;
  readonly listQuestionAttempts: (
    assessmentAttemptId: AssessmentAttemptId,
    cursor?: string,
  ) => Promise<CursorPage<StudentQuestionAttempt>>;
  readonly getAttempt: (attemptId: QuestionAttemptId) => Promise<StudentQuestionAttempt>;
  /** Returns the regenerated, answer-free Question Presentation; grading stays server-side. */
  readonly getIssuedQuestion: (
    courseId: CourseInstanceId,
    assessmentId: AssessmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionPresentation>;
  /** Creates an iMathAS Question Backend launch by same-origin POST, then returns its inert shell route. */
  readonly beginImathasQuestionBackendLaunch: (
    courseId: CourseInstanceId,
    assessmentId: AssessmentId,
    attemptId: QuestionAttemptId,
  ) => Promise<ImathasQuestionBackendLaunch>;
  /** Instructor command only; current Student Feedback is read through a later summary GET. */
  readonly releaseStudentFeedback: (
    attemptId: QuestionAttemptId,
  ) => Promise<StudentFeedbackReleaseResponse>;
  readonly getAssessmentActivitySummary: (
    studentRecordId: StudentRecordId,
  ) => Promise<StudentAssessmentProgress>;
  /** Same-origin POST that authorizes, audits, and returns one normalized course banner. */
  readonly fetchCourseBanner: (bannerId: CourseBannerId) => Promise<Blob>;
  /** Exact immutable Question Revision asset redirect path; it never issues a capability. */
  readonly assetUrl: (questionRevision: QuestionRevisionTuple, assetId: QuestionAssetId) => string;
  readonly validateResponseFormatOnServer: FormatValidator;
  readonly questionAttemptTimingDecisionOnServer: TimerEvaluator;
  readonly validateAssessmentConfigOnServer: CapabilityValidator;
}

/** The ordinary deployed browser composes HTTP capabilities without test-double transport. */
export interface OrdinaryBrowserApiClient extends ApiClient, LiveDemoClient {}
