// ribbon_route_path.ts - canonical declared route paths for the Ribbon.

import { routeParams, routeScopeKey, type RouteParamName } from "../navigation/route_params";
import {
  parseAssessmentAttemptId,
  parseAssessmentId,
  parseBlueprintChangeProposalHandle,
  parseBlueprintCourseId,
  parseCourseInstanceId,
  parseCourseMembershipId,
  parseDraftQuestionId,
  parseQuestionRouteId,
} from "../navigation/public_route";
import {
  ROUTE_CONTRACT,
  routeContractForPathname,
  type RouteContract,
  type RouteId,
} from "../route_contract";

type RoutePathParams = Readonly<Partial<Record<RouteParamName, string>>>;

type Exact<Shape, Value extends Shape> = Value & Record<Exclude<keyof Value, keyof Shape>, never>;

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

export function declaredParamNames(route: RouteContract): ReadonlyArray<RouteParamName> {
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
  if (route.ribbon.scope === "assessmentAttempt") {
    return extracted.courseInstanceId !== undefined;
  }
  return routeScopeKey(pathname).kind !== "invalid";
}

/**
 * Builds only a canonical declared PLE route. Unknown route IDs, surplus data,
 * URL syntax, malformed public IDs, and incomplete substitutions fail closed.
 */
export function buildRoutePath<Route extends RouteId, Params extends RoutePathParams>(
  routeId: Route,
  params: Exact<RoutePathParams, Params>,
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
