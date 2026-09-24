// routes.ts - executable copy of the frozen product route contract.

import { createComponent, type Component } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";

import { AssessmentOverviewPage } from "./pages/assessment_overview_page";
import {
  MyChangeProposalsLivePage,
  ChangeProposalDetailLivePage,
} from "./features/blueprint_change_proposal/proposal_workspace";
import { NotFoundPage } from "./pages/contract_pages";
import { AssessmentAttemptSummaryPage } from "./pages/assessment_attempt_summary_page";
import { BrowseLibraryRoutePage, LibraryRoutePage } from "./pages/library_route_page";
import { LibraryWatchNotificationsPage } from "./pages/library_watch_notifications_page";
import { QuestionDetailPage } from "./pages/question_detail_page";
import { QuestionDraftEditorPage } from "./pages/question_draft_editor_page";
import { QuestionDraftsPage } from "./pages/question_drafts_page";
import { CourseInstancePage } from "./pages/course_instance_page";
import { InactiveCourseListPage } from "./pages/course_list_page";
import {
  InstructorHomePage,
  RoleHomeResolutionPage,
  StudentHomePage,
  SysadminHomePage,
} from "./pages/role_home_pages";
import { GradebookPage } from "./pages/gradebook_page";
import { CourseAppearancePage } from "./pages/course_appearance_page";
import { withRouteAccessBoundary } from "./route_access_boundary";
import { ROUTE_CONTRACT, type RouteId } from "./route_contract";
import { AssessmentAttemptPage } from "./pages/assessment_attempt_page";
import { CourseRosterPage } from "./pages/course_roster_page";
import { SignInPage } from "./pages/sign_in_page";
import { ProfilePage } from "./pages/profile_page";
import { AccountPendingInvitationsPage } from "./pages/account_pending_invitations_page";
import { StudentCourseInvitationPage } from "./pages/student_course_invitation_page";
import { StudentCourseInvitationsPage } from "./pages/student_course_invitations_page";
import {
  StudentCourseCompletedPage,
  StudentCourseDueSoonPage,
  StudentCourseLandingPage,
} from "./pages/student_course_landing_page";
import { StudentCourseGradesPage } from "./pages/student_course_grades_page";
import { StudentCourseProgressPage } from "./pages/student_course_progress_page";
import { StudentCourseAttemptHistoryPage } from "./pages/student_course_attempt_history_page";
import { StudentCourseResponseStatsPage } from "./pages/student_course_practice_stats_page";
import { InstructorAccountsPage } from "./pages/instructor_accounts_page";
import { ContentDisciplinesPage } from "./pages/content_disciplines_page";
import { AssessmentsDueSoonPage } from "./pages/assessments_due_soon_page";
import { AssessmentTemplatesPage } from "./pages/assessment_templates_page";
import { AssessmentWorkspaceCreatePage } from "./pages/assessment_workspace/assessment_workspace_create_page";
import {
  BlueprintCourseDetailLivePage,
  BlueprintCoursesLivePage,
  PublicBlueprintSearchLivePage,
} from "./pages/blueprint_course_live_pages";
// prettier-ignore
import {
  AssessmentWorkspaceLivePage,
} from "./pages/assessment_workspace/assessment_workspace_live_page";

export { ROUTE_CONTRACT } from "./route_contract";

const routeComponents: Readonly<Record<RouteId, Component>> = {
  myChangeProposals: MyChangeProposalsLivePage,
  changeProposalDetail: ChangeProposalDetailLivePage,
  courses: RoleHomeResolutionPage,
  instructorHome: InstructorHomePage,
  instructorInactiveCourses: InactiveCourseListPage,
  studentHome: StudentHomePage,
  sysadminHome: SysadminHomePage,
  profile: ProfilePage,
  signIn: SignInPage,
  pendingCourseInvitations: AccountPendingInvitationsPage,
  studentCourseInvitations: StudentCourseInvitationsPage,
  studentCourseInvitation: StudentCourseInvitationPage,
  studentCourseLanding: StudentCourseLandingPage,
  studentCourseProgress: StudentCourseProgressPage,
  studentCourseResponseStats: StudentCourseResponseStatsPage,
  studentCourseDueSoon: StudentCourseDueSoonPage,
  studentCourseCompleted: StudentCourseCompletedPage,
  studentCourseAttemptHistory: StudentCourseAttemptHistoryPage,
  studentCourseGrades: StudentCourseGradesPage,
  instructorAccounts: InstructorAccountsPage,
  contentDisciplines: ContentDisciplinesPage,
  courseAssessments: CourseInstancePage,
  assessmentOverview: AssessmentOverviewPage,
  assessmentAttempt: AssessmentAttemptPage,
  assessmentAttemptSummary: AssessmentAttemptSummaryPage,
  library: LibraryRoutePage,
  libraryBrowse: BrowseLibraryRoutePage,
  libraryWatchNotifications: LibraryWatchNotificationsPage,
  questionDetail: QuestionDetailPage,
  questionDrafts: QuestionDraftsPage,
  questionDraftEditor: QuestionDraftEditorPage,
  blueprintCourses: BlueprintCoursesLivePage,
  publicBlueprintSearch: PublicBlueprintSearchLivePage,
  blueprintCourseDetail: BlueprintCourseDetailLivePage,
  assessmentsDueSoon: AssessmentsDueSoonPage,
  assessmentTemplates: AssessmentTemplatesPage,
  assessmentCreate: AssessmentWorkspaceCreatePage,
  assessmentWorkspaceOverview: () =>
    createComponent(AssessmentWorkspaceLivePage, { section: "overview" }),
  assessmentWorkspaceQuestions: () =>
    createComponent(AssessmentWorkspaceLivePage, { section: "questions" }),
  assessmentWorkspacePolicies: () =>
    createComponent(AssessmentWorkspaceLivePage, { section: "policies" }),
  assessmentWorkspaceStudentView: () =>
    createComponent(AssessmentWorkspaceLivePage, { section: "studentView" }),
  gradebook: GradebookPage,
  courseAppearance: CourseAppearancePage,
  courseRoster: CourseRosterPage,
};

/** Router definitions derived from the frozen contract, not a second path list. */
export const appRoutes: ReadonlyArray<RouteDefinition> = ROUTE_CONTRACT.map((route) => ({
  path: route.path,
  component: withRouteAccessBoundary(route, routeComponents[route.id]),
  info: { id: route.id, surface: route.surface },
}));

/** Infrastructure fallback; intentionally excluded from the product routes. */
export const notFoundRoute: RouteDefinition = {
  path: "*unmatched",
  component: NotFoundPage,
};
