// http_client.ts - stable same-origin API facade; capabilities live beside it.

import type { ApiClient } from "./client";
import { createAuthClient } from "./http_client/auth";
import { createEnrollmentClient } from "./http_client/enrollment";
import {
  browserFetch,
  createRequestClient,
  normalizeBasePath,
  type HttpApiClientConfig,
} from "./http_client/request";
import { createResponseClient } from "./http_client/response";

export {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  AssignmentValidationError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
  PublicationValidationError,
  WorkspaceConflictError,
} from "./http_client/error";
export type { ApiFetch, HttpApiClientConfig } from "./http_client/request";

/** Creates the strict same-origin transport from independently owned capabilities. */
export function createHttpApiClient(config: HttpApiClientConfig = {}): ApiClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);
  const client = {} as ApiClient;
  const responses = createResponseClient(fetchImplementation, basePath, () => client);
  const requests = createRequestClient(fetchImplementation, basePath);
  Object.assign(
    client,
    createAuthClient(fetchImplementation, basePath),
    createEnrollmentClient(fetchImplementation, basePath),
    responses,
    requests,
  );
  return client;
}
