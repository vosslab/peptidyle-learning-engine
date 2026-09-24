// ribbon_catalog.ts - declared Ribbon navigation inventory, independent of capability admission.

import type { ProductRole } from "../../generated/api/ProductRole";
import { type RibbonTabId, type RibbonTaskGroupId, type RouteId } from "../route_contract";
import type { RouteParamName } from "../navigation/route_params";

/** A navigation control's place in its task, never its physical size. */
export type RibbonControlRole = "primary" | "supporting";

/** The order in which a control remains directly visible during responsive collapse. */
export type RibbonControlPriority = "critical" | "normal";

/** Catalog-declared density preference, independent of importance and viewport. */
export type RibbonPresentation = "standard" | "compact";

/** Availability is explicit for a Context Control before its usable path lands. */
export type RibbonContextControlAvailability = "Available" | "Checking" | "Unavailable";

/** Context Controls occupy the account endcap rather than the navigation catalog. */
export type RibbonContextControlId = "profile";

export type RibbonContextControlGlyphKey = "profile";

/** A named account-endcap position. It is not a navigation destination. */
export interface RibbonContextControlCatalogEntry {
  readonly id: RibbonContextControlId;
  readonly label: string;
  readonly productRoles: ReadonlyArray<ProductRole>;
  readonly availability: RibbonContextControlAvailability;
  readonly glyph: RibbonContextControlGlyphKey;
}

/** The closed identities for destinations whose backend capability has not landed. */
export type FutureRibbonDestinationId =
  | "instructorAccounts"
  | "blueprintUpdates"
  | "courseSetup"
  | "myActiveCourses"
  | "myInactiveCourses"
  | "searchPublicBlueprintCourses"
  | "myQuestions"
  | "starredQuestions"
  | "watchedQuestions"
  | "teachingOperations"
  | "gradeSettings";

/** A destination is either a declared route or an honest future identity, never a URL guess. */
export type RibbonDestination =
  | { readonly kind: "route"; readonly routeId: RouteId }
  | { readonly kind: "future"; readonly futureId: FutureRibbonDestinationId };

/** Shared navigation metadata. It does not authorize access or perform an operation. */
export interface RibbonCatalogControl<Id extends string> {
  readonly id: Id;
  readonly label: string;
  readonly destination: RibbonDestination;
  /**
   * Task destinations carry their route parameters from the active scope.
   * Tier-one destinations instead resolve from the signed-in shell context.
   */
  readonly requiredParams?: ReadonlyArray<RouteParamName>;
  readonly role: RibbonControlRole;
  readonly priority: RibbonControlPriority;
  readonly presentation: RibbonPresentation;
  /** A glyph is included only where it adds useful recognition value. */
  readonly iconBearing: boolean;
  /** The narrowest profile may hide this label visually; it remains the accessible name. */
  readonly iconOnlySafe: boolean;
}

export type RibbonTaskId =
  | "myBlueprintCourses"
  | "myActiveCourses"
  | "myInactiveCourses"
  | "searchPublicBlueprintCourses"
  | "myQuestions"
  | "myDraftQuestions"
  | "starred"
  | "watched"
  | "searchQuestionLibrary"
  | "browseQuestionLibrary"
  | "assessmentsDueSoon"
  | "assessmentTemplates"
  | "assessments"
  | "students"
  | "gradebook"
  | "teachingOperations"
  | "blueprintUpdates"
  | "courseSetup"
  | "studentProgress"
  | "studentResponseStats"
  | "allCoursework"
  | "dueSoon"
  | "completedCoursework"
  | "activeAttempt"
  | "studentScores"
  | "studentAttemptHistory"
  | "assessmentOverview"
  | "assessmentQuestions"
  | "assessmentPolicies"
  | "assessmentStudentView"
  | "gradeSettings"
  | "appearance";

export type RibbonDestinationId = RibbonTabId | RibbonTaskId;

