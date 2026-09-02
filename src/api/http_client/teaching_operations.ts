// Strict same-origin browser transport for WP-INST-T2 teaching operations.

import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseInvitationReference } from "../../../generated/api/CourseInvitationReference";
import type { CourseInvitationTargetSearchPage } from "../../../generated/api/CourseInvitationTargetSearchPage";
import type { TeachingAccountSearchQuery } from "../../../generated/api/TeachingAccountSearchQuery";
import type { InstructorMembershipsPage } from "../../../generated/api/InstructorMembershipsPage";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import type { TeachingOperationRevisionResponse } from "../../../generated/api/TeachingOperationRevisionResponse";
import type { ApiClient } from "../client";
import {
  decodeInstructorCourseInvitationCreateRequest,
  decodeCourseInvitationTerminalActionRequest,
  decodeCourseInvitationTargetSearchPage,
  decodeCourseInvitationTargetSearchRequest,
  decodeInstructorCourseInvitationsPage,
  decodeCourseStudentMembershipsPage,
  decodeInstructorMembershipRemovalRequest,
  decodeInstructorMembershipsPage,
  decodeAccommodationAdjustmentUpdateRequest,
  decodePendingCourseInvitationsPage,
  decodeTeachingPreviewView,
  decodeTeachingOperationRevisionResponse,
} from "../decoders";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

type JsonDecoder<T> = (value: unknown, path?: string) => T;

function strongRevision(value: TeachingOperationRevision, name: string): string {
  if (!/^[1-9][0-9]*$/u.test(value))
    throw new ApiProtocolError(`${name} must be a positive canonical decimal revision`);
  return `"${value}"`;
}

