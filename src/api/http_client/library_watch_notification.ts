// Same-origin transport for the private Library Watch inbox.

import type { ApiClient } from "../client";
import { decodeLibraryWatchPage } from "../decoders/library_watch_notification";
import type {
  LibraryWatchNotificationClient,
  LibraryWatchPageRequest,
  LibraryWatchPage,
  LibraryWatchQuery,
} from "../library_watch_notification";
import { ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

export function createLibraryWatchNotificationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LibraryWatchNotificationClient> {
  return {
    getLibraryWatchNotifications: async (
      query: LibraryWatchQuery,
      page: LibraryWatchPageRequest,
    ): Promise<LibraryWatchPage> => {
      if (
        !Number.isSafeInteger(page.pageNumber) ||
        page.pageNumber < 1 ||
        !Number.isSafeInteger(page.pageSize) ||
        page.pageSize < 1 ||
        page.pageSize > MAX_PAGE_SIZE
      ) {
        throw new Error("Library Watch inbox page is invalid");
      }
      const params = new URLSearchParams({
        direction: query.direction,
        pageNumber: String(page.pageNumber),
        pageSize: String(page.pageSize),
      });
      if (query.q.trim()) params.set("q", query.q.trim());
      for (const targetKind of query.targetKinds) params.append("targetKind", targetKind);
      for (const eventKind of query.eventKinds) params.append("eventKind", eventKind);
      const path = `/api/library/watch-notifications?${params}`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeLibraryWatchPage(await boundedResponseJson(response, path));
    },
  };
}
