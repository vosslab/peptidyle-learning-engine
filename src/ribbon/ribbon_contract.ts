// ribbon_contract.ts - pure, synchronous Ribbon model derivation.

import type { ProductRole } from "../../generated/api/ProductRole";
import {
  routeScopeKey,
  routeParams,
  type RouteParamName,
  type RouteParams,
} from "../navigation/route_params";
import {
  parseAssignmentAttemptReference,
  parseAssignmentReference,
  parseBlueprintCourseReference,
  parseCourseInstanceReference,
  parseCourseMembershipReference,
  parseDraftQuestionReference,
  parseQuestionRouteReference,
} from "../navigation/public_route";
import {
  ROUTE_CONTRACT,
  productRoleMayAccessRoute,
  routeContractForPathname,
  type ContentLayout,
  type RibbonScope,
  type RibbonTabId,
  type RouteContract,
  type RouteId,
} from "../route_contract";
import {
  CAPABILITY_REGISTRY,
  ribbonAvailability,
  type RibbonAvailability,
  type RibbonRelationshipState,
} from "./capability_registry";
import {
  RIBBON_TASK_CATALOG,
  RIBBON_CONTEXT_CONTROL_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonContextControlAvailability,
  type RibbonContextControlGlyphKey,
  type RibbonContextControlId,
  type RibbonDestination,
  type RibbonDestinationId,
  type RibbonTaskArea,
  type RibbonTaskId,
} from "./ribbon_catalog";
import { ribbonSchemaFor, type RibbonRelationshipRequirement } from "./ribbon_schema";

/** Exactly the synchronous facts that identify the route being rendered. */
export interface RibbonRouteState {
  readonly route: RouteContract;
  readonly params: Exclude<RouteParams, undefined>;
}

/** The immutable session fact the Ribbon may use for presentation admission. */
export interface RibbonViewerIdentity {
  readonly productRole: ProductRole;
}

/**
 * Already-resolved display text only. These values are never identifiers,
 * resources, accessors, callbacks, promises, or projections.
 */
export interface RibbonContextLabels {
  readonly courseShortName?: string;
  readonly courseLongName?: string;
  readonly assignmentTitle?: string;
  readonly assignmentAttemptTitle?: string;
  readonly assignmentAttemptProgress?: string;
}

/** The declared public route parameters are strings only; no resource is admitted here. */
export type DeclaredRibbonRouteParams = Readonly<Partial<Record<RouteParamName, string>>>;

/**
 * Rejects values carrying data outside a public Ribbon boundary, including when
 * the value first passed through a variable. TypeScript's normal excess
 * property check handles literals only, which is insufficient for scope data.
 */
type Exact<Shape, Value extends Shape> = Value & Record<Exclude<keyof Value, keyof Shape>, never>;

type ExactRouteState<Value extends RibbonRouteState> = Exact<RibbonRouteState, Value> & {
  readonly params: Exact<DeclaredRibbonRouteParams, Value["params"]>;
};

/** A static intent for the shell to dispatch; the pure model owns no callback. */
export interface RibbonActionDescriptor {
  readonly kind: "action";
  readonly id: "signOut";
  readonly label: "Sign out";
}

export interface RibbonContextModel {
  readonly productLabel: "Student" | "Instructor" | "Sysadmin";
  readonly scopeLabel?: string;
  readonly assignmentLabel?: string;
  readonly assignmentAttemptProgress?: string;
  /** Account-endcap positions remain modeled while truthful admission withholds them. */
  readonly accountControls: ReadonlyArray<RibbonContextControlModel>;
  readonly signOutAction: RibbonActionDescriptor;
}

/** A Context Control is separate from Tabs and Tasks, which are destinations. */
export interface RibbonContextControlModel {
  readonly id: RibbonContextControlId;
  readonly label: string;
  readonly availability: RibbonContextControlAvailability;
  readonly glyph: RibbonContextControlGlyphKey;
  readonly href?: string;
}

