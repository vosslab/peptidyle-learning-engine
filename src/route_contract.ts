// route_contract.ts - pure data form of the frozen product route contract.

import type { ProductRole } from "../generated/api/ProductRole";

export interface RouteContract {
  readonly id:
    | "courses"
    | "courseAssignments"
    | "assignmentOverview"
    | "assignmentAttempt"
    | "assignmentAttemptSummary"
    | "library"
    | "questionDetail"
    | "blueprintCourses"
    | "blueprintCourseDetail"
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
    | "signIn"
    | "courseRoster"
    | "teachingOperations"
    | "assignmentPreview"
    | "pendingCourseInvitations";
  readonly path: string;
  readonly surface: string;
  /** Product Role gate for the route; each route declares the Product Roles it serves. */
  readonly requiredProductRoles: ReadonlyArray<ProductRole>;
}

/** Product route order used by the application. */
export const ROUTE_CONTRACT = [
  {
    id: "courses",
    path: "/",
    surface: "Course list for the signed-in Product Role",
    requiredProductRoles: [],
  },
  {
    id: "signIn",
    path: "/sign-in",
    surface: "Passwordless account sign-in",
    requiredProductRoles: [],
  },
  {
    id: "pendingCourseInvitations",
    path: "/account/course-invitations",
    surface: "Account-owned pending Course Invitations",
    requiredProductRoles: [],
  },
  {
    id: "courseAssignments",
    path: "/courses/:courseRef",
    surface: "Assignment list with progress and Assignment Attempt counts",
    requiredProductRoles: [],
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseRef/assignments/:assignmentRef",
    surface: "Assignment overview, Assignment Attempt history, and practice entry",
    requiredProductRoles: ["student"],
  },
  {
    id: "assignmentAttempt",
    path: "/assignment-attempts/:assignmentAttemptRef",
    surface: "One-question-at-a-time attempt loop",
    requiredProductRoles: [],
  },
  {
    id: "assignmentAttemptSummary",
    path: "/assignment-attempts/:assignmentAttemptRef/summary",
    surface: "Assignment Attempt result and practice re-entry",
    requiredProductRoles: [],
  },
  {
    id: "library",
    path: "/library",
    surface: "Question Library",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "questionDetail",
    path: "/library/:questionRef",
    surface: "Published question detail",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "blueprintCourses",
    path: "/blueprint-courses",
    surface: "Blueprint Course workspace",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "blueprintCourseDetail",
    path: "/blueprint-courses/:blueprintCourseRef",
    surface: "Blueprint Course inspection and editor",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "workspaceList",
    path: "/workspace",
    surface: "Instructor drafts",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "workspaceEditor",
    path: "/workspace/:workspaceRef",
    surface: "Draft editor, validation, and preview",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentCreate",
    path: "/instructor/courses/:courseRef/assignments/new",
    surface: "Create persisted Assignment and enter Questions",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceOverview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef",
    surface: "Instructor assignment workspace overview",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceQuestions",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/questions",
    surface: "Instructor assignment questions workspace",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspacePolicies",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/policies",
    surface: "Instructor assignment policies workspace",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceStudentView",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/student-view",
    surface: "Instructor assignment Student view",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentWorkspaceGradingOperations",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/grading-operations",
    surface: "Instructor automated-grading operations workspace",
    // ASVS 8.3.1: mirror the server's explicit Instructor authority boundary.
    requiredProductRoles: ["instructor"],
  },
  {
    id: "assignmentPreview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/delivery-check",
    surface: "Instructor-only assignment delivery check",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseRef/gradebook",
    surface: "Calculated Gradebook",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "studentWorkInspection",
    path: "/instructor/courses/:courseRef/gradebook/students/:membershipRef/assignments/:assignmentRef/assignment-attempts/:assignmentAttemptRef",
    surface: "Audited Student-work inspection",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "courseGradeSettings",
    path: "/instructor/courses/:courseRef/grade-settings",
    surface: "Course grade settings and projected totals",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseRef/students",
    surface: "Course roster, invitations, and import",
    requiredProductRoles: ["instructor"],
  },
  {
    id: "teachingOperations",
    path: "/instructor/courses/:courseRef/teaching-operations",
    surface: "Course teaching operations hub",
    requiredProductRoles: ["instructor"],
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
export function productRoleMayAccessRoute(routeId: string, productRole: ProductRole): boolean {
  const route: RouteContract | undefined = ROUTE_CONTRACT.find((item) => item.id === routeId);
  if (route === undefined) {
    return false;
  }
  if (route.requiredProductRoles.length === 0) {
    return true;
  }
  return route.requiredProductRoles.includes(productRole);
}
