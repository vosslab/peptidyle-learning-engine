// app.tsx - persistent application shell and route-level error boundary.

import { A, useLocation, type RouteSectionProps } from "@solidjs/router";
import { createEffect, ErrorBoundary, Show, type JSX } from "solid-js";

import { useSessionBootstrap, type SessionBootstrapState } from "./auth/session_context";

declare global {
  interface Window {
    /** Playwright-only one-shot fault injector for the route-boundary behavior gate. */
    __PLE_ROUTE_FAILURE_TEST__?: () => boolean;
    /** Test-server-only fixture transport switch, set before `main.tsx` loads. */
    __PLE_USE_MOCK_API__?: boolean;
  }
}

function canUseAuthoringTools(state: SessionBootstrapState): boolean {
  if (state.kind !== "authenticated") {
    return false;
  }
  return state.session.user.roles.some((role) =>
    ["instructor", "publisher", "administrator"].includes(role),
  );
}

function SessionContent(props: RouteSectionProps): JSX.Element {
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

function RouteContent(props: RouteSectionProps): JSX.Element {
  const testFailure = window.__PLE_ROUTE_FAILURE_TEST__;
  if (testFailure?.() === true) {
    throw new Error("route-boundary-test-sentinel");
  }
  return <SessionContent {...props} />;
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

  const expired = props.state.kind === "expired";
  const title = expired ? "Your session needs to be renewed" : "We could not restore your session";
  const description = expired
    ? "Return to your sign-in page, then continue where you left off."
    : "Check your connection, then try opening your learning space again.";

  return (
    <section class="route-error" data-session-state={props.state.kind} aria-live="polite">
      <p class="eyebrow">Session recovery</p>
      <h1>{title}</h1>
      <p>{description}</p>
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
  const session = useSessionBootstrap();
  let mainContent: HTMLElement | undefined;
  let previousPath = location.pathname;

  function focusMainContent(): void {
    mainContent?.focus();
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
          <A href="/library" activeClass="active">
            Library
          </A>
          <Show when={canUseAuthoringTools(session.state())}>
            <A href="/workspace" activeClass="active">
              Workspace
            </A>
          </Show>
        </nav>
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
                    The current page could not load. Your navigation and active run remain
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
                <RouteContent {...props} />
              </div>
            </ErrorBoundary>
          )}
        </Show>
      </main>
      <footer class="site-footer">
        <p>Practice for understanding, not memorization.</p>
      </footer>
    </>
  );
}
