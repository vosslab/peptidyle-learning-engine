// ribbon_model_fixtures.ts - catalog-valid presentation models for Ribbon component evidence.

import type { ProductRole } from "../../generated/api/ProductRole";
import type { RouteParamName } from "../../src/navigation/route_params";
import type { ContentLayout, RibbonScope } from "../../src/route_contract";
import { buildRoutePath, type DeclaredRibbonRouteParams } from "../../src/ribbon/ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonDestinationId,
  type RibbonTaskArea,
  type RibbonTaskId,
} from "../../src/ribbon/ribbon_catalog";
import type {
  RibbonControlModel,
  RibbonModel,
  RibbonTaskAreaModel,
} from "../../src/ribbon/ribbon_contract";

const ALL_CATALOG_CONTROLS: ReadonlyArray<RibbonCatalogControl<RibbonDestinationId>> = [
  ...TAB_CATALOG,
  ...RIBBON_TASK_CATALOG,
];

const CANONICAL_FIXTURE_PARAMS = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3-M9QP",
  blueprintCourseRef: "BP-1",
} as const;

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find((candidate) => candidate.id === id);
  if (control === undefined) throw new Error(`Ribbon fixture references unknown catalog ID ${id}.`);
  return control as RibbonCatalogControl<Id>;
}

/**
 * `backToAssignments` needs a source course reference that an attempt-only
 * fixture does not own. Future entries have no backing route. Every other
 * catalog route must build here, so catalog drift fails at fixture creation.
 */
function isDocumentedUnavailableFixtureControl(
  catalog: RibbonCatalogControl<RibbonDestinationId>,
): boolean {
  return catalog.destination.kind === "future" || catalog.id === "backToAssignments";
}

function fixtureHrefFor(catalog: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (catalog.destination.kind !== "route" || catalog.id === "backToAssignments") {
    return undefined;
  }

  const mutableParams: Partial<Record<RouteParamName, string>> = {};
  for (const name of catalog.requiredParams) {
    const value = CANONICAL_FIXTURE_PARAMS[name];
    if (value === undefined) {
      throw new Error(`Ribbon fixture has no canonical value for ${catalog.id}:${name}.`);
    }
    mutableParams[name] = value;
  }
  const params: DeclaredRibbonRouteParams = mutableParams;
  const href = buildRoutePath(catalog.destination.routeId, params);
  if (href === undefined) {
    throw new Error(`Ribbon fixture cannot build declared route for ${catalog.id}.`);
  }
  return href;
}

for (const catalog of ALL_CATALOG_CONTROLS) {
  fixtureHrefFor(catalog);
}

function control<Id extends RibbonDestinationId>(
  id: Id,
  options: { readonly selected?: boolean; readonly available?: boolean } = {},
): RibbonControlModel<Id> {
  const catalog = catalogControl(id);
  const href = fixtureHrefFor(catalog);
  const documentedUnavailable = isDocumentedUnavailableFixtureControl(catalog);
  const available = !documentedUnavailable;
  if (options.available !== undefined && options.available !== available) {
    throw new Error(`Ribbon fixture cannot override admission for ${catalog.id}.`);
  }
  return {
    id: catalog.id,
    label: catalog.label,
    destination: catalog.destination,
    availability: available ? "Available" : "Unavailable",
    selected: options.selected ?? false,
    ...(available && href !== undefined ? { href } : {}),
    role: catalog.role,
    priority: catalog.priority,
    presentation: catalog.presentation,
    iconBearing: catalog.iconBearing,
    iconOnlySafe: catalog.iconOnlySafe,
  };
}

function area(
  id: RibbonTaskArea,
  label: string,
  controls: ReadonlyArray<RibbonControlModel<RibbonTaskId>>,
): RibbonTaskAreaModel {
  return { id, label, controls };
}

function model(
  scope: RibbonScope,
  productRole: ProductRole,
  tabs: RibbonModel["tabs"],
  taskAreas: RibbonModel["taskAreas"],
  contentLayout: ContentLayout,
  context: Omit<RibbonModel["context"], "productLabel">,
): RibbonModel {
  return {
    scope,
    contentLayout,
    context: {
      productLabel:
        productRole === "student"
          ? "Student"
          : productRole === "instructor"
            ? "Instructor"
            : "Sysadmin",
      ...context,
    },
    tabs,
    taskAreas,
  };
}

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign out" } as const;
const COURSE_TITLE = "Biochemistry I";

