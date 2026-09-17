// Same-origin transport for the private Library Watch inbox.

import type { ApiClient } from "../client";
import { decodeLibraryWatchNotifications } from "../decoders/library_watch_notification";
import type {
  LibraryWatchNotification,
  LibraryWatchNotificationClient,
} from "../library_watch_notification";
import { ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 100;

export function createLibraryWatchNotificationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LibraryWatchNotificationClient> {
  return {
    getLibraryWatchNotifications: async (
      limit = DEFAULT_LIMIT,
    ): Promise<ReadonlyArray<LibraryWatchNotification>> => {
      if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_LIMIT) {
        throw new Error("Library Watch inbox limit is invalid");
      }
      const path = `/api/library/watch-notifications?${new URLSearchParams({ limit: String(limit) })}`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeLibraryWatchNotifications(await boundedResponseJson(response, path));
    },
  };
}
