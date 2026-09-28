// Real DOM harness for Course Appearance state transitions.

import { createSignal } from "solid-js";
import { render } from "solid-js/web";

import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { CourseBannerUpdate } from "../../generated/api/CourseBannerUpdate";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { CourseRouteView } from "../../src/api/contracts";
import { ApplicationApiProvider } from "../../src/api/application_api";
import { AppearanceOwner } from "../../src/appearance/appearance_owner";
import { SessionProvider } from "../../src/auth/session_context";
import { CourseAppearancePage } from "../../src/pages/course_appearance_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

const COURSE_INSTANCE_ID: CourseInstanceId = "CI7K3M2QAZ";
const COURSE_PATH = "/instructor/courses/CI7K3M2QAZ/appearance";
const FIXTURE_CLASSIFICATION = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
} satisfies CourseClassification;

function initialCourse(): CourseRouteView {
  return {
    summary: {
      id: COURSE_INSTANCE_ID,
      shortName: "BCHM 301",
      longName: "Biochemistry 301: Proteins and Peptides",
      classification: FIXTURE_CLASSIFICATION,
      term: { startDate: "2026-01-12", endDate: "2026-05-08" },
      role: "instructor",
    },
    appearance: { theme: "grass", banner: null },
  };
}

function secondCourse(): CourseRouteView {
  return {
    ...initialCourse(),
    summary: { ...initialCourse().summary, id: "CI4W8QF9AD", shortName: "BIOL 302" },
    appearance: { theme: "forest", banner: null },
  };
}

interface DeferredSave {
  readonly promise: Promise<CourseAppearanceView>;
  readonly resolve: (appearance: CourseAppearanceView) => void;
  readonly reject: () => void;
}

interface DeferredScope {
  readonly promise: Promise<CourseRouteView>;
  readonly resolve: (course: CourseRouteView) => void;
  readonly reject: () => void;
}

function deferredSave(): DeferredSave {
  let resolve: (appearance: CourseAppearanceView) => void = () => undefined;
  let reject: () => void = () => undefined;
  const promise = new Promise<CourseAppearanceView>((resolvePromise, rejectPromise): void => {
    resolve = resolvePromise;
    reject = (): void => rejectPromise(new Error("Controlled Course Appearance save failure."));
  });
  return { promise, resolve, reject };
}

function deferredScope(): DeferredScope {
  let resolve: (course: CourseRouteView) => void = () => undefined;
  let reject: () => void = () => undefined;
  const promise = new Promise<CourseRouteView>((resolvePromise, rejectPromise): void => {
    resolve = resolvePromise;
    reject = (): void => rejectPromise(new Error("Controlled Course Appearance scope failure."));
  });
  return { promise, resolve, reject };
}

export interface CourseAppearanceM7Harness {
  readonly dispose: () => void;
  readonly hidePage: () => void;
  readonly saveCalls: () => number;
  readonly resolveSave: (appearance: CourseAppearanceView) => void;
  readonly rejectSave: () => void;
  readonly bannerUploadCalls: () => number;
  readonly bannerSetCalls: () => number;
  readonly bannerRemoveCalls: () => number;
  readonly resolveScope: () => void;
  readonly rejectScope: () => void;
  readonly switchCourse: () => void;
}

/** Mounts the production page and scope/theme providers with a controlled save response. */
export function mountCourseAppearanceM7Harness(
  target: HTMLElement,
  initialScope: "resolved" | "pending" = "resolved",
): CourseAppearanceM7Harness {
  const save = deferredSave();
  let currentAppearance: CourseAppearanceView = { theme: "grass", banner: null };
  let saves = 0;
  let bannerUploads = 0;
  let bannerSets = 0;
  let bannerRemovals = 0;
  let initialScopeResolved = false;
  let activeScope: DeferredScope | undefined;
  let activeCourse: (() => CourseRouteView) | undefined;
  const applicationApi = {
    client: {
      getAccountSettings: () =>
        Promise.resolve({
          timeZone: "America/Chicago",
          displayModePreference: null,
          personalTheme: "grass",
        }),
      updateCourseTheme: (
        _courseInstanceId: CourseInstanceId,
        update: { readonly theme: CourseAppearanceView["theme"] },
      ) => {
        saves += 1;
        if (saves === 1) return save.promise;
        currentAppearance = { ...currentAppearance, theme: update.theme };
        return Promise.resolve(currentAppearance);
      },
      uploadCourseBanner: () => {
        bannerUploads += 1;
        return Promise.resolve({ upload: "banner-upload" });
      },
      setCourseBanner: (_courseInstanceId: CourseInstanceId, update: CourseBannerUpdate) => {
        bannerSets += 1;
        currentAppearance = {
          ...currentAppearance,
          banner: { id: "banner-current", alternativeText: update.alternativeText },
        };
        return Promise.resolve(currentAppearance);
      },
      removeCourseBanner: () => {
        bannerRemovals += 1;
        currentAppearance = { ...currentAppearance, banner: null };
        return Promise.resolve(currentAppearance);
      },
      fetchCourseBanner: () => Promise.resolve(new Blob(["hero"], { type: "image/webp" })),
    },
    queries: {
      courseScope: (courseInstanceId: CourseInstanceId) => {
        const course = courseInstanceId === COURSE_INSTANCE_ID ? initialCourse : secondCourse;
        if (initialScope === "resolved" && !initialScopeResolved) {
          initialScopeResolved = true;
          return Promise.resolve(course());
        }
        activeScope = deferredScope();
        activeCourse = course;
        return activeScope.promise;
      },
      assessmentAttemptHistory: () =>
        Promise.reject(
          new Error("Course Appearance harness does not load Assessment Attempt history."),
        ),
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const [pageVisible, setPageVisible] = createSignal(true);
  const [pathname, setPathname] = createSignal(COURSE_PATH);
  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <SessionProvider
          getSession={() =>
            Promise.resolve({
              authenticated: true,
              account: { id: "account-m7", userRole: "instructor" },
            })
          }
          logout={() => Promise.resolve()}
          advanceSessionBoundary={() => undefined}
        >
          <RouteScopeProvider pathname={pathname}>
            <AppearanceOwner>
              {pageVisible() ? <CourseAppearancePage /> : <p data-m7-page-removed>Page removed</p>}
            </AppearanceOwner>
          </RouteScopeProvider>
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    target,
  );
  return {
    dispose,
    hidePage: () => setPageVisible(false),
    saveCalls: () => saves,
    resolveSave: (appearance: CourseAppearanceView): void => {
      currentAppearance = appearance;
      save.resolve(appearance);
    },
    rejectSave: (): void => save.reject(),
    bannerUploadCalls: () => bannerUploads,
    bannerSetCalls: () => bannerSets,
    bannerRemoveCalls: () => bannerRemovals,
    resolveScope: (): void => activeScope?.resolve(activeCourse?.() ?? initialCourse()),
    rejectScope: (): void => activeScope?.reject(),
    switchCourse: (): void => {
      setPathname("/instructor/courses/CI4W8QF9AD/appearance");
    },
  };
}
