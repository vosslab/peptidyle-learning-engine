// route_scope_provider_bundle.ts - load the real provider composition through the Solid compiler.

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import type { ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { RouteScopeProviderHarness } from "./route_scope_provider_harness";

interface RouteScopeProviderHarnessModule {
  readonly mountRouteScopeProviderHarness: (
    applicationApi: ApplicationApi<OrdinaryBrowserApiClient>,
    initialPathname: string,
  ) => RouteScopeProviderHarness;
}

function isRouteScopeProviderHarnessModule(
  value: unknown,
): value is RouteScopeProviderHarnessModule {
  return (
    typeof value === "object" &&
    value !== null &&
    "mountRouteScopeProviderHarness" in value &&
    typeof value.mountRouteScopeProviderHarness === "function"
  );
}

export async function loadRouteScopeProviderHarness(): Promise<RouteScopeProviderHarnessModule> {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("./route_scope_provider_harness.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "route_scope_provider_harness.js",
    // The harness must use Solid's browser runtime so owned effects execute.
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Route scope provider harness bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module: unknown = await import(`data:text/javascript;base64,${encoded}`);
  if (!isRouteScopeProviderHarnessModule(module))
    throw new Error("Route scope provider harness bundle has no mount function export.");
  return module;
}
