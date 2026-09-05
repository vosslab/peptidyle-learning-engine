// route_scope_provider_harness.tsx - compiled Solid composition for scope-provider evidence.

import { createComponent, createEffect, createRoot, createSignal } from "solid-js";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import {
  RouteScopeProvider,
  useRouteScopeData,
  useRouteScopeIdentity,
} from "../../src/ribbon/route_scope_context";

export interface RouteScopeProviderHarness {
  readonly navigate: (pathname: string) => void;
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
  let latest: unknown;
  let mounts = 0;
  createRoot((disposeRoot) => {
    dispose = disposeRoot;
    const [pathname, setPathnameSignal] = createSignal(initialPathname);
    setPathname = setPathnameSignal;
    const Consumer = (): null => {
      const data = useRouteScopeData();
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
          get children() {
            return createComponent(Consumer, {});
          },
        });
      },
    });
    mounts += 1;
  });
  return { navigate: setPathname, latest: () => latest, mounts: () => mounts, dispose };
}
