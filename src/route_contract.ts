// route_contract.ts - pure data form of the frozen product route contract.

import type { AccountRole } from "../generated/api/AccountRole";

export interface RouteContract {
  readonly id:
    | "courses"
    | "courseAssignments"
    | "sysadminInstructorApproval"
    | "assignmentOverview"
    | "assignmentAttempt"
    | "assignmentAttemptSummary"
    | "library"
    | "questionDetail"
    | "curriculum"
    | "curriculumDetail"
    | "workspaceList"
    | "workspaceEditor"
    | "assignmentCreate"
    | "assignmentWorkspaceOverview"
    | "assignmentWorkspaceQuestions"
    | "assignmentWorkspacePolicies"
    | "assignmentWorkspaceStudentView"
    | "assignmentWorkspaceGradingOperations"
    | "gradebook"
    | "studentWorkInspection"
    | "courseGradeSettings"
    | "courseAppearance"
    | "signIn"
    | "courseRoster"
    | "teachingOperations"
    | "assignmentAccess"
    | "assignmentPreview"
    | "pendingCourseInvitations";
  readonly path: string;
  readonly surface: string;
  /** Role gate for the route; each route declares the real roles it serves. */
  readonly requiredRoles: ReadonlyArray<AccountRole>;
}

/** Product routes in the same order as the active implementation plan. */
export const ROUTE_CONTRACT = [
  {
    id: "courses",
    path: "/",
    surface: "Course list for the signed-in role",
    requiredRoles: [],
  },
  {
    id: "signIn",
    path: "/sign-in",
    surface: "Passwordless account sign-in",
    requiredRoles: [],
  },
  {
    id: "pendingCourseInvitations",
    path: "/account/course-invitations",
    surface: "Account-owned pending Course Invitations",
    requiredRoles: [],
  },
  {
    id: "courseAssignments",
    path: "/courses/:courseRef",
    surface: "Assignment list with progress and Assignment Attempt counts",
    requiredRoles: [],
  },
  {
    id: "sysadminInstructorApproval",
    path: "/sysadmin/instructor-approval",
    surface: "Sysadmin Instructor approval workspace",
    requiredRoles: ["sysadmin"],
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseRef/assignments/:assignmentRef",
    surface: "Assignment overview, Assignment Attempt history, and practice entry",
    requiredRoles: ["student"],
  },
  {
    id: "assignmentAttempt",
    path: "/assignment-attempts/:assignmentAttemptRef",
    surface: "One-question-at-a-time attempt loop",
    requiredRoles: [],
  },
  {
    id: "assignmentAttemptSummary",
    path: "/assignment-attempts/:assignmentAttemptRef/summary",
    surface: "Assignment Attempt result and practice re-entry",
    requiredRoles: [],
  },
  {
    id: "library",
    path: "/library",
    surface: "Question Library",
    requiredRoles: ["instructor"],
  },
  {
    id: "questionDetail",
    path: "/library/:questionRef",
    surface: "Published question detail",
    requiredRoles: ["instructor"],
  },
  {
    id: "curriculum",
    path: "/curriculum",
    surface: "Blueprint Course workspace",
    requiredRoles: ["instructor"],
  },
  {
    id: "curriculumDetail",
    path: "/curriculum/:curriculumRef",
    surface: "Blueprint Course inspection and editor",
    requiredRoles: ["instructor"],
  },
  {
    id: "workspaceList",
    path: "/workspace",
    surface: "Instructor drafts",
    requiredRoles: ["instructor"],
  },
  {
    id: "workspaceEditor",
    path: "/workspace/:workspaceRef",
    surface: "Draft editor, validation, and preview",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentCreate",
    path: "/instructor/courses/:courseRef/assignments/new",
    surface: "Create persisted Assignment Working Copy and enter Questions",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceOverview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef",
    surface: "Instructor assignment workspace overview",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceQuestions",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/questions",
    surface: "Instructor assignment questions workspace",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspacePolicies",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/policies",
    surface: "Instructor assignment policies workspace",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceStudentView",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/student-view",
    surface: "Instructor assignment Student view",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceGradingOperations",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/grading-operations",
    surface: "Instructor automated-grading operations workspace",
    // ASVS 8.3.1: mirror the server's explicit Instructor authority boundary.
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentAccess",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/access",
    surface: "Assignment access modifiers and server preview",
    requiredRoles: ["instructor"],
  },
  {
    id: "assignmentPreview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/delivery-check",
    surface: "Instructor-only assignment delivery check",
    requiredRoles: ["instructor"],
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseRef/gradebook",
    surface: "Calculated Gradebook",
    requiredRoles: ["instructor"],
  },
  {
    id: "studentWorkInspection",
    path: "/instructor/courses/:courseRef/gradebook/students/:membershipRef/assignments/:assignmentRef/assignment-attempts/:assignmentAttemptRef",
    surface: "Audited Student-work inspection",
    requiredRoles: ["instructor"],
  },
  {
    id: "courseGradeSettings",
    path: "/instructor/courses/:courseRef/grade-settings",
    surface: "Course grade settings and projected totals",
    requiredRoles: ["instructor"],
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseRef/appearance",
    surface: "Course theme and entry-banner settings",
    requiredRoles: ["instructor"],
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseRef/students",
    surface: "Course roster, invitations, and import",
    requiredRoles: ["instructor"],
  },
  {
    id: "teachingOperations",
    path: "/instructor/courses/:courseRef/teaching-operations",
    surface: "Course teaching operations hub",
    requiredRoles: ["instructor"],
  },
] as const satisfies ReadonlyArray<RouteContract>;

export type RouteId = (typeof ROUTE_CONTRACT)[number]["id"];

function pathMatchesRoutePattern(pathname: string, routePattern: string): boolean {
  if (!pathname.startsWith("/") || pathname.includes("?") || pathname.includes("#")) {
    return false;
  }
  if (pathname === "/" || routePattern === "/") {
    return pathname === routePattern;
  }

  const pathnameSegments = pathname.slice(1).split("/");
  const patternSegments = routePattern.slice(1).split("/");
  if (pathnameSegments.length !== patternSegments.length) {
    return false;
  }
  return patternSegments.every((patternSegment, index) => {
    const pathnameSegment = pathnameSegments[index];
    if (pathnameSegment === undefined || pathnameSegment.length === 0) {
      return false;
    }
    return patternSegment.startsWith(":") || patternSegment === pathnameSegment;
  });
}

/** Resolves only a declared browser pathname; unknown and malformed paths fail closed. */
export function routeContractForPathname(pathname: string): RouteContract | undefined {
  return ROUTE_CONTRACT.find((route) => pathMatchesRoutePattern(pathname, route.path));
}

/** Checks the role boundary declared by a product route without mounting its surface. */
export function accountRoleMayAccessRoute(routeId: string, role: AccountRole): boolean {
  const route: RouteContract | undefined = ROUTE_CONTRACT.find((item) => item.id === routeId);
  if (route === undefined) {
    return false;
  }
  if (route.requiredRoles.length === 0) {
    return true;
  }
  return route.requiredRoles.includes(role);
}
