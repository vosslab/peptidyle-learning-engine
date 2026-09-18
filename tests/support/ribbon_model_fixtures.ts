// ribbon_model_fixtures.ts - catalog-valid presentation models for Ribbon component evidence.

import type { ProductRole } from "../../generated/api/ProductRole";
import type { RouteParamName } from "../../src/navigation/route_params";
import type { ContentLayout, RibbonScope } from "../../src/route_contract";
import { buildRoutePath, type DeclaredRibbonRouteParams } from "../../src/ribbon/ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
  RIBBON_CONTEXT_CONTROL_CATALOG,
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
  courseRef: "CI7K3M2QAZ",
  assessmentRef: "A9D2RX5AF",
  assessmentAttemptRef: "00000000-0000-0000-0000-000000000001",
  membershipRef: "M-1",
  questionRef: "7K3M-79QP",
  draftQuestionId: "0198e000-0000-7000-8000-000000000001",
  blueprintCourseRef: "BP7K3M2QAF",
  proposalId: "e3396265-6653-4c65-bc9b-8d869c142d87",
} as const satisfies Readonly<Record<RouteParamName, string>>;

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find(
    (candidate): candidate is RibbonCatalogControl<Id> => candidate.id === id,
  );
  if (control === undefined) throw new Error(`Ribbon fixture references unknown catalog ID ${id}.`);
  return control;
}

/**
 * `backToAssessments` needs a source course reference that an attempt-only
 * fixture does not own. Future entries have no backing route. Every other
 * catalog route must build here, so catalog drift fails at fixture creation.
 */
function isDocumentedUnavailableFixtureControl(
  catalog: RibbonCatalogControl<RibbonDestinationId>,
): boolean {
  return catalog.destination.kind !== "route" || catalog.id === "backToAssessments";
}

function fixtureHrefFor(catalog: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (catalog.destination.kind !== "route" || catalog.id === "backToAssessments") {
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
  context: Omit<RibbonModel["context"], "productLabel" | "accountControls">,
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
      accountControls: RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) =>
        control.productRoles.includes(productRole),
      ),
    },
    tabs,
    taskAreas,
    breadcrumbs: [],
    breadcrumbPreludeReserved: false,
  };
}

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign out" } as const;
const COURSE_SHORT_NAME = "BCHM 355";

/** All exact scope-by-role schemas, with catalog-valid controls and real declared destinations. */
export const M6_RIBBON_FIXTURES = {
  productStudent: model(
    "product",
    "student",
    [control("courses", { selected: true })],
    [],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  productInstructor: model(
    "product",
    "instructor",
    [control("courses"), control("questions", { selected: true }), control("productAssessments")],
    [
      area("instructorQuestions", "Questions", [
        control("myQuestions"),
        control("myDraftQuestions"),
        control("starred"),
        control("watched"),
        control("searchQuestionLibrary", { selected: true }),
        control("browseQuestionLibrary"),
      ]),
    ],
    "fullWidth",
    { signOutAction: SIGN_OUT },
  ),
  productSysadmin: model(
    "product",
    "sysadmin",
    [control("courses", { selected: true }), control("instructorAccounts")],
    [],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  courseStudent: model(
    "courseInstance",
    "student",
    [control("studentAssessments", { selected: true })],
    [],
    "reading",
    { scopeLabel: COURSE_SHORT_NAME, signOutAction: SIGN_OUT },
  ),
  courseInstructor: model(
    "courseInstance",
    "instructor",
    [
      control("assessments", { selected: true }),
      control("students"),
      control("gradebook"),
      control("teachingOperations"),
      control("blueprintUpdates"),
      control("courseSetup"),
    ],
    [
      area("assessment", "Assessment", [
        control("assessmentOverview", { selected: true }),
        control("assessmentQuestions"),
        control("assessmentPolicies"),
        control("assessmentStudentView"),
      ]),
    ],
    "fullWidth",
    {
      scopeLabel: COURSE_SHORT_NAME,
      assessmentLabel: "Problem Set 7",
      signOutAction: SIGN_OUT,
    },
  ),
  courseSysadmin: model(
    "courseInstance",
    "sysadmin",
    [control("teachingOperations", { selected: true })],
    [],
    "reading",
    {
      scopeLabel: COURSE_SHORT_NAME,
      signOutAction: SIGN_OUT,
    },
  ),
  attemptStudent: model(
    "assessmentAttempt",
    "student",
    [control("attempt", { selected: true })],
    [area("assessmentAttempt", "Assessment attempt", [control("backToAssessments")])],
    "reading",
    {
      assessmentLabel: "Problem Set 7",
      assessmentAttemptProgress: "Question 999 of 999",
      signOutAction: SIGN_OUT,
    },
  ),
  attemptInstructor: model("assessmentAttempt", "instructor", [], [], "reading", {
    signOutAction: SIGN_OUT,
  }),
  attemptSysadmin: model("assessmentAttempt", "sysadmin", [], [], "reading", {
    signOutAction: SIGN_OUT,
  }),
  longCourse: model(
    "courseInstance",
    "instructor",
    [control("assessments", { selected: true }), control("students"), control("gradebook")],
    [
      area("assessment", "Assessment", [
        control("assessmentOverview"),
        control("assessmentQuestions", { selected: true }),
      ]),
    ],
    "fullWidth",
    {
      scopeLabel:
        "Molecular Biology of the Cell: Evidence, Explanation, and Experimental Design " +
        "Across a Very Long Course Instance Title",
      assessmentLabel: "A deliberately long assessment label for a dense professional workspace",
      signOutAction: SIGN_OUT,
    },
  ),
  loadingCourse: model(
    "courseInstance",
    "student",
    [control("studentAssessments", { selected: true })],
    [],
    "reading",
    {
      scopeLabel: "Loading course title...",
      signOutAction: SIGN_OUT,
    },
  ),
  errorCourse: model(
    "courseInstance",
    "student",
    [control("studentAssessments", { selected: true })],
    [],
    "reading",
    {
      scopeLabel: "Unable to refresh course title",
      signOutAction: SIGN_OUT,
    },
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type M6RibbonFixtureName = keyof typeof M6_RIBBON_FIXTURES;
