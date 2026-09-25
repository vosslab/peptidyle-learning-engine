// application_shell.tsx - production-owned persistent Ribbon and content boundary.

import { A, useNavigate } from "@solidjs/router";
import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  ErrorBoundary,
  For,
  onCleanup,
  Show,
  type Accessor,
  type JSX,
} from "solid-js";

import { useApplicationApi } from "./api/application_api";
import { useSessionBootstrap } from "./auth/session_context";
import { PageFrame } from "./components/page_frame";
import type { CourseThemeRouteData } from "./features/course_appearance/course_theme_context";
import { CourseThemeVariables } from "./features/course_appearance/course_theme_variables";
import { AvatarVisual } from "./features/profile_avatar/provided_avatar_picker";
import { RibbonAccountAvatar } from "./features/profile_avatar/ribbon_account_avatar";
import { AppRibbon, SignedOutRibbonAvatar } from "./ribbon/app_ribbon";
import { routeContractForPathname } from "./route_contract";
import type { RibbonBreadcrumbModel, RibbonModel } from "./ribbon/ribbon_contract";
import {
  loadStudentRibbonNavigation,
  type StudentRibbonNavigation,
} from "./ribbon/student_ribbon_navigation";
import {
  RouteScopeProvider,
  useAssessmentTitle,
  useRouteScopeData,
} from "./ribbon/route_scope_context";