/** One designed Ribbon position, retained even while admission withholds it. */
export interface RibbonControlModel<Id extends RibbonDestinationId = RibbonDestinationId> {
  readonly id: Id;
  readonly label: string;
  readonly destination: RibbonDestination;
  readonly availability: RibbonAvailability;
  readonly selected: boolean;
  readonly href?: string;
  readonly role: RibbonCatalogControl<Id>["role"];
  readonly priority: RibbonCatalogControl<Id>["priority"];
  readonly presentation: RibbonCatalogControl<Id>["presentation"];
  readonly iconBearing: boolean;
  readonly iconOnlySafe: boolean;
}

export interface RibbonTaskAreaModel {
  readonly id: RibbonTaskArea;
  readonly label: string;
  readonly controls: ReadonlyArray<RibbonControlModel<RibbonTaskId>>;
}

/** A shell-owned location in the route-derived breadcrumb trail. */
export interface RibbonBreadcrumbModel {
  readonly label: string;
  readonly href?: string;
  readonly current: boolean;
}

/** Complete synchronous input for the two-row Ribbon and shell-owned breadcrumb prelude. */
export interface RibbonModel {
  readonly scope: RibbonScope;
  readonly contentLayout: ContentLayout;
  readonly context: RibbonContextModel;
  readonly tabs: ReadonlyArray<RibbonControlModel<RibbonTabId>>;
  readonly taskAreas: ReadonlyArray<RibbonTaskAreaModel>;
  /**
   * A route-derived content prelude. An empty trail reserves no landmark, but
   * lets the shell retain its declared prelude geometry while labels resolve.
   */
  readonly breadcrumbs: ReadonlyArray<RibbonBreadcrumbModel>;
  /** Route topology reserves the content-prelude footprint while labels resolve. */
  readonly breadcrumbPreludeReserved: boolean;
}

const PRODUCT_LABELS = {
  student: "Student",
  instructor: "Instructor",
  sysadmin: "Sysadmin",
} as const;

const TASK_AREA_LABELS: Readonly<Record<RibbonTaskArea, string>> = Object.freeze({
  instructorCourses: "Courses",
  instructorQuestions: "Questions",
  instructorAssignments: "Assignments",
  assignment: "Assignment",
  courseSetup: "Course setup",
  assignmentAttempt: "Assignment attempt",
});

const RESOLVED_RELATIONSHIP: RibbonRelationshipState = Object.freeze({
  kind: "resolved",
  allowed: true,
});

const OUTSTANDING_RELATIONSHIP: RibbonRelationshipState = Object.freeze({ kind: "outstanding" });

const SIGN_OUT_ACTION: RibbonActionDescriptor = Object.freeze({
  kind: "action",
  id: "signOut",
  label: "Sign out",
});

function accountControlsFor(productRole: ProductRole): ReadonlyArray<RibbonContextControlModel> {
  return Object.freeze(
    RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) => control.productRole === productRole).map(
      (control) =>
        Object.freeze({
          id: control.id,
          label: control.label,
          availability: control.availability,
          glyph: control.glyph,
          ...(control.id === "profile" && control.availability === "Available"
            ? { href: "/profile" }
            : {}),
        }),
    ),
  );
}

type RouteParamParser = (value: string) => string | null;

const ROUTE_PARAM_PARSERS: Readonly<Record<RouteParamName, RouteParamParser>> = {
  courseRef: parseCourseInstanceReference,
  assignmentRef: parseAssignmentReference,
  assignmentAttemptRef: parseAssignmentAttemptReference,
  membershipRef: parseCourseMembershipReference,
  questionRef: parseQuestionRouteReference,
  draftQuestionRef: parseDraftQuestionReference,
  blueprintCourseRef: parseBlueprintCourseReference,
};

function routeForId(routeId: string): RouteContract | undefined {
  return ROUTE_CONTRACT.find((route) => route.id === routeId);
}

function declaredParamNames(route: RouteContract): ReadonlyArray<RouteParamName> {
  const names: RouteParamName[] = [];
  for (const segment of route.path.split("/")) {
    if (!segment.startsWith(":")) continue;
    const name = segment.slice(1);
    if (!(name in ROUTE_PARAM_PARSERS)) return [];
    names.push(name as RouteParamName);
  }
  return names;
}

