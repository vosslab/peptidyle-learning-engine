// route_contract.ts - pure data form of the frozen product route contract.

import type { UserRole } from "../generated/api/UserRole";

type InstructorRouteRole = Extract<UserRole, "instructor" | "sysadmin">;

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
    | "courseGradeSettings"
    | "courseAppearance"
    | "signIn"
    | "liveDemoSysadminSetup"
    | "emailAuthenticationComplete"
    | "emailChangeComplete"
    | "courseInvitation"
    | "accountSecurity"
    | "courseRoster"
    | "teachingOperations"
    | "assignmentAccess"
    | "assignmentPreview"
    | "pendingCoInstructorInvitations";
  readonly path: string;
  readonly surface: string;
  readonly requiredRoles: ReadonlyArray<InstructorRouteRole>;
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
    id: "liveDemoSysadminSetup",
    path: "/live-demo/sysadmin-setup",
    surface: "Operator-discovered live-demo administrator setup",
    requiredRoles: [],
  },
  {
    id: "emailAuthenticationComplete",
    path: "/auth/email/complete",
    surface: "Browser-bound one-time email completion",
    requiredRoles: [],
  },
  {
    id: "emailChangeComplete",
    path: "/auth/account/email/complete",
    surface: "Browser-bound account email change completion",
    requiredRoles: [],
  },
  {
    id: "courseInvitation",
    path: "/course-invitations/redeem",
    surface: "Authenticated learner invitation claim",
    requiredRoles: [],
  },
  {
    id: "accountSecurity",
    path: "/account/security",
    surface: "Multiple-passkey account management",
    requiredRoles: [],
  },
  {
    id: "pendingCoInstructorInvitations",
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
    requiredRoles: [],
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
    surface: "New assignment policy editor",
    requiredRoles: ["instructor", "sysadmin"],
  },
  {
    id: "assignmentEditor",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/edit",
    surface: "Assignment policy editor",
    requiredRoles: ["instructor", "sysadmin"],
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
    surface: "Summary-row gradebook",
    requiredRoles: ["instructor", "sysadmin"],
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
    surface: "Course roster, invitations, import, and grade export",
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
export function rolesMayAccessRoute(routeId: string, roles: ReadonlyArray<UserRole>): boolean {
  const route: RouteContract | undefined = ROUTE_CONTRACT.find((item) => item.id === routeId);
  if (route === undefined) {
    return false;
  }
  if (route.requiredRoles.length === 0) {
    return true;
  }
  return route.requiredRoles.some((requiredRole) => roles.includes(requiredRole));
}
