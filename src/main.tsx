// main.tsx - browser entry point (MOD-UI-SHELL, WP-F3).
//
// Solid's `render` takes a component *function*, not an element. Passing JSX
// directly evaluates it once, outside a reactive root, which is how a Solid app
// ends up rendering a static snapshot that never updates.

import { render } from "solid-js/web";
import { Router } from "@solidjs/router";

import { createMockApiClient } from "./api/mock/client";
import { createHttpApiClient } from "./api/http_client";
import { ApiRuntimeProvider, createApiRuntime } from "./api/runtime";
import { App } from "./app";
import { localCredentialLogin } from "./auth/local_development";
import { SessionProvider } from "./auth/session_context";
import { log } from "./log";
import { appRoutes, notFoundRoute } from "./routes";
import { WasmRuntimeProvider } from "./wasm/context";

const mountPoint = document.getElementById("root");

if (mountPoint === null) {
  // Failing loudly beats rendering into a detached node and showing a blank
  // page with no console output.
  throw new Error("mount point #root missing from index.html");
}

log.info("peptidyle client booting");

/** Preview tests opt into fixtures before this module loads; normal builds always use same-origin HTTP. */
const apiClient =
  window.__PLE_USE_MOCK_API__ === true ? createMockApiClient() : createHttpApiClient();
const apiRuntime = createApiRuntime(apiClient);

render(
  () => (
    <ApiRuntimeProvider runtime={apiRuntime}>
      <SessionProvider
        getSession={apiClient.getSession}
        logout={apiClient.logout}
        loginWithLocalCredential={localCredentialLogin()}
      >
        <WasmRuntimeProvider
          formatFallback={apiClient.validateResponseFormatOnServer}
          timerFallback={apiClient.timerVerdictOnServer}
          capabilityFallback={apiClient.validateAssignmentConfigOnServer}
        >
          <Router root={App}>{[...appRoutes, notFoundRoute]}</Router>
        </WasmRuntimeProvider>
      </SessionProvider>
    </ApiRuntimeProvider>
  ),
  mountPoint,
);
