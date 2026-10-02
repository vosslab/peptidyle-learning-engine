// Mounts the shipped Instructor Accounts page on the real Instructor Account client.

import { render } from "solid-js/web";
import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";

import type { AuthenticatedSession } from "../../src/api/contracts";
import { ApplicationApiProvider, createApplicationApi } from "../../src/api/application_api";
import { createHttpApiClient } from "../../src/api/http_client";
import type { ApiFetch } from "../../src/api/http_client/request";
import { SessionProvider } from "../../src/auth/session_context";
import { InstructorAccountsPage } from "../../src/pages/instructor_accounts_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

export type InstructorAccountApprovalMode = "approved" | "rejected";

interface InstructorAccountApprovalRequest {
  readonly path: string;
  readonly method: string;
  readonly body: string | undefined;
}

interface InstructorAccountApprovalRecord {
  readonly decisionId: string;
  readonly requests: InstructorAccountApprovalRequest[];
}

declare global {
  interface Window {
    instructorAccountApproval: InstructorAccountApprovalRecord;
  }
}

const createdAccount = {
  id: "U7K3M2PA0",
  state: "active",
  lastSuccessfulSignIn: null,
  providedAvatarId: null,
};

function sysadminSession(): Promise<AuthenticatedSession> {
  return Promise.resolve({
    authenticated: true,
    account: { id: "U7K3M2PA0", userRole: "sysadmin" },
  });
}

function decisionId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  const version = bytes[6];
  const variant = bytes[8];
  if (version === undefined || variant === undefined) {
    throw new Error("Instructor identity vetting receipt could not be minted.");
  }
  bytes[6] = (version & 0x0f) | 0x40;
  bytes[8] = (variant & 0x3f) | 0x80;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function requestPath(input: RequestInfo | URL): string {
  if (typeof input === "string") return input;
  if (input instanceof URL) return `${input.pathname}${input.search}`;
  return new URL(input.url).pathname;
}

function jsonResponse(status: number, value: unknown): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

function accountRow(
  id: string,
  state: "active" | "deactivated" | "closed",
): {
  id: string;
  state: "active" | "deactivated" | "closed";
  lastSuccessfulSignIn: null;
  providedAvatarId: null;
} {
  return {
    id,
    state,
    lastSuccessfulSignIn: null,
    providedAvatarId: null,
  };
}

function accountPage(
  accounts: ReadonlyArray<ReturnType<typeof accountRow>>,
  nextCursor: string | null,
): Response {
  return jsonResponse(200, {
    accounts,
    displayTimeZone: "America/New_York",
    nextCursor,
  });
}

function scriptedAccountPage(body: string | undefined): Response {
  const request = JSON.parse(body ?? "{}") as {
    query?: string;
    state?: string | null;
    afterAccountId?: string | null;
  };
  if (request.state === "deactivated" && request.afterAccountId === "U0000035E") {
    return accountPage([accountRow("UABCDEFGM", "deactivated")], null);
  }
  if (request.state === "deactivated") {
    return accountPage([accountRow("U0000035E", "deactivated")], "U0000035E");
  }
  if (typeof request.query === "string" && request.query.length > 0) {
    return accountPage([accountRow("U0000035E", "active")], null);
  }
  return accountPage([accountRow("U7K3M2PA0", "active")], null);
}

function approvalFetch(
  mode: InstructorAccountApprovalMode,
  record: InstructorAccountApprovalRecord,
): ApiFetch {
  return (input, init) => {
    const path = requestPath(input);
    const method = init?.method ?? "GET";
    const body = typeof init?.body === "string" ? init.body : undefined;
    record.requests.push({ path, method, body });
    if (path === "/api/instructor-identity-vetting-decisions" && method === "POST") {
      if (mode === "rejected") return Promise.resolve(jsonResponse(422, {}));
      return Promise.resolve(jsonResponse(201, { vettingDecisionId: record.decisionId }));
    }
    if (path === "/api/instructor-accounts" && method === "GET") {
      return Promise.resolve(accountPage([accountRow("U7K3M2PA0", "active")], null));
    }
    if (path === "/api/instructor-accounts" && method === "POST") {
      return Promise.resolve(jsonResponse(201, createdAccount));
    }
    if (path === "/api/instructor-accounts/find" && method === "POST") {
      return Promise.resolve(scriptedAccountPage(body));
    }
    return Promise.resolve(jsonResponse(404, {}));
  };
}

/** Mounts Instructor Accounts with the shipped client and one scripted vetting outcome. */
export function mountInstructorAccountApproval(
  target: HTMLElement,
  mode: InstructorAccountApprovalMode,
): () => void {
  const record: InstructorAccountApprovalRecord = {
    decisionId: decisionId(),
    requests: [],
  };
  window.instructorAccountApproval = record;
  const applicationApi = createApplicationApi(
    createHttpApiClient({ fetch: approvalFetch(mode, record) }),
  );
  const history = createMemoryHistory();
  history.set({ value: "/sysadmin/instructor-accounts", replace: true });
  return render(
    () => (
      <MemoryRouter history={history}>
        <Route
          path="/sysadmin/instructor-accounts"
          component={() => (
            <SessionProvider
              getSession={sysadminSession}
              logout={() => Promise.resolve()}
              advanceSessionBoundary={() => undefined}
            >
              <ApplicationApiProvider applicationApi={applicationApi}>
                <RouteScopeProvider pathname="/sysadmin/instructor-accounts">
                  <InstructorAccountsPage />
                </RouteScopeProvider>
              </ApplicationApiProvider>
            </SessionProvider>
          )}
        />
      </MemoryRouter>
    ),
    target,
  );
}
