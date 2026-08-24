// main.tsx - browser entry point (MOD-UI-SHELL, WP-F3).
//
// Solid's `render` takes a component *function*, not an element. Passing JSX
// directly evaluates it once, outside a reactive root, which is how a Solid app
// ends up rendering a static snapshot that never updates.

import { render } from "solid-js/web";
import { query, Router } from "@solidjs/router";

import { createBrowserApiClient } from "./api/browser_client";
import { browserFetch } from "./api/http_client";
import { ApiRuntimeProvider, createApiRuntime } from "./api/runtime";
import { App } from "./app";
import { createBrowserSessionBoundary } from "./auth/browser_session_boundary";
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

const sessionBoundary = createBrowserSessionBoundary(browserFetch, query.clear);
const apiClient = createBrowserApiClient({ fetch: sessionBoundary.fetch });
const apiRuntime = createApiRuntime(apiClient);

render(
  () => (
    <ApiRuntimeProvider runtime={apiRuntime}>
      <SessionProvider
        getSession={apiClient.getSession}
        logout={apiClient.logout}
        advanceSessionBoundary={sessionBoundary.advance}
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
