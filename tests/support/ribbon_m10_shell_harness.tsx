// ribbon_m10_shell_harness.tsx - current-source App composition for compiled-harness evidence.

import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";
import { MemoryRouter, createMemoryHistory, useLocation } from "@solidjs/router";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import { App } from "../../src/app";
import { ApplicationShell } from "../../src/application_shell";
import { SessionProvider } from "../../src/auth/session_context";
// prettier-ignore
import {
  useCourseThemePresentation,
} from "../../src/features/course_appearance/course_theme_context";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type {
  AuthenticatedSession,
  CourseRouteView,
  CourseSummary,
  CursorPage,
} from "../../src/api/contracts";
import type { CourseId } from "../../generated/api/CourseId";
// prettier-ignore
import type {
  StudentAssignmentLandingSummary,
} from "../../generated/api/StudentAssignmentLandingSummary";
import { appRoutes, notFoundRoute } from "../../src/routes";
import type {
  RibbonControlModel,
  RibbonModel,
  RibbonTaskAreaModel,
} from "../../src/ribbon/ribbon_contract";
import type { RibbonDestinationId } from "../../src/ribbon/ribbon_catalog";
import { courseRouteData, assignmentAttemptScreenData } from "./route_scope_provider_fixtures";
import { M6_RIBBON_FIXTURES } from "./ribbon_model_fixtures";

interface QueryFunction<Arguments extends ReadonlyArray<unknown>, Result> {
  (...arguments_: Arguments): Promise<Result>;
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
}

function queryFunction<Arguments extends ReadonlyArray<unknown>, Result>(
  key: string,
  resolve: (...arguments_: Arguments) => Promise<Result>,
): QueryFunction<Arguments, Result> {
  const callable = Object.assign(resolve, {
    key,
    keyFor: (...arguments_: Arguments) => `${key}:${JSON.stringify(arguments_)}`,
  });
  return callable;
}

interface DeferredCourseScopes {
  readonly release: (reference: string) => void;
  readonly requestCount: (reference: string) => number;
  readonly waitForRelease: (reference: string) => Promise<void>;
}

interface PresentationQueryCounts {
  readonly assignments: () => number;
}

function deferredCourseScopes(): DeferredCourseScopes {
  const requests = new Map<string, number>();
  const releases = new Map<string, () => void>();

  function release(reference: string): void {
    const resolve = releases.get(reference);
    if (resolve === undefined) {
      throw new Error(
        `Application-shell evidence cannot release an unrequested course scope: ${reference}.`,
      );
    }
    releases.delete(reference);
    resolve();
  }

  function waitForRelease(reference: string): Promise<void> {
    requests.set(reference, (requests.get(reference) ?? 0) + 1);
    if (reference !== "C-1") return Promise.resolve();
    return new Promise((resolve) => {
      releases.set(reference, resolve);
    });
  }

  return { release, requestCount: (reference) => requests.get(reference) ?? 0, waitForRelease };
}

function presentationApi(deferredScopes?: DeferredCourseScopes): {
  readonly api: ApplicationApi<OrdinaryBrowserApiClient>;
  readonly counts: PresentationQueryCounts;
} {
  const courses: CursorPage<CourseSummary> = { items: [], nextCursor: null };
  const assignments: CursorPage<StudentAssignmentLandingSummary> = {
    items: [],
    nextCursor: null,
  };
  let assignmentQueries = 0;
  const queries = {
    courses: queryFunction("courses", () => Promise.resolve(courses)),
    assignments: queryFunction("course-assignments", (_courseId: CourseId) => {
      assignmentQueries += 1;
      return Promise.resolve(assignments);
    }),
    resolveCourse: queryFunction("resolve-course", (reference: string) =>
      Promise.resolve({ courseId: `course-${reference}` }),
    ),
    resolveAssignmentAttempt: queryFunction("resolve-assignment-attempt", (reference: string) =>
      Promise.resolve({ assignmentAttemptId: `attempt-${reference}` }),
    ),
    courseScope: queryFunction("course-scope", (courseId: string) => {
      const reference = courseId.replace("course-", "");
      const released =
        deferredScopes === undefined ? Promise.resolve() : deferredScopes.waitForRelease(reference);
      return released.then(() => instructorCourseRouteData(reference));
    }),
    assignmentAttemptScreen: queryFunction("assignment-attempt-screen", () =>
      Promise.resolve(assignmentAttemptScreenData("C-1")),
    ),
    assignmentAttemptSummary: queryFunction("assignment-attempt-summary", () =>
      Promise.reject(
        new Error("Application-shell evidence does not enter attempt-summary content."),
      ),
    ),
  };
  // The current-source App only reaches the typed query subset above in this
  // controlled browser fixture. The current-source course Assignments page receives an explicit,
  // typed empty page; other routed content has no transport methods to invoke here.
  return {
    api: { client: {}, queries } as unknown as ApplicationApi<OrdinaryBrowserApiClient>,
    counts: { assignments: () => assignmentQueries },
  };
}

