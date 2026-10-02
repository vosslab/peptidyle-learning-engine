// Declared route parameter extraction and syntax-only Ribbon scope identity.

import {
  parseAssessmentAttemptId,
  parseAssessmentId,
  parseBlueprintCourseId,
  parseBlueprintChangeProposalHandle,
  parseCourseInstanceId,
  parseCourseMembershipId,
  parseDraftQuestionId,
  parseQuestionRouteId,
  type AssessmentAttemptRouteId,
  type CourseInstanceRouteId,
} from "./public_route";
import { routeContractForPathname, type RibbonScope, type RouteContract } from "../route_contract";
import { assessmentAttemptRouteStateFromHistory } from "./assessment_attempt_route";

export type DeclaredRouteScope = RibbonScope;

export type RouteParamName =
  | "courseInstanceId"
  | "assessmentId"
  | "assessmentAttemptId"
  | "membershipId"
  | "questionId"
  | "draftQuestionId"
  | "blueprintCourseId"
  | "proposalId";

/**
 * `undefined` means the pathname did not match this declared route. A valid
 * static route returns `{}`, preserving the distinction without partial data.
 */
export type RouteParams = Readonly<Partial<Record<RouteParamName, string>>> | undefined;

export type RouteScopeKey =
  | { readonly kind: "product" }
  | {
      readonly kind: "courseInstance";
      readonly courseInstanceId: CourseInstanceRouteId;
    }
  | {
      readonly kind: "assessmentAttempt";
      readonly courseInstanceId: CourseInstanceRouteId;
      readonly assessmentAttemptId: AssessmentAttemptRouteId;
    }
  | {
      readonly kind: "invalid";
      readonly scope: DeclaredRouteScope | undefined;
    };

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

function isRouteParamName(value: string): value is RouteParamName {
  return Object.prototype.hasOwnProperty.call(ROUTE_PARAM_PARSERS, value);
}

function routeSegments(path: string): ReadonlyArray<string> {
  if (path === "/") return [];
  return path.slice(1).split("/");
}

function declaredRouteFor(route: RouteContract, pathname: string): RouteContract | undefined {
  const matchedRoute = routeContractForPathname(pathname);
  if (matchedRoute === undefined) return undefined;
  if (matchedRoute.id !== route.id || matchedRoute.path !== route.path) return undefined;
  return matchedRoute;
}

/** Extracts accepted path parameters and a valid hidden Attempt selection, when the route uses one. */
export function routeParams(
  route: RouteContract,
  pathname: string,
  historyState?: unknown,
): RouteParams {
  const declaredRoute = declaredRouteFor(route, pathname);
  if (declaredRoute === undefined) return undefined;

  const values: Partial<Record<RouteParamName, string>> = {};
  const patternSegments = routeSegments(declaredRoute.path);
  const pathnameSegments = routeSegments(pathname);
  for (const [index, patternSegment] of patternSegments.entries()) {
    if (!patternSegment.startsWith(":")) continue;
    const name = patternSegment.slice(1);
    const value = pathnameSegments[index];
    if (!isRouteParamName(name) || value === undefined) return undefined;
    values[name] = value;
  }
  if (route.id === "assessmentAttempt" || route.id === "assessmentAttemptSummary") {
    const selection = assessmentAttemptRouteStateFromHistory(historyState);
    if (selection !== undefined) values.assessmentAttemptId = selection.assessmentAttemptId;
  }
  return Object.freeze(values);
}

function allParamsAreValid(params: Exclude<RouteParams, undefined>): boolean {
  for (const [name, value] of Object.entries(params)) {
    if (!isRouteParamName(name) || ROUTE_PARAM_PARSERS[name](value) === null) return false;
  }
  return true;
}

function invalidScope(scope: DeclaredRouteScope | undefined): RouteScopeKey {
  return { kind: "invalid", scope };
}

/**
 * Produces public path scope and validated history selection identity. User Role
 * and service authorization remain enforced by the route boundary and backend policy.
 */
export function routeScopeKey(pathname: string, historyState?: unknown): RouteScopeKey {
  const route = routeContractForPathname(pathname);
  if (route === undefined) return invalidScope(undefined);

  const scope = route.ribbon.scope;
  const params = routeParams(route, pathname, historyState);
  if (params === undefined || !allParamsAreValid(params)) return invalidScope(scope);

  if (scope === "product") return { kind: "product" };
  if (scope === "courseInstance") {
    const courseInstanceId = params.courseInstanceId;
    const parsedCourseInstanceId =
      courseInstanceId === undefined ? null : parseCourseInstanceId(courseInstanceId);
    if (parsedCourseInstanceId === null) return invalidScope(scope);
    return { kind: "courseInstance", courseInstanceId: parsedCourseInstanceId };
  }

  if (route.id === "assessmentAttempt" || route.id === "assessmentAttemptSummary") {
    const courseInstanceId = params.courseInstanceId;
    const parsedCourseInstanceId =
      courseInstanceId === undefined ? null : parseCourseInstanceId(courseInstanceId);
    const selection = assessmentAttemptRouteStateFromHistory(historyState);
    if (parsedCourseInstanceId === null || selection === undefined) return invalidScope(scope);
    return {
      kind: "assessmentAttempt",
      courseInstanceId: parsedCourseInstanceId,
      assessmentAttemptId: selection.assessmentAttemptId,
    };
  }

  const assessmentAttemptId = params.assessmentAttemptId;
  const parsedAssessmentAttemptId =
    assessmentAttemptId === undefined ? null : parseAssessmentAttemptId(assessmentAttemptId);
  if (parsedAssessmentAttemptId === null) return invalidScope(scope);
  return invalidScope(scope);
}
