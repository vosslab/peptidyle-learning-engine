// app.tsx - persistent application shell and route-level error boundary.

import { A, useLocation, useNavigate, type RouteSectionProps } from "@solidjs/router";
import { createEffect, createSignal, ErrorBoundary, Show, type JSX } from "solid-js";

import { useSessionBootstrap, type SessionBootstrapState } from "./auth/session_context";
import {
  accountRoleMayAccessRoute,
  routeContractForPathname,
  type RouteId,
} from "./route_contract";

function canUseAuthoringTools(state: SessionBootstrapState): boolean {
  return canAccessRoute(state, "workspaceList");
}

function canAccessRoute(state: SessionBootstrapState, routeId: RouteId): boolean {
  if (state.kind !== "authenticated") {
    return false;
  }
  return accountRoleMayAccessRoute(routeId, state.session.account.role);
}

function canUseLibrary(state: SessionBootstrapState): boolean {
  return canAccessRoute(state, "library");
}

function canUseBlueprintCourses(state: SessionBootstrapState): boolean {
  return canAccessRoute(state, "blueprintCourses");
}

type ScopedRouteSectionProps = RouteSectionProps & { readonly pathname: string };

function isPublicAccountRoute(pathname: string): boolean {
  const routeId = routeContractForPathname(pathname)?.id;
  return routeId === "signIn";
}

function SessionContent(props: ScopedRouteSectionProps): JSX.Element {
  const session = useSessionBootstrap();
  const state = session.state;

  return (
    <Show
      when={state().kind === "authenticated"}
      fallback={<SessionRecovery state={state()} retry={session.retry} />}
    >
      {props.children}
    </Show>
  );
}

function RouteContent(props: ScopedRouteSectionProps): JSX.Element {
  return isPublicAccountRoute(props.pathname) ? props.children : <SessionContent {...props} />;
}

interface SessionRecoveryProps {
  readonly state: SessionBootstrapState;
  readonly retry: () => Promise<void>;
}

function SessionRecovery(props: SessionRecoveryProps): JSX.Element {
  if (props.state.kind === "authenticated") {
    return <></>;
  }
  if (props.state.kind === "loading") {
    return (
      <section class="page" data-session-state="loading" aria-live="polite">
        <p class="eyebrow">Opening your learning space</p>
        <h1>Loading your session</h1>
        <p class="page-lede">We are confirming your signed-in learning space.</p>
      </section>
    );
  }

  const needsSignIn = props.state.kind === "expired" || props.state.kind === "signedOut";
  const title =
    props.state.kind === "signedOut"
      ? "You are signed out"
      : props.state.kind === "expired"
        ? "Your session needs to be renewed"
        : "We could not restore your session";
  const description =
    props.state.kind === "signedOut"
      ? "This browser no longer has access to your account or course session."
      : props.state.kind === "expired"
        ? "Return to your sign-in page, then continue where you left off."
        : "Check your connection, then try opening your learning space again.";

  return (
    <section class="route-error" data-session-state={props.state.kind} aria-live="polite">
      <p class="eyebrow">Session recovery</p>
      <h1>{title}</h1>
      <p>{description}</p>
      <Show when={needsSignIn}>
        <p>
          <A href="/sign-in">Open sign-in</A>
        </p>
      </Show>
      <div class="action-row">
        <button class="primary-action" type="button" onClick={() => void props.retry()}>
          Try again
        </button>
      </div>
    </section>
  );
}

export function App(props: RouteSectionProps): JSX.Element {
  const location = useLocation();
  const navigate = useNavigate();
  const session = useSessionBootstrap();
  const [signOutBusy, setSignOutBusy] = createSignal(false);
  const [signOutError, setSignOutError] = createSignal("");
  let mainContent: HTMLElement | undefined;
  let previousPath = location.pathname;

  function focusMainContent(): void {
    mainContent?.focus();
  }

  async function signOut(): Promise<void> {
    setSignOutBusy(true);
    setSignOutError("");
    const confirmed = await session.signOut();
    setSignOutBusy(false);
    if (confirmed) {
      navigate("/sign-in");
    } else {
      setSignOutError("Sign-out could not be confirmed. Your session is still open; please retry.");
    }
  }

  createEffect(() => {
    const nextPath = location.pathname;
    if (nextPath === previousPath) {
      return;
    }
    previousPath = nextPath;
    queueMicrotask(focusMainContent);
  });

  return (
    <>
      <a class="skip-link" href="#main-content" onClick={() => queueMicrotask(focusMainContent)}>
        Skip to learning content
      </a>
      <header class="site-header">
        <A class="brand" href="/" aria-label="Peptidyle home">
          <span class="brand-mark" aria-hidden="true">
            P
          </span>
          <span>Peptidyle</span>
        </A>
        <nav aria-label="Primary navigation">
          <A href="/" activeClass="active" end>
            Courses
          </A>
          <Show when={canUseLibrary(session.state())}>
            <A href="/library" activeClass="active">
              Library
            </A>
          </Show>
          <Show when={canUseBlueprintCourses(session.state())}>
            <A href="/blueprint-courses" activeClass="active">
              Blueprint Courses
            </A>
          </Show>
          <Show when={canUseAuthoringTools(session.state())}>
            <A href="/workspace" activeClass="active">
              Workspace
            </A>
          </Show>
          <Show when={session.state().kind === "authenticated"}>
            <A href="/account/course-invitations" activeClass="active">
              Invitations
            </A>
            <button
              class="nav-action"
              type="button"
              disabled={signOutBusy()}
              onClick={() => void signOut()}
            >
              {signOutBusy() ? "Signing out..." : "Sign out"}
            </button>
          </Show>
        </nav>
        <span class="sr-only" role="status" aria-live="polite">
          {signOutError()}
        </span>
      </header>
      <main
        id="main-content"
        class="shell"
        tabindex="-1"
        ref={(element: HTMLElement) => {
          mainContent = element;
        }}
      >
        <Show when={location.pathname} keyed>
          {(pathname) => (
            <ErrorBoundary
              fallback={(_error, reset) => (
                <section class="route-error" role="alert">
                  <p class="eyebrow">This page needs another try</p>
                  <h1>The learning space is still available</h1>
                  <p>
                    The current page could not load. Your navigation and active Assignment Attempt remain
                    available.
                  </p>
                  <div class="action-row">
                    <button class="primary-action" type="button" onClick={reset}>
                      Try this page again
                    </button>
                    <A class="quiet-link" href="/">
                      Return to courses
                    </A>
                  </div>
                </section>
              )}
            >
              <div data-current-path={pathname}>
                <RouteContent {...props} pathname={pathname} />
              </div>
            </ErrorBoundary>
          )}
        </Show>
      </main>
    </>
  );
}
