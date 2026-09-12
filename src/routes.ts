// routes.ts - executable copy of the frozen product route contract.

import { createComponent, type Component } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";

import { AssignmentOverviewPage } from "./pages/assignment_overview_page";
import { NotFoundPage } from "./pages/contract_pages";
import { AssignmentAttemptSummaryPage } from "./pages/assignment_attempt_summary_page";
import { LibraryRoutePage } from "./pages/library_route_page";
import { QuestionDetailPage } from "./pages/question_detail_page";
import { QuestionDraftEditorPage } from "./pages/question_draft_editor_page";
import { QuestionDraftsPage } from "./pages/question_drafts_page";
import { CourseInstancePage } from "./pages/course_instance_page";
import { CourseListPage } from "./pages/course_list_page";
import { GradebookPage } from "./pages/gradebook_page";
import { CourseGradeSettingsPage } from "./pages/course_grade_settings_page";
import { CourseAppearancePage } from "./pages/course_appearance_page";
import { withRouteAccessBoundary } from "./route_access_boundary";
import { ROUTE_CONTRACT, type RouteId } from "./route_contract";
import { AssignmentAttemptPage } from "./pages/assignment_attempt_page";
import { CourseRosterPage } from "./pages/course_roster_page";
import { SignInPage } from "./pages/sign_in_page";
import { InstructorProfilePage } from "./pages/instructor_profile_page";
import { AccountPendingInvitationsPage } from "./pages/account_pending_invitations_page";
import { StudentCourseInvitationPage } from "./pages/student_course_invitation_page";
import { StudentCourseInvitationsPage } from "./pages/student_course_invitations_page";
import { StudentCourseLandingPage } from "./pages/student_course_landing_page";
import { TeachingOperationsPage } from "./pages/teaching_operations_page";
import { InstructorAccountsPage } from "./pages/instructor_accounts_page";
import { SupportRosterPage } from "./pages/support_roster_page";
import { AssignmentPreviewPage } from "./pages/assignment_preview_page";
import { AssignmentsDueSoonPage } from "./pages/assignments_due_soon_page";
import { AssignmentWorkspaceCreatePage } from "./pages/assignment_workspace/assignment_workspace_create_page";
import {
  BlueprintCourseDetailLivePage,
  BlueprintCoursesLivePage,
} from "./pages/blueprint_course_live_pages";
// prettier-ignore
import {
  AssignmentWorkspaceLivePage,
} from "./pages/assignment_workspace/assignment_workspace_live_page";

export { ROUTE_CONTRACT } from "./route_contract";

const routeComponents: Readonly<Record<RouteId, Component>> = {
  courses: CourseListPage,
  instructorProfile: InstructorProfilePage,
  signIn: SignInPage,
  pendingCourseInvitations: AccountPendingInvitationsPage,
  studentCourseInvitations: StudentCourseInvitationsPage,
  studentCourseInvitation: StudentCourseInvitationPage,
  studentCourseLanding: StudentCourseLandingPage,
  instructorAccounts: InstructorAccountsPage,
  supportRoster: SupportRosterPage,
  courseAssignments: CourseInstancePage,
  assignmentOverview: AssignmentOverviewPage,
  assignmentAttempt: AssignmentAttemptPage,
  assignmentAttemptSummary: AssignmentAttemptSummaryPage,
  library: LibraryRoutePage,
  questionDetail: QuestionDetailPage,
  questionDrafts: QuestionDraftsPage,
  questionDraftEditor: QuestionDraftEditorPage,
  blueprintCourses: BlueprintCoursesLivePage,
  blueprintCourseDetail: BlueprintCourseDetailLivePage,
  assignmentsDueSoon: AssignmentsDueSoonPage,
  assignmentCreate: AssignmentWorkspaceCreatePage,
  assignmentWorkspaceOverview: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "overview" }),
  assignmentWorkspaceQuestions: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "questions" }),
  assignmentWorkspacePolicies: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "policies" }),
  assignmentWorkspaceStudentView: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "studentView" }),
  assignmentPreview: AssignmentPreviewPage,
  gradebook: GradebookPage,
  courseGradeSettings: CourseGradeSettingsPage,
  courseAppearance: CourseAppearancePage,
  courseRoster: CourseRosterPage,
  teachingOperations: TeachingOperationsPage,
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
