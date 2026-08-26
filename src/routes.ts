// routes.ts - executable copy of the frozen product route contract.

import type { Component } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";

import { CourseAppearancePage } from "./features/course_appearance/course_appearance_page";
import { AssignmentOverviewPage } from "./pages/assignment_overview_page";
import { NotFoundPage } from "./pages/contract_pages";
import { AssignmentEditorLivePage } from "./pages/assignment_editor_live_page";
import { RunSummaryPage } from "./pages/run_summary_page";
import { LibraryRoutePage } from "./pages/library_route_page";
import { ProblemDetailPage } from "./pages/problem_detail_page";
import { CourseAssignmentsPage } from "./pages/course_assignments_page";
import { CourseListPage } from "./pages/course_list_page";
import { GradebookPage } from "./pages/gradebook_page";
import { CourseGradeSettingsPage } from "./pages/course_grade_settings_page";
import { withRouteAccessBoundary } from "./route_access_boundary";
import { ROUTE_CONTRACT, type RouteId } from "./route_contract";
import { RunPage } from "./pages/run_page";
import { WorkspaceEditorLivePage, WorkspaceListLivePage } from "./pages/editor_live_pages";
import { AccountSecurityPage } from "./pages/account_security_page";
import { CourseInvitationPage } from "./pages/course_invitation_page";
import { CourseRosterPage } from "./pages/course_roster_page";
import { EmailAuthenticationCompletePage, SignInPage } from "./pages/sign_in_page";
import { EmailChangeCompletePage } from "./pages/email_change_complete_page";
import { AccountPendingInvitationsPage } from "./pages/account_pending_invitations_page";
import { AssignmentAccessLivePage } from "./pages/assignment_access_live_page";
import { TeachingOperationsPage } from "./pages/teaching_operations_page";
import { AssignmentPreviewPage } from "./pages/assignment_preview_page";
import { CurriculumDetailLivePage, CurriculumLivePage } from "./pages/curriculum_live_pages";
import { CurriculumAdoptionLivePage } from "./pages/curriculum_adoption_live_page";

export { ROUTE_CONTRACT } from "./route_contract";

const routeComponents: Readonly<Record<RouteId, Component>> = {
  courses: CourseListPage,
  signIn: SignInPage,
  emailAuthenticationComplete: EmailAuthenticationCompletePage,
  emailChangeComplete: EmailChangeCompletePage,
  courseInvitation: CourseInvitationPage,
  accountSecurity: AccountSecurityPage,
  pendingCoInstructorInvitations: AccountPendingInvitationsPage,
  courseAssignments: CourseAssignmentsPage,
  assignmentOverview: AssignmentOverviewPage,
  runAttempt: RunPage,
  runSummary: RunSummaryPage,
  library: LibraryRoutePage,
  problemDetail: ProblemDetailPage,
  curriculum: CurriculumLivePage,
  curriculumDetail: CurriculumDetailLivePage,
  workspaceList: WorkspaceListLivePage,
  workspaceEditor: WorkspaceEditorLivePage,
  assignmentCreate: AssignmentEditorLivePage,
  assignmentEditor: AssignmentEditorLivePage,
  assignmentAccess: AssignmentAccessLivePage,
  assignmentPreview: AssignmentPreviewPage,
  gradebook: GradebookPage,
  courseGradeSettings: CourseGradeSettingsPage,
  courseAppearance: CourseAppearancePage,
  courseRoster: CourseRosterPage,
  teachingOperations: TeachingOperationsPage,
  curriculumAdoption: CurriculumAdoptionLivePage,
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
