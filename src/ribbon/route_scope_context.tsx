// route_scope_context.tsx - stable presentation scope identity and cached route data.

import { createContext, useContext, type Accessor, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { CourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { createRouteScopeController } from "./route_scope_controller";
import type { RouteScopeKey } from "../navigation/route_params";

/** URL-syntax scope identity only; route access and service policy remain separate. */
export type RouteScopeIdentity = RouteScopeKey;

export interface RouteScopeProviderProps {
  readonly pathname: Accessor<string> | string;
  readonly children: JSX.Element;
}

interface RouteScopeContextValue {
  readonly identity: Accessor<RouteScopeIdentity>;
  readonly data: Accessor<CourseThemeRouteData | undefined>;
}

const RouteScopeContext = createContext<RouteScopeContextValue>();

/**
 * Holds presentation caches across route transitions outside keyed content;
 * this component itself renders no loading or fallback chrome.
 */
export function RouteScopeProvider(props: RouteScopeProviderProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const controller = createRouteScopeController(props.pathname, applicationApi.queries);
  return (
    <RouteScopeContext.Provider value={controller}>{props.children}</RouteScopeContext.Provider>
  );
}

function useRouteScopeContext(): RouteScopeContextValue {
  const context = useContext(RouteScopeContext);
  if (context === undefined)
    throw new Error("RouteScopeProvider is missing from the application shell");
  return context;
}

/** Reads the current synchronous URL-syntax identity; it does not authorize a route. */
export function useRouteScopeIdentity(): RouteScopeIdentity {
  return useRouteScopeContext().identity();
}

/**
 * Returns the cached route-data accessor. Consumers must read it from a Solid
 * reactive boundary so a deferred scope can resolve without remounting shell
 * chrome or capturing the initial `undefined` projection.
 */
export function useRouteScopeData(): Accessor<CourseThemeRouteData | undefined> {
  return useRouteScopeContext().data;
}
