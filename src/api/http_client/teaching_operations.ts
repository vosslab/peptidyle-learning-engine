// Strict same-origin browser transport for Course teaching operations.

import type { ApiClient } from "../client";
import {
  decodeCourseInvitationTerminalActionRequest,
  decodePendingCourseInvitationsPage,
} from "../decoders";
import { ApiProtocolError, ApiRequestError } from "./error";
import { ifMatchHeaderForPositiveNumber } from "./conditional_request";
import { requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

type JsonDecoder<T> = (value: unknown, path?: string) => T;

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100))
    throw new ApiProtocolError("teaching page size must be an integer from 1 through 100");
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("after", cursor);
  if (pageSize !== undefined) query.set("size", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

async function teachingJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: JsonDecoder<T>,
  options: {
    readonly method?: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
    readonly body?: unknown;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = { accept: "application/json" };
  if (options.body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus)
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  const value = await boundedResponseJson(response, path);
  return { body: decoder(value, "response"), response };
}

async function noContent(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: {
    readonly method: "DELETE" | "POST";
    readonly body?: unknown;
    readonly ifMatch: string;
  },
): Promise<void> {
  const headers: Record<string, string> = {
    accept: "application/json",
    "if-match": ifMatchHeaderForPositiveNumber(options.ifMatch, path, "request state precondition"),
  };
  if (options.body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method,
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204)
    throw new ApiProtocolError(`API response ${path} must use status 204`);
  if ((await response.text()).length !== 0)
    throw new ApiProtocolError(`API response ${path} must have an empty 204 body`);
}

export function createTeachingOperationsClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, "listPendingCourseInvitations" | "respondToCourseInvitation"> {
  return {
    listPendingCourseInvitations: (cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath("/api/account/course-invitations", cursor, pageSize),
        decodePendingCourseInvitationsPage,
      ).then((result) => result.body),
    respondToCourseInvitation: (invitation, request, statePrecondition): Promise<void> => {
      const body = decodeCourseInvitationTerminalActionRequest(request, "request");
      return noContent(
        fetchImplementation,
        basePath,
        `/api/account/course-invitations/${encodeURIComponent(invitation)}`,
        { method: "POST", body, ifMatch: statePrecondition },
      );
    },
  };
}
