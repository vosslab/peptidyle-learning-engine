// ribbon_design_models.ts - closed, hand-written Ribbon models for design review.

import type { ProductRole } from "../../generated/api/ProductRole";
import type { RouteParamName } from "../../src/navigation/route_params";
import type { RibbonScope } from "../../src/route_contract";
import {
  buildRoutePath,
  type RibbonControlModel,
  type RibbonModel,
  type RibbonTaskAreaModel,
} from "../../src/ribbon/ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
  RIBBON_CONTEXT_CONTROL_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonDestinationId,
  type RibbonTaskArea,
  type RibbonTaskId,
} from "../../src/ribbon/ribbon_catalog";

export const RIBBON_DESIGN_TREATMENTS = ["fieldstation", "atlas"] as const;
export type RibbonDesignTreatment = (typeof RIBBON_DESIGN_TREATMENTS)[number];

/**
 * The design-review decision. This belongs beside the closed fixture inventory so the
 * static laboratory can make the chosen production direction reviewable without
 * pretending that either treatment has already been shipped.
 */
export const RIBBON_DESIGN_DECISION = {
  selectedTreatment: "fieldstation",
  retainedAlternative: "atlas",
  rationale: [
    "Fieldstation's continuous surface and restrained course signal unify the rows:",
    "Context is quiet, Tabs carry the strongest rhythm, and Tasks form a lighter",
    "grouped work band. Selection uses weight, underline, and wash without geometry",
    "change. Atlas remains credible, but its corner slash is visually ambiguous and",
    "its cell divisions weaken the single-surface composition.",
  ].join(" "),
  productionNonNegotiables: [
    "One surface with one outer bottom edge.",
    "Preserve the where, which, then work hierarchy.",
    "No state-driven geometry, with at least two non-color-compatible selection channels.",
    [
      "Labels remain primary and desktop Instructor controls remain direct; later",
      "icons support labels.",
    ].join(" "),
    "All-theme signal migrates to exactly three semantic accent placements.",
    "Forced-color, focus, reduced-motion, and overflow reachability remain preserved.",
  ],
  productionReadinessBoundary: [
    "Fieldstation is the direction, not already production-ready: literal spacing",
    "becomes tokens and accent use narrows in the final treatment.",
  ].join(" "),
} as const satisfies {
  readonly selectedTreatment: RibbonDesignTreatment;
  readonly retainedAlternative: RibbonDesignTreatment;
  readonly rationale: string;
  readonly productionNonNegotiables: readonly string[];
  readonly productionReadinessBoundary: string;
};

export const RIBBON_DESIGN_AVAILABILITIES = ["Available", "Unavailable", "Checking"] as const;
export type RibbonDesignAvailability = (typeof RIBBON_DESIGN_AVAILABILITIES)[number];

const ALL_CATALOG_CONTROLS: ReadonlyArray<RibbonCatalogControl<RibbonDestinationId>> = [
  ...TAB_CATALOG,
  ...RIBBON_TASK_CATALOG,
];

const CANONICAL_PARAMS = {
  courseInstanceId: "CI7K3M2QAZ",
  assessmentId: "A9D2RX5AF",
  assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
  membershipId: "00000000-0000-0000-0000-00000000000b",
  questionId: "7K3M-79QP",
  draftQuestionId: "0198e000-0000-7000-8000-000000000001",
  blueprintCourseId: "BP7K3M2QAF",
  proposalId: "e3396265-6653-4c65-bc9b-8d869c142d87",
} as const satisfies Readonly<Record<RouteParamName, string>>;

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign out" } as const;
const SHORT_COURSE_NAME = "BCHM 355";
export const VERY_LONG_COURSE_TITLE = [
  "Molecular Biology of the Cell: Evidence, Explanation, and Experimental Design",
  "Across a Very Long Course Instance Title",
].join(" ");
export const WIDEST_ATTEMPT_PROGRESS = "Question 999 of 999";

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find(
    (candidate): candidate is RibbonCatalogControl<Id> => candidate.id === id,
  );
  if (control === undefined) throw new Error(`Unknown Ribbon catalog control: ${id}.`);
  return control;
}

function routeHref(catalog: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (catalog.destination.kind !== "route") return undefined;
  const params: Partial<Record<RouteParamName, string>> =
    catalog.id === "coursework" || catalog.id === "grades"
      ? { courseInstanceId: CANONICAL_PARAMS.courseInstanceId }
      : {};
  for (const name of catalog.requiredParams ?? []) {
    const value = CANONICAL_PARAMS[name];
    if (value === undefined)
      throw new Error(`No design fixture parameter for ${catalog.id}:${name}.`);
    params[name] = value;
  }
  const href = buildRoutePath(catalog.destination.routeId, params);
  if (href === undefined) throw new Error(`Cannot make design fixture route for ${catalog.id}.`);
  return href;
}

