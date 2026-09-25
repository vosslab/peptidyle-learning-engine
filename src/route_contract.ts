// route_contract.ts - pure data form of the frozen product route contract.

import type { ProductRole } from "../generated/api/ProductRole";

/** Stable presentation scope for a declared route; this is never authorization. */
export type RibbonScope = "product" | "courseInstance" | "assessmentAttempt";

/** Page geometry selected by a route; reading is the implicit default. */
export type ContentLayout = "reading" | "fullWidth";

/** The role-level Ribbon tab that conceptually contains a route. */
export type TierOneArea =
  | "courses"
  | "questions"
  | "productAssessments"
  | "coursework"
  | "grades"
  | "instructorAccounts"
  | "disciplines"
  | "account";

/** Designed Ribbon tabs, including unbacked catalog positions retained for future capabilities. */
export const RIBBON_TAB_IDS = [
  "courses",
  "questions",
  "productAssessments",
  "coursework",
  "grades",
  "instructorAccounts",
  "disciplines",
] as const;

export type RibbonTabId = (typeof RIBBON_TAB_IDS)[number];

/** Route-selected Ribbon state. It describes presentation, not access permission. */
export interface RouteRibbonContract {
  readonly scope: RibbonScope;
  /** Role-level Ribbon tab that selects this route. */
  readonly tierOneArea: TierOneArea;
}

export interface RouteContract {
  readonly id:
    | "courses"
    | "instructorHome"
    | "instructorInactiveCourses"
    | "studentHome"
    | "studentCourses"
    | "sysadminHome"
    | "courseAssessments"
    | "assessmentOverview"
    | "assessmentAttempt"
    | "assessmentAttemptSummary"
    | "library"
    | "libraryBrowse"
    | "libraryWatchNotifications"
    | "questionDetail"
    | "questionDrafts"
    | "questionDraftEditor"
    | "blueprintCourses"
    | "publicBlueprintSearch"
    | "blueprintCourseDetail"
    | "myChangeProposals"
    | "changeProposalDetail"
    | "assessmentCreate"
    | "assessmentWorkspaceOverview"
    | "assessmentWorkspaceQuestions"
    | "assessmentWorkspacePolicies"
    | "assessmentWorkspaceStudentView"
    | "assessmentsDueSoon"
    | "assessmentTemplates"
    | "gradebook"
    | "courseAppearance"
    | "signIn"
    | "profile"
    | "courseRoster"
    | "instructorAccounts"
    | "contentDisciplines"
    | "pendingCourseInvitations"
    | "studentCourseInvitations"
    | "studentCourseInvitation"
    | "studentCourseLanding"
    | "studentScores"
    | "studentCourseProgress"
    | "studentResponseStats"
    | "studentDueSoon"
    | "studentCompleted"
    | "studentAttemptHistory";
  readonly path: string;
  readonly surface: string;
  /** Product Role gate for the route; each route declares the Product Roles it serves. */
  readonly requiredProductRoles: ReadonlyArray<ProductRole>;
  readonly ribbon: RouteRibbonContract;
  /** Omit for the normal reading width; dense workspaces may select full width. */
  readonly pageLayout?: "fullWidth";
}