function isExactParameterRecord(
  params: Readonly<Record<string, unknown>>,
  names: ReadonlyArray<RouteParamName>,
): boolean {
  if (Object.getOwnPropertySymbols(params).length !== 0) return false;

  const keys = Object.getOwnPropertyNames(params);
  if (keys.length !== names.length) return false;
  for (const name of names) {
    const descriptor = Object.getOwnPropertyDescriptor(params, name);
    if (
      descriptor === undefined ||
      !("value" in descriptor) ||
      typeof descriptor.value !== "string"
    ) {
      return false;
    }
  }
  return keys.every((key) => names.includes(key as RouteParamName));
}

function canonicalParameterValues(
  params: Readonly<Record<string, unknown>>,
  names: ReadonlyArray<RouteParamName>,
): Readonly<Record<RouteParamName, string>> | undefined {
  const values: Partial<Record<RouteParamName, string>> = {};
  for (const name of names) {
    const value = params[name];
    if (typeof value !== "string") return undefined;
    const canonicalValue = ROUTE_PARAM_PARSERS[name](value);
    if (canonicalValue === null) return undefined;
    values[name] = canonicalValue;
  }
  return values as Readonly<Record<RouteParamName, string>>;
}

function routePathWithValues(
  route: RouteContract,
  values: Readonly<Record<RouteParamName, string>>,
): string | undefined {
  const segments = route.path.split("/");
  const pathSegments: string[] = [];
  for (const segment of segments) {
    if (!segment.startsWith(":")) {
      pathSegments.push(segment);
      continue;
    }
    const name = segment.slice(1) as RouteParamName;
    const value = values[name];
    if (value === undefined) return undefined;
    pathSegments.push(encodeURIComponent(value));
  }
  return pathSegments.join("/");
}

function isRoundTripFor(route: RouteContract, pathname: string): boolean {
  const matchedRoute = routeContractForPathname(pathname);
  if (matchedRoute?.id !== route.id) return false;
  const extracted = routeParams(route, pathname);
  if (extracted === undefined) return false;
  return routeScopeKey(pathname).kind !== "invalid";
}

/**
 * Builds only a canonical declared PLE route. Unknown route IDs, surplus data,
 * URL syntax, malformed public references, and incomplete substitutions fail closed.
 */
export function buildRoutePath<Route extends RouteId, Params extends DeclaredRibbonRouteParams>(
  routeId: Route,
  params: Exact<DeclaredRibbonRouteParams, Params>,
): string | undefined;
export function buildRoutePath(routeId: unknown, params: unknown): string | undefined {
  if (typeof routeId !== "string") return undefined;
  const route = routeForId(routeId);
  if (route === undefined || params === null || typeof params !== "object") return undefined;

  const names = declaredParamNames(route);
  const parameterRecord = params as Readonly<Record<string, unknown>>;
  if (!isExactParameterRecord(parameterRecord, names)) return undefined;
  const values = canonicalParameterValues(parameterRecord, names);
  if (values === undefined) return undefined;
  const pathname = routePathWithValues(route, values);
  if (
    pathname === undefined ||
    pathname.includes(":") ||
    pathname.includes("?") ||
    pathname.includes("#")
  ) {
    return undefined;
  }
  return isRoundTripFor(route, pathname) ? pathname : undefined;
}

function relationshipStateFor(requirement: RibbonRelationshipRequirement): RibbonRelationshipState {
  return requirement === "none" ? RESOLVED_RELATIONSHIP : OUTSTANDING_RELATIONSHIP;
}

function targetParamsFor(
  control: RibbonCatalogControl<RibbonDestinationId>,
  routeState: RibbonRouteState,
): DeclaredRibbonRouteParams | undefined {
  const values: Partial<Record<RouteParamName, string>> = {};
  for (const name of control.requiredParams) {
    const value = routeState.params[name];
    if (value === undefined) return undefined;
    values[name] = value;
  }
  return values;
}

function hrefFor(
  control: RibbonCatalogControl<RibbonDestinationId>,
  routeState: RibbonRouteState,
  availability: RibbonAvailability,
): string | undefined {
  if (availability !== "Available" || control.destination.kind !== "route") return undefined;
  const targetParams = targetParamsFor(control, routeState);
  if (targetParams === undefined) return undefined;
  return buildRoutePath(control.destination.routeId, targetParams);
}

function selectedFor(
  control: RibbonCatalogControl<RibbonDestinationId>,
  route: RouteContract,
): boolean {
  if (TAB_CATALOG.some((tab) => tab.id === control.id)) {
    return route.ribbon.tab === control.id;
  }
  return control.destination.kind === "route" && control.destination.routeId === route.id;
}

