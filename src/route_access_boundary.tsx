// route_access_boundary.tsx - fail-closed role boundary for executable product routes.

import { A, useLocation } from "@solidjs/router";
import { createMemo, onMount, Show, type Component, type JSX } from "solid-js";

import { useSessionBootstrap } from "./auth/session_context";
import { PageFrame } from "./components/page_frame";
import {
  productRoleMayAccessRoute,
  routeContractForPathname,
  type RouteContract,
} from "./route_contract";

interface RouteAccessDeniedProps {
  readonly route: RouteContract;
}

function RouteAccessDenied(props: RouteAccessDeniedProps): JSX.Element {
  onMount(() => {
    queueMicrotask(() => document.getElementById("route-access-denied-heading")?.focus());
  });
  return (
    <div role="alert" aria-atomic="true">
      <PageFrame
        contentClass="route-error"
        routeSurface="routeAccessDenied"
        deniedRoute={props.route.id}
        headingId="route-access-denied-heading"
        headingTabIndex={-1}
        eyebrow="Account tools"
        title="This page is not available to this account"
        lede="Your available account tools remain available."
      >
        <A class="primary-link" href="/">
          Return to courses
        </A>
      </PageFrame>
    </div>
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