/** Product route order used by the application. */
export const ROUTE_CONTRACT = [
  {
    id: "myChangeProposals",
    path: "/blueprint-change-proposals",
    surface: "Instructor own Blueprint Change Proposal records",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "changeProposalDetail",
    path: "/blueprint-change-proposals/:proposalId",
    surface: "Participant frozen Blueprint Change Proposal review",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "courses",
    path: "/",
    surface: "Signed-in Product Role home resolution",
    requiredProductRoles: [],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "instructorHome",
    path: "/instructor",
    surface: "Instructor Course Instance home dashboard",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "instructorInactiveCourses",
    path: "/instructor/courses/inactive",
    surface: "Instructor past Course Instance list",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "studentHome",
    path: "/student",
    surface: "All Coursework across the Student's enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "coursework" },
  },
  {
    id: "studentCourses",
    path: "/student/courses",
    surface: "Student's current Course list and invitations",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "courses" },
  },
  {
    id: "sysadminHome",
    path: "/sysadmin",
    surface: "Sysadmin operations home dashboard",
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tierOneArea: "courses" },
  },
  {
    id: "profile",
    path: "/profile",
    surface: "Authenticated Account profile",
    requiredProductRoles: ["student", "instructor", "sysadmin"],
    ribbon: { scope: "product", tierOneArea: "account" },
  },
  {
    id: "signIn",
    path: "/sign-in",
    surface: "Passwordless account sign-in",
    requiredProductRoles: [],
    ribbon: { scope: "product", tierOneArea: "account" },
  },
  {
    id: "pendingCourseInvitations",
    path: "/account/course-invitations",
    surface: "Account-owned pending Course Invitations",
    requiredProductRoles: [],
    ribbon: { scope: "product", tierOneArea: "account" },
  },
  {
    id: "studentCourseInvitations",
    path: "/student/course-invitations",
    surface: "Student pending Course Invitation index",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "courses" },
  },
  {
    id: "studentCourseInvitation",
    path: "/courses/:courseInstanceId/invitation",
    surface: "Student Course Invitation acceptance",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "account" },
  },
  {
    id: "studentCourseLanding",
    path: "/student/courses/:courseInstanceId",
    surface: "Student Course content",
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tierOneArea: "courses" },
  },
  {
    id: "studentCourseProgress",
    path: "/student/courses/:courseInstanceId/progress",
    surface: "Student self-only Course Progress",
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tierOneArea: "courses" },
  },
  {
    id: "studentResponseStats",
    path: "/student/grades/response-stats",
    surface: "Student self-only Response Stats across enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "grades" },
  },
  {
    id: "studentDueSoon",
    path: "/student/due-soon",
    surface: "Student Coursework due soon across enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "coursework" },
  },
  {
    id: "studentCompleted",
    path: "/student/completed",
    surface: "Student completed Coursework across enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "coursework" },
  },
  {
    id: "studentScores",
    path: "/student/grades",
    surface: "Student released Scores across enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "grades" },
  },
  {
    id: "studentAttemptHistory",
    path: "/student/grades/attempt-history",
    surface: "Student Attempt History across enrolled Courses",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tierOneArea: "grades" },
  },
  {
    id: "instructorAccounts",
    path: "/sysadmin/instructor-accounts",
    surface: "Sysadmin Instructor Account lifecycle workspace",
    // ASVS 8.3.1: browser route gating mirrors the server's Sysadmin-only policy.
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tierOneArea: "instructorAccounts" },
  },
  {
    id: "contentDisciplines",
    path: "/sysadmin/disciplines",
    surface: "Sysadmin Discipline lifecycle workspace",
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tierOneArea: "disciplines" },
  },
  {
    id: "courseAssessments",
    path: "/courses/:courseInstanceId",
    surface: "Course Instance Teaching Team, roster, and Assessment delivery workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "courses",
    },
  },
  {
    id: "assessmentOverview",
    path: "/courses/:courseInstanceId/assessments/:assessmentId",
    surface: "Student Assessment Access",
    // ASVS 8.3.1: client admission targets the separately role-gated Student landing route;
    // the server remains the authorization boundary for the exact Student Record.
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tierOneArea: "coursework" },
  },
  {
    id: "assessmentAttempt",
    path: "/assessment-attempts/:assessmentAttemptId",
    surface: "One-question-at-a-time attempt loop",
    requiredProductRoles: ["student"],
    ribbon: {
      scope: "assessmentAttempt",
      tierOneArea: "coursework",
    },
  },
  {
    id: "assessmentAttemptSummary",
    path: "/assessment-attempts/:assessmentAttemptId/summary",
    surface: "Assessment Attempt results and feedback",
    requiredProductRoles: ["student"],
    ribbon: {
      scope: "assessmentAttempt",
      tierOneArea: "grades",
    },
  },
  {
    id: "library",
    path: "/library",
    surface: "Question Library",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "libraryBrowse",
    path: "/library/browse",
    surface: "Browse Question Library",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "libraryWatchNotifications",
    path: "/library/watch-notifications",
    surface: "Instructor private Question Library Watch inbox",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
  },
  {
    id: "questionDetail",
    path: "/library/:questionId",
    surface: "Published question detail",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
  },
  {
    id: "questionDrafts",
    path: "/authoring/drafts",
    surface: "My Question Drafts private Authoring Workspace view",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "questionDraftEditor",
    path: "/authoring/drafts/:draftQuestionId",
    surface: "Private Draft Question editor",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "questions",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "blueprintCourses",
    path: "/blueprint-courses",
    surface: "Blueprint Course workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "publicBlueprintSearch",
    path: "/blueprint-courses/search/public",
    surface: "Search Public Blueprint Courses",
    // ASVS 8.3.1: browser admission mirrors the server's Instructor-only boundary.
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "blueprintCourseDetail",
    path: "/blueprint-courses/:blueprintCourseId",
    surface: "Blueprint Course inspection and editor",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "courses",
    },
  },
  {
    id: "assessmentsDueSoon",
    path: "/assessments/due-soon",
    surface: "Instructor cross-Course Assessments Due Soon",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "assessmentTemplates",
    path: "/assessment-templates",
    surface: "Instructor-owned Assessment Templates",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "assessmentCreate",
    path: "/instructor/courses/:courseInstanceId/assessments/new",
    surface: "Create persisted Assessment and enter Questions",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
  },
  {
    id: "assessmentWorkspaceOverview",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId",
    surface: "Instructor assessment workspace overview",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "assessmentWorkspaceQuestions",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/questions",
    surface: "Instructor assessment questions workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "assessmentWorkspacePolicies",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties",
    surface: "Instructor Assessment Properties workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "assessmentWorkspaceStudentView",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/student-view",
    surface: "Instructor assessment Student view",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseInstanceId/gradebook",
    surface: "Answer-free Gradebook evidence",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "productAssessments",
    },
    pageLayout: "fullWidth",
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseInstanceId/appearance",
    surface: "Instructor Course Appearance",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "courses",
    },
  },
  {
    id: "courseRoster",
    path: "/instructor/courses/:courseInstanceId/students",
    surface: "Course roster, invitations, and import",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tierOneArea: "courses",
    },
    pageLayout: "fullWidth",
  },
] as const satisfies ReadonlyArray<RouteContract>;

export type RouteId = (typeof ROUTE_CONTRACT)[number]["id"];

/** One explicit Product Role home route, used by root and Ribbon home resolution. */
export function productRoleHomeRouteId(productRole: ProductRole): RouteId {
  switch (productRole) {
    case "instructor":
      return "instructorHome";
    case "student":
      return "studentHome";
    case "sysadmin":
      return "sysadminHome";
  }
}

/** Canonical, role-scoped home path. It is a declared route rather than a URL convention. */
export function productRoleHomePath(productRole: ProductRole): string {
  const routeId = productRoleHomeRouteId(productRole);
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
  if (route === undefined)
    throw new Error(`Product Role home route is missing for ${productRole}.`);
  return route.path;
}

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
