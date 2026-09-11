// application_shell.tsx - production-owned persistent Ribbon and content boundary.

import { A, useNavigate } from "@solidjs/router";
import {
  createEffect,
  createMemo,
  createSignal,
  ErrorBoundary,
  For,
  Show,
  type Accessor,
  type JSX,
} from "solid-js";

import { useSessionBootstrap } from "./auth/session_context";
import type { CourseThemeRouteData } from "./features/course_appearance/course_theme_context";
import { CourseThemeVariables } from "./features/course_appearance/course_theme_variables";
import { AppRibbon } from "./ribbon/app_ribbon";
import type { RibbonBreadcrumbModel, RibbonModel } from "./ribbon/ribbon_contract";
import { RouteScopeProvider, useRouteScopeData } from "./ribbon/route_scope_context";

export interface ApplicationShellProps {
  readonly pathname: Accessor<string>;
  readonly ribbonModel: (routeData: CourseThemeRouteData | undefined) => RibbonModel | undefined;
  readonly content: (pathname: string) => JSX.Element;
}

function isRibbonSignOutAction(
  value: unknown,
): value is { readonly id: "signOut"; readonly kind: "action" } {
  if (value === null || typeof value !== "object" || !("id" in value) || !("kind" in value)) {
    return false;
  }
  return value.id === "signOut" && value.kind === "action";
}

interface ContentErrorProps {
  readonly reset: () => void;
}

function BreadcrumbPrelude(props: { readonly model: RibbonModel | undefined }): JSX.Element {
  const breadcrumbs = (): ReadonlyArray<RibbonBreadcrumbModel> => props.model?.breadcrumbs ?? [];
  const reserved = (): boolean => props.model?.breadcrumbPreludeReserved === true;
  return (
    <Show when={reserved()}>
      <div class="ple-shell__breadcrumb-prelude" aria-live="polite">
        <Show
          when={breadcrumbs().length > 0}
          fallback={<span class="sr-only">Loading location</span>}
        >
          <nav aria-label="Breadcrumb">
            <ol>
              <For each={breadcrumbs()}>
                {(breadcrumb) => (
                  <li>
                    <Show
                      when={breadcrumb.href}
                      fallback={
                        <span aria-current={breadcrumb.current ? "page" : undefined}>
                          {breadcrumb.label}
                        </span>
                      }
                    >
                      {(href) => <A href={href()}>{breadcrumb.label}</A>}
                    </Show>
                  </li>
                )}
              </For>
            </ol>
          </nav>
        </Show>
      </div>
    </Show>
  );
}

function ContentError(props: ContentErrorProps): JSX.Element {
  return (
    <section class="route-error" role="alert">
      <p class="eyebrow">This page needs another try</p>
      <h1>The learning space is still available</h1>
      <p>
        The current page could not load. Your navigation and active Assignment Attempt remain
        available.
      </p>
      <div class="action-row">
        <button class="primary-action" type="button" onClick={props.reset}>
          Try this page again
        </button>
        <A class="quiet-link" href="/">
          Return to courses
        </A>
      </div>
    </section>
  );
}

/**
 * The stable application composition. App supplies the only production Ribbon
 * model source; structural evidence may supply an explicit fixture source
 * without changing route admission, authorization, or the capability registry.
 */
export function ApplicationShell(props: ApplicationShellProps): JSX.Element {
  const navigate = useNavigate();
  const session = useSessionBootstrap();
  const [signOutBusy, setSignOutBusy] = createSignal(false);
  const [signOutError, setSignOutError] = createSignal("");
  let mainContent: HTMLElement | undefined;
  let previousPath = props.pathname();

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
      return;
    }
    setSignOutError("Sign-out could not be confirmed. Your session is still open; please retry.");
  }

  function handleRibbonAction(event: Event): void {
    if (!(event instanceof CustomEvent)) return;
    if (!isRibbonSignOutAction(event.detail) || signOutBusy()) return;
    void signOut();
  }

  createEffect(() => {
    const nextPath = props.pathname();
    if (nextPath === previousPath) return;
    previousPath = nextPath;
    queueMicrotask(focusMainContent);
  });

  function ShellInterior(): JSX.Element {
    // Read route scope data from a memo owned by the persistent shell. Calling
    // the hook at component construction would capture its initial (often
    // unresolved) value and prevent later cache resolution from reaching the
    // Ribbon model.
    const routeData = useRouteScopeData();
    const ribbonModel = createMemo(() => props.ribbonModel(routeData()));
    const ribbonTaskRow = createMemo(() => {
      const model = ribbonModel();
      if (model === undefined) return undefined;
      // ribbon_contract.ts derives taskAreas from declared taskGroup topology,
      // not control admission, so this parent geometry stays admission-independent.
      return model.taskAreas.length > 0 ? "reserved" : "absent";
    });
    function ContentRegion(): JSX.Element {
      return (
        <main class="shell">
          <BreadcrumbPrelude model={ribbonModel()} />
          <section
            id="main-content"
            tabindex="-1"
            ref={(element: HTMLElement) => {
              mainContent = element;
            }}
          >
            <Show when={props.pathname()} keyed>
              {(currentPathname) => (
                <ErrorBoundary fallback={(_error, reset) => <ContentError reset={reset} />}>
                  <div data-current-path={currentPathname}>{props.content(currentPathname)}</div>
                </ErrorBoundary>
              )}
            </Show>
          </section>
        </main>
      );
    }

    return (
      <CourseThemeVariables>
        <a class="skip-link" href="#main-content" onClick={() => queueMicrotask(focusMainContent)}>
          Skip to learning content
        </a>
        <span class="sr-only" role="status" aria-live="polite">
          {signOutError()}
        </span>
        <div
          classList={{
            "ple-shell-frame": true,
            "ple-ribbon-shell-grid": ribbonModel() !== undefined,
          }}
          data-ribbon-task-row={ribbonTaskRow()}
        >
          <Show
            when={ribbonModel()}
            fallback={
              <header class="site-header">
                <A class="brand" href="/" aria-label="Peptidyle home">
                  <span class="brand-mark" aria-hidden="true">
                    P
                  </span>
                  <span>Peptidyle</span>
                </A>
              </header>
            }
          >
            {(model) => (
              <div on:ple-ribbon-action={handleRibbonAction}>
                <AppRibbon model={model()} />
              </div>
            )}
          </Show>
          <ContentRegion />
        </div>
      </CourseThemeVariables>
    );
  }

  return (
    <RouteScopeProvider pathname={props.pathname}>
      <ShellInterior />
    </RouteScopeProvider>
  );
}
