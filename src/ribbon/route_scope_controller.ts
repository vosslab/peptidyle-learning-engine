// route_scope_controller.ts - browser-neutral reactive cache for route-scope presentation data.

import { createEffect, createMemo, createSignal, type Accessor } from "solid-js";

import type { ApplicationApi } from "../api/application_api";
import type { OrdinaryBrowserApiClient } from "../api/client";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import {
  courseRouteView,
  type CourseThemeRouteData,
  type ReplaceCourseAppearance,
} from "../features/course_appearance/course_theme_context";
import { routeContractForPathname } from "../route_contract";
import { routeScopeKey, type RouteScopeKey } from "../navigation/route_params";

export type RouteScopeQueries = Pick<
  ApplicationApi<OrdinaryBrowserApiClient>["queries"],
  | "resolveCourse"
  | "resolveAssignmentAttempt"
  | "courseScope"
  | "assignmentAttemptScreen"
  | "assignmentAttemptSummary"
>;

type ScopeDataEntry =
  | { readonly state: "pending" }
  | { readonly state: "resolved"; readonly data: CourseThemeRouteData }
  | { readonly state: "rejected" };

/** Presentation-only state for the current route-scope request. */
export type RouteScopeLoadState = ScopeDataEntry["state"] | "unavailable";

function pathnameAccessor(pathname: Accessor<string> | string): Accessor<string> {
  return typeof pathname === "function" ? pathname : (): string => pathname;
}

function isAssignmentAttemptSummary(pathname: string): boolean {
  return routeContractForPathname(pathname)?.id === "assignmentAttemptSummary";
}

function scopeCacheKey(scope: RouteScopeKey, pathname: string): string | undefined {
  switch (scope.kind) {
    case "courseInstance":
      return `course:${scope.courseReference}`;
    case "assignmentAttempt":
      return (
        `${isAssignmentAttemptSummary(pathname) ? "attempt-summary" : "attempt-screen"}:` +
        scope.assignmentAttemptReference
      );
    case "product":
    case "invalid":
      return undefined;
  }
}

/** Stable reactive owner behind RouteScopeProvider; it is not an access boundary. */
export interface RouteScopeController {
  readonly identity: Accessor<RouteScopeKey>;
  readonly data: Accessor<CourseThemeRouteData | undefined>;
  /** Reports the current scope request without treating it as authorization. */
  readonly loadState: Accessor<RouteScopeLoadState>;
  /** Starts a fresh current-scope read after a presentation request failed. */
  readonly retry: () => void;
  /** Replaces only a resolved Course's saved appearance presentation data. */
  readonly replaceCourseAppearance: ReplaceCourseAppearance;
}

function withCourseAppearance(
  data: CourseThemeRouteData,
  appearance: CourseAppearanceView,
): CourseThemeRouteData {
  switch (data.kind) {
    case "course":
      return { ...data, course: { ...data.course, appearance } };
    case "assignmentAttempt":
      return { ...data, screen: { ...data.screen, course: { ...data.screen.course, appearance } } };
    case "assignmentAttemptSummary":
      return {
        ...data,
        response: { ...data.response, course: { ...data.response.course, appearance } },
      };
  }
}

/**
 * Caches presentation data by canonical public reference, retaining separate
 * Attempt screen/summary views while sharing the Attempt identity lookup.
 */