function control<Id extends RibbonDestinationId>(
  id: Id,
  options: { readonly selected?: boolean; readonly availability?: RibbonDesignAvailability } = {},
): RibbonControlModel<Id> {
  const catalog = catalogControl(id);
  const href = routeHref(catalog);
  const defaultAvailability: RibbonDesignAvailability =
    href === undefined ? "Unavailable" : "Available";
  const availability = options.availability ?? defaultAvailability;
  return {
    id: catalog.id,
    label: catalog.label,
    destination: catalog.destination,
    availability,
    selected: options.selected ?? false,
    ...(availability === "Available" && href !== undefined ? { href } : {}),
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

function productLabel(role: ProductRole): RibbonModel["context"]["productLabel"] {
  return role === "student" ? "Student" : role === "instructor" ? "Instructor" : "Sysadmin";
}

function model(
  scope: RibbonScope,
  role: ProductRole,
  tabs: RibbonModel["tabs"],
  taskAreas: RibbonModel["taskAreas"],
  _unusedContentLayout: string,
  _contextLabels: Readonly<Record<string, unknown>>,
): RibbonModel {
  return {
    scope,
    tabs,
    taskAreas,
    context: {
      productLabel: productLabel(role),
      signOutAction: SIGN_OUT,
      accountControls: RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) =>
        control.productRoles.includes(role),
      ),
    },
    breadcrumbs: [],
  };
}

/** The exact nine scope-by-role schemas, expressed as reviewable fixed models. */
export const RIBBON_DESIGN_SCHEMAS = {
  productStudent: model(
    "product",
    "student",
    [control("courses", { selected: true }), control("coursework"), control("grades")],
    [
      area("studentCourses", "Courses", [
        control("studentProgress", { availability: "Unavailable" }),
        control("studentPracticeStats", { availability: "Unavailable" }),
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
      area("studentCoursework", "Coursework", [
        control("allCoursework", { selected: true }),
        control("dueSoon"),
        control("completedCoursework"),
      ]),
    ],
    "reading",
    { scopeLabel: SHORT_COURSE_NAME, signOutAction: SIGN_OUT },
  ),
  courseInstructor: model(
    "courseInstance",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments", { selected: true })],
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
      scopeLabel: SHORT_COURSE_NAME,
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
      scopeLabel: SHORT_COURSE_NAME,
      signOutAction: SIGN_OUT,
    },
  ),
  attemptStudent: model(
    "assessmentAttempt",
    "student",
    [control("courses"), control("coursework", { selected: true }), control("grades")],
    [
      area("studentCoursework", "Coursework", [
        control("allCoursework"),
        control("dueSoon"),
        control("completedCoursework"),
      ]),
    ],
    "reading",
    {
      assessmentLabel: "Problem Set 7",
      assessmentAttemptProgress: WIDEST_ATTEMPT_PROGRESS,
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
} as const satisfies Readonly<Record<string, RibbonModel>>;

function cloneWithCourseTitle(title: string): RibbonModel {
  const base = RIBBON_DESIGN_SCHEMAS.courseInstructor;
  return model(base.scope, "instructor", base.tabs, base.taskAreas, "reading", {
    ...base.context,
    scopeLabel: title,
  });
}

/** Explicit state specimens in addition to the nine stable schemas. */
export const RIBBON_DESIGN_STATE_SPECIMENS = {
  selectedAndUnselected: RIBBON_DESIGN_SCHEMAS.courseInstructor,
  courseAppearance: model(
    "courseInstance",
    "instructor",
    [control("courses"), control("questions"), control("productAssessments", { selected: true })],
    [
      area("courseSetup", "Course setup", [
        control("gradeSettings"),
        control("appearance", { selected: true }),
      ]),
    ],
    "reading",
    {
      scopeLabel: SHORT_COURSE_NAME,
      signOutAction: SIGN_OUT,
    },
  ),
  emptyTaskRow: RIBBON_DESIGN_SCHEMAS.courseStudent,
  populatedTaskRow: RIBBON_DESIGN_SCHEMAS.productInstructor,
  veryLongCourseTitle: cloneWithCourseTitle(VERY_LONG_COURSE_TITLE),
  widestAttemptProgress: RIBBON_DESIGN_SCHEMAS.attemptStudent,
  unavailableAdmission: model(
    "courseInstance",
    "instructor",
    [control("courses", { selected: true }), control("questions", { availability: "Unavailable" })],
    [],
    "fullWidth",
    { scopeLabel: SHORT_COURSE_NAME, signOutAction: SIGN_OUT },
  ),
  checkingAdmission: model(
    "courseInstance",
    "instructor",
    [control("courses", { selected: true }), control("questions", { availability: "Checking" })],
    [],
    "fullWidth",
    { scopeLabel: SHORT_COURSE_NAME, signOutAction: SIGN_OUT },
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type RibbonDesignSchemaName = keyof typeof RIBBON_DESIGN_SCHEMAS;
export type RibbonDesignSpecimenName = keyof typeof RIBBON_DESIGN_STATE_SPECIMENS;
