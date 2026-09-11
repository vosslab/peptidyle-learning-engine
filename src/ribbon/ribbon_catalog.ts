// ribbon_catalog.ts - declared Ribbon navigation inventory, independent of capability admission.

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
  readonly productRole: "instructor";
  readonly availability: RibbonContextControlAvailability;
  readonly glyph: RibbonContextControlGlyphKey;
}

/** The closed identities for destinations whose backend capability has not landed. */
export type FutureRibbonDestinationId =
  | "instructorAccounts"
  | "supportRoster"
  | "blueprintUpdates"
  | "courseSetup"
  | "productAssignments"
  | "myActiveCourses"
  | "myInactiveCourses"
  | "searchPublicBlueprintCourses"
  | "myQuestions"
  | "starredQuestions"
  | "watchedQuestions"
  | "assignmentsDueSoon"
  | "assignmentTemplates";

/** A destination is either a declared route or an honest future identity, never a URL guess. */
export type RibbonDestination =
  | { readonly kind: "route"; readonly routeId: RouteId }
  | { readonly kind: "future"; readonly futureId: FutureRibbonDestinationId };

/** Shared navigation metadata. It does not authorize access or perform an operation. */
export interface RibbonCatalogControl<Id extends string> {
  readonly id: Id;
  readonly label: string;
  readonly destination: RibbonDestination;
  readonly requiredParams: ReadonlyArray<RouteParamName>;
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
  | "assignmentsDueSoon"
  | "assignmentTemplates"
  | "assignmentOverview"
  | "assignmentQuestions"
  | "assignmentPolicies"
  | "assignmentGradingOperations"
  | "assignmentStudentView"
  | "gradeSettings"
  | "appearance"
  | "backToAssignments";

export type RibbonDestinationId = RibbonTabId | RibbonTaskId;

export type RibbonTaskArea =
  | "instructorCourses"
  | "instructorQuestions"
  | "instructorAssignments"
  | "assignment"
  | "courseSetup"
  | "assignmentAttempt";

export interface RibbonTaskCatalogEntry extends RibbonCatalogControl<RibbonTaskId> {
  readonly taskGroup: RibbonTaskGroupId;
  readonly area: RibbonTaskArea;
}

const pairedIconFlags = {
  iconBearing: true,
  iconOnlySafe: false,
} as const;

/**
 * Instructor Profile has a settled account-endcap position, but no usable
 * browser path yet. M17 will back this exact declaration.
 */
export const RIBBON_CONTEXT_CONTROL_CATALOG = [
  {
    id: "profile",
    label: "Profile",
    productRole: "instructor",
    availability: "Unavailable",
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
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "questions",
    label: "Questions",
    destination: { kind: "route", routeId: "library" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "productAssignments",
    label: "Assignments",
    destination: { kind: "future", futureId: "productAssignments" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignments",
    label: "Assignments",
    destination: { kind: "route", routeId: "courseAssignments" },
    requiredParams: ["courseRef"],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "studentAssignments",
    label: "Assignments",
    destination: { kind: "route", routeId: "studentCourseLanding" },
    requiredParams: ["courseRef"],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "students",
    label: "Students",
    destination: { kind: "route", routeId: "courseRoster" },
    requiredParams: ["courseRef"],
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "gradebook",
    label: "Gradebook",
    destination: { kind: "route", routeId: "gradebook" },
    requiredParams: ["courseRef"],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "teachingOperations",
    label: "Teaching Operations",
    destination: { kind: "route", routeId: "teachingOperations" },
    requiredParams: ["courseRef"],
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "blueprintUpdates",
    label: "Blueprint Updates",
    destination: { kind: "future", futureId: "blueprintUpdates" },
    requiredParams: ["courseRef"],
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...pairedIconFlags,
  },
  {
    id: "courseSetup",
    label: "Course Setup",
    destination: { kind: "future", futureId: "courseSetup" },
    requiredParams: ["courseRef"],
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...pairedIconFlags,
  },
  {
    id: "attempt",
    label: "Attempt",
    destination: { kind: "route", routeId: "assignmentAttempt" },
    requiredParams: ["assignmentAttemptRef"],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "instructorAccounts",
    label: "Instructor Accounts",
    destination: { kind: "route", routeId: "instructorAccounts" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "supportRoster",
    label: "Scoped Support",
    destination: { kind: "route", routeId: "supportRoster" },
    requiredParams: [],
    role: "supporting",
    priority: "normal",
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
    destination: { kind: "future", futureId: "myActiveCourses" },
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
    destination: { kind: "future", futureId: "myInactiveCourses" },
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
    destination: { kind: "future", futureId: "searchPublicBlueprintCourses" },
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
    destination: { kind: "future", futureId: "watchedQuestions" },
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
    id: "assignmentsDueSoon",
    label: "Assignments Due Soon",
    destination: { kind: "future", futureId: "assignmentsDueSoon" },
    requiredParams: [],
    taskGroup: "instructorAssignments",
    area: "instructorAssignments",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentTemplates",
    label: "My Assignment Templates",
    destination: { kind: "future", futureId: "assignmentTemplates" },
    requiredParams: [],
    taskGroup: "instructorAssignments",
    area: "instructorAssignments",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentOverview",
    label: "Overview",
    destination: { kind: "route", routeId: "assignmentWorkspaceOverview" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignment",
    area: "assignment",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentQuestions",
    label: "Questions",
    destination: { kind: "route", routeId: "assignmentWorkspaceQuestions" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignment",
    area: "assignment",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentPolicies",
    label: "Policies",
    destination: { kind: "route", routeId: "assignmentWorkspacePolicies" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignment",
    area: "assignment",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentGradingOperations",
    label: "Grading Operations",
    destination: { kind: "route", routeId: "assignmentWorkspaceGradingOperations" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignment",
    area: "assignment",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assignmentStudentView",
    label: "Student View",
    destination: { kind: "route", routeId: "assignmentWorkspaceStudentView" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignment",
    area: "assignment",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "gradeSettings",
    label: "Grade Settings",
    destination: { kind: "route", routeId: "courseGradeSettings" },
    requiredParams: ["courseRef"],
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
    requiredParams: ["courseRef"],
    taskGroup: "courseSetup",
    area: "courseSetup",
    role: "supporting",
    priority: "normal",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "backToAssignments",
    label: "Back to Assignments",
    destination: { kind: "route", routeId: "assignmentOverview" },
    requiredParams: ["courseRef", "assignmentRef"],
    taskGroup: "assignmentAttempt",
    area: "assignmentAttempt",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
] as const satisfies ReadonlyArray<RibbonTaskCatalogEntry>;
