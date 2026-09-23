// route_scope_context.tsx - stable presentation scope identity and cached route data.

import {
  createContext,
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

/** URL-syntax scope identity only; route access and service policy remain separate. */
export type RouteScopeIdentity = RouteScopeKey;

export interface RouteScopeProviderProps {
  readonly pathname: Accessor<string> | string;
  readonly children: JSX.Element;
}

interface RouteScopeContextValue {
  readonly identity: Accessor<RouteScopeIdentity>;
  readonly currentCourseInstanceId: Accessor<string | undefined>;
  readonly contentLayout: Accessor<ContentLayout>;
  readonly data: Accessor<CourseThemeRouteData | undefined>;
  readonly loadState: Accessor<RouteScopeLoadState>;
  readonly retry: () => void;
  readonly replaceCourseAppearance: ReplaceCourseAppearance;
  /** Current Assessment title supplied by its already-loaded direct workspace resource. */
  readonly assessmentTitle: Accessor<string | undefined>;
  /** Ignores delayed workspace results that no longer describe the visible pathname. */
  readonly setAssessmentTitleForPath: (pathname: string, title: string) => void;
}

const RouteScopeContext = createContext<RouteScopeContextValue>();

interface AssessmentTitleForPath {
  readonly pathname: string;
  readonly title: string;
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
  const [publishedAssessmentTitle, setPublishedAssessmentTitle] =
    createSignal<AssessmentTitleForPath>();
  const assessmentTitle = (): string | undefined => {
    const published = publishedAssessmentTitle();
    return published?.pathname === currentPathname() ? published.title : undefined;
  };
  const contentLayout = createMemo((): ContentLayout => {
    return routeContractForPathname(currentPathname())?.pageLayout ?? "reading";
  });

  function setAssessmentTitleForPath(pathname: string, title: string): void {
    // This presentation-only value comes from the existing workspace request.
    // A delayed response cannot replace the current path's title.
    if (currentPathname() === pathname) setPublishedAssessmentTitle({ pathname, title });
  }

  return (
    <RouteScopeContext.Provider
      value={{ ...controller, contentLayout, assessmentTitle, setAssessmentTitleForPath }}
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

/** Reads the session-pinned Course Instance for Student tier-one navigation. */
export function useCurrentCourseInstanceId(): Accessor<string | undefined> {
  return useRouteScopeContext().currentCourseInstanceId;
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

/** Reads the loaded Assessment title for the exact current workspace pathname. */
export function useAssessmentTitle(): Accessor<string | undefined> {
  return useRouteScopeContext().assessmentTitle;
}

/** Publishes an already-loaded Assessment title to the persistent shell. */
export function useSetAssessmentTitleForPath(): (pathname: string, title: string) => void {
  return useRouteScopeContext().setAssessmentTitleForPath;
}
