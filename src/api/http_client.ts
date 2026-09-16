// http_client.ts - stable same-origin API facade; capabilities live beside it.

import type { OrdinaryBrowserApiClient } from "./client";
import { createAuthClient } from "./http_client/auth";
import { createLiveDemoClient } from "./http_client/live_demo";
import {
  browserFetch,
  createRequestClient,
  normalizeBasePath,
  type HttpApiClientConfig,
} from "./http_client/request";
import { createResponseClient } from "./http_client/response";
import { createTeachingOperationsClient } from "./http_client/teaching_operations";
import { createBlueprintCourseClient } from "./http_client/blueprint_course";
import { createCourseInstanceClient } from "./http_client/course_instance";
import { createLiveCourseRosterClient } from "./http_client/course_roster";
import { createLiveInvitationExportClient } from "./http_client/invitation_export";
import { createLiveAssessmentReleaseClient } from "./http_client/assessment_release";
import { createAssessmentPoolForkClient } from "./http_client/assessment_pool_fork";
import { createAssessmentStudentTimeAccommodationClient } from "./http_client/assessment_student_time_accommodation";
import { createLiveAssessmentAttemptIssuanceClient } from "./http_client/assessment_attempt_issuance";
import { createStudentAssessmentAttemptHistoryClient } from "./http_client/assessment_attempt_history";
import { createStudentAssessmentAttemptNavigationClient } from "./http_client/assessment_attempt_navigation";
import { createInstructorAccountClient } from "./http_client/instructor_account";
import { createCourseGradebookClient } from "./http_client/live_gradebook";
import { createLiveStudentCourseLandingClient } from "./http_client/live_student_course_landing";
import { createQuestionAvailabilityClient } from "./http_client/question_availability";
import { createQuestionWatchClient } from "./http_client/question_watch";
import { createQuestionStarClient } from "./http_client/question_star";
import { createQuestionPoolLibraryClient } from "./http_client/question_pool_library";
import { createQuestionPoolCreationClient } from "./http_client/question_pool_creation";
import { createAssessmentStudentViewClient } from "./http_client/assessment_student_view";
import { createAssessmentTemplateClient } from "./http_client/assessment_template";
import { createQuestionBulkMetadataClient } from "./http_client/question_bulk_metadata";
import { createContentClassificationClient } from "./http_client/content_classification";

export {
  ApiProtocolError,
  ApiRequestError,
  AssessmentConflictError,
  BlueprintCourseConflictError,
} from "./http_client/error";
export type { ApiFetch, HttpApiClientConfig } from "./http_client/request";
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
    createLiveDemoClient(fetchImplementation, basePath),
    createTeachingOperationsClient(fetchImplementation, basePath),
    createBlueprintCourseClient(fetchImplementation, basePath),
    createCourseInstanceClient(fetchImplementation, basePath),
    createLiveCourseRosterClient(fetchImplementation, basePath),
    createLiveInvitationExportClient(fetchImplementation, basePath),
    createLiveAssessmentReleaseClient(fetchImplementation, basePath),
    createAssessmentPoolForkClient(fetchImplementation, basePath),
    createAssessmentStudentTimeAccommodationClient(fetchImplementation, basePath),
    createLiveAssessmentAttemptIssuanceClient(fetchImplementation, basePath),
    createStudentAssessmentAttemptHistoryClient(fetchImplementation, basePath),
    createStudentAssessmentAttemptNavigationClient(fetchImplementation, basePath),
    createInstructorAccountClient(fetchImplementation, basePath),
    createCourseGradebookClient(fetchImplementation, basePath),
    createLiveStudentCourseLandingClient(fetchImplementation, basePath),
    createQuestionAvailabilityClient(fetchImplementation, basePath),
    createQuestionWatchClient(fetchImplementation, basePath),
    createQuestionStarClient(fetchImplementation, basePath),
    createQuestionPoolLibraryClient({ fetch: fetchImplementation, basePath }),
    createQuestionPoolCreationClient(fetchImplementation, basePath),
    createAssessmentStudentViewClient(fetchImplementation, basePath),
    createAssessmentTemplateClient(fetchImplementation, basePath),
    createQuestionBulkMetadataClient(fetchImplementation, basePath),
    createContentClassificationClient(fetchImplementation, basePath),
    responses,
    requests,
  );
  return client;
}
