// route_access_boundary.tsx - fail-closed role boundary for executable product routes.

import { A, useLocation } from "@solidjs/router";
import { createMemo, onMount, Show, type Component, type JSX } from "solid-js";

import { useSessionBootstrap } from "./auth/session_context";
import {
  productRoleMayAccessRoute,
  routeContractForPathname,
  type RouteContract,
} from "./route_contract";

interface RouteAccessDeniedProps {
  readonly route: RouteContract;
}

function RouteAccessDenied(props: RouteAccessDeniedProps): JSX.Element {
  let heading: HTMLHeadingElement | undefined;
  onMount(() => {
    queueMicrotask(() => heading?.focus());
  });
  return (
    <section
      class="page route-error"
      data-route-surface="routeAccessDenied"
      data-denied-route={props.route.id}
      role="alert"
      aria-atomic="true"
    >
      <p class="eyebrow">Instructor tools</p>
      <h1
        tabindex="-1"
        ref={(element: HTMLHeadingElement) => {
          heading = element;
        }}
      >
        This page is available to instructors only
      </h1>
      <p>Your available account tools remain available.</p>
      <A class="primary-link" href="/">
        Return to courses
      </A>
    </section>
  );
}

/** Wraps one route component in the role policy declared by that route's contract row. */
export function withRouteAccessBoundary(
  route: RouteContract,
  ProtectedComponent: Component,
): Component {
  return function RouteAccessBoundary(): JSX.Element {
    const location = useLocation();
    const session = useSessionBootstrap();
    const accessGranted = createMemo((): boolean => {
      const matchedRoute = routeContractForPathname(location.pathname);
      if (matchedRoute?.id !== route.id) {
        return false;
      }
      if (route.requiredProductRoles.length === 0) {
        return true;
      }
      const state = session.state();
      if (state.kind !== "authenticated") {
        return false;
      }
      return productRoleMayAccessRoute(route.id, state.session.account.productRole);
    });
    return (
      <Show when={accessGranted()} fallback={<RouteAccessDenied route={route} />}>
        <ProtectedComponent />
      </Show>
    );
  };
}
