// ribbon_deferred_content_harness.tsx - real routed deferred-content evidence.

import { render } from "solid-js/web";
import { MemoryRouter, createMemoryHistory } from "@solidjs/router";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { AuthenticatedSession, CourseRouteView } from "../../src/api/contracts";
import { SessionProvider } from "../../src/auth/session_context";
import { App } from "../../src/app";
import { routeScopeKey } from "../../src/navigation/route_params";
import { appRoutes, notFoundRoute } from "../../src/routes";
import type { CourseClassification } from "../../generated/api/CourseClassification";

type EvidenceCase = "policies" | "studentView" | "workspace" | "roster";

interface DeferredContentHarness {
  readonly dispose: () => void;
  readonly ready: () => boolean;
  readonly navigate: (caseName: EvidenceCase) => void;
  readonly release: (caseName: EvidenceCase) => void;
  readonly count: (caseName: EvidenceCase, name: string) => number;
}

const PATHS: Readonly<Record<EvidenceCase, string>> = {
  // This is a role-authorized Instructor route; the fixture's session remains
  // Instructor for every routed case in this harness.
  policies: "/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/properties",
  studentView: "/instructor/courses/CI4W8QF9AD/assessments/A5G7K3MA0/student-view",
  workspace: "/instructor/courses/CI2N7H5XAH/assessments/A2N7H5XA1",
  roster: "/instructor/courses/CI9P6R4VA5/students",
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

const COURSE_INSTANCE_ID: Readonly<Record<EvidenceCase, string>> = {
  policies: "CI7K3M2QAZ",
  studentView: "CI4W8QF9AD",
  workspace: "CI2N7H5XAH",
  roster: "CI9P6R4VA5",
};
const FIXTURE_CLASSIFICATION = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
} satisfies CourseClassification;

function instructorCourse(courseInstanceId: string): CourseRouteView {
  return {
    summary: {
      id: courseInstanceId,
      shortName: `Course ${courseInstanceId}`,
      longName: `Deferred content evidence course ${courseInstanceId}`,
      classification: FIXTURE_CLASSIFICATION,
      term: { startDate: "2026-01-12", endDate: "2026-05-08" },
      role: "instructor",
    },
    appearance: { theme: "grass", banner: null },
  };
}

function instructorSession(): AuthenticatedSession {
  return {
    authenticated: true,
    account: { id: "deferred-content-evidence-account", productRole: "instructor" },
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
export function mountRibbonDeferredContentHarness(target: HTMLElement): DeferredContentHarness {
  assertFixturePathsHaveValidScope();
  const history = createMemoryHistory();
  const counts = new Map<EvidenceCase, Map<string, number>>();
  const releases = new Map<EvidenceCase, () => void>();
  const scopeCaseByCourseId = new Map<string, EvidenceCase>(
    Object.entries(COURSE_INSTANCE_ID).map(([caseName, courseInstanceId]) => [
      courseInstanceId,
      caseName as EvidenceCase,
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
        if (property === "getProfileAvatar") {
          return () => Promise.resolve({ avatar: null });
        }
        if (property === "resolveNavigation") {
          return (id: string) => {
            increment(activeTransportCase(), "resolveNavigation");
            if (id.startsWith("A")) {
              return Promise.resolve({
                kind: "assessment",
                courseId: `course-${COURSE_INSTANCE_ID[activeTransportCase()]}`,
                assessmentId: `assessment-${id}`,
              });
            }
            return undefined;
          };
        }
        if (property === "getLiveAssessmentWorkspace")
          return () => unresolved("getLiveAssessmentWorkspace");
        if (property === "getLiveCourseRoster") return () => unresolved("getLiveCourseRoster");
        if (property === "questionImageUrl") return () => "/question-image";
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
    assessments: queryFunction("assessments", () => Promise.reject(new Error("unused"))),
    assessment: queryFunction("assessment", () => Promise.reject(new Error("unused"))),
    assessmentSummary: queryFunction("assessment-summary", () =>
      Promise.reject(new Error("unused")),
    ),
    courseScope: queryFunction("course-scope", (courseInstanceId: string) => {
      const caseName = scopeCaseByCourseId.get(courseInstanceId);
      if (caseName === undefined)
        return Promise.reject(
          new Error(`Deferred-content course scope requested for unexpected ${courseInstanceId}`),
        );
      increment(caseName, "scopeCourse");
      return deferred(caseName).then(() => instructorCourse(COURSE_INSTANCE_ID[caseName]));
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
