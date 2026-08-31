// route_contract.ts - pure data form of the frozen product route contract.

import type { AccountRole } from "../generated/api/AccountRole";

export interface RouteContract {
  readonly id:
    | "courses"
    | "courseAssignments"
    | "assignmentOverview"
    | "runAttempt"
    | "runSummary"
    | "library"
    | "problemDetail"
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
    path: "/account/co-instructor-invitations",
    surface: "Account-owned pending co-instructor invitations",
    requiredRoles: [],
  },
  {
    id: "courseAssignments",
    path: "/courses/:courseRef",
    surface: "Assignment list with progress and run counts",
    requiredRoles: [],
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseRef/assignments/:assignmentRef",
    surface: "Assignment overview, run history, and practice entry",
    requiredRoles: ["student"],
  },
  {
    id: "runAttempt",
    path: "/runs/:runRef",
    surface: "One-question-at-a-time attempt loop",
    requiredRoles: [],
  },
  {
    id: "runSummary",
    path: "/runs/:runRef/summary",
    surface: "Run result and practice re-entry",
    requiredRoles: [],
  },
  {
    id: "library",
    path: "/library",
    surface: "Shared problem browser",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "problemDetail",
    path: "/library/:problemRef",
    surface: "Published question detail",
    requiredRoles: ["instructor", "sysadmin"],
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
    surface: "Reusable curriculum inspection and editor",
    requiredRoles: ["instructor"],
  },
  {
    id: "workspaceList",
    path: "/workspace",
    surface: "Instructor drafts",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "workspaceEditor",
    path: "/workspace/:workspaceRef",
    surface: "Draft editor, validation, and preview",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentCreate",
    path: "/instructor/courses/:courseRef/assignments/new",
    surface: "Create persisted assignment draft and enter Questions",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentWorkspaceOverview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef",
    surface: "Instructor assignment workspace overview",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentWorkspaceQuestions",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/questions",
    surface: "Instructor assignment questions workspace",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentWorkspacePolicies",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/policies",
    surface: "Instructor assignment policies workspace",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentWorkspaceStudentView",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/student-view",
    surface: "Instructor assignment Student view",
    requiredRoles: ["instructor", "sysadmin"],
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
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentPreview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/delivery-check",
    surface: "Instructor-only assignment delivery check",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseRef/gradebook",
    surface: "Calculated Gradebook",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "studentWorkInspection",
    path: "/instructor/courses/:courseRef/gradebook/students/:membershipRef/assignments/:assignmentRef/runs/:runRef",
    surface: "Audited Student-work inspection",
    requiredRoles: ["instructor"],
  },
  {
    id: "courseGradeSettings",
    path: "/instructor/courses/:courseRef/grade-settings",
    surface: "Course grade settings and projected totals",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseRef/appearance",
    surface: "Course theme and entry-banner settings",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseRef/students",
    surface: "Course roster, invitations, and import",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "teachingOperations",
    path: "/instructor/courses/:courseRef/teaching-operations",
    surface: "Course teaching operations hub",
    requiredRoles: ["instructor", "sysadmin"],
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
