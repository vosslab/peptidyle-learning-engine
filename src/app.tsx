// app.tsx - persistent application shell and route-level error boundary.

import { A, useLocation, type RouteSectionProps } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { ApplicationShell } from "./application_shell";
import { useSessionBootstrap, type SessionBootstrapState } from "./auth/session_context";
import {
  courseRouteView,
  type CourseThemeRouteData,
} from "./features/course_appearance/course_theme_context";
import { routeParams } from "./navigation/route_params";
import { routeContractForPathname } from "./route_contract";
import {
  deriveRibbonModel,
  type RibbonContextLabels,
  type RibbonModel,
} from "./ribbon/ribbon_contract";

type ScopedRouteSectionProps = RouteSectionProps & { readonly pathname: string };

function isPublicAccountRoute(pathname: string): boolean {
  const routeId = routeContractForPathname(pathname)?.id;
  return routeId === "signIn";
}

function accountLabelFor(
  state: Extract<SessionBootstrapState, { readonly kind: "authenticated" }>,
): string {
  switch (state.session.account.productRole) {
    case "student":
      return "Student account";
    case "instructor":
      return "Instructor account";
    case "sysadmin":
      return "System administrator";
  }
}

function ribbonLabelsFor(
  state: Extract<SessionBootstrapState, { readonly kind: "authenticated" }>,
  routeData: CourseThemeRouteData | undefined,
): RibbonContextLabels {
  if (routeData === undefined) return { accountLabel: accountLabelFor(state) };

  const courseTitle = courseRouteView(routeData).summary.title;
  if (routeData.kind !== "assignmentAttempt") {
    return { accountLabel: accountLabelFor(state), courseTitle };
  }
  const attemptNumber = routeData.screen.assignmentAttempt.attemptNumber;
  return {
    accountLabel: accountLabelFor(state),
    courseTitle,
    assignmentAttemptTitle: routeData.screen.assignment.title,
    assignmentAttemptProgress: `Attempt ${String(attemptNumber)}`,
  };
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
  const session = useSessionBootstrap();
  const pathname = (): string => location.pathname;
  function ribbonModel(routeData: CourseThemeRouteData | undefined): RibbonModel | undefined {
    const currentPathname = pathname();
    if (isPublicAccountRoute(currentPathname)) return undefined;

    const route = routeContractForPathname(currentPathname);
    const state = session.state();
    if (route === undefined || state.kind !== "authenticated") return undefined;
    // `routeParams` deliberately preserves raw declared segments. That lets a
    // matched malformed scoped URL retain its declared Ribbon schema while the
    // scope provider rejects resolution. `deriveRibbonModel` cannot turn those
    // raw values into navigation URLs, so the model remains data-free and all
    // affected route controls stay unavailable.
    const params = routeParams(route, currentPathname);
    if (params === undefined) return undefined;
    return deriveRibbonModel(
      { route, params },
      { productRole: state.session.account.productRole },
      ribbonLabelsFor(state, routeData),
    );
  }

  return (
    <ApplicationShell
      pathname={pathname}
      ribbonModel={ribbonModel}
      content={(currentPathname) => <RouteContent {...props} pathname={currentPathname} />}
    />
  );
}
