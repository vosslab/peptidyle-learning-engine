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
  TAB_CATALOG,
  type RibbonCatalogControl,
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
  readonly accountLabel: string;
  readonly courseTitle?: string;
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
  readonly accountLabel: string;
  readonly scopeLabel?: string;
  readonly assignmentLabel?: string;
  readonly assignmentAttemptProgress?: string;
  readonly signOutAction: RibbonActionDescriptor;
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

/** Complete synchronous Ribbon input for the three permanent rows. */
export interface RibbonModel {
  readonly scope: RibbonScope;
  readonly contentLayout: ContentLayout;
  readonly context: RibbonContextModel;
  readonly tabs: ReadonlyArray<RibbonControlModel<RibbonTabId>>;
  readonly taskAreas: ReadonlyArray<RibbonTaskAreaModel>;
}

const PRODUCT_LABELS = {
  student: "Student",
  instructor: "Instructor",
  sysadmin: "Sysadmin",
} as const;

const TASK_AREA_LABELS: Readonly<Record<RibbonTaskArea, string>> = Object.freeze({
  questionDestinations: "Question destinations",
  questionRelationships: "Question relationships",
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

type RouteParamParser = (value: string) => string | null;

const ROUTE_PARAM_PARSERS: Readonly<Record<RouteParamName, RouteParamParser>> = {
  courseRef: parseCourseInstanceReference,
  assignmentRef: parseAssignmentReference,
  assignmentAttemptRef: parseAssignmentAttemptReference,
  membershipRef: parseCourseMembershipReference,
  questionRef: parseQuestionRouteReference,
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
  const scopeLabel = route.ribbon.scope === "courseInstance" ? labels.courseTitle : undefined;
  const assignmentLabel =
    route.ribbon.scope === "courseInstance"
      ? labels.assignmentTitle
      : labels.assignmentAttemptTitle;
  const assignmentAttemptProgress =
    route.ribbon.scope === "assignmentAttempt" ? labels.assignmentAttemptProgress : undefined;
  return Object.freeze({
    productLabel: PRODUCT_LABELS[productRole],
    accountLabel: labels.accountLabel,
    ...(scopeLabel === undefined ? {} : { scopeLabel }),
    ...(assignmentLabel === undefined ? {} : { assignmentLabel }),
    ...(assignmentAttemptProgress === undefined ? {} : { assignmentAttemptProgress }),
    signOutAction: SIGN_OUT_ACTION,
  });
}

function taskAreasFor(
  routeState: RibbonRouteState,
  productRole: ProductRole,
): ReadonlyArray<RibbonTaskAreaModel> {
  const group = routeState.route.ribbon.taskGroup;
  if (group === undefined) return Object.freeze([]);

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
  return Object.freeze({
    scope: routeState.route.ribbon.scope,
    contentLayout: routeState.route.ribbon.contentLayout,
    context,
    tabs: Object.freeze(tabs),
    taskAreas,
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
