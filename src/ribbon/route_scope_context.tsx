// route_scope_context.tsx - stable presentation scope identity and cached route data.

import {
  createContext,
  createEffect,
  createMemo,
  createSignal,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type {
  CourseThemeRouteData,
  ReplaceCourseAppearance,
} from "../features/course_appearance/course_theme_context";
import { createRouteScopeController, type RouteScopeLoadState } from "./route_scope_controller";
import type { RouteScopeKey } from "../navigation/route_params";
import { routeContractForPathname, type ContentLayout } from "../route_contract";
import type { RibbonContextLabels, RibbonContextNavigation } from "./ribbon_contract";

/** URL-syntax scope identity only; route access and service policy remain separate. */
export type RouteScopeIdentity = RouteScopeKey;

export interface RouteScopeProviderProps {
  readonly pathname: Accessor<string> | string;
  /** Changes whenever the authenticated browser session crosses a safe boundary. */
  readonly sessionBoundary?: Accessor<unknown>;
  readonly children: JSX.Element;
}

/** Opaque current-route proof required when publishing an already loaded display label. */
export interface RouteScopePublication {
  readonly pathname: string;
  readonly routeGeneration: number;
  readonly sessionGeneration: number;
}

interface RouteScopeContextValue {
  readonly identity: Accessor<RouteScopeIdentity>;
  readonly contentLayout: Accessor<ContentLayout>;
  readonly data: Accessor<CourseThemeRouteData | undefined>;
  readonly loadState: Accessor<RouteScopeLoadState>;
  readonly retry: () => void;
  readonly replaceCourseAppearance: ReplaceCourseAppearance;
  /** Display-only labels supplied by already authorized, direct resource loads. */
  readonly labels: Accessor<RibbonContextLabels>;
  /** Validated link state supplied by the exact current route. */
  readonly navigation: Accessor<RibbonContextNavigation>;
  /** Current route/session proof for a content-owned publication. */
  readonly publication: Accessor<RouteScopePublication>;
  /** Rejects stale route/session writes and replaces the labels for the exact current route. */
  readonly publishLabels: (publication: RouteScopePublication, labels: RibbonContextLabels) => void;
  /** Rejects stale route/session writes and replaces navigation state for the exact current route. */
  readonly publishNavigation: (
    publication: RouteScopePublication,
    navigation: RibbonContextNavigation,
  ) => void;
  /** Clears only labels owned by the exact current route/session publication. */
  readonly clearLabels: (publication: RouteScopePublication) => void;
}

const RouteScopeContext = createContext<RouteScopeContextValue>();

interface PublishedRouteLabels {
  readonly publication: RouteScopePublication;
  readonly labels: RibbonContextLabels;
}

interface PublishedRouteNavigation {
  readonly publication: RouteScopePublication;
  readonly navigation: RibbonContextNavigation;
}

/**
 * Holds presentation caches across route transitions outside keyed content;
 * this component itself renders no loading or fallback chrome.
 */
export function RouteScopeProvider(props: RouteScopeProviderProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const controller = createRouteScopeController(props.pathname, applicationApi.queries);
  const pathname = props.pathname;
  const currentPathname = typeof pathname === "function" ? pathname : (): string => pathname;
  const sessionBoundary: Accessor<unknown> =
    props.sessionBoundary ??
    function noSessionBoundary(): undefined {
      return undefined;
    };
  const [routeGeneration, setRouteGeneration] = createSignal(0);
  const [sessionGeneration, setSessionGeneration] = createSignal(0);
  const [publishedLabels, setPublishedLabels] = createSignal<PublishedRouteLabels>();
  const [publishedNavigation, setPublishedNavigation] = createSignal<PublishedRouteNavigation>();
  let previousPathname: string | undefined;
  let previousSessionBoundary: unknown = Symbol("initial-route-scope-session-boundary");

  createEffect(() => {
    const nextPathname = currentPathname();
    if (nextPathname === previousPathname) return;
    previousPathname = nextPathname;
    setRouteGeneration((current) => current + 1);
    setPublishedLabels(undefined);
    setPublishedNavigation(undefined);
  });

  createEffect(() => {
    const nextSessionBoundary = sessionBoundary();
    if (nextSessionBoundary === previousSessionBoundary) return;
    previousSessionBoundary = nextSessionBoundary;
    setSessionGeneration((current) => current + 1);
    setPublishedLabels(undefined);
    setPublishedNavigation(undefined);
  });

  const publication = createMemo((): RouteScopePublication => {
    return Object.freeze({
      pathname: currentPathname(),
      routeGeneration: routeGeneration(),
      sessionGeneration: sessionGeneration(),
    });
  });
  const labels = (): RibbonContextLabels => {
    const published = publishedLabels();
    return published === undefined || !isCurrentPublication(published.publication)
      ? {}
      : published.labels;
  };
  const navigation = (): RibbonContextNavigation => {
    const published = publishedNavigation();
    return published === undefined || !isCurrentPublication(published.publication)
      ? {}
      : published.navigation;
  };
  const contentLayout = createMemo((): ContentLayout => {
    return routeContractForPathname(currentPathname())?.pageLayout ?? "reading";
  });

  function isCurrentPublication(candidate: RouteScopePublication): boolean {
    const current = publication();
    return (
      candidate.pathname === current.pathname &&
      candidate.routeGeneration === current.routeGeneration &&
      candidate.sessionGeneration === current.sessionGeneration
    );
  }

  function publishLabels(candidate: RouteScopePublication, nextLabels: RibbonContextLabels): void {
    if (!isCurrentPublication(candidate)) return;
    setPublishedLabels({ publication: candidate, labels: Object.freeze({ ...nextLabels }) });
  }

  function publishNavigation(
    candidate: RouteScopePublication,
    nextNavigation: RibbonContextNavigation,
  ): void {
    if (!isCurrentPublication(candidate)) return;
    setPublishedNavigation({
      publication: candidate,
      navigation: Object.freeze({ ...nextNavigation }),
    });
  }

  function clearLabels(candidate: RouteScopePublication): void {
    const published = publishedLabels();
    if (!isCurrentPublication(candidate)) return;
    if (published?.publication === candidate) setPublishedLabels(undefined);
    const publishedRouteNavigation = publishedNavigation();
    if (publishedRouteNavigation?.publication === candidate) setPublishedNavigation(undefined);
  }

  return (
    <RouteScopeContext.Provider
      value={{
        ...controller,
        contentLayout,
        labels,
        navigation,
        publication,
        publishLabels,
        publishNavigation,
        clearLabels,
      }}
    >
      {props.children}
    </RouteScopeContext.Provider>
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

/** Reads the current route's declared content geometry through the persistent shell context. */
export function useRouteContentLayout(): Accessor<ContentLayout> {
  return useRouteScopeContext().contentLayout;
}

/**
 * Returns the cached route-data accessor. Consumers must read it from a Solid
 * reactive boundary so a deferred scope can resolve without remounting shell
 * chrome or capturing the initial `undefined` projection.
 */
export function useRouteScopeData(): Accessor<CourseThemeRouteData | undefined> {
  return useRouteScopeContext().data;
}

/** Reads the current request state for content-owned loading and recovery copy. */
export function useRouteScopeLoadState(): Accessor<RouteScopeLoadState> {
  return useRouteScopeContext().loadState;
}

/** Retries only the current public route scope after a request failure. */
export function useRetryRouteScope(): () => void {
  return useRouteScopeContext().retry;
}

/** Updates the in-memory saved appearance returned by an authorized mutation. */
export function useReplaceCourseAppearance(): ReplaceCourseAppearance {
  return useRouteScopeContext().replaceCourseAppearance;
}

/** Reads labels that belong to the exact current route and session generation. */
export function useRouteScopeLabels(): Accessor<RibbonContextLabels> {
  return useRouteScopeContext().labels;
}

/** Reads validated navigation state owned by the exact current route and session generation. */
export function useRouteScopeNavigation(): Accessor<RibbonContextNavigation> {
  return useRouteScopeContext().navigation;
}

/** Captures the current route/session generation before a content-owned async load begins. */
export function useRouteScopePublication(): Accessor<RouteScopePublication> {
  return useRouteScopeContext().publication;
}

/** Publishes display-only labels from an existing authorized resource load. */
export function usePublishRouteScopeLabels(): (
  publication: RouteScopePublication,
  labels: RibbonContextLabels,
) => void {
  return useRouteScopeContext().publishLabels;
}

/** Publishes validated link state from an existing authorized direct resource load. */
export function usePublishRouteScopeNavigation(): (
  publication: RouteScopePublication,
  navigation: RibbonContextNavigation,
) => void {
  return useRouteScopeContext().publishNavigation;
}

/** Clears labels after a content-owned load failure or when its content is disposed. */
export function useClearRouteScopeLabels(): (publication: RouteScopePublication) => void {
  return useRouteScopeContext().clearLabels;
}
