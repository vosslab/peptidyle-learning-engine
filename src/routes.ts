// routes.ts - executable copy of the frozen product route contract.

import { createComponent, type Component } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";

import { CourseAppearancePage } from "./features/course_appearance/course_appearance_page";
import { AssignmentOverviewPage } from "./pages/assignment_overview_page";
import { NotFoundPage } from "./pages/contract_pages";
import { AssignmentAttemptSummaryPage } from "./pages/assignment_attempt_summary_page";
import { LibraryRoutePage } from "./pages/library_route_page";
import { ProblemDetailPage } from "./pages/problem_detail_page";
import { CourseAssignmentsPage } from "./pages/course_assignments_page";
import { CourseListPage } from "./pages/course_list_page";
import { GradebookPage } from "./pages/gradebook_page";
import { StudentWorkInspectionPage } from "./pages/student_work_inspection_page";
import { CourseGradeSettingsPage } from "./pages/course_grade_settings_page";
import { withRouteAccessBoundary } from "./route_access_boundary";
import { ROUTE_CONTRACT, type RouteId } from "./route_contract";
import { AssignmentAttemptPage } from "./pages/assignment_attempt_page";
import { WorkspaceEditorLivePage, WorkspaceListLivePage } from "./pages/editor_live_pages";
import { CourseRosterPage } from "./pages/course_roster_page";
import { SignInPage } from "./pages/sign_in_page";
import { AccountPendingInvitationsPage } from "./pages/account_pending_invitations_page";
import { AssignmentAccessLivePage } from "./pages/assignment_access_live_page";
import { TeachingOperationsPage } from "./pages/teaching_operations_page";
import { AssignmentPreviewPage } from "./pages/assignment_preview_page";
import { CurriculumDetailLivePage, CurriculumLivePage } from "./pages/curriculum_live_pages";
import { AssignmentWorkspaceLivePage } from "./pages/assignment_workspace/assignment_workspace_live_page";
import { AssignmentWorkspaceCreatePage } from "./pages/assignment_workspace/assignment_workspace_create_page";

export { ROUTE_CONTRACT } from "./route_contract";

const routeComponents: Readonly<Record<RouteId, Component>> = {
  courses: CourseListPage,
  signIn: SignInPage,
  pendingCourseInvitations: AccountPendingInvitationsPage,
  courseAssignments: CourseAssignmentsPage,
  assignmentOverview: AssignmentOverviewPage,
  assignmentAttempt: AssignmentAttemptPage,
  assignmentAttemptSummary: AssignmentAttemptSummaryPage,
  library: LibraryRoutePage,
  problemDetail: ProblemDetailPage,
  curriculum: CurriculumLivePage,
  curriculumDetail: CurriculumDetailLivePage,
  workspaceList: WorkspaceListLivePage,
  workspaceEditor: WorkspaceEditorLivePage,
  assignmentCreate: AssignmentWorkspaceCreatePage,
  assignmentWorkspaceOverview: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "overview" }),
  assignmentWorkspaceQuestions: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "questions" }),
  assignmentWorkspacePolicies: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "policies" }),
  assignmentWorkspaceStudentView: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "studentView" }),
  assignmentWorkspaceGradingOperations: () =>
    createComponent(AssignmentWorkspaceLivePage, { section: "gradingOperations" }),
  assignmentAccess: AssignmentAccessLivePage,
  assignmentPreview: AssignmentPreviewPage,
  gradebook: GradebookPage,
  studentWorkInspection: StudentWorkInspectionPage,
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
