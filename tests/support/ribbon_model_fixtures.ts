// ribbon_model_fixtures.ts - catalog-valid presentation models for Ribbon component evidence.

import type { ProductRole } from "../../generated/api/ProductRole";
import { routeParams, type RouteParamName } from "../../src/navigation/route_params";
import {
  productRoleMayAccessRoute,
  ROUTE_CONTRACT,
  type RibbonScope,
  type RouteContract,
  type RouteId,
} from "../../src/route_contract";
import {
  buildRoutePath,
  deriveRibbonModel,
  type DeclaredRibbonRouteParams,
} from "../../src/ribbon/ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
  RIBBON_CONTEXT_CONTROL_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonDestinationId,
  type RibbonStudentCourseId,
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
  courseInstanceId: "CI7K3M2QAZ",
  assessmentId: "A9D2RX5AF",
  assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
  membershipId: "00000000-0000-0000-0000-00000000000b",
  questionId: "7K3M-79QP",
  draftQuestionId: "0198e000-0000-7000-8000-000000000001",
  blueprintCourseId: "BP7K3M2QAF",
  proposalId: "e3396265-6653-4c65-bc9b-8d869c142d87",
} as const satisfies Readonly<Record<RouteParamName, string>>;

const RIBBON_ROUTE_PRODUCT_ROLES = ["instructor", "student", "sysadmin"] as const;
const COURSE_SHORT_NAME = "BCHM 355";

export interface RibbonRouteMaterialization {
  readonly productRole: ProductRole;
  readonly route: RouteContract;
  readonly pathname: string;
  readonly model: RibbonModel;
}

function paramsForRoute(route: RouteContract): DeclaredRibbonRouteParams {
  const params: Partial<Record<RouteParamName, string>> = {};
  for (const segment of route.path.split("/")) {
    if (!segment.startsWith(":")) continue;
    const name = segment.slice(1) as RouteParamName;
    params[name] = CANONICAL_FIXTURE_PARAMS[name];
  }
  return params;
}

/**
 * Builds a Ribbon model only from one declared, signed-in route and role.
 * This is the shared browser-harness source for complete route/role evidence.
 */
export function materializeRibbonRoute(
  productRole: ProductRole,
  routeId: RouteId,
): RibbonRouteMaterialization {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
  if (route === undefined) throw new Error(`Ribbon fixture has no declared route ${routeId}.`);
  if (!productRoleMayAccessRoute(route.id, productRole)) {
    throw new Error(`Ribbon fixture cannot give ${productRole} access to ${route.id}.`);
  }
  if (route.id === "signIn") {
    throw new Error("Ribbon fixture does not materialize the signed-out sign-in route.");
  }
  const pathname = buildRoutePath(route.id, paramsForRoute(route));
  if (pathname === undefined) throw new Error(`Ribbon fixture cannot build ${route.id}.`);
  const params = routeParams(route, pathname);
  if (params === undefined) throw new Error(`Ribbon fixture cannot parse ${route.id}.`);
  const ribbonParams =
    route.ribbon.scope === "assessmentAttempt"
      ? { ...params, courseInstanceId: CANONICAL_FIXTURE_PARAMS.courseInstanceId }
      : params;
  const model = deriveRibbonModel(
    {
      route,
      params: ribbonParams,
      ...(productRole === "student"
        ? {
            studentCourses: [
              {
                id: CANONICAL_FIXTURE_PARAMS.courseInstanceId,
                shortName: COURSE_SHORT_NAME,
              },
            ],
            ...(route.id === "assessmentAttempt"
              ? { activeAttemptId: CANONICAL_FIXTURE_PARAMS.assessmentAttemptId }
              : {}),
            ...(route.id === "assessmentAttemptSummary"
              ? { latestFeedbackAttemptId: CANONICAL_FIXTURE_PARAMS.assessmentAttemptId }
              : {}),
          }
        : {}),
    },
    { productRole },
    {
      assessmentTitle: "Problem Set 7",
    },
  );
  return Object.freeze({ productRole, route, pathname, model });
}

/** Every signed-in role/route pair that the declared browser boundary admits. */
export const RIBBON_ROUTE_MATERIALIZATIONS: ReadonlyArray<RibbonRouteMaterialization> =
  Object.freeze(
    RIBBON_ROUTE_PRODUCT_ROLES.flatMap((productRole) =>
      ROUTE_CONTRACT.filter(
        (route) => route.id !== "signIn" && productRoleMayAccessRoute(route.id, productRole),
      ).map((route) => materializeRibbonRoute(productRole, route.id)),
    ),
  );

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find(
    (candidate): candidate is RibbonCatalogControl<Id> => candidate.id === id,
  );
  if (control === undefined) throw new Error(`Ribbon fixture references unknown catalog ID ${id}.`);
  return control;
}