function modelForControl<Id extends RibbonDestinationId>(
  control: RibbonCatalogControl<Id>,
  routeState: RibbonRouteState,
  productRole: ProductRole,
): RibbonControlModel<Id> {
  const entry = CAPABILITY_REGISTRY[control.id];
  const admission = ribbonAvailability(
    entry,
    productRole,
    relationshipStateFor(entry.relationshipRequirement),
  );
  const href = hrefFor(control, routeState, admission);
  const availability = href === undefined && admission === "Available" ? "Unavailable" : admission;
  return Object.freeze({
    id: control.id,
    label: control.label,
    destination: Object.freeze(
      control.destination.kind === "route"
        ? { kind: "route" as const, routeId: control.destination.routeId }
        : { kind: "future" as const, futureId: control.destination.futureId },
    ),
    availability,
    selected: selectedFor(control, routeState.route),
    ...(href === undefined ? {} : { href }),
    role: control.role,
    priority: control.priority,
    presentation: control.presentation,
    iconBearing: control.iconBearing,
    iconOnlySafe: control.iconOnlySafe,
  });
}

function contextFor(
  route: RouteContract,
  productRole: ProductRole,
  labels: RibbonContextLabels,
): RibbonContextModel {
  const scopeLabel = route.ribbon.scope === "courseInstance" ? labels.courseShortName : undefined;
  const assignmentLabel =
    route.ribbon.scope === "courseInstance"
      ? labels.assignmentTitle
      : labels.assignmentAttemptTitle;
  const assignmentAttemptProgress =
    route.ribbon.scope === "assignmentAttempt" ? labels.assignmentAttemptProgress : undefined;
  return Object.freeze({
    productLabel: PRODUCT_LABELS[productRole],
    ...(scopeLabel === undefined ? {} : { scopeLabel }),
    ...(assignmentLabel === undefined ? {} : { assignmentLabel }),
    ...(assignmentAttemptProgress === undefined ? {} : { assignmentAttemptProgress }),
    accountControls: accountControlsFor(productRole),
    signOutAction: SIGN_OUT_ACTION,
  });
}

function taskAreasFor(
  routeState: RibbonRouteState,
  productRole: ProductRole,
): ReadonlyArray<RibbonTaskAreaModel> {
  const group = routeState.route.ribbon.taskGroup;
  if (group === undefined) return Object.freeze([]);
  const instructorProductGroup =
    group === "instructorCourses" ||
    group === "instructorQuestions" ||
    group === "instructorAssignments";
  if (instructorProductGroup && productRole !== "instructor") return Object.freeze([]);

  const areas: RibbonTaskAreaModel[] = [];
  for (const control of RIBBON_TASK_CATALOG) {
    if (control.taskGroup !== group) continue;
    const existing = areas[areas.length - 1];
    if (existing === undefined || existing.id !== control.area) {
      areas.push({
        id: control.area,
        label: TASK_AREA_LABELS[control.area],
        controls: [],
      });
    }
    const area = areas[areas.length - 1];
    if (area === undefined) throw new Error("Ribbon task area construction failed.");
    const controlModel = modelForControl(control, routeState, productRole);
    (area.controls as RibbonControlModel<RibbonTaskId>[]).push(controlModel);
  }
  return Object.freeze(
    areas.map((area) => Object.freeze({ ...area, controls: Object.freeze([...area.controls]) })),
  );
}

function canonicalPathForRouteState(routeState: RibbonRouteState): string | undefined {
  const params: Partial<Record<RouteParamName, string>> = {};
  for (const name of declaredParamNames(routeState.route)) {
    const value = routeState.params[name];
    if (value === undefined) return undefined;
    params[name] = value;
  }
  return buildRoutePath(routeState.route.id, params);
}

function breadcrumbLink(
  routeId: RouteId,
  params: DeclaredRibbonRouteParams = {},
): string | undefined {
  return buildRoutePath(routeId, params);
}

function breadcrumbCurrent(label: string): RibbonBreadcrumbModel {
  return Object.freeze({ label, current: true });
}

