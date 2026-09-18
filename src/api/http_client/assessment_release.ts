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
import { decodeCourseBlueprintUpdateReview } from "../decoders/course_blueprint_update";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import {
  parseAssessmentId,
  parseCourseInstanceId,
} from "../../navigation/public_route";

export class LiveAssessmentWorkspaceConflictError extends ApiRequestError {
  public constructor(path: string) {
    super(412, path);
    this.name = "LiveAssessmentWorkspaceConflictError";
  }
}

function coursePath(course: CourseInstanceId): string {
  if (parseCourseInstanceId(course) === null) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}`;
}

function assessmentPath(course: CourseInstanceId, assessment: AssessmentId): string {
  if (parseAssessmentId(assessment) === null) {
    throw new ApiProtocolError("Assessment reference must be canonical");
  }
  return `${coursePath(course)}/assessments/${encodeURIComponent(assessment)}`;
}

function requireWorkspaceEtag(response: Response, editNumber: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || etag !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Assessment Edit Number`);
  }
  return etag;
}

function quotedStrongEtag(etag: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(etag) || BigInt(etag.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(
      `API ${path} If-Match must be one quoted strong Assessment Edit Number`,
    );
  }
  return etag;
}

async function assessmentJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
    readonly status?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> =
    options.etag === undefined ? {} : { "if-match": quotedStrongEtag(options.etag, path) };
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
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
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
          etag: `"${editNumber}"`,
          status: 200,
        },
      );
      requireWorkspaceEtag(result.response, result.body.editNumber, path);
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
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
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
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
    },
    saveLiveAssessment: async (
      course,
      assessment,
      input,
      etag,
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
          etag,
          status: 200,
        },
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
    },
    saveBaseAssessmentPolicy: async (
      course,
      assessment,
      input,
      etag,
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
          etag,
          status: 200,
        },
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
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
      etag,
    ): Promise<LiveAssessmentWorkspaceResponse> => {
      const path = `${assessmentPath(course, assessment)}/release`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssessmentWorkspace,
        {
          method: "POST",
          etag,
          status: 200,
        },
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
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
      etag,
    ): Promise<{ readonly result: UnreleasedLiveAssessment; readonly etag: string }> => {
      const path = `${assessmentPath(course, assessment)}/unrelease`;
      const result = await assessmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeUnreleasedLiveAssessment,
        {
          method: "POST",
          body: { confirmationTitle },
          etag,
          status: 200,
        },
      );
      return {
        result: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.assessment.editNumber, path),
      };
    },
  };
}
