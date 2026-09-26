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

function coursePath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}`;
}

function assessmentPath(courseInstanceId: CourseInstanceId, assessmentId: AssessmentId): string {
  if (parseAssessmentId(assessmentId) === null) {
    throw new ApiProtocolError("Assessment ID must be canonical");
  }
  return `${coursePath(courseInstanceId)}/assessments/${encodeURIComponent(assessmentId)}`;
}

function loadedWorkspace(
  body: LiveAssessmentWorkspaceResponse["workspace"],
  response: Response,
  path: string,
): LiveAssessmentWorkspaceResponse {
  assertResponseMatchesPositiveNumber(
    response,
    body.assessmentEditNumber,
    path,
    "Assessment Edit Number",
  );
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
    getCourseBlueprintUpdateReview: async (courseInstanceId) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(courseInstanceId)}/blueprint-update-review`,
          decodeCourseBlueprintUpdateReview,
          { status: 200 },
        )
      ).body,
    getAssessmentBlueprintUpdateReview: async (courseInstanceId, assessmentId) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(courseInstanceId, assessmentId)}/blueprint-update`,
          decodeAssessmentBlueprintUpdateReview,
          { status: 200 },
        )
      ).body,
    applyAssessmentBlueprintUpdate: async (
      courseInstanceId,
      assessmentId,
      input,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(courseInstanceId, assessmentId)}/blueprint-update`;
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
    listCourseAssessments: async (courseInstanceId) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(courseInstanceId)}/assessments`,
          decodeCourseAssessments,
        )
      ).body,
    saveLiveAssessmentInline: async (
      courseInstanceId,
      assessmentId,
      input,
      editNumber,
    ): Promise<CourseAssessmentSummary> => {
      const path = `${assessmentPath(courseInstanceId, assessmentId)}/inline`;
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
        result.body.assessmentEditNumber,
        path,
        "Assessment Edit Number",
      );
      return result.body;
    },
    createLiveAssessment: async (
      courseInstanceId,
      input,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${coursePath(courseInstanceId)}/assessments`;
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
      courseInstanceId,
      assessmentId,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = assessmentPath(courseInstanceId, assessmentId);
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
      );
      return loadedWorkspace(result.body, result.response, path);
    },
    saveLiveAssessment: async (
      courseInstanceId,
      assessmentId,
      input,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = assessmentPath(courseInstanceId, assessmentId);
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
      courseInstanceId,
      assessmentId,
      input,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(courseInstanceId, assessmentId)}/policies`;
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
    validateLiveAssessmentRelease: async (courseInstanceId, assessmentId) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(courseInstanceId, assessmentId)}/release-validation`,
          decodeAssessmentReleaseValidation,
        )
      ).body,
    releaseLiveAssessment: async (
      courseInstanceId,
      assessmentId,
      expectedAssessmentEditNumber,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(courseInstanceId, assessmentId)}/release`;
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
    getLiveAssessmentUnreleaseImpact: async (courseInstanceId, assessmentId) =>
      (
        await assessmentJson(
          fetchImplementation,
          basePath,
          `${assessmentPath(courseInstanceId, assessmentId)}/unrelease-impact`,
          decodeAssessmentUnreleaseImpact,
        )
      ).body,
    unreleaseLiveAssessment: async (
      courseInstanceId,
      assessmentId,
      confirmationTitle,
      expectedAssessmentEditNumber,
    ): Promise<UnreleasedLiveAssessment> => {
      const path = `${assessmentPath(courseInstanceId, assessmentId)}/unrelease`;
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
        result.body.assessment.assessmentEditNumber,
        path,
        "Assessment Edit Number",
      );
      return result.body;
    },
  };
}
