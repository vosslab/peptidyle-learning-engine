// session_context.tsx - safe browser session bootstrap for the persistent shell.

import {
  createContext,
  createSignal,
  onMount,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import type { AuthSession } from "../api/contracts";

/**
 * Browser-visible session state deliberately contains identity and roles only.
 * The HttpOnly credential, response content, and grading data stay outside it.
 */
export type SessionBootstrapState =
  | { readonly kind: "loading" }
  | { readonly kind: "authenticated"; readonly session: AuthSession }
  | { readonly kind: "signedOut" }
  | { readonly kind: "expired" }
  | { readonly kind: "error" };

export interface SessionBootstrap {
  readonly state: Accessor<SessionBootstrapState>;
  readonly retry: () => Promise<void>;
  readonly signOut: () => Promise<boolean>;
}

/** Creates a retryable, injected session bootstrap without coupling it to HTTP. */
export function createSessionBootstrap(
  getSession: () => Promise<AuthSession>,
  logout: () => Promise<void>,
  advanceSessionBoundary: () => void,
): SessionBootstrap {
  const [state, setState] = createSignal<SessionBootstrapState>({ kind: "loading" });
  let initialized = false;
  let operationRevision = 0;

  async function retry(): Promise<void> {
    const operation = ++operationRevision;
    const advancesGeneration = initialized || operation > 1;
    setState({ kind: "loading" });
    let nextState: SessionBootstrapState;
    try {
      const session = await getSession();
      nextState = { kind: "authenticated", session };
    } catch (error: unknown) {
      nextState = sessionFailureState(error);
    }
    if (operation !== operationRevision) return;
    if (advancesGeneration) advanceSessionBoundary();
    initialized = true;
    setState(nextState);
  }

  async function signOut(): Promise<boolean> {
    const operation = ++operationRevision;
    try {
      await logout();
      if (operation !== operationRevision) return false;
      advanceSessionBoundary();
      setState({ kind: "signedOut" });
      return true;
    } catch {
      return false;
    }
  }

  return {
    state,
    retry,
    signOut,
  };
}

/** Classifies only the safe recovery path; the original error remains private. */
export function sessionFailureState(error: unknown): SessionBootstrapState {
  if (hasHttpStatus(error, 401) || hasHttpStatus(error, 403)) {
    return { kind: "expired" };
  }
  return { kind: "error" };
}

function hasHttpStatus(error: unknown, expectedStatus: number): boolean {
  if (typeof error !== "object" || error === null || !("status" in error)) {
    return false;
  }
  return error.status === expectedStatus;
}

const SessionContext = createContext<SessionBootstrap>();

export interface SessionProviderProps {
  readonly getSession: () => Promise<AuthSession>;
  readonly logout: () => Promise<void>;
  readonly advanceSessionBoundary: () => void;
  readonly children: JSX.Element;
}

/** Installs the one session bootstrap at the application composition root. */
export function SessionProvider(props: SessionProviderProps): JSX.Element {
  const bootstrap = createSessionBootstrap(
    props.getSession,
    props.logout,
    props.advanceSessionBoundary,
  );
  onMount(() => {
    void bootstrap.retry();
  });
  return <SessionContext.Provider value={bootstrap}>{props.children}</SessionContext.Provider>;
}

/** Reads the shell-owned session state without making client authorization decisions. */
export function useSessionBootstrap(): SessionBootstrap {
  const bootstrap = useContext(SessionContext);
  if (bootstrap === undefined) {
    throw new Error("SessionProvider is missing from the application root");
  }
  return bootstrap;
}