export type RibbonTaskArea =
  | "instructorCourses"
  | "instructorQuestions"
  | "instructorAssessments"
  | "studentCourses"
  | "studentCoursework"
  | "studentGrades"
  | "course"
  | "assessment"
  | "courseSetup";

export interface RibbonTaskCatalogEntry extends RibbonCatalogControl<RibbonTaskId> {
  readonly requiredParams: ReadonlyArray<RouteParamName>;
  readonly taskGroup: RibbonTaskGroupId;
  readonly area: RibbonTaskArea;
}

const pairedIconFlags = {
  iconBearing: true,
  iconOnlySafe: false,
} as const;

/**
 * Profile is a backed, authenticated-self account-endcap Context Control.
 */
export const RIBBON_CONTEXT_CONTROL_CATALOG = [
  {
    id: "profile",
    label: "Profile",
    productRoles: ["student", "instructor", "sysadmin"],
    availability: "Available",
    glyph: "profile",
  },
] as const satisfies ReadonlyArray<RibbonContextControlCatalogEntry>;

/**
 * Every designed tab has a fixed identity and canonical label before capability
 * admission. The capability registry decides whether a control is shown.
 */
export const TAB_CATALOG = [
  {
    id: "courses",
    label: "Courses",
    destination: { kind: "route", routeId: "courses" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "questions",
    label: "Questions",
    destination: { kind: "route", routeId: "library" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "productAssessments",
    label: "Assessments",
    destination: { kind: "route", routeId: "assessmentsDueSoon" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "coursework",
    label: "Coursework",
    destination: { kind: "route", routeId: "studentCourseLanding" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "grades",
    label: "Grades",
    destination: { kind: "route", routeId: "studentCourseGrades" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "instructorAccounts",
    label: "Instructor Accounts",
    destination: { kind: "route", routeId: "instructorAccounts" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "disciplines",
    label: "Disciplines",
    destination: { kind: "route", routeId: "contentDisciplines" },
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
] as const satisfies ReadonlyArray<RibbonCatalogControl<RibbonTabId>>;

/**
 * Ordered Ribbon Tasks, including truthful future destinations. Task controls
 * navigate; Page Actions and Context Controls belong to their own surfaces.
 */
export const RIBBON_TASK_CATALOG = [
  {
    id: "myBlueprintCourses",
    label: "My Blueprint Courses",
    destination: { kind: "route", routeId: "blueprintCourses" },
    requiredParams: [],
    taskGroup: "instructorCourses",
    area: "instructorCourses",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "myActiveCourses",
    label: "My Active Courses",
    destination: { kind: "route", routeId: "instructorHome" },
    requiredParams: [],
    taskGroup: "instructorCourses",
    area: "instructorCourses",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "myInactiveCourses",
    label: "My Inactive Courses",
    destination: { kind: "route", routeId: "instructorInactiveCourses" },
    requiredParams: [],
    taskGroup: "instructorCourses",
    area: "instructorCourses",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "searchPublicBlueprintCourses",
    label: "Search Public Blueprint Courses",
    destination: { kind: "route", routeId: "publicBlueprintSearch" },
    requiredParams: [],
    taskGroup: "instructorCourses",
    area: "instructorCourses",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "myQuestions",
    label: "My Questions",
    destination: { kind: "future", futureId: "myQuestions" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "myDraftQuestions",
    label: "My Draft Questions",
    destination: { kind: "route", routeId: "questionDrafts" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "starred",
    label: "Starred",
    destination: { kind: "future", futureId: "starredQuestions" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "watched",
    label: "Watched",
    destination: { kind: "route", routeId: "libraryWatchNotifications" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "searchQuestionLibrary",
    label: "Search Question Library",
    destination: { kind: "route", routeId: "library" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "browseQuestionLibrary",
    label: "Browse Question Library",
    destination: { kind: "route", routeId: "libraryBrowse" },
    requiredParams: [],
    taskGroup: "instructorQuestions",
    area: "instructorQuestions",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentsDueSoon",
    label: "Assessments Due Soon",
    destination: { kind: "route", routeId: "assessmentsDueSoon" },
    requiredParams: [],
    taskGroup: "instructorAssessments",
    area: "instructorAssessments",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentTemplates",
    label: "My Assessment Templates",
    destination: { kind: "route", routeId: "assessmentTemplates" },
    requiredParams: [],
    taskGroup: "instructorAssessments",
    area: "instructorAssessments",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessments",
    label: "Assessments",
    destination: { kind: "route", routeId: "courseAssessments" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "students",
    label: "Students",
    destination: { kind: "route", routeId: "courseRoster" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "gradebook",
    label: "Gradebook",
    destination: { kind: "route", routeId: "gradebook" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "teachingOperations",
    label: "Teaching Operations",
    destination: { kind: "future", futureId: "teachingOperations" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "blueprintUpdates",
    label: "Blueprint Updates",
    destination: { kind: "future", futureId: "blueprintUpdates" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...pairedIconFlags,
  },
  {
    id: "courseSetup",
    label: "Course Setup",
    destination: { kind: "future", futureId: "courseSetup" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "course",
    area: "course",
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...pairedIconFlags,
  },
  {
    id: "studentProgress",
    label: "Progress",
    destination: { kind: "route", routeId: "studentCourseProgress" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentCourses",
    area: "studentCourses",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "allCoursework",
    label: "All Coursework",
    destination: { kind: "route", routeId: "studentCourseLanding" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentCoursework",
    area: "studentCoursework",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "dueSoon",
    label: "Due Soon",
    destination: { kind: "route", routeId: "studentCourseDueSoon" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentCoursework",
    area: "studentCoursework",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "completedCoursework",
    label: "Completed",
    destination: { kind: "route", routeId: "studentCourseCompleted" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentCoursework",
    area: "studentCoursework",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "activeAttempt",
    label: "Active Attempt",
    destination: { kind: "route", routeId: "assessmentAttempt" },
    requiredParams: ["assessmentAttemptId"],
    taskGroup: "studentCoursework",
    area: "studentCoursework",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "studentScores",
    label: "Scores",
    destination: { kind: "route", routeId: "studentCourseGrades" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentGrades",
    area: "studentGrades",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "studentResponseStats",
    label: "Response Stats",
    destination: { kind: "route", routeId: "studentCourseResponseStats" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentGrades",
    area: "studentGrades",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "studentAttemptHistory",
    label: "Attempt History",
    destination: { kind: "route", routeId: "studentCourseAttemptHistory" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "studentGrades",
    area: "studentGrades",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentOverview",
    label: "Overview",
    destination: { kind: "route", routeId: "assessmentWorkspaceOverview" },
    requiredParams: ["courseInstanceId", "assessmentId"],
    taskGroup: "assessment",
    area: "assessment",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentQuestions",
    label: "Questions",
    destination: { kind: "route", routeId: "assessmentWorkspaceQuestions" },
    requiredParams: ["courseInstanceId", "assessmentId"],
    taskGroup: "assessment",
    area: "assessment",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentPolicies",
    label: "Properties",
    destination: { kind: "route", routeId: "assessmentWorkspacePolicies" },
    requiredParams: ["courseInstanceId", "assessmentId"],
    taskGroup: "assessment",
    area: "assessment",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentStudentView",
    label: "Student View",
    destination: { kind: "route", routeId: "assessmentWorkspaceStudentView" },
    requiredParams: ["courseInstanceId", "assessmentId"],
    taskGroup: "assessment",
    area: "assessment",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "gradeSettings",
    label: "Grade Settings",
    destination: { kind: "future", futureId: "gradeSettings" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "courseSetup",
    area: "courseSetup",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "appearance",
    label: "Appearance",
    destination: { kind: "route", routeId: "courseAppearance" },
    requiredParams: ["courseInstanceId"],
    taskGroup: "courseSetup",
    area: "courseSetup",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
] as const satisfies ReadonlyArray<RibbonTaskCatalogEntry>;
