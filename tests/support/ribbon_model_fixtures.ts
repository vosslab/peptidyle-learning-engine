// ribbon_model_fixtures.ts - catalog-valid presentation models for Ribbon component evidence.

import type { UserRole } from "../../generated/api/UserRole";
import type { RouteParamName } from "../../src/navigation/route_params";
import type { RibbonScope } from "../../src/route_contract";
import { buildRoutePath } from "../../src/ribbon/ribbon_route_path";
import type { DeclaredRibbonRouteParams } from "../../src/ribbon/ribbon_contract";
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
  accountId: "U00000009",
} as const satisfies Readonly<Record<RouteParamName, string>>;

const COURSE_SHORT_NAME = "BCHM 355";

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find(
    (candidate): candidate is RibbonCatalogControl<Id> => candidate.id === id,
  );
  if (control === undefined) throw new Error(`Ribbon fixture references unknown catalog ID ${id}.`);
  return control;
}

/**
 * Future entries have no backing route. Used route-backed controls must build
 * their declared destinations when fixtures are created.
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
  userRole: UserRole,
  tabs: RibbonModel["tabs"],
  taskAreas: RibbonModel["taskAreas"],
): RibbonModel {
  return {
    scope,
    context: {
      productLabel:
        userRole === "student" ? "Student" : userRole === "instructor" ? "Instructor" : "Sysadmin",
      signOutAction: SIGN_OUT,
      accountControls: RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) =>
        control.userRoles.includes(userRole),
      ),
    },
    tabs,
    taskAreas,
    breadcrumbs: [],
  };
}

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign Out" } as const;

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
  ),
  attemptInstructor: model(
    "assessmentAttempt",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments")],
    [],
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
  ),
  loadingCourse: model(
    "courseInstance",
    "student",
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [],
  ),
  errorCourse: model(
    "courseInstance",
    "student",
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [],
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type M6RibbonFixtureName = keyof typeof M6_RIBBON_FIXTURES;
