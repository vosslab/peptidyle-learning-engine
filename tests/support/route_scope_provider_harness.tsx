// route_scope_provider_harness.tsx - compiled Solid composition for scope-provider evidence.

import { createComponent, createEffect, createRoot, createSignal } from "solid-js";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import {
  RouteScopeProvider,
  useClearRouteScopeLabels,
  useRouteScopeData,
  useRouteScopeIdentity,
  useRouteScopeLabels,
  useRouteScopePublication,
  usePublishRouteScopeLabels,
  type RouteScopePublication,
} from "../../src/ribbon/route_scope_context";
import type { RibbonContextLabels } from "../../src/ribbon/ribbon_contract";

export interface RouteScopeProviderHarness {
  readonly navigate: (pathname: string) => void;
  readonly advanceSession: () => void;
  readonly capturePublication: () => RouteScopePublication;
  readonly publishLabels: (publication: RouteScopePublication, labels: RibbonContextLabels) => void;
  readonly clearLabels: (publication: RouteScopePublication) => void;
  readonly labels: () => RibbonContextLabels;
  readonly latest: () => unknown;
  readonly mounts: () => number;
  readonly dispose: () => void;
}

/** Runs real nested providers with a reactive pathname signal and no application chrome. */
export function mountRouteScopeProviderHarness(
  applicationApi: ApplicationApi<OrdinaryBrowserApiClient>,
  initialPathname: string,
): RouteScopeProviderHarness {
  let dispose: () => void = () => undefined;
  let setPathname: (pathname: string) => void = () => undefined;
  let advanceSession: () => void = () => undefined;
  let capturePublication: () => RouteScopePublication = () => {
    throw new Error("Route scope provider harness is not mounted.");
  };
  let publishLabels: (
    publication: RouteScopePublication,
    labels: RibbonContextLabels,
  ) => void = () => undefined;
  let clearLabels: (publication: RouteScopePublication) => void = () => undefined;
  let currentLabels: () => RibbonContextLabels = () => ({});
  let latest: unknown;
  let mounts = 0;
  createRoot((disposeRoot) => {
    dispose = disposeRoot;
    const [pathname, setPathnameSignal] = createSignal(initialPathname);
    const [sessionGeneration, setSessionGeneration] = createSignal(0);
    setPathname = setPathnameSignal;
    advanceSession = (): void => {
      setSessionGeneration((generation) => generation + 1);
    };
    const Consumer = (): null => {
      const data = useRouteScopeData();
      const labels = useRouteScopeLabels();
      const publication = useRouteScopePublication();
      const publishRouteLabels = usePublishRouteScopeLabels();
      const clear = useClearRouteScopeLabels();
      capturePublication = publication;
      publishLabels = publishRouteLabels;
      clearLabels = clear;
      currentLabels = labels;
      const publish = (): void => {
        latest = { identity: useRouteScopeIdentity(), data: data() };
      };
      publish();
      createEffect(() => {
        publish();
      });
      return null;
    };
    createComponent(ApplicationApiProvider, {
      applicationApi,
      get children() {
        return createComponent(RouteScopeProvider, {
          pathname,
          sessionBoundary: sessionGeneration,
          get children() {
            return createComponent(Consumer, {});
          },
        });
      },
    });
    mounts += 1;
  });
  return {
    navigate: setPathname,
    advanceSession,
    capturePublication,
    publishLabels,
    clearLabels,
    labels: currentLabels,
    latest: () => latest,
    mounts: () => mounts,
    dispose,
  };
}
