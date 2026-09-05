// Declared route parameter extraction and syntax-only Ribbon scope identity.

import {
  parseAssignmentAttemptReference,
  parseAssignmentReference,
  parseBlueprintCourseReference,
  parseCourseInstanceReference,
  parseCourseMembershipReference,
  parseQuestionRouteReference,
  type AssignmentAttemptRouteReference,
  type CourseInstanceRouteReference,
} from "./public_route";
import { routeContractForPathname, type RibbonScope, type RouteContract } from "../route_contract";

export type DeclaredRouteScope = RibbonScope;

export type RouteParamName =
  | "courseRef"
  | "assignmentRef"
  | "assignmentAttemptRef"
  | "membershipRef"
  | "questionRef"
  | "blueprintCourseRef";

/**
 * `undefined` means the pathname did not match this declared route. A valid
 * static route returns `{}`, preserving the distinction without partial data.
 */
export type RouteParams = Readonly<Partial<Record<RouteParamName, string>>> | undefined;

export type RouteScopeKey =
  | { readonly kind: "product" }
  | {
      readonly kind: "courseInstance";
      readonly courseReference: CourseInstanceRouteReference;
    }
  | {
      readonly kind: "assignmentAttempt";
      readonly assignmentAttemptReference: AssignmentAttemptRouteReference;
    }
  | {
      readonly kind: "invalid";
      readonly scope: DeclaredRouteScope | undefined;
    };

type RouteParamParser = (value: string) => string | null;

const ROUTE_PARAM_PARSERS: Readonly<Record<RouteParamName, RouteParamParser>> = {
  courseRef: parseCourseInstanceReference,
  assignmentRef: parseAssignmentReference,
  assignmentAttemptRef: parseAssignmentAttemptReference,
  membershipRef: parseCourseMembershipReference,
  questionRef: parseQuestionRouteReference,
  blueprintCourseRef: parseBlueprintCourseReference,
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

/**
 * Extracts raw declared parameters only after the current route matcher accepts this declared row.
 */
export function routeParams(route: RouteContract, pathname: string): RouteParams {
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
 * Produces only URL-syntax scope identity. Product Role and service authorization
 * remain enforced by the route access boundary and backend policy.
 */
export function routeScopeKey(pathname: string): RouteScopeKey {
  const route = routeContractForPathname(pathname);
  if (route === undefined) return invalidScope(undefined);

  const scope = route.ribbon.scope;
  const params = routeParams(route, pathname);
  if (params === undefined || !allParamsAreValid(params)) return invalidScope(scope);

  if (scope === "product") return { kind: "product" };
  if (scope === "courseInstance") {
    const courseReference = params.courseRef;
    const parsedCourseReference =
      courseReference === undefined ? null : parseCourseInstanceReference(courseReference);
    if (parsedCourseReference === null) return invalidScope(scope);
    return { kind: "courseInstance", courseReference: parsedCourseReference };
  }

  const attemptReference = params.assignmentAttemptRef;
  const parsedAttemptReference =
    attemptReference === undefined ? null : parseAssignmentAttemptReference(attemptReference);
  if (parsedAttemptReference === null) return invalidScope(scope);
  return { kind: "assignmentAttempt", assignmentAttemptReference: parsedAttemptReference };
}
