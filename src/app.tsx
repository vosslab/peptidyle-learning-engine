// app.tsx - persistent application shell and route-level error boundary.

import { A, useLocation, type RouteSectionProps } from "@solidjs/router";
import { ErrorBoundary, type JSX } from "solid-js";

export function App(props: RouteSectionProps): JSX.Element {
  const location = useLocation();
  return (
    <>
      <a class="skip-link" href="#main-content">
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
          <A href="/workspace" activeClass="active">
            Workspace
          </A>
        </nav>
      </header>
      <main id="main-content" class="shell" tabindex="-1">
        <ErrorBoundary
          fallback={(error, reset) => (
            <section class="route-error" role="alert">
              <p class="eyebrow">This page needs another try</p>
              <h1>The learning space is still available</h1>
              <p>
                The current page could not load. Your navigation and active run remain available.
              </p>
              <div class="action-row">
                <button class="primary-action" type="button" onClick={reset}>
                  Try this page again
                </button>
                <A class="quiet-link" href="/">
                  Return to courses
                </A>
              </div>
              <span class="visually-hidden">{String(error)}</span>
            </section>
          )}
        >
          <div data-current-path={location.pathname}>{props.children}</div>
        </ErrorBoundary>
      </main>
      <footer class="site-footer">
        <p>Practice for understanding, not memorization.</p>
      </footer>
    </>
  );
}
