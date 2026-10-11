// Mounts the routed production App with controlled session, Course scopes, and API data.

import { render } from "solid-js/web";
import { MemoryRouter, createMemoryHistory } from "@solidjs/router";

import "../../src/browser_environment";

import { ApplicationApiProvider } from "../../src/api/application_api";
import type { AuthenticatedSession } from "../../src/api/contracts";
import { App } from "../../src/app";
import { SessionProvider } from "../../src/auth/session_context";
import { InstalledWasmFacadeProvider } from "../../src/wasm/context";
import type {
  CapabilityValidator,
  ResponseFormatValidator,
  TimerEvaluator,
  WasmFacade,
} from "../../src/wasm";
import { appRoutes, notFoundRoute } from "../../src/routes";
import { presentationApi } from "./ribbon_shell_harness_api";

interface DeferredCourseScopes {
  readonly release: (courseInstanceId: string) => void;
  readonly requestCount: (courseInstanceId: string) => number;
  readonly waitForRelease: (courseInstanceId: string) => Promise<void>;
}

interface DeferredSession {
  readonly release: () => void;
  readonly waitForSession: () => Promise<AuthenticatedSession>;
}

function deferredSession(): DeferredSession {
  let resolveSession: (() => void) | undefined;
  const sessionReady = new Promise<void>((resolve) => {
    resolveSession = resolve;
  });
  function release(): void {
    if (resolveSession === undefined) {
      throw new Error("Application-shell evidence released the session more than once.");
    }
    const resolve = resolveSession;
    resolveSession = undefined;
    resolve();
  }
  function waitForSession(): Promise<AuthenticatedSession> {
    return sessionReady.then(instructorSession);
  }
  return { release, waitForSession };
}

function deferredCourseScopes(): DeferredCourseScopes {
  const requests = new Map<string, number>();
  const releases = new Map<string, () => void>();

  function release(courseInstanceId: string): void {
    const resolve = releases.get(courseInstanceId);
    if (resolve === undefined) {
      throw new Error(
        `Application-shell evidence cannot release an unrequested course scope: ${courseInstanceId}.`,
      );
    }
    releases.delete(courseInstanceId);
    resolve();
  }

  function waitForRelease(courseInstanceId: string): Promise<void> {
    requests.set(courseInstanceId, (requests.get(courseInstanceId) ?? 0) + 1);
    if (courseInstanceId !== "CI7K3M2QAZ") return Promise.resolve();
    return new Promise((resolve) => {
      releases.set(courseInstanceId, resolve);
    });
  }

  return {
    release,
    requestCount: (courseInstanceId) => requests.get(courseInstanceId) ?? 0,
    waitForRelease,
  };
}

function instructorSession(): AuthenticatedSession {
  return {
    authenticated: true,
    account: { id: "account-m10", userRole: "instructor" },
  };
}

export interface RibbonShellHarness {
  readonly dispose: () => void;
  readonly currentNavigate: (pathname: string) => void;
  readonly currentPathname: () => string;
  readonly scopeRequestCount: (courseInstanceId: string) => number;
  readonly revokedRosterIds: () => readonly string[];
  readonly holdLiveCourseRoster: () => void;
  readonly releaseLiveCourseRoster: () => void;
  readonly failNextRosterRevoke: () => void;
  readonly clearStudentViewTransport: () => void;
  readonly studentViewTransport: () => readonly string[];
  readonly seedAuthoringClassification: () => void;
  readonly vocabularyWrites: () => readonly string[];
  readonly releaseSession: () => void;
  readonly releaseCourseScope: (courseInstanceId: string) => void;
}

const ribbonFormatFallback: ResponseFormatValidator = () =>
  Promise.reject(new Error("Ribbon evidence does not validate responses."));
const ribbonTimerFallback: TimerEvaluator = () =>
  Promise.reject(new Error("Ribbon evidence does not evaluate timers."));
const ribbonCapabilityFallback: CapabilityValidator = () =>
  Promise.reject(new Error("Ribbon evidence does not validate assessment config."));
const ribbonWasmFacade: WasmFacade = {
  mode: "serverFallback",
  validateResponseFormat: ribbonFormatFallback,
  questionAttemptTimingDecision: ribbonTimerFallback,
  assessmentAttemptRemainingMilliseconds: () =>
    Promise.reject(new Error("Ribbon evidence does not measure remaining time.")),
  validateAssessmentConfig: ribbonCapabilityFallback,
  previewPleDraft: (request) =>
    Promise.resolve({
      kind: "unavailable",
      backend: request.questionBackend,
      capability: "offlinePreview",
    }),
  verifyNativeStaticPresentationDescriptor: () => Promise.resolve({ kind: "unavailable" }),
};

/** The loader and browser consumers share this single routed App mount. */
export function mountRibbonShellHarness(target: HTMLElement): RibbonShellHarness {
  const currentHistory = createMemoryHistory();
  const currentDeferredScopes = deferredCourseScopes();
  const currentSession = deferredSession();
  const currentPresentation = presentationApi(currentDeferredScopes.waitForRelease);
  const currentTarget = document.createElement("section");
  currentTarget.dataset.m10Case = "current-production";
  currentTarget.setAttribute("aria-label", "Current-source routed application");
  target.replaceChildren(currentTarget);

  const disposeRender = render(
    () => (
      <ApplicationApiProvider applicationApi={currentPresentation.api}>
        <SessionProvider
          getSession={currentSession.waitForSession}
          logout={() => Promise.resolve()}
          advanceSessionBoundary={() => undefined}
        >
          <InstalledWasmFacadeProvider facade={ribbonWasmFacade}>
            <MemoryRouter history={currentHistory} root={App}>
              {[...appRoutes, notFoundRoute]}
            </MemoryRouter>
          </InstalledWasmFacadeProvider>
        </SessionProvider>
      </ApplicationApiProvider>
    ),
    currentTarget,
  );
  let disposed = false;
  function dispose(): void {
    if (disposed) return;
    disposed = true;
    disposeRender();
  }

  return {
    dispose,
    currentNavigate: (pathname: string) => currentHistory.set({ value: pathname }),
    currentPathname: currentHistory.get,
    scopeRequestCount: currentDeferredScopes.requestCount,
    revokedRosterIds: currentPresentation.revokedRosterIds,
    holdLiveCourseRoster: currentPresentation.holdLiveCourseRoster,
    releaseLiveCourseRoster: currentPresentation.releaseLiveCourseRoster,
    failNextRosterRevoke: currentPresentation.failNextRosterRevoke,
    clearStudentViewTransport: currentPresentation.clearStudentViewTransport,
    studentViewTransport: currentPresentation.studentViewTransport,
    seedAuthoringClassification: currentPresentation.seedAuthoringClassification,
    vocabularyWrites: currentPresentation.vocabularyWrites,
    releaseSession: currentSession.release,
    releaseCourseScope: currentDeferredScopes.release,
  };
}
