// ribbon_catalog.ts - declared Ribbon navigation inventory, independent of capability admission.

import { type RibbonTabId, type RibbonTaskGroupId, type RouteId } from "../route_contract";
import type { RouteParamName } from "../navigation/route_params";

/** A navigation control's place in its task, never its physical size. */
export type RibbonControlRole = "primary" | "supporting";

/** The order in which a control remains directly visible during responsive collapse. */
export type RibbonControlPriority = "critical" | "normal";

/** Catalog-declared density preference, independent of importance and viewport. */
export type RibbonPresentation = "standard" | "compact";

/** The closed identities for destinations whose backend capability has not landed. */
export type FutureRibbonDestinationId =
  | "instructorAccounts"
  | "blueprintUpdates"
  | "courseSetup"
  | "myQuestions"
  | "myQuestionDrafts"
  | "starredQuestions"
  | "watchedQuestions"
  | "courseAppearance";

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
  | "allQuestions"
  | "myQuestions"
  | "myQuestionDrafts"
  | "starred"
  | "watched"
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
  | "questionDestinations"
  | "questionRelationships"
  | "assignment"
  | "courseSetup"
  | "assignmentAttempt";

export interface RibbonTaskCatalogEntry extends RibbonCatalogControl<RibbonTaskId> {
  readonly taskGroup: RibbonTaskGroupId;
  readonly area: RibbonTaskArea;
}

const textOnlyIconFlags = {
  iconBearing: false,
  iconOnlySafe: false,
} as const;

/** Glyphs supplement labels only where their conventional meaning improves scanning. */
const pairedIconFlags = {
  iconBearing: true,
  iconOnlySafe: false,
} as const;

/** The narrowest profile may retain these universally understood destination glyphs alone. */
const conventionalIconOnlyFlags = {
  iconBearing: true,
  iconOnlySafe: true,
} as const;

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
    id: "questionLibrary",
    label: "Question Library",
    destination: { kind: "route", routeId: "library" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...pairedIconFlags,
  },
  {
    id: "blueprintCourses",
    label: "Blueprint Courses",
    destination: { kind: "route", routeId: "blueprintCourses" },
    requiredParams: [],
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...textOnlyIconFlags,
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
    ...textOnlyIconFlags,
  },
  {
    id: "blueprintUpdates",
    label: "Blueprint Updates",
    destination: { kind: "future", futureId: "blueprintUpdates" },
    requiredParams: ["courseRef"],
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...textOnlyIconFlags,
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
    destination: { kind: "future", futureId: "instructorAccounts" },
    requiredParams: [],
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...textOnlyIconFlags,
  },
] as const satisfies ReadonlyArray<RibbonCatalogControl<RibbonTabId>>;

/**
 * Ordered Ribbon Tasks, including truthful future destinations. Task controls
 * navigate; Page Actions and Context Controls belong to their own surfaces.
 */
export const RIBBON_TASK_CATALOG = [
  {
    id: "allQuestions",
    label: "All Questions",
    destination: { kind: "route", routeId: "library" },
    requiredParams: [],
    taskGroup: "questionLibrary",
    area: "questionDestinations",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...textOnlyIconFlags,
  },
  {
    id: "myQuestions",
    label: "My Questions",
    destination: { kind: "future", futureId: "myQuestions" },
    requiredParams: [],
    taskGroup: "questionLibrary",
    area: "questionDestinations",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...textOnlyIconFlags,
  },
  {
    id: "myQuestionDrafts",
    label: "My Question Drafts",
    destination: { kind: "future", futureId: "myQuestionDrafts" },
    requiredParams: [],
    taskGroup: "questionLibrary",
    area: "questionDestinations",
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
    taskGroup: "questionLibrary",
    area: "questionRelationships",
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...conventionalIconOnlyFlags,
  },
  {
    id: "watched",
    label: "Watched",
    destination: { kind: "future", futureId: "watchedQuestions" },
    requiredParams: [],
    taskGroup: "questionLibrary",
    area: "questionRelationships",
    role: "supporting",
    priority: "normal",
    presentation: "compact",
    ...conventionalIconOnlyFlags,
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
    ...textOnlyIconFlags,
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
    ...textOnlyIconFlags,
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
    ...textOnlyIconFlags,
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
    ...textOnlyIconFlags,
  },
  {
    id: "appearance",
    label: "Appearance",
    destination: { kind: "future", futureId: "courseAppearance" },
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
    destination: { kind: "route", routeId: "courseAssignments" },
    requiredParams: ["courseRef"],
    taskGroup: "assignmentAttempt",
    area: "assignmentAttempt",
    role: "primary",
    priority: "critical",
    presentation: "standard",
    ...conventionalIconOnlyFlags,
  },
] as const satisfies ReadonlyArray<RibbonTaskCatalogEntry>;
