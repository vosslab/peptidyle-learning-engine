// browser_session_boundary.ts - one abortable request generation per browser session.

import type { ApiFetch } from "../api/http_client";

export interface BrowserSessionBoundary {
  readonly fetch: ApiFetch;
  readonly advance: () => void;
}

/**
 * Keeps requests and router query entries inside the session generation that
 * created them. Advancing aborts the old generation before cached projections
 * are discarded, so its late responses cannot become another account's data.
 */
export function createBrowserSessionBoundary(
  fetchImplementation: ApiFetch,
  clearCachedQueries: () => void,
): BrowserSessionBoundary {
  let controller = new AbortController();

  const sessionFetch: ApiFetch = (input, init) => {
    const requestSignal = init?.signal;
    const signal =
      requestSignal === undefined || requestSignal === null
        ? controller.signal
        : AbortSignal.any([controller.signal, requestSignal]);
    return fetchImplementation(input, { ...init, signal });
  };

  function advance(): void {
    controller.abort();
    controller = new AbortController();
    clearCachedQueries();
  }

  return { fetch: sessionFetch, advance };
}