function instructorSession(): AuthenticatedSession {
  return {
    authenticated: true,
    account: { id: "account-m10", productRole: "instructor" },
  };
}

/** The controlled current-source course page is an instructor-owned C-1 surface. */
function instructorCourseRouteData(reference: string): CourseRouteView {
  const course = courseRouteData(reference);
  return { ...course, summary: { ...course.summary, role: "instructor" as const } };
}

export interface RibbonM10ShellHarness {
  readonly dispose: () => void;
  readonly disposeCurrent: () => void;
  readonly currentNavigate: (pathname: string) => void;
  readonly currentPathname: () => string;
  readonly fixtureNavigate: (pathname: string) => void;
  readonly fixturePathname: () => string;
  readonly scopeRequestCount: (reference: string) => number;
  readonly assignmentQueryCount: () => number;
  readonly releaseCourseScope: (reference: string) => void;
  readonly throwFixtureContent: (value: boolean) => void;
  readonly signOutActions: () => number;
}

function withSelectedControl<Id extends RibbonDestinationId>(
  controls: ReadonlyArray<RibbonControlModel<Id>>,
  selectedId: Id,
): ReadonlyArray<RibbonControlModel<Id>> {
  return controls.map((control) => ({ ...control, selected: control.id === selectedId }));
}

function withSelectedTaskControl(
  areas: ReadonlyArray<RibbonTaskAreaModel>,
  selectedId: string,
): ReadonlyArray<RibbonTaskAreaModel> {
  return areas.map((area) => ({
    ...area,
    controls: area.controls.map((control) => ({ ...control, selected: control.id === selectedId })),
  }));
}

function courseFixture(
  reference: string,
  selectedTab: "assignments" | "students" | "gradebook" = "assignments",
): RibbonModel {
  const source = M6_RIBBON_FIXTURES.courseInstructor;
  return {
    ...source,
    context: { ...source.context, scopeLabel: `Course ${reference}` },
    tabs: withSelectedControl(source.tabs, selectedTab),
    taskAreas: withSelectedTaskControl(source.taskAreas, "assignmentOverview"),
  };
}

function productFixture(
  selectedTab: "courses" | "questionLibrary" | "blueprintCourses",
): RibbonModel {
  const source = M6_RIBBON_FIXTURES.productInstructor;
  return {
    ...source,
    tabs: withSelectedControl(source.tabs, selectedTab),
    taskAreas: withSelectedTaskControl(source.taskAreas, "allQuestions"),
  };
}

/** Explicit presentation-only projection for the structural shell fixture. */
function fixtureModelForPathname(pathname: string): RibbonModel {
  if (pathname === "/") return productFixture("courses");
  if (pathname === "/library") return productFixture("questionLibrary");
  if (pathname === "/blueprint-courses") return productFixture("blueprintCourses");
  if (pathname === "/instructor/courses/C-1/students") return courseFixture("C-1", "students");
  if (pathname === "/instructor/courses/C-1/gradebook") return courseFixture("C-1", "gradebook");
  if (pathname === "/courses/C-2") return courseFixture("C-2");
  if (pathname === "/assignment-attempts/R-1") return M6_RIBBON_FIXTURES.attemptInstructor;
  return courseFixture("C-1");
}

