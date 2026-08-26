// http_client.ts - stable same-origin API facade; capabilities live beside it.

import type { OrdinaryBrowserApiClient } from "./client";
import { createAuthClient } from "./http_client/auth";
import { createEnrollmentClient } from "./http_client/enrollment";
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
import { createProblemCurationClient } from "./http_client/problem_curation";
import { createReusableCurriculumClient } from "./http_client/reusable_curriculum";
import { createCurriculumAdoptionClient } from "./http_client/curriculum_adoption";

export {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  PreviewPlaneConflictError,
  AssignmentValidationError,
  AssignmentTeachingSettingsValidationError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
  CourseGradeSchemeConflictError,
  ProblemCurationConflictError,
  ReusableCurriculumConflictError,
  CourseTermValidationError,
  PublicationValidationError,
  WorkspaceConflictError,
} from "./http_client/error";
export type { ApiFetch, HttpApiClientConfig } from "./http_client/request";
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
    createEnrollmentClient(fetchImplementation, basePath),
    createLiveDemoClient(fetchImplementation, basePath),
    createTeachingOperationsClient(fetchImplementation, basePath),
    createPreviewPlaneClient(fetchImplementation, basePath),
    createProblemCurationClient(fetchImplementation, basePath),
    createReusableCurriculumClient(fetchImplementation, basePath),
    createCurriculumAdoptionClient(fetchImplementation, basePath),
    responses,
    requests,
  );
  return client;
}
