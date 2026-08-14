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
    | "assignmentCreate"
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
    path: "/courses/:courseRef",
    surface: "Assignment list with progress and run counts",
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseRef/assignments/:assignmentRef",
    surface: "Assignment overview, run history, and practice entry",
  },
  { id: "runAttempt", path: "/runs/:runRef", surface: "One-question-at-a-time attempt loop" },
  { id: "runSummary", path: "/runs/:runRef/summary", surface: "Run result and practice re-entry" },
  { id: "library", path: "/library", surface: "Shared problem browser" },
  {
    id: "problemDetail",
    path: "/library/:problemRef",
    surface: "Published question detail",
  },
  { id: "workspaceList", path: "/workspace", surface: "Instructor drafts" },
  {
    id: "workspaceEditor",
    path: "/workspace/:workspaceRef",
    surface: "Draft editor, validation, and preview",
  },
  {
    id: "assignmentCreate",
    path: "/instructor/courses/:courseRef/assignments/new",
    surface: "New assignment policy editor",
  },
  {
    id: "assignmentEditor",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    surface: "Assignment policy editor",
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseRef/gradebook",
    surface: "Summary-row gradebook",
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseRef/appearance",
    surface: "Course theme and entry-banner settings",
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseRef/students",
    surface: "Course roster, invitations, import, and grade export",
  },
] as const satisfies ReadonlyArray<RouteContract>;

export type RouteId = (typeof ROUTE_CONTRACT)[number]["id"];