function breadcrumbLinkItem(label: string, href: string): RibbonBreadcrumbModel {
  return Object.freeze({ label, href, current: false });
}

function breadcrumbsFor(
  routeState: RibbonRouteState,
  labels: RibbonContextLabels,
): ReadonlyArray<RibbonBreadcrumbModel> {
  // A malformed declared route must never surface a reference-shaped label or
  // a guessed ancestor. The shell retains its route-class geometry separately.
  if (canonicalPathForRouteState(routeState) === undefined) return Object.freeze([]);

  const courses = breadcrumbLink("courses");
  const library = breadcrumbLink("library");
  const drafts = breadcrumbLink("questionDrafts");
  const blueprints = breadcrumbLink("blueprintCourses");
  const courseParams = { courseRef: routeState.params.courseRef };
  const courseAssignments = breadcrumbLink("courseAssignments", courseParams);
  const studentCourse = breadcrumbLink("studentCourseLanding", courseParams);
  const assignmentParams = {
    courseRef: routeState.params.courseRef,
    assignmentRef: routeState.params.assignmentRef,
  };
  const instructorAssignment = breadcrumbLink("assignmentWorkspaceOverview", assignmentParams);
  const studentAssignment = breadcrumbLink("assignmentOverview", assignmentParams);
  const assignmentLabel = labels.assignmentTitle ?? "Assignment";

  function courseTrail(current: string, courseHref: string | undefined): RibbonBreadcrumbModel[] {
    if (courses === undefined || courseHref === undefined || labels.courseLongName === undefined)
      return [];
    return [
      breadcrumbLinkItem("Courses", courses),
      breadcrumbLinkItem(labels.courseLongName, courseHref),
      breadcrumbCurrent(current),
    ];
  }

  switch (routeState.route.id) {
    case "courseAssignments":
    case "studentCourseLanding":
      return courses !== undefined && labels.courseLongName !== undefined
        ? Object.freeze([
            breadcrumbLinkItem("Courses", courses),
            breadcrumbCurrent(labels.courseLongName),
          ])
        : Object.freeze([]);
    case "questionDetail":
      return library === undefined
        ? Object.freeze([])
        : Object.freeze([
            breadcrumbLinkItem("Questions", library),
            breadcrumbLinkItem("Question Library", library),
            breadcrumbCurrent("Question"),
          ]);
    case "questionDraftEditor":
      return drafts === undefined
        ? Object.freeze([])
        : Object.freeze([
            breadcrumbLinkItem("Questions", library ?? drafts),
            breadcrumbLinkItem("My Draft Questions", drafts),
            breadcrumbCurrent("Draft Question"),
          ]);
    case "blueprintCourseDetail":
      return blueprints === undefined
        ? Object.freeze([])
        : Object.freeze([
            breadcrumbLinkItem("Courses", courses ?? blueprints),
            breadcrumbLinkItem("My Blueprint Courses", blueprints),
            breadcrumbCurrent("Blueprint Course"),
          ]);
    case "assignmentWorkspaceQuestions":
    case "assignmentWorkspacePolicies":
    case "assignmentWorkspaceStudentView": {
      const section = {
        assignmentWorkspaceQuestions: "Questions",
        assignmentWorkspacePolicies: "Settings",
        assignmentWorkspaceStudentView: "Student View",
      }[routeState.route.id];
      const base = courseTrail(assignmentLabel, courseAssignments);
      if (base.length === 0 || instructorAssignment === undefined) return Object.freeze([]);
      return Object.freeze([
        ...base.slice(0, -1),
        breadcrumbLinkItem(assignmentLabel, instructorAssignment),
        breadcrumbCurrent(section),
      ]);
    }
    case "assignmentWorkspaceOverview":
      return Object.freeze(courseTrail(assignmentLabel, courseAssignments));
    case "assignmentOverview":
      return Object.freeze(courseTrail(assignmentLabel, studentCourse));
    case "assignmentCreate":
      return Object.freeze(courseTrail("New Assignment", courseAssignments));
    case "assignmentPreview":
      return Object.freeze(courseTrail("Delivery Check", courseAssignments));
    case "gradebook":
      return Object.freeze(courseTrail("Gradebook", courseAssignments));
    case "courseGradeSettings":
      return Object.freeze(courseTrail("Grade Settings", courseAssignments));
    case "courseAppearance":
      return Object.freeze(courseTrail("Appearance", courseAssignments));
    case "courseRoster":
      return Object.freeze(courseTrail("Students", courseAssignments));
    case "teachingOperations":
      return Object.freeze(courseTrail("Teaching Operations", courseAssignments));
    case "assignmentAttempt":
      if (
        courses === undefined ||
        studentCourse === undefined ||
        studentAssignment === undefined ||
        labels.courseLongName === undefined ||
        labels.assignmentAttemptTitle === undefined
      ) {
        return Object.freeze([]);
      }
      return Object.freeze([
        breadcrumbLinkItem("Courses", courses),
        breadcrumbLinkItem(labels.courseLongName, studentCourse),
        breadcrumbLinkItem(labels.assignmentAttemptTitle, studentAssignment),
        breadcrumbCurrent("Assignment attempt"),
      ]);
    case "assignmentAttemptSummary":
      return courses !== undefined &&
        studentCourse !== undefined &&
        studentAssignment !== undefined &&
        labels.courseLongName !== undefined &&
        labels.assignmentAttemptTitle !== undefined
        ? Object.freeze([
            breadcrumbLinkItem("Courses", courses),
            breadcrumbLinkItem(labels.courseLongName, studentCourse),
            breadcrumbLinkItem(labels.assignmentAttemptTitle, studentAssignment),
            breadcrumbCurrent("Assignment attempt"),
          ])
        : Object.freeze([]);
    default:
      return Object.freeze([]);
  }
}