export interface ApplicationShellProps {
  readonly pathname: Accessor<string>;
  readonly ribbonModel: (
    routeData: CourseThemeRouteData | undefined,
    assessmentTitle: string | undefined,
    studentNavigation: StudentRibbonNavigation | undefined,
  ) => RibbonModel | undefined;
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

function courseBreadcrumbNeedsCompactLabel(nav: HTMLElement): boolean {
  const list = nav.querySelector("ol");
  const visibleLabel = nav.querySelector("[data-course-breadcrumb-visible]");
  const measuredLabel = nav.querySelector("[data-course-breadcrumb-measure]");
  if (
    !(list instanceof HTMLOListElement) ||
    !(visibleLabel instanceof HTMLElement) ||
    !(measuredLabel instanceof HTMLElement)
  ) {
    return false;
  }
  const fullTrailWidth =
    list.getBoundingClientRect().width +
    measuredLabel.getBoundingClientRect().width -
    visibleLabel.getBoundingClientRect().width;
  return fullTrailWidth > nav.clientWidth;
}

interface ContentErrorProps {
  readonly reset: () => void;
}

function BreadcrumbPrelude(props: { readonly model: RibbonModel | undefined }): JSX.Element {
  const breadcrumbs = (): ReadonlyArray<RibbonBreadcrumbModel> => props.model?.breadcrumbs ?? [];
  const [trail, setTrail] = createSignal<HTMLElement>();
  const [compactCourse, setCompactCourse] = createSignal(false);
  const location = createMemo(() =>
    JSON.stringify(
      breadcrumbs().map(({ label, compactLabel, href, current }) => [
        label,
        compactLabel,
        href,
        current,
      ]),
    ),
  );
  let focusRevision = 0;
  createEffect(() => {
    const element = trail();
    location();
    if (element === undefined) return;
    const observer = new ResizeObserver(() => {
      setCompactCourse(courseBreadcrumbNeedsCompactLabel(element));
      if (!element.contains(document.activeElement)) element.scrollLeft = 0;
    });
    observer.observe(element);
    const list = element.querySelector("ol");
    if (list !== null) observer.observe(list);
    const measuredLabel = element.querySelector("[data-course-breadcrumb-measure]");
    if (measuredLabel instanceof HTMLElement) observer.observe(measuredLabel);
    onCleanup(() => {
      observer.disconnect();
    });
  });
  createEffect(() => {
    const element = trail();
    const currentLocation = location();
    if (element === undefined || currentLocation === "[]") return;
    const focusVersion = focusRevision;
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    // Keep the recognizable Home/Course prefix in the resting state. Keyboard
    // focus still scrolls an individual link into view when needed.
    queueMicrotask(() => {
      if (!cancelled && focusVersion === focusRevision && trail() === element) {
        element.scrollLeft = 0;
      }
    });
  });
  return (
    <Show when={props.model !== undefined}>
      <div
        class="ple-shell__breadcrumb-prelude"
        aria-live="polite"
        data-product-role={props.model?.context.productLabel.toLowerCase()}
      >
        <Show
          when={breadcrumbs().length > 0}
          fallback={<span class="sr-only">Loading location</span>}
        >
          <nav
            aria-label="Breadcrumb"
            ref={setTrail}
            onFocusIn={(event) => {
              if (!(event.target instanceof HTMLAnchorElement)) return;
              focusRevision += 1;
              const viewport = event.currentTarget.getBoundingClientRect();
              const link = event.target.getBoundingClientRect();
              const delta =
                link.left < viewport.left
                  ? link.left - viewport.left
                  : link.right > viewport.right
                    ? link.right - viewport.right
                    : 0;
              event.currentTarget.scrollLeft += delta;
            }}
          >
            <ol>
              <For each={breadcrumbs()}>
                {(breadcrumb) => {
                  const hasCompactLabel = breadcrumb.compactLabel !== undefined;
                  return (
                    <li>
                      <A
                        href={breadcrumb.href}
                        end
                        aria-current={breadcrumb.current ? "page" : undefined}
                      >
                        <span data-course-breadcrumb-visible={hasCompactLabel ? "true" : undefined}>
                          {hasCompactLabel && compactCourse()
                            ? breadcrumb.compactLabel
                            : breadcrumb.label}
                        </span>
                      </A>
                      <Show when={hasCompactLabel}>
                        <span
                          aria-hidden="true"
                          class="ple-shell__breadcrumb-measure"
                          data-course-breadcrumb-measure
                        >
                          {breadcrumb.label}
                        </span>
                      </Show>
                    </li>
                  );
                }}
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
    <div role="alert">
      <PageFrame
        routeSurface="contentError"
        headingId="shell-content-error-heading"
        eyebrow="This page needs another try"
        title="The learning space is still available"
        lede="The current page could not load. Your navigation and active Attempt remain available."
        actions={
          <>
            <button class="primary-action" type="button" onClick={props.reset}>
              Try this page again
            </button>
            <A class="quiet-link" href="/">
              Return to courses
            </A>
          </>
        }
      />
    </div>
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
    const applicationApi = useApplicationApi();
    // Read route scope data from a memo owned by the persistent shell. Calling
    // the hook at component construction would capture its initial (often
    // unresolved) value and prevent later cache resolution from reaching the
    // Ribbon model.
    const routeData = useRouteScopeData();
    const assessmentTitle = useAssessmentTitle();
    const studentNavigationPath = createMemo(() => {
      const identity = session.state();
      if (identity.kind !== "authenticated" || identity.session.account.productRole !== "student") {
        return undefined;
      }
      const route = routeContractForPathname(props.pathname());
      if (
        route?.requiredProductRoles.includes("student") !== true ||
        !["coursework", "grades", "courses"].includes(route.ribbon.tierOneArea)
      ) {
        return undefined;
      }
      return props.pathname();
    });
    const [studentNavigationLookup] = createResource(studentNavigationPath, () =>
      loadStudentRibbonNavigation(applicationApi.client),
    );
    const studentNavigation = createMemo(() =>
      studentNavigationPath() === undefined ? undefined : studentNavigationLookup(),
    );
    const ribbonModel = createMemo(() =>
      props.ribbonModel(routeData(), assessmentTitle(), studentNavigation()),
    );
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
          data-ribbon-product-role={ribbonModel()?.context.productLabel.toLowerCase()}
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
                <Show
                  when={session.state().kind === "authenticated"}
                  fallback={<SignedOutRibbonAvatar />}
                >
                  <A
                    class="ple-app-ribbon__profile"
                    href="/profile"
                    aria-label="Profile"
                    title="Profile"
                  >
                    <RibbonAccountAvatar
                      client={applicationApi.client}
                      renderProvidedAvatar={(providedAvatarId) => (
                        <AvatarVisual avatarId={providedAvatarId} decorative size={24} />
                      )}
                    />
                  </A>
                </Show>
              </header>
            }
          >
            {(model) => (
              <div on:ple-ribbon-action={handleRibbonAction}>
                <AppRibbon
                  model={model()}
                  renderProfileAvatar={(): JSX.Element => (
                    <RibbonAccountAvatar
                      client={applicationApi.client}
                      renderProvidedAvatar={(providedAvatarId) => (
                        <AvatarVisual avatarId={providedAvatarId} decorative size={24} />
                      )}
                    />
                  )}
                />
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
