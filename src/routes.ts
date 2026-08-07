// routes.ts - executable copy of the frozen eleven-route product contract.

import type { Component } from "solid-js";
import type { RouteDefinition } from "@solidjs/router";

import { AssignmentOverviewPage } from "./pages/assignment_overview_page";
import {
  AssignmentEditorPage,
  GradebookPage,
  LibraryPage,
  NotFoundPage,
  ProblemDetailPage,
  RunSummaryPage,
  WorkspaceEditorPage,
  WorkspaceListPage,
} from "./pages/contract_pages";
import { CourseAssignmentsPage } from "./pages/course_assignments_page";
import { CourseListPage } from "./pages/course_list_page";
import { ROUTE_CONTRACT, type RouteId } from "./route_contract";
import { RunPage } from "./pages/run_page";

export { ROUTE_CONTRACT } from "./route_contract";

const routeComponents: Readonly<Record<RouteId, Component>> = {
  courses: CourseListPage,
  courseAssignments: CourseAssignmentsPage,
  assignmentOverview: AssignmentOverviewPage,
  runAttempt: RunPage,
  runSummary: RunSummaryPage,
  library: LibraryPage,
  problemDetail: ProblemDetailPage,
  workspaceList: WorkspaceListPage,
  workspaceEditor: WorkspaceEditorPage,
  assignmentEditor: AssignmentEditorPage,
  gradebook: GradebookPage,
};

/** Router definitions derived from the frozen contract, not a second path list. */
export const appRoutes: ReadonlyArray<RouteDefinition> = ROUTE_CONTRACT.map((route) => ({
  path: route.path,
  component: routeComponents[route.id],
  info: { id: route.id, surface: route.surface },
}));

/** Infrastructure fallback; intentionally excluded from the eleven product routes. */
export const notFoundRoute: RouteDefinition = {
  path: "*unmatched",
  component: NotFoundPage,
};
