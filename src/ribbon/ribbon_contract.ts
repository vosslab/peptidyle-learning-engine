// ribbon_contract.ts - pure, synchronous Ribbon model derivation.

import type { ProductRole } from "../../generated/api/ProductRole";
import {
  routeScopeKey,
  routeParams,
  type RouteParamName,
  type RouteParams,
} from "../navigation/route_params";
import {
  parseAssessmentAttemptId,
  parseAssessmentId,
  parseBlueprintCourseId,
  parseBlueprintChangeProposalHandle,
  parseCourseInstanceId,
  parseCourseMembershipId,
  parseDraftQuestionId,
  parseQuestionRouteId,
} from "../navigation/public_route";
import {
  ROUTE_CONTRACT,
  productRoleMayAccessRoute,
  productRoleHomeRouteId,
  productRoleHomePath,
  routeContractForPathname,
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
  /** Server-selected latest Attempt review that currently exposes feedback. */
  readonly latestFeedbackAttemptId?: string;
  /** Current enrolled Student Courses, in their stable Ribbon order. */
  readonly studentCourses?: ReadonlyArray<{
    readonly id: string;
    readonly shortName: string;
  }>;
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
  readonly assessmentTitle?: string;
  readonly assessmentAttemptTitle?: string;
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
  label: "Sign out",
});

