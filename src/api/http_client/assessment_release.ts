// Strict same-origin transport for the Assessment Workspace and release boundary.

import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type {
  CourseAssessmentSummary,
  DueSoonAssessments,
  LiveAssessmentReleaseClient,
  LiveAssessmentWorkspaceResponse,
  UnreleasedLiveAssessment,
} from "../assessment_release";
import {
  decodeAssessmentBlueprintUpdateReview,
  decodeApplyAssessmentBlueprintUpdateInput,
  decodeAssessmentQuestionPicker,
  decodeAssessmentReleaseValidation,
  decodeAssessmentUnreleaseImpact,
  decodeCourseAssessmentSummary,
  decodeCourseAssessments,
  decodeDueSoonAssessments,
  decodeCreateLiveAssessmentInput,
  decodeLiveAssessmentWorkspace,
  decodeSaveLiveAssessmentInlineInput,
  decodeSaveLiveAssessmentInput,
  decodeSaveBaseAssessmentPolicyInput,
  decodeUnreleasedLiveAssessment,
} from "../decoders/assessment_release";
import { ApiProtocolError, ApiRequestError } from "./error";
import {
  assertResponseMatchesPositiveNumber,
  ifMatchHeaderForPositiveNumber,
} from "./conditional_request";
import { decodeCourseBlueprintUpdateReview } from "../decoders/course_blueprint_update";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseAssessmentId, parseCourseInstanceId } from "../../navigation/public_route";

export class LiveAssessmentWorkspaceConflictError extends ApiRequestError {
  public constructor(path: string) {
    super(412, path);
    this.name = "LiveAssessmentWorkspaceConflictError";
  }
}

function coursePath(course: CourseInstanceId): string {
  if (parseCourseInstanceId(course) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}`;
}

function assessmentPath(course: CourseInstanceId, assessment: AssessmentId): string {
  if (parseAssessmentId(assessment) === null) {
    throw new ApiProtocolError("Assessment ID must be canonical");
  }
  return `${coursePath(course)}/assessments/${encodeURIComponent(assessment)}`;
}

function loadedWorkspace(
  body: LiveAssessmentWorkspaceResponse["workspace"],
  response: Response,
  path: string,
): LiveAssessmentWorkspaceResponse {
  assertResponseMatchesPositiveNumber(response, body.editNumber, path, "Assessment Edit Number");
  return { workspace: body };
}

async function assessmentJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly expectedAssessmentEditNumber?: string;
    readonly status?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> =
    options.expectedAssessmentEditNumber === undefined
      ? {}
      : {
          "if-match": ifMatchHeaderForPositiveNumber(
            options.expectedAssessmentEditNumber,
            path,
            "Assessment Edit Number",
          ),
        };
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new LiveAssessmentWorkspaceConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.status !== undefined && response.status !== options.status) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.status}`);
  }
  return { body: decoder(await boundedResponseJson(response, path), "response"), response };
}

/** Composes this capability separately from stale generic Assessment Workspace clients. */
export function createLiveAssessmentReleaseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveAssessmentReleaseClient> {
  return {
    getCourseBlueprintUpdateReview: async (course) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(course)}/blueprint-update-review`,
          decodeCourseBlueprintUpdateReview,
          { status: 200 },
        )
      ).body,
    getAssessmentBlueprintUpdateReview: async (course, assessment) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(course, assessment)}/blueprint-update`,
          decodeAssessmentBlueprintUpdateReview,
          { status: 200 },
        )
      ).body,
    applyAssessmentBlueprintUpdate: async (
      course,
      assessment,
      input,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(course, assessment)}/blueprint-update`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "POST",
          body: decodeApplyAssessmentBlueprintUpdateInput(input),
          status: 200,
        },
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    listAssessmentsDueSoon: async (): Promise<DueSoonAssessments> =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          "/api/assessments/due-soon",
          decodeDueSoonAssessments,
        )
      ).body,
    listCourseAssessments: async (course) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(course)}/assessments`,
          decodeCourseAssessments,
        )
      ).body,
    saveLiveAssessmentInline: async (
      course,
      assessment,
      input,
      editNumber,
    ): Promise<CourseAssessmentSummary> => {
      const path = `${assessmentPath(course, assessment)}/inline`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseAssessmentSummary,
        {
          method: "PUT",
          body: decodeSaveLiveAssessmentInlineInput(input),
          expectedAssessmentEditNumber: editNumber,
          status: 200,
        },
      );
      assertResponseMatchesPositiveNumber(
        result.response,
        result.body.editNumber,
        path,
        "Assessment Edit Number",
      );
      return result.body;
    },
    listLiveAssessmentQuestionPicker: async (course) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(course)}/assessment-question-picker`,
          decodeAssessmentQuestionPicker,
        )
      ).body,
    createLiveAssessment: async (course, input): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${coursePath(course)}/assessments`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "POST",
          body: decodeCreateLiveAssessmentInput(input),
          status: 201,
        },
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    getLiveAssessmentWorkspace: async (
      course,
      assessment,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = assessmentPath(course, assessment);
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    saveLiveAssessment: async (
      course,
      assessment,
      input,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = assessmentPath(course, assessment);
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "PUT",
          body: decodeSaveLiveAssessmentInput(input),
          expectedAssessmentEditNumber,
          status: 200,
        },
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    saveBaseAssessmentPolicy: async (
      course,
      assessment,
      input,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(course, assessment)}/policies`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "PUT",
          body: decodeSaveBaseAssessmentPolicyInput(input),
          expectedAssessmentEditNumber,
          status: 200,
        },
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    validateLiveAssessmentRelease: async (course, assessment) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(course, assessment)}/release-validation`,
          decodeAssessmentReleaseValidation,
        )
      ).body,
    releaseLiveAssessment: async (
      course,
      assessment,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(course, assessment)}/release`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "POST",
          expectedAssessmentEditNumber,
          status: 200,
        },
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    getLiveAssessmentUnreleaseImpact: async (course, assessment) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(course, assessment)}/unrelease-impact`,
          decodeAssessmentUnreleaseImpact,
        )
      ).body,
    unreleaseLiveAssessment: async (
      course,
      assessment,
      confirmationTitle,
      expectedAssessmentEditNumber,
    ): Promise<UnreleasedLiveAssessment> => {
      const path = `${assessmentPath(course, assessment)}/unrelease`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeUnreleasedLiveAssessment,
        {
          method: "POST",
          body: { confirmationTitle },
          expectedAssessmentEditNumber,
          status: 200,
        },
      );
      assertResponseMatchesPositiveNumber(
        result.response,
        result.body.assessment.editNumber,
        path,
        "Assessment Edit Number",
      );
      return result.body;
    },
  };
}
