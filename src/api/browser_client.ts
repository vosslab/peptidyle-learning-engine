//! Browser transport for the ordinary live application artifact.
//!
//! This module is the live build boundary. Keep its graph limited to the
//! same-origin HTTP client so the installed site cannot carry browser-test
//! handlers or their generated data.

import { createHttpApiClient } from "./http_client";
import type { OrdinaryBrowserApiClient } from "./client";

/** Creates the API client for the ordinary live browser application. */
export function createBrowserApiClient(): OrdinaryBrowserApiClient {
  return createHttpApiClient();
}