/**
 * Future entries have no backing route. Every route-backed catalog entry must
 * build here, so catalog drift fails at fixture creation.
 */
function isDocumentedUnavailableFixtureControl(
  catalog: RibbonCatalogControl<RibbonDestinationId>,
): boolean {
  return catalog.destination.kind !== "route";
}

function fixtureHrefFor(catalog: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (catalog.destination.kind !== "route") {
    return undefined;
  }

  const mutableParams: Partial<Record<RouteParamName, string>> = {};
  for (const name of catalog.requiredParams ?? []) {
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
  const available = options.available ?? !documentedUnavailable;
  if (options.available === true && documentedUnavailable) {
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
  controls: ReadonlyArray<RibbonControlModel<RibbonTaskId | RibbonStudentCourseId>>,
): RibbonTaskAreaModel {
  return { id, controls };
}

function studentCourseControl(
  courseInstanceId: string,
  label: string,
  selected = false,
): RibbonControlModel<RibbonStudentCourseId> {
  const href = buildRoutePath("studentCourseLanding", { courseInstanceId });
  if (href === undefined) throw new Error("Student Course fixture needs a canonical Course ID.");
  return {
    id: `studentCourse:${courseInstanceId}`,
    label,
    destination: { kind: "route", routeId: "studentCourseLanding" },
    availability: "Available",
    selected,
    href,
    role: "primary",
    priority: "critical",
    presentation: "standard",
    iconBearing: false,
    iconOnlySafe: false,
  };
}

function model(
  scope: RibbonScope,
  productRole: ProductRole,
  tabs: RibbonModel["tabs"],
  taskAreas: RibbonModel["taskAreas"],
  _unusedContentLayout: string,
  _contextLabels: Readonly<Record<string, unknown>>,
): RibbonModel {
  return {
    scope,
    context: {
      productLabel:
        productRole === "student"
          ? "Student"
          : productRole === "instructor"
            ? "Instructor"
            : "Sysadmin",
      signOutAction: SIGN_OUT,
      accountControls: RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) =>
        control.productRoles.includes(productRole),
      ),
    },
    tabs,
    taskAreas,
    breadcrumbs: [],
  };
}

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign out" } as const;

/** All exact scope-by-role schemas, with catalog-valid controls and real declared destinations. */
export const M6_RIBBON_FIXTURES = {
  productStudent: model(
    "product",
    "student",
    [control("coursework"), control("grades"), control("courses", { selected: true })],
    [
      area("studentCourses", [
        studentCourseControl(CANONICAL_FIXTURE_PARAMS.courseInstanceId, COURSE_SHORT_NAME, true),
      ]),
    ],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  productInstructor: model(
    "product",
    "instructor",
    [control("courses"), control("questions", { selected: true }), control("productAssessments")],
    [
      area("instructorQuestions", [
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
    [
      control("courses", { selected: true }),
      control("questions"),
      control("instructorAccounts"),
      control("disciplines"),
    ],
    [],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  courseStudent: model(
    "courseInstance",
    "student",
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [
      area("studentCoursework", [
        control("allCoursework", { selected: true }),
        control("dueSoon"),
        control("completedCoursework"),
        control("activeAttempt", { available: false }),
      ]),
    ],
    "reading",
    { scopeLabel: COURSE_SHORT_NAME, signOutAction: SIGN_OUT },
  ),
  courseInstructor: model(
    "courseInstance",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments", { selected: true })],
    [
      area("assessment", [
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
    [
      control("courses", { selected: true }),
      control("questions"),
      control("instructorAccounts"),
      control("disciplines"),
    ],
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
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [
      area("studentCoursework", [
        control("allCoursework"),
        control("dueSoon"),
        control("completedCoursework"),
        control("activeAttempt", { available: false }),
      ]),
    ],
    "reading",
    {
      assessmentLabel: "Problem Set 7",
      assessmentAttemptProgress: "Question 999 of 999",
      signOutAction: SIGN_OUT,
    },
  ),
  attemptInstructor: model(
    "assessmentAttempt",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments")],
    [],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  attemptSysadmin: model(
    "assessmentAttempt",
    "sysadmin",
    [
      control("courses"),
      control("questions"),
      control("instructorAccounts"),
      control("disciplines"),
    ],
    [],
    "reading",
    { signOutAction: SIGN_OUT },
  ),
  longCourse: model(
    "courseInstance",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments", { selected: true })],
    [
      area("assessment", [
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
    [control("courses"), control("coursework", { selected: true }), control("grades")],
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
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [],
    "reading",
    {
      scopeLabel: "Unable to refresh course title",
      signOutAction: SIGN_OUT,
    },
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type M6RibbonFixtureName = keyof typeof M6_RIBBON_FIXTURES;
