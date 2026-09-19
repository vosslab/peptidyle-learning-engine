// route_contract.ts - pure data form of the frozen product route contract.

import type { ProductRole } from "../generated/api/ProductRole";

/** Stable presentation scope for a declared route; this is never authorization. */
export type RibbonScope = "product" | "courseInstance" | "assessmentAttempt";

/** Content geometry selected mechanically from the current route-level overrides. */
export type ContentLayout = "reading" | "fullWidth";

/** Designed Ribbon tabs, including unbacked catalog positions retained for future capabilities. */
export const RIBBON_TAB_IDS = [
  "courses",
  "questions",
  "productAssessments",
  "assessments",
  "studentAssessments",
  "students",
  "gradebook",
  "teachingOperations",
  "blueprintUpdates",
  "courseSetup",
  "attempt",
  "instructorAccounts",
  "disciplines",
] as const;

export type RibbonTabId = (typeof RIBBON_TAB_IDS)[number];

export type RibbonTaskGroupId =
  | "instructorCourses"
  | "instructorQuestions"
  | "instructorAssessments"
  | "assessment"
  | "courseSetup"
  | "assessmentAttempt";

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
    | "instructorHome"
    | "instructorInactiveCourses"
    | "studentHome"
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
    | "accountSettings"
    | "courseRoster"
    | "instructorAccounts"
    | "contentDisciplines"
    | "pendingCourseInvitations"
    | "studentCourseInvitations"
    | "studentCourseInvitation"
    | "studentCourseLanding";
  readonly path: string;
  readonly surface: string;
  /** Product Role gate for the route; each route declares the Product Roles it serves. */
  readonly requiredProductRoles: ReadonlyArray<ProductRole>;
  readonly ribbon: RouteRibbonContract;
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
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "changeProposalDetail",
    path: "/blueprint-change-proposals/:proposalId",
    surface: "Participant frozen Blueprint Change Proposal review",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "courses",
    path: "/",
    surface: "Signed-in Product Role home resolution",
    requiredProductRoles: [],
    ribbon: {
      scope: "product",
      contentLayout: "reading",
    },
  },
  {
    id: "instructorHome",
    path: "/instructor",
    surface: "Instructor Course Instance home dashboard",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "reading",
    },
  },
  {
    id: "instructorInactiveCourses",
    path: "/instructor/courses/inactive",
    surface: "Instructor past Course Instance list",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "reading",
    },
  },
  {
    id: "studentHome",
    path: "/student",
    surface: "Student learning home dashboard",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tab: "courses", contentLayout: "reading" },
  },
  {
    id: "sysadminHome",
    path: "/sysadmin",
    surface: "Sysadmin operations home dashboard",
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tab: "courses", contentLayout: "reading" },
  },
  {
    id: "profile",
    path: "/profile",
    surface: "Authenticated Account profile",
    requiredProductRoles: ["student", "instructor", "sysadmin"],
    ribbon: { scope: "product", contentLayout: "reading" },
  },
  {
    id: "accountSettings",
    path: "/account-settings",
    surface: "Authenticated Account Settings",
    requiredProductRoles: ["student", "instructor", "sysadmin"],
    ribbon: { scope: "product", contentLayout: "reading" },
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
    id: "studentCourseInvitations",
    path: "/student/course-invitations",
    surface: "Student pending Course Invitation index",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", tab: "courses", contentLayout: "reading" },
  },
  {
    id: "studentCourseInvitation",
    path: "/courses/:courseInstanceId/invitation",
    surface: "Student Course Invitation acceptance",
    requiredProductRoles: ["student"],
    ribbon: { scope: "product", contentLayout: "reading" },
  },
  {
    id: "studentCourseLanding",
    path: "/student/courses/:courseInstanceId",
    surface: "Student answer-free Course Instance and released Assessment landing",
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tab: "studentAssessments", contentLayout: "reading" },
  },
  {
    id: "instructorAccounts",
    path: "/sysadmin/instructor-accounts",
    surface: "Sysadmin Instructor Account lifecycle workspace",
    // ASVS 8.3.1: browser route gating mirrors the server's Sysadmin-only policy.
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tab: "instructorAccounts", contentLayout: "reading" },
  },
  {
    id: "contentDisciplines",
    path: "/sysadmin/disciplines",
    surface: "Sysadmin Discipline lifecycle workspace",
    requiredProductRoles: ["sysadmin"],
    ribbon: { scope: "product", tab: "disciplines", contentLayout: "reading" },
  },
  {
    id: "courseAssessments",
    path: "/courses/:courseInstanceId",
    surface: "Course Instance Teaching Team, roster, and Assessment delivery workspace",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "assessments", contentLayout: "reading" },
  },
  {
    id: "assessmentOverview",
    path: "/courses/:courseInstanceId/assessments/:assessmentId",
    surface: "Student Assessment Access",
    // ASVS 8.3.1: client admission targets the separately role-gated Student landing route;
    // the server remains the authorization boundary for the exact Student Record.
    requiredProductRoles: ["student"],
    ribbon: { scope: "courseInstance", tab: "studentAssessments", contentLayout: "reading" },
  },
  {
    id: "assessmentAttempt",
    path: "/assessment-attempts/:assessmentAttemptId",
    surface: "One-question-at-a-time attempt loop",
    requiredProductRoles: ["student"],
    ribbon: {
      scope: "assessmentAttempt",
      tab: "attempt",
      taskGroup: "assessmentAttempt",
      contentLayout: "reading",
    },
  },
  {
    id: "assessmentAttemptSummary",
    path: "/assessment-attempts/:assessmentAttemptId/summary",
    surface: "Assessment Attempt result and practice re-entry",
    requiredProductRoles: ["student"],
    ribbon: {
      scope: "assessmentAttempt",
      tab: "attempt",
      taskGroup: "assessmentAttempt",
      contentLayout: "reading",
    },
  },
  {
    id: "library",
    path: "/library",
    surface: "Question Library",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "libraryBrowse",
    path: "/library/browse",
    surface: "Browse Question Library",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "libraryWatchNotifications",
    path: "/library/watch-notifications",
    surface: "Instructor private Question Library Watch inbox",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "reading",
    },
  },
  {
    id: "questionDetail",
    path: "/library/:questionId",
    surface: "Published question detail",
    requiredProductRoles: ["instructor", "sysadmin"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "reading",
    },
  },
  {
    id: "questionDrafts",
    path: "/authoring/drafts",
    surface: "My Question Drafts private Authoring Workspace view",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "questionDraftEditor",
    path: "/authoring/drafts/:draftQuestionId",
    surface: "Private Draft Question editor",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "questions",
      taskGroup: "instructorQuestions",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "blueprintCourses",
    path: "/blueprint-courses",
    surface: "Blueprint Course workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "reading",
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
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "reading",
    },
  },
  {
    id: "blueprintCourseDetail",
    path: "/blueprint-courses/:blueprintCourseId",
    surface: "Blueprint Course inspection and editor",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "courses",
      taskGroup: "instructorCourses",
      contentLayout: "reading",
    },
  },
  {
    id: "assessmentsDueSoon",
    path: "/assessments/due-soon",
    surface: "Instructor cross-Course Assessments Due Soon",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "productAssessments",
      taskGroup: "instructorAssessments",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assessmentTemplates",
    path: "/assessment-templates",
    surface: "Instructor-owned Assessment Templates",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "product",
      tab: "productAssessments",
      taskGroup: "instructorAssessments",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assessmentCreate",
    path: "/instructor/courses/:courseInstanceId/assessments/new",
    surface: "Create persisted Assessment and enter Questions",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "assessments", contentLayout: "reading" },
  },
  {
    id: "assessmentWorkspaceOverview",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId",
    surface: "Instructor assessment workspace overview",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assessments",
      taskGroup: "assessment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assessmentWorkspaceQuestions",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/questions",
    surface: "Instructor assessment questions workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assessments",
      taskGroup: "assessment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assessmentWorkspacePolicies",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/properties",
    surface: "Instructor Assessment Properties workspace",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assessments",
      taskGroup: "assessment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "assessmentWorkspaceStudentView",
    path: "/instructor/courses/:courseInstanceId/assessments/:assessmentId/student-view",
    surface: "Instructor assessment Student view",
    requiredProductRoles: ["instructor"],
    ribbon: {
      scope: "courseInstance",
      tab: "assessments",
      taskGroup: "assessment",
      contentLayout: "fullWidth",
    },
  },
  {
    id: "gradebook",
    path: "/instructor/courses/:courseInstanceId/gradebook",
    surface: "Answer-free Gradebook evidence",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "gradebook", contentLayout: "fullWidth" },
  },
  {
    id: "courseAppearance",
    path: "/instructor/courses/:courseInstanceId/appearance",
    surface: "Instructor Course Appearance",
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
    path: "/instructor/courses/:courseInstanceId/students",
    surface: "Course roster, invitations, and import",
    requiredProductRoles: ["instructor"],
    ribbon: { scope: "courseInstance", tab: "students", contentLayout: "fullWidth" },
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
