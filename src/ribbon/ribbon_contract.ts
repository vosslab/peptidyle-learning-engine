// ribbon_contract.ts - pure, synchronous Ribbon model derivation.

import type { CourseInstanceLifecycleState } from "../../generated/api/CourseInstanceLifecycleState";
import type { UserRole } from "../../generated/api/UserRole";
import { type RouteParamName, type RouteParams } from "../navigation/route_params";
import { assessmentAttemptRouteState } from "../navigation/assessment_attempt_route";
import { parseAssessmentAttemptId, parseCourseInstanceId } from "../navigation/public_route";
import {
  userRoleMayAccessRoute,
  userRoleHomeRouteId,
  userRoleHomePath,
  type RibbonScope,
  type RibbonTabId,
  type RouteContract,
} from "../route_contract";
import { presentRibbonBreadcrumbs } from "./ribbon_breadcrumbs";
import { buildRoutePath } from "./ribbon_route_path";

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
  type RibbonStudentCourseId,
  type RibbonTaskArea,
  type RibbonTaskId,
} from "./ribbon_catalog";
import {
  ribbonSchemaFor,
  ribbonTierTwoSchemaFor,
  type RibbonRelationshipRequirement,
} from "./ribbon_schema";

/** Exactly the synchronous facts that identify the route being rendered. */
export interface RibbonRouteState {
  readonly route: RouteContract;
  readonly params: Exclude<RouteParams, undefined>;
  /** Server-selected unsubmitted Attempt with a running clock across enrolled Courses, if any. */
  readonly activeAttemptId?: string;
  readonly activeAttemptCourseInstanceId?: string;
  /** Server-selected latest Attempt review that currently exposes feedback. */
  readonly latestFeedbackAttemptId?: string;
  readonly latestFeedbackCourseInstanceId?: string;
  /** Current enrolled Student Courses, in their stable Ribbon order. */
  readonly studentCourses?: ReadonlyArray<{
    readonly id: string;
    readonly shortName: string;
  }>;
}

/** The immutable session fact the Ribbon may use for presentation admission. */
export interface RibbonViewerIdentity {
  readonly userRole: UserRole;
}

/** The two authorized collection parents for a directly loaded Blueprint Course. */
export type BlueprintBreadcrumbParent = "myBlueprintCourses" | "publicBlueprintSearch";

/**
 * Already-resolved labels and Course lifecycle facts. These values are never identifiers,
 * resources, accessors, callbacks, promises, or projections.
 */
export interface RibbonContextLabels {
  readonly courseShortName?: string;
  readonly courseLongName?: string;
  readonly courseLifecycleState?: CourseInstanceLifecycleState;
  readonly assessmentTitle?: string;
  readonly assessmentAttemptTitle?: string;
  readonly questionTitle?: string;
  /** Resolved title for either kind admitted by the shared Library detail route. */
  readonly libraryObjectTitle?: string;
  readonly blueprintCourseTitle?: string;
  readonly blueprintBreadcrumbParent?: BlueprintBreadcrumbParent;
}

/** Validated link state from the current route, kept separate from display labels. */
export type RibbonContextNavigation = Readonly<Record<string, never>>;

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
  readonly label: "Sign Out";
}