function verifyRevision(
  response: Response,
  revision: TeachingOperationRevision,
  path: string,
): void {
  if (response.headers.get("etag") !== strongRevision(revision, "response revision"))
    throw new ApiProtocolError(`API response ${path} ETag does not match its revision`);
}

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100))
    throw new ApiProtocolError("teaching page size must be an integer from 1 through 100");
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("after", cursor);
  if (pageSize !== undefined) query.set("size", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

function instructorInvitationTargetSearchPath(
  courseId: CourseId,
  query: TeachingAccountSearchQuery,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  const request = decodeCourseInvitationTargetSearchRequest(
    { query, after: cursor ?? null, size: pageSize ?? 50 },
    "request",
  );
  const parameters = new URLSearchParams({ query: request.query });
  if (request.after !== null) parameters.set("after", request.after);
  if (pageSize !== undefined) parameters.set("size", String(request.size));
  return `/api/courses/${encodedId(courseId)}/instructor-course-invitation-targets?${parameters.toString()}`;
}

async function teachingJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: JsonDecoder<T>,
  options: {
    readonly method?: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
    readonly body?: unknown;
    readonly revision?: TeachingOperationRevision;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = { accept: "application/json" };
  if (options.body !== undefined) headers["content-type"] = "application/json";
  if (options.revision !== undefined)
    headers["if-match"] = strongRevision(options.revision, "request revision");
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
    readonly revision: TeachingOperationRevision;
  },
): Promise<void> {
  const headers: Record<string, string> = {
    accept: "application/json",
    "if-match": strongRevision(options.revision, "request revision"),
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

async function createdEmpty(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  body: unknown,
): Promise<Response> {
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(body),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 201)
    throw new ApiProtocolError(`API response ${path} must use status 201`);
  if ((await response.text()).length !== 0)
    throw new ApiProtocolError(`API response ${path} must have an empty 201 body`);
  return response;
}

function requireLocation(response: Response, path: string): string {
  const location = response.headers.get("location");
  if (location === null || location.length === 0)
    throw new ApiProtocolError(`API response ${path} must include a Location header`);
  return location;
}

function verifyLocationRevision(response: Response, path: string): void {
  const revision = response.headers.get("etag");
  if (revision === null || !/^"[1-9][0-9]*"$/u.test(revision))
    throw new ApiProtocolError(
      `API response ${path} must include one positive strong numeric ETag`,
    );
}

function invitationReferenceFromLocation(location: string, path: string): string {
  const prefix = `${path}/`;
  if (!location.startsWith(prefix))
    throw new ApiProtocolError(
      `API response ${path} Location must identify the created invitation`,
    );
  const encodedReference = location.slice(prefix.length);
  if (encodedReference.length === 0 || encodedReference.includes("/"))
    throw new ApiProtocolError(
      `API response ${path} Location must contain one invitation reference`,
    );
  try {
    return decodeURIComponent(encodedReference);
  } catch (_error: unknown) {
    throw new ApiProtocolError(
      `API response ${path} Location must contain an encoded invitation reference`,
    );
  }
}

async function modifierMutation(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  body: unknown,
  revision: TeachingOperationRevision,
  method: "PUT" | "DELETE" = "PUT",
): Promise<TeachingOperationRevisionResponse> {
  const result = await teachingJson(
    fetchImplementation,
    basePath,
    path,
    decodeTeachingOperationRevisionResponse,
    { method, body, revision, expectedStatus: 200 },
  );
  verifyRevision(result.response, result.body.revision, path);
  return result.body;
}

export function createTeachingOperationsClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "listCourseStudentTargets"
  | "putAccommodation"
  | "deleteAccommodation"
  | "getTeachingPreview"
  | "listInstructorCourseInvitations"
  | "searchInstructorCourseInvitationTargets"
  | "createInstructorCourseInvitation"
  | "revokeInstructorCourseInvitation"
  | "listPendingCourseInvitations"
  | "respondToCourseInvitation"
  | "listCourseInstructors"
  | "removeCourseInstructor"
> {
  return {
    listCourseStudentTargets: (courseId, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(`/api/courses/${encodedId(courseId)}/student-targets`, cursor, pageSize),
        decodeCourseStudentMembershipsPage,
      ).then((result) => result.body),
    putAccommodation: (courseId, assignmentId, student, request, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/accommodations/${encodeURIComponent(student)}`,
        decodeAccommodationAdjustmentUpdateRequest(request, "request"),
        revision,
      ),
    deleteAccommodation: (courseId, assignmentId, student, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/accommodations/${encodeURIComponent(student)}`,
        undefined,
        revision,
        "DELETE",
      ),
    getTeachingPreview: (courseId, assignmentId, student) =>
      teachingJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/policy-preview/${encodeURIComponent(student)}`,
        decodeTeachingPreviewView,
      ).then((result) => result.body),
    listInstructorCourseInvitations: (courseId, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(
          `/api/courses/${encodedId(courseId)}/instructor-course-invitations`,
          cursor,
          pageSize,
        ),
        decodeInstructorCourseInvitationsPage,
      ).then((result) => result.body),
    searchInstructorCourseInvitationTargets: async (
      courseId,
      query,
      cursor,
      pageSize,
    ): Promise<CourseInvitationTargetSearchPage> => {
      const path = instructorInvitationTargetSearchPath(courseId, query, cursor, pageSize);
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseInvitationTargetSearchPage,
      );
      return result.body;
    },
    createInstructorCourseInvitation: async (
      courseId,
      request,
    ): Promise<CourseInvitationReference> => {
      const path = `/api/courses/${encodedId(courseId)}/instructor-course-invitations`;
      const body = decodeInstructorCourseInvitationCreateRequest(request, "request");
      const response = await createdEmpty(fetchImplementation, basePath, path, body);
      const location = requireLocation(response, path);
      const reference = invitationReferenceFromLocation(location, path);
      verifyLocationRevision(response, path);
      return reference;
    },
    revokeInstructorCourseInvitation: (courseId, invitation, revision) =>
      noContent(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/instructor-course-invitations/${encodeURIComponent(invitation)}`,
        { method: "DELETE", revision },
      ),
    listPendingCourseInvitations: (cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath("/api/account/course-invitations", cursor, pageSize),
        decodePendingCourseInvitationsPage,
      ).then((result) => result.body),
    respondToCourseInvitation: (invitation, request, revision): Promise<void> => {
      const body = decodeCourseInvitationTerminalActionRequest(request, "request");
      return noContent(
        fetchImplementation,
        basePath,
        `/api/account/course-invitations/${encodeURIComponent(invitation)}`,
        { method: "POST", body, revision },
      );
    },
    listCourseInstructors: async (
      courseId,
      cursor,
      pageSize,
    ): Promise<InstructorMembershipsPage> => {
      const path = pagePath(`/api/courses/${encodedId(courseId)}/instructors`, cursor, pageSize);
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeInstructorMembershipsPage,
      );
      verifyRevision(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    removeCourseInstructor: (courseId, membership, request, revision): Promise<void> => {
      const body = decodeInstructorMembershipRemovalRequest(request, "request");
      return noContent(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/instructors/${encodeURIComponent(membership)}`,
        { method: "DELETE", body, revision },
      );
    },
  };
}
