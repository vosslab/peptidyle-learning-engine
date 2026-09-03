// http_client.ts - stable same-origin API facade; capabilities live beside it.

import type { OrdinaryBrowserApiClient } from "./client";
import { createAuthClient } from "./http_client/auth";
import { createCourseRosterClient } from "./http_client/enrollment";
import { createLiveDemoClient } from "./http_client/live_demo";
import {
  browserFetch,
  createRequestClient,
  normalizeBasePath,
  type HttpApiClientConfig,
} from "./http_client/request";
import { createResponseClient } from "./http_client/response";
import { createTeachingOperationsClient } from "./http_client/teaching_operations";
import { createPreviewPlaneClient } from "./http_client/preview_plane";
import { createBlueprintCourseClient } from "./http_client/blueprint_course";
import { createBlueprintOperationsClient } from "./http_client/blueprint_operations";
import { createGradingOperationsClient } from "./http_client/grading_operations";
import { createCalculatedGradebookClient } from "./http_client/calculated_gradebook";

export {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  AssignmentSuccessorRevisionRequiredError,
  AssignmentPoliciesValidationError,
  PreviewPlaneConflictError,
  CourseGradeSchemeConflictError,
  BlueprintCourseConflictError,
  CourseTermValidationError,
  resolveAssignmentContentSaveFailure,
} from "./http_client/error";
export type { ApiFetch, HttpApiClientConfig } from "./http_client/request";
export type { AssignmentContentSaveFailure } from "./http_client/error";
export { browserFetch };

/** Creates the strict same-origin transport from independently owned capabilities. */
export function createHttpApiClient(config: HttpApiClientConfig = {}): OrdinaryBrowserApiClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);
  const client = {} as OrdinaryBrowserApiClient;
  const responses = createResponseClient(fetchImplementation, basePath, () => client);
  const requests = createRequestClient(fetchImplementation, basePath);
  Object.assign(
    client,
    createAuthClient(fetchImplementation, basePath),
    createCourseRosterClient(fetchImplementation, basePath),
    createLiveDemoClient(fetchImplementation, basePath),
    createTeachingOperationsClient(fetchImplementation, basePath),
    createPreviewPlaneClient(fetchImplementation, basePath),
    createBlueprintCourseClient(fetchImplementation, basePath),
    createBlueprintOperationsClient(fetchImplementation, basePath),
    createGradingOperationsClient(fetchImplementation, basePath),
    createCalculatedGradebookClient(fetchImplementation, basePath),
    responses,
    requests,
  );
  return client;
}