function FixtureContent(props: {
  readonly pathname: string;
  readonly shouldThrow: () => boolean;
}): JSX.Element {
  const presentAppearance = useCourseThemePresentation();

  function presentOceanTheme(): void {
    if (presentAppearance === undefined) {
      throw new Error("Shell fixture requires the source course-theme presentation context.");
    }
    presentAppearance({ ...courseRouteData("C-1").appearance, theme: "ocean" });
  }

  if (props.shouldThrow()) throw new Error("Shell fixture content sentinel failure");
  return (
    <section class="page" data-m10-fixture-content={props.pathname}>
      Fixture content
      <button type="button" data-m10-theme-swap onClick={presentOceanTheme}>
        Present Ocean course theme
      </button>
    </section>
  );
}

/**
 * Mounts two deliberately labelled cases. The first is the current-source routed App;
 * the second uses only ApplicationShell's explicit model seam for structural
 * navigation/error evidence and never changes route admission.
 */
export function mountRibbonM10ShellHarness(target: HTMLElement): RibbonM10ShellHarness {
  const currentHistory = createMemoryHistory();
  const fixtureHistory = createMemoryHistory();
  const [fixtureShouldThrow, setFixtureShouldThrow] = createSignal(false);
  const [signOutActions, setSignOutActions] = createSignal(0);
  let logoutAttempts = 0;
  const currentDeferredScopes = deferredCourseScopes();
  const currentPresentation = presentationApi(currentDeferredScopes);

  function fixtureNavigate(pathname: string): void {
    fixtureHistory.set({ value: pathname });
  }

  function FixtureInterior(): JSX.Element {
    const location = useLocation();
    function fixtureContent(pathname: string): JSX.Element {
      return <FixtureContent pathname={pathname} shouldThrow={fixtureShouldThrow} />;
    }
    function fixtureRibbonModel(): RibbonModel {
      return fixtureModelForPathname(location.pathname);
    }
    return (
      <ApplicationShell
        pathname={() => location.pathname}
        ribbonModel={fixtureRibbonModel}
        content={fixtureContent}
      />
    );
  }

  const currentTarget = document.createElement("section");
  currentTarget.dataset.m10Case = "current-production";
  currentTarget.setAttribute("aria-label", "Current-source empty admission");
  const fixtureTarget = document.createElement("section");
  fixtureTarget.dataset.m10Case = "fixture-shell";
  fixtureTarget.setAttribute("aria-label", "Structural fixture shell evidence");
  target.replaceChildren(currentTarget, fixtureTarget);

  const disposeCurrentRender = render(
    () => (
      <ApplicationApiProvider applicationApi={currentPresentation.api}>
        <SessionProvider
          getSession={() => Promise.resolve(instructorSession())}
          logout={() => Promise.resolve()}
          advanceSessionBoundary={() => undefined}
        >
          <MemoryRouter history={currentHistory} root={App}>
            {[...appRoutes, notFoundRoute]}
          </MemoryRouter>
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    currentTarget,
  );
  const disposeFixtureRender = render(
    () => (
      <ApplicationApiProvider applicationApi={presentationApi().api}>
        <SessionProvider
          getSession={() => Promise.resolve(instructorSession())}
          logout={() => {
            logoutAttempts += 1;
            setSignOutActions((count) => count + 1);
            if (logoutAttempts === 1) {
              return Promise.reject(new Error("fixture first sign-out remains unconfirmed"));
            }
            return Promise.resolve();
          }}
          advanceSessionBoundary={() => undefined}
        >
          <p class="sr-only">
            Structural fixture evidence, not a dist or real-stack browser workflow.
          </p>
          <MemoryRouter history={fixtureHistory} root={FixtureInterior} />
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    fixtureTarget,
  );
  let currentDisposed = false;
  function disposeCurrent(): void {
    if (currentDisposed) return;
    currentDisposed = true;
    disposeCurrentRender();
  }
  function dispose(): void {
    disposeCurrent();
    disposeFixtureRender();
  }

  return {
    dispose,
    disposeCurrent,
    currentNavigate: (pathname: string) => currentHistory.set({ value: pathname }),
    currentPathname: currentHistory.get,
    fixtureNavigate,
    fixturePathname: fixtureHistory.get,
    scopeRequestCount: currentDeferredScopes.requestCount,
    assignmentQueryCount: currentPresentation.counts.assignments,
    releaseCourseScope: currentDeferredScopes.release,
    throwFixtureContent: setFixtureShouldThrow,
    signOutActions,
  };
}
