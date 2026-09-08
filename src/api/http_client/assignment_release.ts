// Strict same-origin transport for the Assignment Workspace and release boundary.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type {
  LiveAssignmentReleaseClient,
  RevisionedLiveAssignmentWorkspace,
} from "../assignment_release";
import {
  decodeAssignmentPreview,
  decodeAssignmentQuestionPicker,
  decodeAssignmentReleaseValidation,
  decodeCreateLiveAssignmentInput,
  decodeLiveAssignmentWorkspace,
  decodeReleasedLiveAssignment,
  decodeSaveLiveAssignmentInput,
} from "../decoders/assignment_release";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export class LiveAssignmentWorkspaceConflictError extends ApiRequestError {
  public constructor(path: string) {
    super(412, path);
    this.name = "LiveAssignmentWorkspaceConflictError";
  }
}

function coursePath(course: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}`;
}

function assignmentPath(course: CourseInstanceReference, assignment: AssignmentReference): string {
  if (!/^A-[1-9][0-9]{0,9}$/u.test(assignment) || Number(assignment.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Assignment reference must be canonical");
  }
  return `${coursePath(course)}/assignments/${encodeURIComponent(assignment)}`;
}

function requireWorkspaceEtag(response: Response, editNumber: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || etag !== `"${editNumber}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Assignment Edit Number`);
  }
  return etag;
}

function quotedStrongEtag(etag: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(etag) || BigInt(etag.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(
      `API ${path} If-Match must be one quoted strong Assignment Edit Number`,
    );
  }
  return etag;
}

async function assignmentJson<T>(
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
  if (response.status === 412) throw new LiveAssignmentWorkspaceConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.status !== undefined && response.status !== options.status) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.status}`);
  }
  return { body: decoder(await boundedResponseJson(response, path), "response"), response };
}

/** Composes this capability separately from stale generic Assignment Workspace clients. */
export function createLiveAssignmentReleaseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveAssignmentReleaseClient> {
  return {
    listLiveAssignmentQuestionPicker: async (course) =>
      (
        await assignmentJson(
          fetchImplementation,
          basePath,
          `${coursePath(course)}/assignment-question-picker`,
          decodeAssignmentQuestionPicker,
        )
      ).body,
    createLiveAssignment: async (course, input): Promise<RevisionedLiveAssignmentWorkspace> => {
      const path = `${coursePath(course)}/assignments`;
      const result = await assignmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssignmentWorkspace,
        {
          method: "POST",
          body: decodeCreateLiveAssignmentInput(input),
          status: 201,
        },
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
    },
    getLiveAssignmentWorkspace: async (
      course,
      assignment,
    ): Promise<RevisionedLiveAssignmentWorkspace> => {
      const path = assignmentPath(course, assignment);
      const result = await assignmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssignmentWorkspace,
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
    },
    saveLiveAssignment: async (
      course,
      assignment,
      input,
      etag,
    ): Promise<RevisionedLiveAssignmentWorkspace> => {
      const path = assignmentPath(course, assignment);
      const result = await assignmentJson(
        fetchImplementation,
        basePath,
        path,
        decodeLiveAssignmentWorkspace,
        {
          method: "PUT",
          body: decodeSaveLiveAssignmentInput(input),
          etag,
          status: 200,
        },
      );
      return {
        workspace: result.body,
        etag: requireWorkspaceEtag(result.response, result.body.editNumber, path),
      };
    },
    validateLiveAssignmentRelease: async (course, assignment) =>
      (
        await assignmentJson(
          fetchImplementation,
          basePath,
          `${assignmentPath(course, assignment)}/release-validation`,
          decodeAssignmentReleaseValidation,
        )
      ).body,
    getLiveAssignmentPreview: async (course, assignment) =>
      (
        await assignmentJson(
          fetchImplementation,
          basePath,
          `${assignmentPath(course, assignment)}/preview`,
          decodeAssignmentPreview,
        )
      ).body,
    releaseLiveAssignment: async (course, assignment, etag) =>
      (
        await assignmentJson(
          fetchImplementation,
          basePath,
          `${assignmentPath(course, assignment)}/release`,
          decodeReleasedLiveAssignment,
          {
            method: "POST",
            etag,
            status: 201,
          },
        )
      ).body,
  };
}
