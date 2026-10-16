// Same-origin transport for the private Library Watch inbox.

import type { ApiClient } from "../client";
import { decodeLibraryWatchPage } from "../decoders/library_watch_notification";
import type {
  LibraryWatchNotificationClient,
  LibraryWatchPage,
  LibraryWatchQuery,
} from "../library_watch_notification";
import { ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 250;

export function createLibraryWatchNotificationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LibraryWatchNotificationClient> {
  return {
    getLibraryWatchNotifications: async (
      query: LibraryWatchQuery,
      cursor: string | null = null,
      limit = DEFAULT_LIMIT,
    ): Promise<LibraryWatchPage> => {
      if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_LIMIT) {
        throw new Error("Library Watch inbox limit is invalid");
      }
      const params = new URLSearchParams({ limit: String(limit), direction: query.direction });
      if (query.q.trim()) params.set("q", query.q.trim());
      for (const targetKind of query.targetKinds) params.append("targetKind", targetKind);
      for (const eventKind of query.eventKinds) params.append("eventKind", eventKind);
      if (cursor !== null) params.set("cursor", cursor);
      const path = `/api/library/watch-notifications?${params}`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeLibraryWatchPage(await boundedResponseJson(response, path));
    },
  };
}
