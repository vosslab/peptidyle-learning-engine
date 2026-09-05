// route_contract.ts - pure data form of the frozen product route contract.

import type { ProductRole } from "../generated/api/ProductRole";

/** Stable presentation scope for a declared route; this is never authorization. */
export type RibbonScope = "product" | "courseInstance" | "assignmentAttempt";

/** Content geometry selected mechanically from the current route-level overrides. */
export type ContentLayout = "reading" | "fullWidth";

/** Designed Ribbon tabs, including unbacked catalog positions retained for future capabilities. */
export const RIBBON_TAB_IDS = [
  "courses",
  "questionLibrary",
  "blueprintCourses",
  "assignments",
  "students",
  "gradebook",
  "teachingOperations",
  "blueprintUpdates",
  "courseSetup",
  "attempt",
  "instructorAccounts",
] as const;

export type RibbonTabId = (typeof RIBBON_TAB_IDS)[number];

export type RibbonTaskGroupId =
  "questionLibrary" | "assignment" | "courseSetup" | "assignmentAttempt";

/** Route-selected Ribbon state. It describes presentation, not access permission. */
export interface RouteRibbonContract {
  readonly scope: RibbonScope;
  /** Omitted for a Ribbon Context Control route with no selected tab. */
  readonly tab?: RibbonTabId;
  /** Omitted when the reserved Task Row has no selected task area. */
  readonly taskGroup?: RibbonTaskGroupId;
  readonly contentLayout: ContentLayout;
}

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
  readonly ribbon: RouteRibbonContract;
}

const STUDENT_WORK_INSPECTION_PATH =
  "/instructor/courses/:courseRef/gradebook/students/:membershipRef/assignments/" +
  ":assignmentRef/assignment-attempts/:assignmentAttemptRef";

/** Product route order used by the application. */
export const ROUTE_CONTRACT = [
  {
    id: "courses",
    path: "/",
    surface: "Course list for the signed-in Product Role",
    requiredProductRoles: [],
    ribbon: { scope: "product", tab: "courses", contentLayout: "reading" },
  },
  {
    id: "signIn",
    path: "/sign-in",
    surface: "Passwordless account sign-in",
    requiredProductRoles: [],
    ribbon: { scope: "product", contentLayout: "reading" },
  },
  {
    id: "pendingCourseInvitations",
    path: "/account/course-invitations",
    surface: "Account-owned pending Course Invitations",
    requiredProductRoles: [],
    ribbon: { scope: "product", contentLayout: "reading" },
  },
  {
    id: "courseAssignments",
    path: "/courses/:courseRef",
    surface: "Assignment list with progress and Assignment Attempt counts",
    requiredProductRoles: [],
    ribbon: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  },
  {
    id: "assignmentOverview",
    path: "/courses/:courseRef/assignments/:assignmentRef",
    surface: "Assignment overview, Assignment Attempt history, and practice entry",
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  },
  {
    id: "assignmentAttempt",
    path: "/assignment-attempts/:assignmentAttemptRef",
    surface: "One-question-at-a-time attempt loop",
    requiredProductRoles: [],
    ribbon: {
      scope: "assignmentAttempt",
      tab: "attempt",
      taskGroup: "assignmentAttempt",
      contentLayout: "reading",
    },
  },
  {
    id: "assignmentAttemptSummary",
    path: "/assignment-attempts/:assignmentAttemptRef/summary",
    surface: "Assignment Attempt result and practice re-entry",
    requiredProductRoles: [],
    ribbon: {
      scope: "assignmentAttempt",
      tab: "attempt",
      taskGroup: "assignmentAttempt",
      contentLayout: "reading",
    },
  },
  {
    id: "library",
    path: "/library",
    surface: "Question Library",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "questionLibrary",
      taskGroup: "questionLibrary",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "questionDetail",
    path: "/library/:questionRef",
    surface: "Published question detail",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "questionLibrary",
      taskGroup: "questionLibrary",
      contentLayout: "reading",
    },
  },
  {
    id: "blueprintCourses",
    path: "/blueprint-courses",
    surface: "Blueprint Course workspace",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "product", tab: "blueprintCourses", contentLayout: "reading" },
  },
  {
    id: "blueprintCourseDetail",
    path: "/blueprint-courses/:blueprintCourseRef",
    surface: "Blueprint Course inspection and editor",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "product", tab: "blueprintCourses", contentLayout: "reading" },
  },
  {
    id: "assignmentCreate",
    path: "/instructor/courses/:courseRef/assignments/new",
    surface: "Create persisted Assignment and enter Questions",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  },
  {
    id: "assignmentWorkspaceOverview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef",
    surface: "Instructor assignment workspace overview",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assignments",
      taskGroup: "assignment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assignmentWorkspaceQuestions",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/questions",
    surface: "Instructor assignment questions workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assignments",
      taskGroup: "assignment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assignmentWorkspacePolicies",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/policies",
    surface: "Instructor assignment policies workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assignments",
      taskGroup: "assignment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assignmentWorkspaceStudentView",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/student-view",
    surface: "Instructor assignment Student view",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assignments",
      taskGroup: "assignment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assignmentWorkspaceGradingOperations",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/grading-operations",
    surface: "Instructor automated-grading operations workspace",
    // ASVS 8.3.1: mirror the server's explicit Instructor authority boundary.
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assignments",
      taskGroup: "assignment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assignmentPreview",
    path: "/instructor/courses/:courseRef/assignments/:assignmentRef/delivery-check",
    surface: "Instructor-only assignment delivery check",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "assignments", contentLayout: "reading" },
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseRef/gradebook",
    surface: "Calculated Gradebook",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "gradebook", contentLayout: "fullWidth" },
  },
  {
    id: "studentWorkInspection",
    path: STUDENT_WORK_INSPECTION_PATH,
    surface: "Audited Student-work inspection",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "gradebook", contentLayout: "reading" },
  },
  {
    id: "courseGradeSettings",
    path: "/instructor/courses/:courseRef/grade-settings",
    surface: "Course grade settings and projected totals",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "courseSetup",
      taskGroup: "courseSetup",
      contentLayout: "reading",
    },
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseRef/students",
    surface: "Course roster, invitations, and import",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "students", contentLayout: "fullWidth" },
  },
  {
    id: "teachingOperations",
    path: "/instructor/courses/:courseRef/teaching-operations",
    surface: "Course teaching operations hub",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "teachingOperations", contentLayout: "reading" },
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

/**
 * Checks a product route's role boundary without asserting Browser Surface
 * availability.
 */
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
