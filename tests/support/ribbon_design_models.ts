// ribbon_design_models.ts - closed, hand-written Ribbon models for design review.

import type { ProductRole } from "../../generated/api/ProductRole";
import type { ContentLayout, RibbonScope } from "../../src/route_contract";
import {
  buildRoutePath,
  type DeclaredRibbonRouteParams,
  type RibbonControlModel,
  type RibbonModel,
  type RibbonTaskAreaModel,
} from "../../src/ribbon/ribbon_contract";
import {
  RIBBON_TASK_CATALOG,
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
  m9bBoundary: [
    "Fieldstation is the direction, not already production-ready: literal spacing",
    "becomes tokens and accent use narrows in the final treatment.",
  ].join(" "),
} as const satisfies {
  readonly selectedTreatment: RibbonDesignTreatment;
  readonly retainedAlternative: RibbonDesignTreatment;
  readonly rationale: string;
  readonly productionNonNegotiables: readonly string[];
  readonly m9bBoundary: string;
};

export const RIBBON_DESIGN_AVAILABILITIES = ["Available", "Unavailable", "Checking"] as const;
export type RibbonDesignAvailability = (typeof RIBBON_DESIGN_AVAILABILITIES)[number];

const ALL_CATALOG_CONTROLS: ReadonlyArray<RibbonCatalogControl<RibbonDestinationId>> = [
  ...TAB_CATALOG,
  ...RIBBON_TASK_CATALOG,
];

const CANONICAL_PARAMS = {
  courseRef: "C-1",
  assignmentRef: "A-1",
  assignmentAttemptRef: "R-1",
  membershipRef: "M-1",
  questionRef: "7K3-M9QP",
  blueprintCourseRef: "BP-1",
} as const;

const SIGN_OUT = { kind: "action", id: "signOut", label: "Sign out" } as const;
const SHORT_COURSE_TITLE = "Biochemistry I";
export const VERY_LONG_COURSE_TITLE = [
  "Molecular Biology of the Cell: Evidence, Explanation, and Experimental Design",
  "Across a Very Long Course Instance Title",
].join(" ");
export const WIDEST_ATTEMPT_PROGRESS = "Question 999 of 999";

function catalogControl<Id extends RibbonDestinationId>(id: Id): RibbonCatalogControl<Id> {
  const control = ALL_CATALOG_CONTROLS.find((candidate) => candidate.id === id);
  if (control === undefined) throw new Error(`Unknown Ribbon catalog control: ${id}.`);
  return control as RibbonCatalogControl<Id>;
}

function routeHref(catalog: RibbonCatalogControl<RibbonDestinationId>): string | undefined {
  if (catalog.destination.kind !== "route" || catalog.id === "backToAssignments") return undefined;
  const pairs = catalog.requiredParams.map((name) => {
    const value = CANONICAL_PARAMS[name];
    if (value === undefined)
      throw new Error(`No design fixture parameter for ${catalog.id}:${name}.`);
    return [name, value] as const;
  });
  const params: DeclaredRibbonRouteParams = Object.fromEntries(pairs);
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
  contentLayout: ContentLayout,
  context: Omit<RibbonModel["context"], "productLabel">,
): RibbonModel {
  return {
    scope,
    contentLayout,
    tabs,
    taskAreas,
    context: { productLabel: productLabel(role), ...context },
  };
}

/** The exact nine scope-by-role schemas, expressed as reviewable fixed models. */
export const RIBBON_DESIGN_SCHEMAS = {
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
    { accountLabel: "Student account", scopeLabel: SHORT_COURSE_TITLE, signOutAction: SIGN_OUT },
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
      scopeLabel: SHORT_COURSE_TITLE,
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
    {
      accountLabel: "System administrator",
      scopeLabel: SHORT_COURSE_TITLE,
      signOutAction: SIGN_OUT,
    },
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
      assignmentAttemptProgress: WIDEST_ATTEMPT_PROGRESS,
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
} as const satisfies Readonly<Record<string, RibbonModel>>;

function cloneWithCourseTitle(title: string): RibbonModel {
  const base = RIBBON_DESIGN_SCHEMAS.courseInstructor;
  return model(base.scope, "instructor", base.tabs, base.taskAreas, base.contentLayout, {
    ...base.context,
    scopeLabel: title,
  });
}

/** Explicit state specimens in addition to the nine stable schemas. */
export const RIBBON_DESIGN_STATE_SPECIMENS = {
  selectedAndUnselected: RIBBON_DESIGN_SCHEMAS.courseInstructor,
  emptyTaskRow: RIBBON_DESIGN_SCHEMAS.courseStudent,
  populatedTaskRow: RIBBON_DESIGN_SCHEMAS.productInstructor,
  veryLongCourseTitle: cloneWithCourseTitle(VERY_LONG_COURSE_TITLE),
  widestAttemptProgress: RIBBON_DESIGN_SCHEMAS.attemptStudent,
  unavailableAdmission: model(
    "courseInstance",
    "instructor",
    [
      control("assignments", { selected: true }),
      control("gradebook", { availability: "Unavailable" }),
    ],
    [],
    "fullWidth",
    { accountLabel: "Instructor account", scopeLabel: SHORT_COURSE_TITLE, signOutAction: SIGN_OUT },
  ),
  checkingAdmission: model(
    "courseInstance",
    "instructor",
    [
      control("assignments", { selected: true }),
      control("gradebook", { availability: "Checking" }),
    ],
    [],
    "fullWidth",
    { accountLabel: "Instructor account", scopeLabel: SHORT_COURSE_TITLE, signOutAction: SIGN_OUT },
  ),
} as const satisfies Readonly<Record<string, RibbonModel>>;

export type RibbonDesignSchemaName = keyof typeof RIBBON_DESIGN_SCHEMAS;
export type RibbonDesignSpecimenName = keyof typeof RIBBON_DESIGN_STATE_SPECIMENS;
