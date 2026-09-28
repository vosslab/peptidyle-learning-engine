// ribbon_test_support.ts - focused, server-free support for Ribbon acceptance checks.

import {
  createComponent,
  createContext,
  createSignal,
  useContext,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { renderToString } from "solid-js/web";

import type { UserRole } from "../../generated/api/UserRole";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import { createHttpApiClient, type ApiFetch } from "../../src/api/http_client";

export interface UserRoleFixture {
  readonly userRole: UserRole;
  readonly accountId: string;
}

const UserRoleFixtureContext = createContext<UserRoleFixture>();

/** Reads the role fixture supplied by a rendered test component tree. */
export function useUserRoleFixture(): UserRoleFixture {
  const fixture = useContext(UserRoleFixtureContext);
  if (fixture === undefined) throw new Error("User Role fixture provider is missing");
  return fixture;
}

/** Stable authority fixtures for every current immutable User Role. */
export const USER_ROLE_FIXTURES = {
  student: { userRole: "student", accountId: "00000000-0000-4000-8000-000000000001" },
  instructor: { userRole: "instructor", accountId: "00000000-0000-4000-8000-000000000002" },
  sysadmin: { userRole: "sysadmin", accountId: "00000000-0000-4000-8000-000000000003" },
} as const satisfies Readonly<Record<UserRole, UserRoleFixture>>;

export interface CountedApiRequest {
  readonly path: string;
  readonly method: string;
}

export interface CountingApplicationApi<ApplicationApiOutput> {
  readonly applicationApi: ApplicationApiOutput;
  readonly requests: () => readonly CountedApiRequest[];
  readonly countRequests: (path: string) => number;
}

function requestPath(input: RequestInfo | URL): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return input.pathname;
  return new URL(input.url).pathname;
}

function sessionResponse(fixture: UserRoleFixture): Response {
  const payload = JSON.stringify({
    authenticated: true,
    account: { id: fixture.accountId, userRole: fixture.userRole },
  });
  return new Response(payload, { headers: { "content-type": "application/json" } });
}

/**
 * Builds a real API client around an in-memory counted transport, then injects it
 * through createApplicationApi supplied by the browser or a focused test.
 */
export function createCountingApplicationApi<ApplicationApiOutput>(
  createApplicationApi: (client: OrdinaryBrowserApiClient) => ApplicationApiOutput,
  fixture: UserRoleFixture,
): CountingApplicationApi<ApplicationApiOutput> {
  const requests: CountedApiRequest[] = [];
  const fetch: ApiFetch = (input, init) => {
    const path = requestPath(input);
    const method = init?.method ?? "GET";
    requests.push({ path, method });
    if (path === "/api/auth/session") return Promise.resolve(sessionResponse(fixture));
    return Promise.resolve(new Response("not found", { status: 404, statusText: "Not Found" }));
  };
  const client = createHttpApiClient({ fetch });
  const applicationApi = createApplicationApi(client);
  return {
    applicationApi,
    requests: () => requests,
    countRequests: (path) => requests.filter((request) => request.path === path).length,
  };
}

export interface MountedTransitionApp {
  readonly navigate: (pathname: string) => void | Promise<void>;
  readonly dispose?: () => void;
}

export interface TransitionDriverOptions {
  readonly pathnames: readonly string[];
  readonly mount: () => MountedTransitionApp;
}

/** Walks pathnames through exactly one mounted application instance. */
export async function walkPathnamesThroughMountedApp(
  options: TransitionDriverOptions,
): Promise<void> {
  if (options.pathnames.length === 0)
    throw new Error("transition driver needs at least one pathname");
  const app = options.mount();
  try {
    for (const pathname of options.pathnames) await app.navigate(pathname);
  } finally {
    app.dispose?.();
  }
}

export interface DeferredResolution<Value> {
  readonly promise: Promise<Value>;
  readonly resolve: (value: Value) => void;
  readonly reject: (reason: unknown) => void;
}

/** Creates a promise whose result the consumer deliberately releases. */
export function createDeferredResolution<Value>(): DeferredResolution<Value> {
  let resolve: (value: Value) => void = () => undefined;
  let reject: (reason: unknown) => void = () => undefined;
  const promise = new Promise<Value>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

export interface ScrollIntoViewStub {
  readonly element: { readonly scrollIntoView: (options?: ScrollIntoViewOptions) => void };
  readonly calls: () => readonly ScrollIntoViewOptions[];
}

/** Records scroll requests without requiring a browser layout engine. */
export function createScrollIntoViewStub(): ScrollIntoViewStub {
  const calls: ScrollIntoViewOptions[] = [];
  return {
    element: {
      scrollIntoView: (options = {}) => calls.push({ ...options }),
    },
    calls: () => calls,
  };
}

export interface RoutingInFlightSignal {
  readonly inFlight: Accessor<boolean>;
  readonly setInFlight: (value: boolean) => void;
}

/** Supplies router progress as an explicit reactive dependency for Ribbon tests. */
export function createRoutingInFlightSignal(initial = false): RoutingInFlightSignal {
  const [inFlight, setInFlight] = createSignal(initial);
  return { inFlight, setInFlight };
}

/** Renders a consumer component under every immutable User Role fixture. */
export function mountForEachUserRole(consumer: Component): readonly string[] {
  return Object.values(USER_ROLE_FIXTURES).map(function renderFixture(fixture): string {
    function FixtureTree(): JSX.Element {
      return createComponent(UserRoleFixtureContext.Provider, {
        value: fixture,
        get children(): JSX.Element {
          return createComponent(consumer, {});
        },
      });
    }
    return renderToString(FixtureTree);
  });
}
