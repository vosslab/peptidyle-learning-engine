// main.tsx - browser application entry point.
//
// Solid's `render` takes a component *function*, not an element. Passing JSX
// directly evaluates it once, outside a reactive root, which is how a Solid app
// ends up rendering a static snapshot that never updates.

import { render } from "solid-js/web";
import { query, Router } from "@solidjs/router";

import { createBrowserApiClient } from "./api/browser_client";
import { browserFetch } from "./api/http_client";
import { ApplicationApiProvider, createApplicationApi } from "./api/application_api";
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
  throw new Error("application root #root missing from index.html");
}

log.info("peptidyle client booting");

const sessionBoundary = createBrowserSessionBoundary(browserFetch, query.clear);
const apiClient = createBrowserApiClient({ fetch: sessionBoundary.fetch });
const applicationApi = createApplicationApi(apiClient);

render(
  () => (
    <ApplicationApiProvider applicationApi={applicationApi}>
      <SessionProvider
        getSession={apiClient.getSession}
        logout={apiClient.logout}
        advanceSessionBoundary={sessionBoundary.advance}
      >
        <WasmRuntimeProvider
          formatFallback={apiClient.validateResponseFormatOnServer}
          timerFallback={apiClient.questionAttemptTimingDecisionOnServer}
          capabilityFallback={apiClient.validateAssignmentConfigOnServer}
        >
          <Router root={App}>{[...appRoutes, notFoundRoute]}</Router>
        </WasmRuntimeProvider>
      </SessionProvider>
    </ApplicationApiProvider>
  ),
  mountPoint,
);