function breadcrumbPreludeReservedFor(route: RouteContract): boolean {
  switch (route.id) {
    case "courseAssignments":
    case "studentCourseLanding":
    case "questionDetail":
    case "questionDraftEditor":
    case "blueprintCourseDetail":
    case "assignmentCreate":
    case "assignmentWorkspaceOverview":
    case "assignmentWorkspaceQuestions":
    case "assignmentWorkspacePolicies":
    case "assignmentWorkspaceStudentView":
    case "assignmentPreview":
    case "gradebook":
    case "courseGradeSettings":
    case "courseAppearance":
    case "courseRoster":
    case "teachingOperations":
    case "assignmentOverview":
    case "assignmentAttempt":
    case "assignmentAttemptSummary":
      return true;
    default:
      return false;
  }
}

/**
 * Synchronously derives fixed Ribbon topology from declared route and catalog data.
 * This controls UI admission only; route and server authorization remain independent.
 */
export function deriveRibbonModel<
  RouteState extends RibbonRouteState,
  ViewerIdentity extends RibbonViewerIdentity,
  ContextLabels extends RibbonContextLabels,
>(
  routeState: ExactRouteState<RouteState>,
  viewerIdentity: Exact<RibbonViewerIdentity, ViewerIdentity>,
  contextLabels: Exact<RibbonContextLabels, ContextLabels>,
): RibbonModel {
  const schema = ribbonSchemaFor(routeState.route.ribbon.scope, viewerIdentity.productRole);
  const tabs = schema.map((slot) => {
    const control = TAB_CATALOG.find((candidate) => candidate.id === slot.id);
    if (control === undefined) throw new Error(`Ribbon schema references unknown tab ${slot.id}.`);
    return modelForControl(control, routeState, viewerIdentity.productRole);
  });
  const taskAreas = taskAreasFor(routeState, viewerIdentity.productRole);
  const context = contextFor(routeState.route, viewerIdentity.productRole, contextLabels);
  const breadcrumbs = breadcrumbsFor(routeState, contextLabels);
  return Object.freeze({
    scope: routeState.route.ribbon.scope,
    contentLayout: routeState.route.ribbon.contentLayout,
    context,
    tabs: Object.freeze(tabs),
    taskAreas,
    breadcrumbs,
    breadcrumbPreludeReserved: breadcrumbPreludeReservedFor(routeState.route),
  });
}

/** The same boundary predicate retained beside derivation for model-level consumers. */
export function ribbonModelAvailabilityMayAccessRoute(
  control: RibbonControlModel,
  productRole: ProductRole,
): boolean {
  if (control.availability !== "Available" || control.destination.kind !== "route") return true;
  return productRoleMayAccessRoute(control.destination.routeId, productRole);
}
