// ribbon_showcase_model.ts - explicit populated model for the Live Demo design preview.

import type { RouteParamName } from "../navigation/route_params";
import type { RibbonTabId } from "../route_contract";
import { buildRoutePath, type DeclaredRibbonRouteParams } from "./ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonDestinationId,
  type RibbonTaskId,
} from "./ribbon_catalog";
import type { RibbonControlModel, RibbonModel, RibbonTaskAreaModel } from "./ribbon_contract";

export const COURSE_INSTRUCTOR_SHOWCASE_TAB_IDS = [
  "assignments",
  "students",
  "gradebook",
  "teachingOperations",
  "blueprintUpdates",
  "courseSetup",
] as const satisfies ReadonlyArray<RibbonTabId>;

export const COURSE_INSTRUCTOR_SHOWCASE_TASK_IDS = [
  "assignmentOverview",
  "assignmentQuestions",
  "assignmentPolicies",
  "assignmentGradingOperations",
  "assignmentStudentView",
] as const satisfies ReadonlyArray<RibbonTaskId>;

export type CourseInstructorShowcaseTabId = (typeof COURSE_INSTRUCTOR_SHOWCASE_TAB_IDS)[number];
export type CourseInstructorShowcaseTaskId = (typeof COURSE_INSTRUCTOR_SHOWCASE_TASK_IDS)[number];

export interface RibbonShowcaseSelection {
  readonly tab: CourseInstructorShowcaseTabId;
  readonly task: CourseInstructorShowcaseTaskId;
}

export const DEFAULT_RIBBON_SHOWCASE_SELECTION: RibbonShowcaseSelection = Object.freeze({
  tab: "assignments",
  task: "assignmentOverview",
});

const ALL_CATALOG_CONTROLS: ReadonlyArray<RibbonCatalogControl<RibbonDestinationId>> = [
  ...TAB_CATALOG,
  ...RIBBON_TASK_CATALOG,
];

const CANONICAL_SHOWCASE_PARAMS = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3-M9QP",
  draftQuestionRef: "D-1",
  blueprintCourseRef: "BP-1",
} as const;

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find((candidate) => candidate.id === id);
  if (control === undefined) {
    throw new Error(`Ribbon showcase references unknown catalog ID ${id}.`);
  }
  return control as RibbonCatalogControl<Id>;
}

function showcaseHref(control: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (control.destination.kind !== "route") return undefined;

  const values: Partial<Record<RouteParamName, string>> = {};
  for (const name of control.requiredParams) values[name] = CANONICAL_SHOWCASE_PARAMS[name];
  const params: DeclaredRibbonRouteParams = values;
  const href = buildRoutePath(control.destination.routeId, params);
  if (href === undefined) {
    throw new Error(`Ribbon showcase cannot build declared route for ${control.id}.`);
  }
  return href;
}

function showcaseControl<Id extends RibbonDestinationId>(
  id: Id,
  selected: boolean,
): RibbonControlModel<Id> {
  const catalog = catalogControl(id);
  const href = showcaseHref(catalog);
  return Object.freeze({
    id: catalog.id,
    label: catalog.label,
    destination: catalog.destination,
    availability: href === undefined ? "Unavailable" : "Available",
    selected,
    ...(href === undefined ? {} : { href }),
    role: catalog.role,
    priority: catalog.priority,
    presentation: catalog.presentation,
    iconBearing: catalog.iconBearing,
    iconOnlySafe: catalog.iconOnlySafe,
  });
}

/**
 * Builds the established populated Instructor fixture without changing production admission.
 * Its canonical hrefs are intercepted by the showcase page and never claim a backed workflow.
 */
export function courseInstructorRibbonShowcaseModel(
  selection: RibbonShowcaseSelection,
): RibbonModel {
  const taskArea: RibbonTaskAreaModel = Object.freeze({
    id: "assignment",
    label: "Assignment",
    controls: Object.freeze(
      COURSE_INSTRUCTOR_SHOWCASE_TASK_IDS.map((id) => showcaseControl(id, id === selection.task)),
    ),
  });
  return Object.freeze({
    scope: "courseInstance",
    contentLayout: "fullWidth",
    context: Object.freeze({
      productLabel: "Instructor",
      accountLabel: "Instructor account",
      scopeLabel: "Biochemistry I",
      assignmentLabel: "Problem Set 7",
      signOutAction: Object.freeze({ kind: "action", id: "signOut", label: "Sign out" }),
    }),
    tabs: Object.freeze(
      COURSE_INSTRUCTOR_SHOWCASE_TAB_IDS.map((id) => showcaseControl(id, id === selection.tab)),
    ),
    taskAreas: Object.freeze([taskArea]),
  });
}