/** All exact scope-by-role schemas, with catalog-valid controls and real declared destinations. */
export const M6_RIBBON_FIXTURES = {
  productStudent: model(
    "product",
    "student",
    [control("courses", { selected: true })],
    [],
    "reading",
    {
      accountLabel: "Student account",
      signOutAction: SIGN_OUT,
    },
  ),
  productInstructor: model(
    "product",
    "instructor",
    [
      control("courses"),
      control("questionLibrary", { selected: true }),
      control("blueprintCourses"),
    ],
    [
      area("questionDestinations", "Question destinations", [
        control("allQuestions", { selected: true }),
        control("myQuestions"),
        control("myQuestionDrafts"),
      ]),
      area("questionRelationships", "Question relationships", [
        control("starred"),
        control("watched"),
      ]),
    ],
    "fullWidth",
    { accountLabel: "Instructor account", signOutAction: SIGN_OUT },
  ),
  productSysadmin: model(
    "product",
    "sysadmin",
    [control("courses", { selected: true }), control("instructorAccounts")],
    [],
    "reading",
    { accountLabel: "System administrator", signOutAction: SIGN_OUT },
  ),
  courseStudent: model(
    "courseInstance",
    "student",
    [control("assignments", { selected: true })],
    [],
    "reading",
    { accountLabel: "Student account", scopeLabel: COURSE_TITLE, signOutAction: SIGN_OUT },
  ),
  courseInstructor: model(
    "courseInstance",
    "instructor",
    [
      control("assignments", { selected: true }),
      control("students"),
      control("gradebook"),
      control("teachingOperations"),
      control("blueprintUpdates"),
      control("courseSetup"),
    ],
    [
      area("assignment", "Assignment", [
        control("assignmentOverview", { selected: true }),
        control("assignmentQuestions"),
        control("assignmentPolicies"),
        control("assignmentGradingOperations"),
        control("assignmentStudentView"),
      ]),
    ],
    "fullWidth",
    {
      accountLabel: "Instructor account",
      scopeLabel: COURSE_TITLE,
      assignmentLabel: "Problem Set 7",
      signOutAction: SIGN_OUT,
    },
  ),
  courseSysadmin: model(
    "courseInstance",
    "sysadmin",
    [control("teachingOperations", { selected: true })],
    [],
    "reading",
    { accountLabel: "System administrator", scopeLabel: COURSE_TITLE, signOutAction: SIGN_OUT },
  ),
  attemptStudent: model(
    "assignmentAttempt",
    "student",
    [control("attempt", { selected: true })],
    [area("assignmentAttempt", "Assignment attempt", [control("backToAssignments")])],
    "reading",
    {
      accountLabel: "Student account",
      assignmentLabel: "Problem Set 7",
      assignmentAttemptProgress: "Question 999 of 999",
      signOutAction: SIGN_OUT,
    },
  ),
  attemptInstructor: model("assignmentAttempt", "instructor", [], [], "reading", {
    accountLabel: "Instructor account",
    signOutAction: SIGN_OUT,
  }),
  attemptSysadmin: model("assignmentAttempt", "sysadmin", [], [], "reading", {
    accountLabel: "System administrator",
    signOutAction: SIGN_OUT,
  }),
  longCourse: model(
    "courseInstance",
    "instructor",
    [control("assignments", { selected: true }), control("students"), control("gradebook")],
    [
      area("assignment", "Assignment", [
        control("assignmentOverview"),
        control("assignmentQuestions", { selected: true }),
      ]),
    ],
    "fullWidth",
    {
      accountLabel: "Instructor account",
      scopeLabel:
        "Molecular Biology of the Cell: Evidence, Explanation, and Experimental Design " +
        "Across a Very Long Course Instance Title",
      assignmentLabel: "A deliberately long assignment label for a dense professional workspace",
      signOutAction: SIGN_OUT,
    },
  ),
  loadingCourse: model(
    "courseInstance",
    "student",
    [control("assignments", { selected: true })],
    [],
    "reading",
    {
      accountLabel: "Student account",
      scopeLabel: "Loading course title...",
      signOutAction: SIGN_OUT,
    },
  ),
  errorCourse: model(
    "courseInstance",
    "student",
    [control("assignments", { selected: true })],
    [],
    "reading",
    {
      accountLabel: "Student account",
      scopeLabel: "Unable to refresh course title",
      signOutAction: SIGN_OUT,
    },
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type M6RibbonFixtureName = keyof typeof M6_RIBBON_FIXTURES;
