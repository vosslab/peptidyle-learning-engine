// Real DOM harness for Course Appearance state transitions.

import { createSignal } from "solid-js";
import { render } from "solid-js/web";

import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { CourseBannerUpdate } from "../../generated/api/CourseBannerUpdate";
import type { CourseId } from "../../generated/api/CourseId";
import type { ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { CourseRouteView } from "../../src/api/contracts";
import { ApplicationApiProvider } from "../../src/api/application_api";
import { CourseThemeVariables } from "../../src/features/course_appearance/course_theme_variables";
import { CourseAppearancePage } from "../../src/pages/course_appearance_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

const COURSE_ID = "course-m7" as CourseId;
const COURSE_PATH = "/instructor/courses/C-1/appearance";

function initialCourse(): CourseRouteView {
  return {
    summary: {
      id: COURSE_ID,
      reference: "C-1",
      title: "Course one",
      term: { startDate: "2026-01-12", endDate: "2026-05-08", timeZone: "America/Chicago" },
      role: "instructor",
    },
    appearance: { theme: "grass", banner: null },
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
  const applicationApi = {
    client: {
      updateCourseTheme: (
        _courseId: CourseId,
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
      setCourseBanner: (_courseId: CourseId, update: CourseBannerUpdate) => {
        bannerSets += 1;
        currentAppearance = {
          ...currentAppearance,
          banner: { reference: "banner-current", alternativeText: update.alternativeText },
        };
        return Promise.resolve(currentAppearance);
      },
      removeCourseBanner: () => {
        bannerRemovals += 1;
        currentAppearance = { ...currentAppearance, banner: null };
        return Promise.resolve(currentAppearance);
      },
      fetchCourseBanner: () => Promise.resolve(new Blob(["hero"], { type: "image/webp" })),
      fetchCourseBannerCard: () => Promise.resolve(new Blob(["card"], { type: "image/webp" })),
    },
    queries: {
      resolveCourse: () => Promise.resolve({ courseId: COURSE_ID }),
      courseScope: () => {
        if (initialScope === "resolved" && !initialScopeResolved) {
          initialScopeResolved = true;
          return Promise.resolve(initialCourse());
        }
        activeScope = deferredScope();
        return activeScope.promise;
      },
      resolveAssignmentAttempt: () =>
        Promise.reject(
          new Error("Course Appearance harness does not resolve Assignment Attempts."),
        ),
      assignmentAttemptSummary: () =>
        Promise.reject(
          new Error("Course Appearance harness does not load Assignment Attempt summaries."),
        ),
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const [pageVisible, setPageVisible] = createSignal(true);
  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <RouteScopeProvider pathname={COURSE_PATH}>
          <CourseThemeVariables>
            {pageVisible() ? <CourseAppearancePage /> : <p data-m7-page-removed>Page removed</p>}
          </CourseThemeVariables>
        </RouteScopeProvider>
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
    resolveScope: (): void => activeScope?.resolve(initialCourse()),
    rejectScope: (): void => activeScope?.reject(),
  };
}
