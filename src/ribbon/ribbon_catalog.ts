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
  | "assessmentsDueSoon"
  | "assessmentTemplates"
  | "assessmentOverview"
  | "assessmentQuestions"
  | "assessmentPolicies"
  | "assessmentStudentView"
  | "gradeSettings"
  | "appearance"
  | "backToAssessments";

export type RibbonDestinationId = RibbonTabId | RibbonTaskId;

export type RibbonTaskArea =
  | "instructorCourses"
  | "instructorQuestions"
  | "instructorAssessments"
  | "assessment"
  | "courseSetup"
  | "assessmentAttempt";

export interface RibbonTaskCatalogEntry extends RibbonCatalogControl<RibbonTaskId> {
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
    id: "productAssessments",
    label: "Assessments",
    destination: { kind: "route", routeId: "assessmentsDueSoon" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessments",
    label: "Assessments",
    destination: { kind: "route", routeId: "courseAssessments" },
    requiredParams: ["courseRef"],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "studentAssessments",
    label: "Assessments",
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
    destination: { kind: "future", futureId: "teachingOperations" },
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
    destination: { kind: "route", routeId: "assessmentAttempt" },
    requiredParams: ["assessmentAttemptRef"],
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
    id: "assessmentOverview",
    label: "Overview",
    destination: { kind: "route", routeId: "assessmentWorkspaceOverview" },
    requiredParams: ["courseRef", "assessmentRef"],
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
    requiredParams: ["courseRef", "assessmentRef"],
    taskGroup: "assessment",
    area: "assessment",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "assessmentPolicies",
    label: "Policies",
    destination: { kind: "route", routeId: "assessmentWorkspacePolicies" },
    requiredParams: ["courseRef", "assessmentRef"],
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
    requiredParams: ["courseRef", "assessmentRef"],
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
    id: "backToAssessments",
    label: "Back to Assessments",
    destination: { kind: "route", routeId: "assessmentOverview" },
    requiredParams: ["courseRef", "assessmentRef"],
    taskGroup: "assessmentAttempt",
    area: "assessmentAttempt",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
] as const satisfies ReadonlyArray<RibbonTaskCatalogEntry>;
