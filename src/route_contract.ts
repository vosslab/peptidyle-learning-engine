// route_contract.ts - pure data form of the frozen product route contract.

export interface RouteContract {
  readonly id:
    | "courses"
    | "courseAssignments"
    | "assignmentOverview"
    | "runAttempt"
    | "runSummary"
    | "library"
    | "problemDetail"
    | "workspaceList"
    | "workspaceEditor"
    | "assignmentEditor"
    | "gradebook"
    | "courseAppearance"
    | "signIn"
    | "emailAuthenticationComplete"
    | "emailChangeComplete"
    | "courseInvitation"
    | "accountSecurity"
    | "courseRoster";
  readonly path: string;
  readonly surface: string;
}

/** Product routes in the same order as the active implementation plan. */
export const ROUTE_CONTRACT = [
  { id: "courses", path: "/", surface: "Course list for the signed-in role" },
  { id: "signIn", path: "/sign-in", surface: "Passwordless account sign-in" },
  {
    id: "emailAuthenticationComplete",
    path: "/auth/email/complete",
    surface: "Browser-bound one-time email completion",
  },
  {
    id: "emailChangeComplete",
    path: "/auth/account/email/complete",
    surface: "Browser-bound account email change completion",
  },
  {
    id: "courseInvitation",
    path: "/course-invitations/redeem",
    surface: "Authenticated learner invitation claim",
  },
  {
    id: "accountSecurity",
    path: "/account/security",
    surface: "Multiple-passkey account management",
  },
  {
    id: "courseAssignments",
    path: "/courses/:courseId",
    surface: "Assignment list with progress and run counts",
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseId/assignments/:assignmentId",
    surface: "Assignment overview, run history, and practice entry",
  },
  { id: "runAttempt", path: "/runs/:runId", surface: "One-question-at-a-time attempt loop" },
  { id: "runSummary", path: "/runs/:runId/summary", surface: "Run result and practice re-entry" },
  { id: "library", path: "/library", surface: "Shared problem browser" },
  {
    id: "problemDetail",
    path: "/library/:problemId/versions/:versionId",
    surface: "Published problem version detail",
  },
  { id: "workspaceList", path: "/workspace", surface: "Instructor drafts" },
  {
    id: "workspaceEditor",
    path: "/workspace/:workspaceId",
    surface: "Draft editor, validation, and preview",
  },
  {
    id: "assignmentEditor",
    path: "/instructor/courses/:courseId/assignments/:assignmentId/edit",
    surface: "Assignment policy editor",
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseId/gradebook",
    surface: "Summary-row gradebook",
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseId/appearance",
    surface: "Course theme and entry-banner settings",
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseId/students",
    surface: "Course roster, invitations, import, and grade export",
  },
] as const satisfies ReadonlyArray<RouteContract>;

export type RouteId = (typeof ROUTE_CONTRACT)[number]["id"];
