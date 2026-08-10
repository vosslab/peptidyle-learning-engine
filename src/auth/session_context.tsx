// session_context.tsx - safe browser session bootstrap for the persistent shell.

import {
  createContext,
  createSignal,
  onMount,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import type { ApiClient } from "../api/client";
import type { AuthSession } from "../api/contracts";

/**
 * Browser-visible session state deliberately contains identity and roles only.
 * The HttpOnly credential, response content, and grading data stay outside it.
 */
export type SessionBootstrapState =
  | { readonly kind: "loading" }
  | { readonly kind: "authenticated"; readonly session: AuthSession }
  | { readonly kind: "expired" }
  | { readonly kind: "error" };

export interface SessionBootstrap {
  readonly state: Accessor<SessionBootstrapState>;
  readonly retry: () => Promise<void>;
  readonly signInWithLocalCredential: (credential: string) => Promise<boolean>;
}

/** Creates a retryable, injected session bootstrap without coupling it to HTTP. */
export function createSessionBootstrap(
  getSession: ApiClient["getSession"],
  loginWithLocalCredential: ApiClient["loginWithLocalCredential"] = () =>
    Promise.reject(new Error("local sign-in unavailable")),
): SessionBootstrap {
  const [state, setState] = createSignal<SessionBootstrapState>({ kind: "loading" });

  async function retry(): Promise<void> {
    setState({ kind: "loading" });
    try {
      const session = await getSession();
      setState({ kind: "authenticated", session });
    } catch (error: unknown) {
      setState(sessionFailureState(error));
    }
  }

  async function signInWithLocalCredential(credential: string): Promise<boolean> {
    setState({ kind: "loading" });
    try {
      const session = await loginWithLocalCredential(credential);
      setState({ kind: "authenticated", session });
      return true;
    } catch (error: unknown) {
      setState(sessionFailureState(error));
      return false;
    }
  }

  return { state, retry, signInWithLocalCredential };
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
  readonly getSession: ApiClient["getSession"];
  readonly loginWithLocalCredential: ApiClient["loginWithLocalCredential"];
  readonly children: JSX.Element;
}

/** Installs the one session bootstrap at the application composition root. */
export function SessionProvider(props: SessionProviderProps): JSX.Element {
  const bootstrap = createSessionBootstrap(props.getSession, props.loginWithLocalCredential);
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
