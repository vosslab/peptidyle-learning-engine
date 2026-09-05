// ribbon_m11_deferred_content_harness.tsx - real routed deferred-content evidence.

import { render } from "solid-js/web";
import { MemoryRouter, createMemoryHistory } from "@solidjs/router";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { AuthenticatedSession, CourseRouteView } from "../../src/api/contracts";
import { SessionProvider } from "../../src/auth/session_context";
import { App } from "../../src/app";
import { routeScopeKey } from "../../src/navigation/route_params";
import { appRoutes, notFoundRoute } from "../../src/routes";
import { assignmentAttemptSummaryData } from "./route_scope_provider_fixtures";

type EvidenceCase = "summary" | "preview" | "workspace" | "roster" | "teaching";

interface DeferredContentHarness {
  readonly dispose: () => void;
  readonly ready: () => boolean;
  readonly navigate: (caseName: EvidenceCase) => void;
  readonly release: (caseName: EvidenceCase) => void;
  readonly count: (caseName: EvidenceCase, name: string) => number;
}

const PATHS: Readonly<Record<EvidenceCase, string>> = {
  // R-1 is the public Attempt reference carried by assignmentAttemptSummaryData.
  summary: "/assignment-attempts/R-1/summary",
  preview: "/instructor/courses/C-2/assignments/A-2/delivery-check",
  workspace: "/instructor/courses/C-3/assignments/A-3",
  roster: "/instructor/courses/C-4/students",
  teaching: "/instructor/courses/C-5/teaching-operations",
};

/**
 * Evidence paths are public URLs, not arbitrary router strings. Keep this
 * preflight beside their declaration so a malformed fixture cannot turn an
 * intended deferred-scope assertion into a fail-closed no-Ribbon route.
 */
function assertFixturePathsHaveValidScope(): void {
  for (const [caseName, pathname] of Object.entries(PATHS)) {
    const scope = routeScopeKey(pathname);
    if (scope.kind === "invalid")
      throw new Error(
        `Deferred-content evidence ${caseName} has invalid public route scope: ${pathname}`,
      );
  }
}

const COURSE_REFERENCE: Readonly<Record<Exclude<EvidenceCase, "summary">, string>> = {
  preview: "C-2",
  workspace: "C-3",
  roster: "C-4",
  teaching: "C-5",
};

function instructorCourse(reference: string): CourseRouteView {
  return {
    summary: {
      id: `course-${reference}`,
      reference,
      title: `Deferred content evidence course ${reference}`,
      term: { startDate: "2026-01-12", endDate: "2026-05-08", timeZone: "America/Chicago" },
      role: "instructor",
    },
    appearance: { theme: "grass", revision: "1", banner: null },
  };
}

function instructorSession(): AuthenticatedSession {
  return {
    authenticated: true,
    account: { id: "account-m11", productRole: "instructor" },
  };
}

function queryFunction<Arguments extends ReadonlyArray<unknown>, Result>(
  key: string,
  callable: (...arguments_: Arguments) => Promise<Result>,
): ((...arguments_: Arguments) => Promise<Result>) & {
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
} {
  return Object.assign(callable, {
    key,
    keyFor: (...arguments_: Arguments) => `${key}:${JSON.stringify(arguments_)}`,
  });
}

/**
 * Uses the real App's RouteScopeProvider. Scope requests are intentionally
 * withheld; page transports are distinct counters, so release proves that a
 * content-local child (not copied test JSX) starts exactly once.
 */