export interface RibbonContextModel {
  readonly productLabel: "Student" | "Instructor" | "Sysadmin";
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
export interface RibbonControlModel<
  Id extends RibbonDestinationId | RibbonStudentCourseId =
    RibbonDestinationId | RibbonStudentCourseId,
> {
  readonly id: Id;
  readonly label: string;
  readonly destination: RibbonDestination;
  readonly availability: RibbonAvailability;
  readonly selected: boolean;
  readonly href?: string;
  readonly state?: Readonly<{ assessmentAttemptId: string }>;
  readonly role: RibbonCatalogControl<Id>["role"];
  readonly priority: RibbonCatalogControl<Id>["priority"];
  readonly presentation: RibbonCatalogControl<Id>["presentation"];
  readonly iconBearing: boolean;
  readonly iconOnlySafe: boolean;
}

export interface RibbonTaskAreaModel {
  readonly id: RibbonTaskArea;
  readonly controls: ReadonlyArray<RibbonControlModel<RibbonTaskId | RibbonStudentCourseId>>;
}

/** A shell-owned location in the route-derived breadcrumb trail. */
export interface RibbonBreadcrumbModel {
  readonly label: string;
  readonly compactLabel?: string;
  readonly href: string;
  readonly current: boolean;
  /** Attempt destinations use the same history state as their Ribbon control. */
  readonly state?: Readonly<{ assessmentAttemptId: string }>;
}

/** Complete synchronous input for the two-row Ribbon and shell-owned breadcrumb prelude. */
export interface RibbonModel {
  readonly scope: RibbonScope;
  readonly context: RibbonContextModel;
  readonly tabs: ReadonlyArray<RibbonControlModel<RibbonTabId>>;
  readonly taskAreas: ReadonlyArray<RibbonTaskAreaModel>;
  /** Role-home-rooted locations; unresolved scopes retain the home link. */
  readonly breadcrumbs: ReadonlyArray<RibbonBreadcrumbModel>;
}

const PRODUCT_LABELS = {
  student: "Student",
  instructor: "Instructor",
  sysadmin: "Sysadmin",
} as const;

const RESOLVED_RELATIONSHIP: RibbonRelationshipState = Object.freeze({
  kind: "resolved",
  allowed: true,
});

const OUTSTANDING_RELATIONSHIP: RibbonRelationshipState = Object.freeze({ kind: "outstanding" });

const SIGN_OUT_ACTION: RibbonActionDescriptor = Object.freeze({
  kind: "action",
  id: "signOut",
  label: "Sign Out",
});

function accountControlsFor(userRole: UserRole): ReadonlyArray<RibbonContextControlModel> {
  return Object.freeze(
    RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) => control.userRoles.includes(userRole)).map(
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

function relationshipStateFor(requirement: RibbonRelationshipRequirement): RibbonRelationshipState {
  return requirement === "none" ? RESOLVED_RELATIONSHIP : OUTSTANDING_RELATIONSHIP;
}

function targetParamsFor(
  control: RibbonCatalogControl<RibbonDestinationId>,
  routeState: RibbonRouteState,
): DeclaredRibbonRouteParams | undefined {
  const values: Partial<Record<RouteParamName, string>> = {};
  for (const name of control.requiredParams ?? []) {
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
  userRole: UserRole,
): string | undefined {
  if (availability !== "Available" || control.destination.kind !== "route") return undefined;
  if (control.id === "activeAttempt") {
    const attemptId = routeState.activeAttemptId;
    if (attemptId === undefined) return undefined;
    if (parseAssessmentAttemptId(attemptId) === null) return undefined;
    const courseInstanceId = routeState.activeAttemptCourseInstanceId;
    return courseInstanceId === undefined
      ? undefined
      : buildRoutePath(control.destination.routeId, { courseInstanceId });
  }
  if (control.id === "studentLatestFeedback") {
    const attemptId = routeState.latestFeedbackAttemptId;
    if (attemptId === undefined || parseAssessmentAttemptId(attemptId) === null) return undefined;
    const courseInstanceId = routeState.latestFeedbackCourseInstanceId;
    return courseInstanceId === undefined
      ? undefined
      : buildRoutePath(control.destination.routeId, { courseInstanceId });
  }
  if (control.id === "coursework" || control.id === "grades") {
    return buildRoutePath(control.destination.routeId, {});
  }
  if (control.id === "courses" && userRole === "student") {
    return buildRoutePath("studentCourses", {});
  }
  // The Courses tab is the role's stable home, not the anonymous root resolver.
  // This preserves direct role navigation even if a caller does not first visit `/`.
  if (control.id === "courses") {
    return buildRoutePath(userRoleHomeRouteId(userRole), {});
  }
  const targetParams = targetParamsFor(control, routeState);
  if (targetParams === undefined) return undefined;
  return buildRoutePath(control.destination.routeId, targetParams);
}

function selectedFor(
  control: RibbonCatalogControl<RibbonDestinationId>,
  routeState: RibbonRouteState,
): boolean {
  if (TAB_CATALOG.some((tab) => tab.id === control.id)) {
    return routeState.route.ribbon.tierOneArea === control.id;
  }
  if (control.id === "activeAttempt") {
    return (
      routeState.activeAttemptId !== undefined &&
      routeState.params.assessmentAttemptId === routeState.activeAttemptId
    );
  }
  if (control.id === "studentLatestFeedback") {
    return (
      routeState.latestFeedbackAttemptId !== undefined &&
      routeState.params.assessmentAttemptId === routeState.latestFeedbackAttemptId
    );
  }
  if (typeof control.id === "string" && control.id.startsWith("studentCourse:")) {
    return control.id === `studentCourse:${routeState.params.courseInstanceId ?? ""}`;
  }
  return (
    control.destination.kind === "route" && control.destination.routeId === routeState.route.id
  );
}

function modelForControl<Id extends RibbonDestinationId>(
  control: RibbonCatalogControl<Id>,
  routeState: RibbonRouteState,
  userRole: UserRole,
): RibbonControlModel<Id> {
  const entry = CAPABILITY_REGISTRY[control.id];
  const admission = ribbonAvailability(
    entry,
    userRole,
    relationshipStateFor(entry.relationshipRequirement),
  );
  const hasContextualTarget =
    control.id === "activeAttempt"
      ? routeState.activeAttemptId !== undefined
      : control.id === "studentLatestFeedback"
        ? routeState.latestFeedbackAttemptId !== undefined
        : true;
  const admitted = hasContextualTarget ? admission : "Unavailable";
  const href = hrefFor(control, routeState, admitted, userRole);
  const availability = href === undefined && admitted === "Available" ? "Unavailable" : admitted;
  const attemptId =
    control.id === "activeAttempt"
      ? routeState.activeAttemptId
      : control.id === "studentLatestFeedback"
        ? routeState.latestFeedbackAttemptId
        : undefined;
  const parsedAttemptId = attemptId === undefined ? null : parseAssessmentAttemptId(attemptId);
  return Object.freeze({
    id: control.id,
    label: control.label,
    destination: Object.freeze(
      control.destination.kind === "route"
        ? { kind: "route" as const, routeId: control.destination.routeId }
        : { kind: "future" as const, futureId: control.destination.futureId },
    ),
    availability,
    selected: selectedFor(control, routeState),
    ...(href === undefined ? {} : { href }),
    ...(parsedAttemptId === null ? {} : { state: assessmentAttemptRouteState(parsedAttemptId) }),
    role: control.role,
    priority: control.priority,
    presentation: control.presentation,
    iconBearing: control.iconBearing,
    iconOnlySafe: control.iconOnlySafe,
  });
}

function contextFor(userRole: UserRole): RibbonContextModel {
  return Object.freeze({
    productLabel: PRODUCT_LABELS[userRole],
    accountControls: accountControlsFor(userRole),
    signOutAction: SIGN_OUT_ACTION,
  });
}

function taskAreasFor(
  routeState: RibbonRouteState,
  userRole: UserRole,
): ReadonlyArray<RibbonTaskAreaModel> {
  const tierTwo = ribbonTierTwoSchemaFor(userRole, routeState.route.ribbon.tierOneArea);
  if (tierTwo.length === 0) return Object.freeze([]);

  const areas: Array<{
    id: RibbonTaskArea;
    controls: Array<RibbonControlModel<RibbonTaskId | RibbonStudentCourseId>>;
  }> = [];
  for (const slot of tierTwo) {
    if (slot.kind === "currentStudentCourses") {
      const area: RibbonTaskArea = "studentCourses";
      let modelArea = areas.find((candidate) => candidate.id === area);
      if (modelArea === undefined) {
        modelArea = { id: area, controls: [] };
        areas.push(modelArea);
      }
      const controls = (routeState.studentCourses ?? []).flatMap((course) => {
        const courseInstanceId = parseCourseInstanceId(course.id);
        if (courseInstanceId === null) return [];
        const href = buildRoutePath("studentCourseLanding", { courseInstanceId });
        if (href === undefined) return [];
        return [
          Object.freeze({
            id: `studentCourse:${courseInstanceId}` as const,
            label: course.shortName,
            destination: Object.freeze({
              kind: "route" as const,
              routeId: "studentCourseLanding" as const,
            }),
            availability: "Available" as const,
            selected: routeState.params.courseInstanceId === courseInstanceId,
            href,
            role: "primary" as const,
            priority: "critical" as const,
            presentation: "standard" as const,
            iconBearing: false,
            iconOnlySafe: false,
          }),
        ];
      });
      modelArea.controls.push(...controls);
      continue;
    }

    const control = RIBBON_TASK_CATALOG.find((candidate) => candidate.id === slot.id);
    if (control === undefined) throw new Error(`Ribbon schema references unknown task ${slot.id}.`);
    let modelArea = areas.find((candidate) => candidate.id === control.area);
    if (modelArea === undefined) {
      modelArea = {
        id: control.area,
        controls: [],
      };
      areas.push(modelArea);
    }
    modelArea.controls.push(modelForControl(control, routeState, userRole));
  }
  return Object.freeze(
    areas.map((area) => Object.freeze({ ...area, controls: Object.freeze([...area.controls]) })),
  );
}

/**
 * Ancestor selection applies only when the current Tier 2 row has no exact match.
 * Active Attempt and Latest Feedback stay exact matches, so they win over a parent.
 */
function ancestorTaskId(
  routeState: RibbonRouteState,
  userRole: UserRole,
  labels: RibbonContextLabels,
): RibbonTaskId | undefined {
  const route = routeState.route;
  let ancestor: RibbonTaskId | undefined;
  if (route.ribbon.tierTwoParent !== undefined) {
    ancestor = route.ribbon.tierTwoParent;
  } else if (
    route.id === "courseAssessments" ||
    route.id === "courseRoster" ||
    route.id === "courseAppearance"
  ) {
    if (labels.courseLifecycleState === "active") ancestor = "myActiveCourses";
    else if (labels.courseLifecycleState === "inactive") ancestor = "myInactiveCourses";
  } else if (route.id === "blueprintCourseDetail") {
    if (labels.blueprintBreadcrumbParent === "myBlueprintCourses") ancestor = "myBlueprintCourses";
    else if (labels.blueprintBreadcrumbParent === "publicBlueprintSearch") {
      ancestor = "searchPublicBlueprintCourses";
    }
  }
  if (ancestor === undefined) return undefined;
  const listed = ribbonTierTwoSchemaFor(userRole, route.ribbon.tierOneArea).some(
    (slot) => slot.kind === "destination" && slot.id === ancestor,
  );
  return listed ? ancestor : undefined;
}

function selectAncestorTask(
  areas: ReadonlyArray<RibbonTaskAreaModel>,
  ancestorId: RibbonTaskId | undefined,
): ReadonlyArray<RibbonTaskAreaModel> {
  if (ancestorId === undefined) return areas;
  if (areas.some((area) => area.controls.some((control) => control.selected))) return areas;
  return Object.freeze(
    areas.map((area) =>
      Object.freeze({
        ...area,
        controls: Object.freeze(
          area.controls.map((control) =>
            control.id === ancestorId ? Object.freeze({ ...control, selected: true }) : control,
          ),
        ),
      }),
    ),
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
  _navigation: RibbonContextNavigation = {},
): RibbonModel {
  const schema = ribbonSchemaFor(viewerIdentity.userRole);
  const tabs = schema.map((slot) => {
    const control = TAB_CATALOG.find((candidate) => candidate.id === slot.id);
    if (control === undefined) throw new Error(`Ribbon schema references unknown tab ${slot.id}.`);
    return modelForControl(control, routeState, viewerIdentity.userRole);
  });
  const taskAreas = selectAncestorTask(
    taskAreasFor(routeState, viewerIdentity.userRole),
    ancestorTaskId(routeState, viewerIdentity.userRole, contextLabels),
  );
  const context = contextFor(viewerIdentity.userRole);
  const breadcrumbs = presentRibbonBreadcrumbs(
    routeState,
    viewerIdentity.userRole,
    contextLabels,
    tabs,
    taskAreas,
  );
  return Object.freeze({
    scope: routeState.route.ribbon.scope,
    context,
    tabs: Object.freeze(tabs),
    taskAreas,
    breadcrumbs:
      breadcrumbs.length > 0
        ? breadcrumbs
        : Object.freeze([
            Object.freeze({
              label: "Home",
              href: userRoleHomePath(viewerIdentity.userRole),
              current: false,
            }),
          ]),
  });
}

/** The same boundary predicate retained beside derivation for model-level consumers. */
export function ribbonModelAvailabilityMayAccessRoute(
  control: RibbonControlModel,
  userRole: UserRole,
): boolean {
  if (control.availability !== "Available" || control.destination.kind !== "route") return true;
  return userRoleMayAccessRoute(control.destination.routeId, userRole);
}
