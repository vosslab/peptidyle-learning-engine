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
import { createCourseInstanceClient } from "./http_client/course_instance";
import { createLiveCourseRosterClient } from "./http_client/course_roster";
import { createLiveInvitationExportClient } from "./http_client/invitation_export";
import { createBlueprintOperationsClient } from "./http_client/blueprint_operations";
import { createGradingOperationsClient } from "./http_client/grading_operations";
import { createCalculatedGradebookClient } from "./http_client/calculated_gradebook";
import { createLiveAssignmentReleaseClient } from "./http_client/assignment_release";
import { createLiveAssignmentAttemptIssuanceClient } from "./http_client/assignment_attempt_issuance";
import { createStudentAssignmentAttemptNavigationClient } from "./http_client/assignment_attempt_navigation";
import { createInstructorAccountClient } from "./http_client/instructor_account";
import { createSupportCapabilityClient } from "./http_client/support_roster";
import { createLiveDemoGradebookClient } from "./http_client/live_gradebook";
import { createLiveStudentCourseLandingClient } from "./http_client/live_student_course_landing";

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
  const responses = createResponseClient(fetchImplementation, basePath);
  const requests = createRequestClient(fetchImplementation, basePath);
  Object.assign(
    client,
    createAuthClient(fetchImplementation, basePath),
    createCourseRosterClient(fetchImplementation, basePath),
    createLiveDemoClient(fetchImplementation, basePath),
    createTeachingOperationsClient(fetchImplementation, basePath),
    createPreviewPlaneClient(fetchImplementation, basePath),
    createBlueprintCourseClient(fetchImplementation, basePath),
    createCourseInstanceClient(fetchImplementation, basePath),
    createLiveCourseRosterClient(fetchImplementation, basePath),
    createLiveInvitationExportClient(fetchImplementation, basePath),
    createLiveAssignmentReleaseClient(fetchImplementation, basePath),
    createLiveAssignmentAttemptIssuanceClient(fetchImplementation, basePath),
    createStudentAssignmentAttemptNavigationClient(fetchImplementation, basePath),
    createInstructorAccountClient(fetchImplementation, basePath),
    createSupportCapabilityClient(fetchImplementation, basePath),
    createLiveDemoGradebookClient(fetchImplementation, basePath),
    createLiveStudentCourseLandingClient(fetchImplementation, basePath),
    createBlueprintOperationsClient(fetchImplementation, basePath),
    createGradingOperationsClient(fetchImplementation, basePath),
    createCalculatedGradebookClient(fetchImplementation, basePath),
    responses,
    requests,
  );
  return client;
}