export function mountRibbonM11DeferredContentHarness(target: HTMLElement): DeferredContentHarness {
  assertFixturePathsHaveValidScope();
  const history = createMemoryHistory();
  const counts = new Map<EvidenceCase, Map<string, number>>();
  const releases = new Map<EvidenceCase, () => void>();
  const scopeCaseByCourseId = new Map<string, Exclude<EvidenceCase, "summary">>(
    Object.entries(COURSE_REFERENCE).map(([caseName, reference]) => [
      `course-${reference}`,
      caseName as Exclude<EvidenceCase, "summary">,
    ]),
  );
  let activeCase: EvidenceCase | undefined;

  const increment = (caseName: EvidenceCase, name: string): void => {
    const caseCounts = counts.get(caseName) ?? new Map<string, number>();
    caseCounts.set(name, (caseCounts.get(name) ?? 0) + 1);
    counts.set(caseName, caseCounts);
  };
  const deferred = (caseName: EvidenceCase): Promise<void> =>
    new Promise((resolve) => releases.set(caseName, resolve));
  const activeTransportCase = (): EvidenceCase => {
    if (activeCase === undefined)
      throw new Error(
        "Deferred-content evidence page transport began before a named case was active.",
      );
    return activeCase;
  };
  const unresolved = <Result,>(name: string): Promise<Result> => {
    increment(activeTransportCase(), name);
    return new Promise<Result>(() => undefined);
  };

  const client = new Proxy(
    {},
    {
      get(_target, property): unknown {
        if (property === "resolveNavigation") {
          return (reference: string) => {
            increment(activeTransportCase(), "resolveNavigation");
            if (reference.startsWith("A-")) {
              return Promise.resolve({
                kind: "assignment",
                courseId: `course-${
                  COURSE_REFERENCE[activeTransportCase() as Exclude<EvidenceCase, "summary">]
                }`,
                assignmentId: `assignment-${reference}`,
              });
            }
            return Promise.resolve({
              kind: "assignmentAttempt",
              courseId: "course-C-6",
              assignmentId: "assignment-A-1",
              assignmentAttemptId: "attempt-R-1",
            });
          };
        }
        if (property === "getAssignmentWorkspace")
          return () => unresolved("getAssignmentWorkspace");
        if (property === "listPreviewSchedule") return () => unresolved("listPreviewSchedule");
        if (property === "listCourseRoster") return () => unresolved("listCourseRoster");
        if (property === "listCourseInstructors") return () => unresolved("listCourseInstructors");
        if (property === "listInstructorCourseInvitations")
          return () => unresolved("listInstructorCourseInvitations");
        if (property === "getAssignmentAttemptSummary")
          return () => unresolved("postMountAssignmentAttemptSummary");
        if (property === "assetUrl") return () => "/asset";
        return () =>
          Promise.reject(
            new Error(`Unexpected deferred-content evidence transport: ${String(property)}`),
          );
      },
    },
  ) as OrdinaryBrowserApiClient;

  const queries = {
    courses: queryFunction("courses", () => Promise.resolve({ items: [], nextCursor: null })),
    questionSearch: queryFunction("question-search", () => Promise.reject(new Error("unused"))),
    questionDetails: queryFunction("question-details", () => Promise.reject(new Error("unused"))),
    gradebook: queryFunction("gradebook", () => Promise.reject(new Error("unused"))),
    assignments: queryFunction("assignments", () => Promise.reject(new Error("unused"))),
    assignment: queryFunction("assignment", () => Promise.reject(new Error("unused"))),
    assignmentSummary: queryFunction("assignment-summary", () =>
      Promise.reject(new Error("unused")),
    ),
    resolveCourse: queryFunction("resolve-course", (reference: string) =>
      Promise.resolve({ courseId: `course-${reference}` }),
    ),
    resolveAssignmentAttempt: queryFunction("resolve-attempt", (reference: string) => {
      if (reference !== "R-1")
        return Promise.reject(
          new Error(`Deferred-content summary scope resolved unexpected Attempt ${reference}`),
        );
      return Promise.resolve({
        courseId: "course-C-6",
        assignmentId: "assignment-A-1",
        assignmentAttemptId: "attempt-R-1",
      });
    }),
    courseScope: queryFunction("course-scope", (courseId: string) => {
      const caseName = scopeCaseByCourseId.get(courseId);
      if (caseName === undefined)
        return Promise.reject(
          new Error(`Deferred-content course scope requested for unexpected ${courseId}`),
        );
      increment(caseName, "scopeCourse");
      return deferred(caseName).then(() => instructorCourse(COURSE_REFERENCE[caseName]));
    }),
    assignmentAttemptScreen: queryFunction("attempt-screen", () =>
      Promise.reject(new Error("unused")),
    ),
    assignmentAttemptSummary: queryFunction("attempt-summary", () => {
      increment("summary", "scopeSummary");
      return deferred("summary").then(() => assignmentAttemptSummaryData("C-6"));
    }),
  };
  const applicationApi = { client, queries } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;

  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <SessionProvider
          getSession={() => Promise.resolve(instructorSession())}
          logout={() => Promise.resolve()}
          advanceSessionBoundary={() => undefined}
        >
          <MemoryRouter history={history} root={App}>
            {[...appRoutes, notFoundRoute]}
          </MemoryRouter>
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    target,
  );

  return {
    dispose,
    ready: (): boolean => target.querySelector('[aria-label="PLE application Ribbon"]') !== null,
    navigate(caseName: EvidenceCase): void {
      activeCase = caseName;
      history.set({ value: PATHS[caseName] });
    },
    release(caseName: EvidenceCase): void {
      const resolve = releases.get(caseName);
      if (resolve === undefined)
        throw new Error(`Deferred-content evidence released ${caseName} before its scope request.`);
      releases.delete(caseName);
      resolve();
    },
    count(caseName: EvidenceCase, name: string): number {
      return counts.get(caseName)?.get(name) ?? 0;
    },
  };
}
