// Strict same-origin browser transport for WP-INST-T2 teaching operations.

import type { AccountApprovalView } from "../../../generated/api/AccountApprovalView";
import type { SysadminInstructorCandidateSearchPage } from "../../../generated/api/SysadminInstructorCandidateSearchPage";
import type { SysadminInstructorCandidateSearchRequest } from "../../../generated/api/SysadminInstructorCandidateSearchRequest";
import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseGroupCreateRequest } from "../../../generated/api/CourseGroupCreateRequest";
import type { CourseGroupPurpose } from "../../../generated/api/CourseGroupPurpose";
import type { CourseGroupPurposePolicyUpdateRequest } from "../../../generated/api/CourseGroupPurposePolicyUpdateRequest";
import type { CourseGroupPurposePolicyView } from "../../../generated/api/CourseGroupPurposePolicyView";
import type { CourseGroupReference } from "../../../generated/api/CourseGroupReference";
import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { CourseGroupUpdateRequest } from "../../../generated/api/CourseGroupUpdateRequest";
import type { CoInstructorInvitationReference } from "../../../generated/api/CoInstructorInvitationReference";
import type { CoInstructorTargetSearchPage } from "../../../generated/api/CoInstructorTargetSearchPage";
import type { CoInstructorTargetSearchQuery } from "../../../generated/api/CoInstructorTargetSearchQuery";
import type { InstructorMembershipsPage } from "../../../generated/api/InstructorMembershipsPage";
import type { RetentionActionResponse } from "../../../generated/api/RetentionActionResponse";
import type { RetentionArchiveRequest } from "../../../generated/api/RetentionArchiveRequest";
import type { RetentionExtendRequest } from "../../../generated/api/RetentionExtendRequest";
import type { RetentionReadView } from "../../../generated/api/RetentionReadView";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import type { TeachingOperationRevisionResponse } from "../../../generated/api/TeachingOperationRevisionResponse";
import type { ApiClient, SysadminInstructorCandidateClient } from "../client";
import {
  decodeAccountApprovalView,
  decodeAssignmentPolicyPatchUpdateRequest,
  decodeCoInstructorInvitationCreateRequest,
  decodeCoInstructorInvitationTerminalActionRequest,
  decodeCoInstructorTargetSearchPage,
  decodeCoInstructorTargetSearchRequest,
  decodeCourseCoInstructorInvitationsPage,
  decodeCourseGroupCreateRequest,
  decodeCourseGroupDetailView,
  decodeCourseGroupListPage,
  decodeCourseGroupMembershipWarningView,
  decodeCourseGroupPurposePolicyUpdateRequest,
  decodeCourseGroupPurposePolicyView,
  decodeCourseGroupSummary,
  decodeCourseGroupUpdateRequest,
  decodeCourseStudentMembershipsPage,
  decodeInstructorMembershipRemovalRequest,
  decodeInstructorMembershipsPage,
  decodeGroupScheduleOffsetUpdateRequest,
  decodeIndividualPolicyPatchUpdateRequest,
  decodePendingCoInstructorInvitationsPage,
  decodeRetentionActionResponse,
  decodeRetentionArchiveRequest,
  decodeRetentionExtendRequest,
  decodeRetentionReadView,
  decodeSysadminInstructorCandidateSearchPage,
  decodeSysadminInstructorCandidateSearchRequest,
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

function targetSearchPath(
  courseId: CourseId,
  query: CoInstructorTargetSearchQuery,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  const request = decodeCoInstructorTargetSearchRequest(
    { query, after: cursor ?? null, size: pageSize ?? 50 },
    "request",
  );
  const parameters = new URLSearchParams({ query: request.query });
  if (request.after !== null) parameters.set("after", request.after);
  if (pageSize !== undefined) parameters.set("size", String(request.size));
  return `/api/courses/${encodedId(courseId)}/co-instructor-targets?${parameters.toString()}`;
}

function sysadminCandidateSearchPath(request: SysadminInstructorCandidateSearchRequest): string {
  const decoded = decodeSysadminInstructorCandidateSearchRequest(request, "request");
  const parameters = new URLSearchParams({ query: decoded.query });
  if (decoded.after !== null) parameters.set("after", decoded.after);
  parameters.set("size", String(decoded.size));
  return `/api/teaching/instructor-approval-candidates?${parameters.toString()}`;
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

function groupPath(courseId: CourseId, group: CourseGroupReference): string {
  return `/api/courses/${encodedId(courseId)}/groups/${encodeURIComponent(group)}`;
}

function groupPolicyPath(courseId: CourseId, purpose: CourseGroupPurpose): string {
  return `/api/courses/${encodedId(courseId)}/group-purpose-policies/${encodeURIComponent(purpose)}`;
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

function createdGroupLocation(location: string, path: string): void {
  if (!location.startsWith(`${path}/`))
    throw new ApiProtocolError(`API response ${path} Location must identify the created group`);
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
  ApiClient & SysadminInstructorCandidateClient,
  | "listCourseGroups"
  | "getCourseGroup"
  | "createCourseGroup"
  | "updateCourseGroup"
  | "deleteCourseGroup"
  | "getCourseGroupPurposePolicy"
  | "updateCourseGroupPurposePolicy"
  | "getCourseGroupMembershipWarnings"
  | "listCourseStudentTargets"
  | "putGroupScheduleOffset"
  | "deleteGroupScheduleOffset"
  | "putGroupAccommodation"
  | "deleteGroupAccommodation"
  | "putIndividualPolicyException"
  | "deleteIndividualPolicyException"
  | "getTeachingPreview"
  | "approveInstructorAccount"
  | "revokeInstructorApproval"
  | "searchSysadminInstructorCandidates"
  | "listCourseCoInstructorInvitations"
  | "searchCourseCoInstructorTargets"
  | "createCourseCoInstructorInvitation"
  | "revokeCourseCoInstructorInvitation"
  | "listPendingCoInstructorInvitations"
  | "respondToCoInstructorInvitation"
  | "listCourseInstructors"
  | "removeCourseInstructor"
  | "getCourseRetention"
  | "endCourseRetention"
  | "archiveCourseRetention"
  | "deleteCourseRetention"
  | "extendCourseRetention"
> {
  return {
    listCourseGroups: (courseId, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(`/api/courses/${encodedId(courseId)}/groups`, cursor, pageSize),
        decodeCourseGroupListPage,
      ).then((result) => result.body),
    getCourseGroup: (courseId, group, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(groupPath(courseId, group), cursor, pageSize),
        decodeCourseGroupDetailView,
      ).then((result) => result.body),
    createCourseGroup: async (courseId, request): Promise<CourseGroupSummaryView> => {
      const path = `/api/courses/${encodedId(courseId)}/groups`;
      const body: CourseGroupCreateRequest = decodeCourseGroupCreateRequest(request, "request");
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseGroupSummary,
        {
          method: "POST",
          body,
          expectedStatus: 201,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      createdGroupLocation(requireLocation(result.response, path), path);
      return result.body;
    },
    updateCourseGroup: async (
      courseId,
      group,
      request,
      revision,
    ): Promise<CourseGroupSummaryView> => {
      const path = groupPath(courseId, group);
      const body: CourseGroupUpdateRequest = decodeCourseGroupUpdateRequest(request, "request");
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseGroupSummary,
        {
          method: "PUT",
          body,
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    deleteCourseGroup: (courseId, group, revision) =>
      noContent(fetchImplementation, basePath, groupPath(courseId, group), {
        method: "DELETE",
        revision,
      }),
    getCourseGroupPurposePolicy: async (
      courseId,
      purpose,
    ): Promise<CourseGroupPurposePolicyView> => {
      const path = groupPolicyPath(courseId, purpose);
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseGroupPurposePolicyView,
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    updateCourseGroupPurposePolicy: async (
      courseId,
      purpose,
      request,
      revision,
    ): Promise<CourseGroupPurposePolicyView> => {
      const path = groupPolicyPath(courseId, purpose);
      const body: CourseGroupPurposePolicyUpdateRequest =
        decodeCourseGroupPurposePolicyUpdateRequest(request, "request");
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCourseGroupPurposePolicyView,
        {
          method: "PUT",
          body,
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    getCourseGroupMembershipWarnings: (courseId) =>
      teachingJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/group-membership-warnings`,
        decodeCourseGroupMembershipWarningView,
      ).then((result) => result.body),
    listCourseStudentTargets: (courseId, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(`/api/courses/${encodedId(courseId)}/student-targets`, cursor, pageSize),
        decodeCourseStudentMembershipsPage,
      ).then((result) => result.body),
    putGroupScheduleOffset: (courseId, assignmentId, group, request, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/group-schedule-offsets/${encodeURIComponent(group)}`,
        decodeGroupScheduleOffsetUpdateRequest(request, "request"),
        revision,
      ),
    deleteGroupScheduleOffset: (courseId, assignmentId, group, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/group-schedule-offsets/${encodeURIComponent(group)}`,
        undefined,
        revision,
        "DELETE",
      ),
    putGroupAccommodation: (courseId, assignmentId, group, request, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/group-accommodations/${encodeURIComponent(group)}`,
        decodeAssignmentPolicyPatchUpdateRequest(request, "request"),
        revision,
      ),
    deleteGroupAccommodation: (courseId, assignmentId, group, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/group-accommodations/${encodeURIComponent(group)}`,
        undefined,
        revision,
        "DELETE",
      ),
    putIndividualPolicyException: (courseId, assignmentId, student, request, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/individual-policy-exceptions/${encodeURIComponent(student)}`,
        decodeIndividualPolicyPatchUpdateRequest(request, "request"),
        revision,
      ),
    deleteIndividualPolicyException: (courseId, assignmentId, student, revision) =>
      modifierMutation(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/assignments/${encodedId(assignmentId)}/individual-policy-exceptions/${encodeURIComponent(student)}`,
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
    approveInstructorAccount: async (account, revision): Promise<AccountApprovalView> => {
      const path = `/api/teaching/instructor-approvals/${encodeURIComponent(account)}`;
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeAccountApprovalView,
        {
          method: "PUT",
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    revokeInstructorApproval: async (account, revision): Promise<AccountApprovalView> => {
      const path = `/api/teaching/instructor-approvals/${encodeURIComponent(account)}`;
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeAccountApprovalView,
        {
          method: "DELETE",
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    searchSysadminInstructorCandidates: async (
      request,
    ): Promise<SysadminInstructorCandidateSearchPage> => {
      const path = sysadminCandidateSearchPath(request);
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeSysadminInstructorCandidateSearchPage,
      );
      return result.body;
    },
    listCourseCoInstructorInvitations: (courseId, cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath(`/api/courses/${encodedId(courseId)}/co-instructor-invitations`, cursor, pageSize),
        decodeCourseCoInstructorInvitationsPage,
      ).then((result) => result.body),
    searchCourseCoInstructorTargets: async (
      courseId,
      query,
      cursor,
      pageSize,
    ): Promise<CoInstructorTargetSearchPage> => {
      const path = targetSearchPath(courseId, query, cursor, pageSize);
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeCoInstructorTargetSearchPage,
      );
      return result.body;
    },
    createCourseCoInstructorInvitation: async (
      courseId,
      request,
    ): Promise<CoInstructorInvitationReference> => {
      const path = `/api/courses/${encodedId(courseId)}/co-instructor-invitations`;
      const body = decodeCoInstructorInvitationCreateRequest(request, "request");
      const response = await createdEmpty(fetchImplementation, basePath, path, body);
      const location = requireLocation(response, path);
      const reference = invitationReferenceFromLocation(location, path);
      verifyLocationRevision(response, path);
      return reference;
    },
    revokeCourseCoInstructorInvitation: (courseId, invitation, revision) =>
      noContent(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/co-instructor-invitations/${encodeURIComponent(invitation)}`,
        { method: "DELETE", revision },
      ),
    listPendingCoInstructorInvitations: (cursor, pageSize) =>
      teachingJson(
        fetchImplementation,
        basePath,
        pagePath("/api/account/co-instructor-invitations", cursor, pageSize),
        decodePendingCoInstructorInvitationsPage,
      ).then((result) => result.body),
    respondToCoInstructorInvitation: (invitation, request, revision): Promise<void> => {
      const body = decodeCoInstructorInvitationTerminalActionRequest(request, "request");
      return noContent(
        fetchImplementation,
        basePath,
        `/api/account/co-instructor-invitations/${encodeURIComponent(invitation)}`,
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
    getCourseRetention: async (courseId): Promise<RetentionReadView> => {
      const path = `/api/courses/${encodedId(courseId)}/retention`;
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeRetentionReadView,
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    endCourseRetention: async (courseId): Promise<RetentionReadView> => {
      const path = `/api/courses/${encodedId(courseId)}/retention/end`;
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeRetentionReadView,
        {
          method: "POST",
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    archiveCourseRetention: async (
      courseId,
      request,
      revision,
    ): Promise<RetentionActionResponse> => {
      const path = `/api/courses/${encodedId(courseId)}/retention/archive`;
      const body: RetentionArchiveRequest = decodeRetentionArchiveRequest(request, "request");
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeRetentionActionResponse,
        {
          method: "POST",
          body,
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    deleteCourseRetention: async (courseId, revision): Promise<RetentionActionResponse> => {
      const path = `/api/courses/${encodedId(courseId)}/retention/delete`;
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeRetentionActionResponse,
        {
          method: "POST",
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
    extendCourseRetention: async (courseId, request, revision): Promise<RetentionReadView> => {
      const path = `/api/courses/${encodedId(courseId)}/retention/extend`;
      const body: RetentionExtendRequest = decodeRetentionExtendRequest(request, "request");
      const result = await teachingJson(
        fetchImplementation,
        basePath,
        path,
        decodeRetentionReadView,
        {
          method: "PATCH",
          body,
          revision,
          expectedStatus: 200,
        },
      );
      verifyRevision(result.response, result.body.revision, path);
      return result.body;
    },
  };
}