export function createRouteScopeController(
  pathname: Accessor<string> | string,
  queries: RouteScopeQueries,
): RouteScopeController {
  const currentPathname = pathnameAccessor(pathname);
  const identity = createMemo(() => routeScopeKey(currentPathname()));
  const [cacheVersion, setCacheVersion] = createSignal(0);
  const entries = new Map<string, ScopeDataEntry>();
  const courseIdentities = new Map<string, ReturnType<RouteScopeQueries["resolveCourse"]>>();
  const assignmentAttemptIdentities = new Map<
    string,
    ReturnType<RouteScopeQueries["resolveAssignmentAttempt"]>
  >();

  const resolveCourse = (
    reference: Parameters<RouteScopeQueries["resolveCourse"]>[0],
  ): ReturnType<RouteScopeQueries["resolveCourse"]> => {
    const cached = courseIdentities.get(reference);
    if (cached !== undefined) return cached;
    const request = queries.resolveCourse(reference);
    courseIdentities.set(reference, request);
    return request;
  };

  const resolveAssignmentAttempt = (
    reference: Parameters<RouteScopeQueries["resolveAssignmentAttempt"]>[0],
  ): ReturnType<RouteScopeQueries["resolveAssignmentAttempt"]> => {
    const cached = assignmentAttemptIdentities.get(reference);
    if (cached !== undefined) return cached;
    const request = queries.resolveAssignmentAttempt(reference);
    assignmentAttemptIdentities.set(reference, request);
    return request;
  };

  const load = (scope: RouteScopeKey, pathnameForScope: string, key: string): void => {
    entries.set(key, { state: "pending" });
    setCacheVersion((version) => version + 1);
    let request: Promise<CourseThemeRouteData>;
    switch (scope.kind) {
      case "courseInstance":
        request = resolveCourse(scope.courseReference)
          .then((resolved) => queries.courseScope(resolved.courseId))
          .then((course) => ({ kind: "course", course }));
        break;
      case "assignmentAttempt":
        request = resolveAssignmentAttempt(scope.assignmentAttemptReference).then((resolved) => {
          if (isAssignmentAttemptSummary(pathnameForScope)) {
            return queries
              .assignmentAttemptSummary(resolved.assignmentAttemptId)
              .then((response) => ({ kind: "assignmentAttemptSummary", response }) as const);
          }
          return queries
            .assignmentAttemptScreen(resolved.assignmentAttemptId)
            .then((screen) => ({ kind: "assignmentAttempt", screen }) as const);
        });
        break;
      case "product":
      case "invalid":
        return;
    }
    void request.then(
      (loaded) => {
        entries.set(key, { state: "resolved", data: loaded });
        setCacheVersion((version) => version + 1);
      },
      () => {
        entries.set(key, { state: "rejected" });
        setCacheVersion((version) => version + 1);
      },
    );
  };

  // Scope changes own loading. A consumer may read `data` late (or not at all)
  // without changing which route scope has begun resolving.
  createEffect(() => {
    const pathnameForScope = currentPathname();
    const scope = identity();
    const key = scopeCacheKey(scope, pathnameForScope);
    if (key !== undefined && entries.get(key) === undefined) load(scope, pathnameForScope, key);
  });

  // This remains a pure projection of the current keyed cache entry.
  const data = createMemo((): CourseThemeRouteData | undefined => {
    cacheVersion();
    const scope = identity();
    const key = scopeCacheKey(scope, currentPathname());
    if (key === undefined) return undefined;
    const entry = entries.get(key);
    if (entry?.state === "resolved") return entry.data;
    return undefined;
  });

  const loadState = createMemo((): RouteScopeLoadState => {
    cacheVersion();
    const scope = identity();
    const key = scopeCacheKey(scope, currentPathname());
    if (key === undefined) return "unavailable";
    return entries.get(key)?.state ?? "unavailable";
  });

  const retry = (): void => {
    const pathnameForScope = currentPathname();
    const scope = identity();
    const key = scopeCacheKey(scope, pathnameForScope);
    if (key === undefined) return;
    // A rejected identity promise must not poison a user-visible retry. The
    // owner evicts only this public route scope before issuing its fresh read.
    switch (scope.kind) {
      case "courseInstance":
        courseIdentities.delete(scope.courseReference);
        break;
      case "assignmentAttempt":
        assignmentAttemptIdentities.delete(scope.assignmentAttemptReference);
        break;
      case "product":
      case "invalid":
        return;
    }
    entries.delete(key);
    load(scope, pathnameForScope, key);
    setCacheVersion((version) => version + 1);
  };

  const replaceCourseAppearance: ReplaceCourseAppearance = (
    courseId: CourseId,
    appearance: CourseAppearanceView,
  ): void => {
    let changed = false;
    for (const [key, entry] of entries) {
      if (entry.state !== "resolved" || courseRouteView(entry.data).summary.id !== courseId)
        continue;
      entries.set(key, { state: "resolved", data: withCourseAppearance(entry.data, appearance) });
      changed = true;
    }
    if (changed) setCacheVersion((version) => version + 1);
  };

  return { identity, data, loadState, retry, replaceCourseAppearance };
}