function accountControlsFor(productRole: ProductRole): ReadonlyArray<RibbonContextControlModel> {
  return Object.freeze(
    RIBBON_CONTEXT_CONTROL_CATALOG.filter((control) =>
      control.productRoles.includes(productRole),
    ).map((control) =>
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
  courseInstanceId: parseCourseInstanceId,
  assessmentId: parseAssessmentId,
  assessmentAttemptId: parseAssessmentAttemptId,
  membershipId: parseCourseMembershipId,
  questionId: parseQuestionRouteId,
  draftQuestionId: parseDraftQuestionId,
  blueprintCourseId: parseBlueprintCourseId,
  proposalId: parseBlueprintChangeProposalHandle,
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
 * URL syntax, malformed public IDs, and incomplete substitutions fail closed.
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
  productRole: ProductRole,
): string | undefined {
  if (availability !== "Available" || control.destination.kind !== "route") return undefined;
  if (control.id === "activeAttempt") {
    const attemptId = routeState.activeAttemptId;
    if (attemptId === undefined) return undefined;
    if (parseAssessmentAttemptId(attemptId) === null) return undefined;
    return buildRoutePath(control.destination.routeId, { assessmentAttemptId: attemptId });
  }
  if (control.id === "studentLatestFeedback") {
    const attemptId = routeState.latestFeedbackAttemptId;
    if (attemptId === undefined || parseAssessmentAttemptId(attemptId) === null) return undefined;
    return buildRoutePath(control.destination.routeId, { assessmentAttemptId: attemptId });
  }
  if (control.id === "coursework" || control.id === "grades") {
    return buildRoutePath(control.destination.routeId, {});
  }
  if (control.id === "courses" && productRole === "student") {
    return buildRoutePath("studentCourses", {});
  }
  // The Courses tab is the role's stable home, not the anonymous root resolver.
  // This preserves direct role navigation even if a caller does not first visit `/`.
  if (control.id === "courses" && routeState.route.ribbon.scope === "product") {
    return buildRoutePath(productRoleHomeRouteId(productRole), {});
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
  productRole: ProductRole,
): RibbonControlModel<Id> {
  const entry = CAPABILITY_REGISTRY[control.id];
  const admission = ribbonAvailability(
    entry,
    productRole,
    relationshipStateFor(entry.relationshipRequirement),
  );
  const hasContextualTarget =
    control.id === "activeAttempt"
      ? routeState.activeAttemptId !== undefined
      : control.id === "studentLatestFeedback"
        ? routeState.latestFeedbackAttemptId !== undefined
        : true;
  const admitted = hasContextualTarget ? admission : "Unavailable";
  const href = hrefFor(control, routeState, admitted, productRole);
  const availability = href === undefined && admitted === "Available" ? "Unavailable" : admitted;
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
    role: control.role,
    priority: control.priority,
    presentation: control.presentation,
    iconBearing: control.iconBearing,
    iconOnlySafe: control.iconOnlySafe,
  });
}

function contextFor(productRole: ProductRole): RibbonContextModel {
  return Object.freeze({
    productLabel: PRODUCT_LABELS[productRole],
    accountControls: accountControlsFor(productRole),
    signOutAction: SIGN_OUT_ACTION,
  });
}

function taskAreasFor(
  routeState: RibbonRouteState,
  productRole: ProductRole,
): ReadonlyArray<RibbonTaskAreaModel> {
  const tierTwo = ribbonTierTwoSchemaFor(productRole, routeState.route.ribbon.tierOneArea);
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
    modelArea.controls.push(modelForControl(control, routeState, productRole));
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

function breadcrumbLinkItem(label: string, href: string): RibbonBreadcrumbModel {
  return Object.freeze({ label, href, current: false });
}

function breadcrumbsFor(
  routeState: RibbonRouteState,
  productRole: ProductRole,
  labels: RibbonContextLabels,
): ReadonlyArray<RibbonBreadcrumbModel> {
  if (routeState.route.id === "signIn") return Object.freeze([]);
  const homeHref = productRoleHomePath(productRole);
  const home = breadcrumbLinkItem("Home", homeHref);
  const currentHref = canonicalPathForRouteState(routeState);
  // A malformed declared route must never surface an ID-shaped label or
  // a guessed ancestor. The known role home remains reachable while scope resolves.
  if (currentHref === undefined) return Object.freeze([home]);
  const breadcrumbCurrent = (label: string): RibbonBreadcrumbModel =>
    Object.freeze({ label, href: currentHref, current: true });

  const library = breadcrumbLink("library");
  const drafts = breadcrumbLink("questionDrafts");
  const blueprints = breadcrumbLink("blueprintCourses");
  const courseParams = { courseInstanceId: routeState.params.courseInstanceId };
  const courseAssessments = breadcrumbLink("courseAssessments", courseParams);
  const studentCourse = breadcrumbLink("studentCourseProgress", courseParams);
  const assessmentParams = {
    courseInstanceId: routeState.params.courseInstanceId,
    assessmentId: routeState.params.assessmentId,
  };
  const instructorAssessment = breadcrumbLink("assessmentWorkspaceOverview", assessmentParams);
  const studentAssessment = breadcrumbLink("assessmentOverview", assessmentParams);
  const courseLabel = labels.courseLongName ?? labels.courseShortName ?? "Course";
  const courseCompactLabel =
    labels.courseLongName !== undefined &&
    labels.courseShortName !== undefined &&
    labels.courseShortName !== courseLabel
      ? labels.courseShortName
      : undefined;

  function courseBreadcrumbItem(href: string, current = false): RibbonBreadcrumbModel {
    return Object.freeze({
      label: courseLabel,
      ...(courseCompactLabel === undefined ? {} : { compactLabel: courseCompactLabel }),
      href,
      current,
    });
  }
  const assessmentLabel = labels.assessmentTitle ?? "Assessment";
  const studentAssessmentAccessLabel = labels.assessmentTitle ?? "Before you start";

  function courseTrail(current: string, courseHref: string | undefined): RibbonBreadcrumbModel[] {
    if (courseHref === undefined) return [];
    return [home, courseBreadcrumbItem(courseHref), breadcrumbCurrent(current)];
  }

  switch (routeState.route.id) {
    case "courses":
    case "instructorHome":
    case "studentHome":
    case "sysadminHome":
      return Object.freeze([{ ...home, current: true }]);
    case "studentCourses":
      return Object.freeze([home, breadcrumbCurrent("Courses")]);
    case "instructorInactiveCourses":
      return Object.freeze([home, breadcrumbCurrent("My Inactive Courses")]);
    case "profile":
      return Object.freeze([home, breadcrumbCurrent("Profile settings")]);
    case "pendingCourseInvitations":
    case "studentCourseInvitations":
      return Object.freeze([home, breadcrumbCurrent("Course Invitations")]);
    case "studentCourseInvitation":
      return Object.freeze([
        home,
        breadcrumbLinkItem(
          "Course Invitations",
          productRole === "student" ? "/student/course-invitations" : "/account/course-invitations",
        ),
        breadcrumbCurrent("Course Invitation"),
      ]);
    case "instructorAccounts":
      return Object.freeze([home, breadcrumbCurrent("Instructor Accounts")]);
    case "contentDisciplines":
      return Object.freeze([home, breadcrumbCurrent("Disciplines")]);
    case "library":
      return Object.freeze([home, breadcrumbCurrent("Question Library")]);
    case "libraryBrowse":
    case "libraryWatchNotifications":
      return Object.freeze([
        home,
        breadcrumbLinkItem("Question Library", library ?? homeHref),
        breadcrumbCurrent(
          routeState.route.id === "libraryBrowse"
            ? "Browse Question Library"
            : "Watch notifications",
        ),
      ]);
    case "questionDrafts":
      return Object.freeze([home, breadcrumbCurrent("My Draft Questions")]);
    case "blueprintCourses":
      return Object.freeze([home, breadcrumbCurrent("My Blueprint Courses")]);
    case "myChangeProposals":
      return Object.freeze([home, breadcrumbCurrent("My Change Proposals")]);
    case "changeProposalDetail":
      return Object.freeze([
        home,
        breadcrumbLinkItem("My Change Proposals", breadcrumbLink("myChangeProposals") ?? homeHref),
        breadcrumbCurrent("Change Proposal"),
      ]);
    case "assessmentsDueSoon":
      return Object.freeze([home, breadcrumbCurrent("Assessments Due Soon")]);
    case "assessmentTemplates":
      return Object.freeze([home, breadcrumbCurrent("My Assessment Templates")]);
    case "courseAssessments":
      return Object.freeze([home, courseBreadcrumbItem(currentHref, true)]);
    case "studentCourseProgress":
      return Object.freeze([home, courseBreadcrumbItem(currentHref, true)]);
    case "studentCourseLanding":
      return Object.freeze([
        home,
        courseBreadcrumbItem(studentCourse ?? currentHref),
        breadcrumbCurrent("All Coursework"),
      ]);
    case "studentResponseStats":
      return Object.freeze([home, breadcrumbCurrent("Response Stats")]);
    case "studentDueSoon":
      return Object.freeze([home, breadcrumbCurrent("Due Soon")]);
    case "studentCompleted":
      return Object.freeze([home, breadcrumbCurrent("Completed")]);
    case "studentScores":
      return Object.freeze([home, breadcrumbCurrent("Scores")]);
    case "studentAttemptHistory":
      return Object.freeze([home, breadcrumbCurrent("Attempt History")]);
    case "questionDetail":
      return library === undefined
        ? Object.freeze([])
        : Object.freeze([
            home,
            breadcrumbLinkItem("Question Library", library),
            breadcrumbCurrent("Question"),
          ]);
    case "questionDraftEditor":
      return drafts === undefined
        ? Object.freeze([])
        : Object.freeze([
            home,
            breadcrumbLinkItem("My Draft Questions", drafts),
            breadcrumbCurrent("Question"),
          ]);
    case "publicBlueprintSearch":
      return Object.freeze([home, breadcrumbCurrent("Search Public Blueprint Courses")]);
    case "blueprintCourseDetail":
      return blueprints === undefined
        ? Object.freeze([])
        : Object.freeze([
            home,
            breadcrumbLinkItem("My Blueprint Courses", blueprints),
            breadcrumbCurrent("Blueprint Course"),
          ]);
    case "assessmentWorkspaceQuestions":
    case "assessmentWorkspacePolicies":
    case "assessmentWorkspaceStudentView": {
      const section = {
        assessmentWorkspaceQuestions: "Questions",
        assessmentWorkspacePolicies: "Properties",
        assessmentWorkspaceStudentView: "Student View",
      }[routeState.route.id];
      const base = courseTrail(assessmentLabel, courseAssessments);
      if (base.length === 0 || instructorAssessment === undefined) return Object.freeze([]);
      return Object.freeze([
        ...base.slice(0, -1),
        breadcrumbLinkItem(assessmentLabel, instructorAssessment),
        breadcrumbCurrent(section),
      ]);
    }
    case "assessmentWorkspaceOverview":
      return Object.freeze(courseTrail(assessmentLabel, courseAssessments));
    case "assessmentOverview":
      return Object.freeze(courseTrail(studentAssessmentAccessLabel, studentCourse));
    case "assessmentCreate":
      return Object.freeze(courseTrail("New Assessment", courseAssessments));
    case "gradebook":
      return Object.freeze(courseTrail("Gradebook", courseAssessments));
    case "courseAppearance":
      return Object.freeze(courseTrail("Appearance", courseAssessments));
    case "courseRoster":
      return Object.freeze(courseTrail("Students", courseAssessments));
    case "assessmentAttempt":
      if (studentCourse === undefined || studentAssessment === undefined) {
        return Object.freeze([home, breadcrumbCurrent("Attempt")]);
      }
      return Object.freeze([
        home,
        courseBreadcrumbItem(studentCourse),
        breadcrumbLinkItem(labels.assessmentAttemptTitle ?? "Assessment", studentAssessment),
        breadcrumbCurrent("Attempt"),
      ]);
    case "assessmentAttemptSummary":
      return studentCourse !== undefined && studentAssessment !== undefined
        ? Object.freeze([
            home,
            courseBreadcrumbItem(studentCourse),
            breadcrumbLinkItem(labels.assessmentAttemptTitle ?? "Assessment", studentAssessment),
            breadcrumbCurrent("Attempt history"),
          ])
        : Object.freeze([home, breadcrumbCurrent("Attempt history")]);
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
  const schema = ribbonSchemaFor(viewerIdentity.productRole);
  const tabs = schema.map((slot) => {
    const control = TAB_CATALOG.find((candidate) => candidate.id === slot.id);
    if (control === undefined) throw new Error(`Ribbon schema references unknown tab ${slot.id}.`);
    return modelForControl(control, routeState, viewerIdentity.productRole);
  });
  const taskAreas = taskAreasFor(routeState, viewerIdentity.productRole);
  const context = contextFor(viewerIdentity.productRole);
  const breadcrumbs = breadcrumbsFor(routeState, viewerIdentity.productRole, contextLabels);
  return Object.freeze({
    scope: routeState.route.ribbon.scope,
    context,
    tabs: Object.freeze(tabs),
    taskAreas,
    breadcrumbs:
      breadcrumbs.length > 0
        ? breadcrumbs
        : Object.freeze([
            breadcrumbLinkItem("Home", productRoleHomePath(viewerIdentity.productRole)),
          ]),
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
