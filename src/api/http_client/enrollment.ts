// Course roster and invitation transport.

import type { CourseRosterClient, RosterImportPreview } from "../enrollment";
import {
  decodeClaimedCourseInvitation,
  decodeCourseEnrollmentPolicyResult,
  decodeCourseInvitationAccepted,
  decodeCourseRosterPage,
  decodeRosterImportCommitResult,
  decodeRosterImportPreview,
  decodeRosterRevisionResult,
} from "../enrollment";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestJson, requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_ROSTER_CSV_BYTES = 1_048_576;

function positiveRevisionHeader(revision: number, name: string): string {
  if (!Number.isSafeInteger(revision) || revision <= 0) {
    throw new ApiProtocolError(`${name} must be a positive safe integer`);
  }
  return `"${revision}"`;
}

function idempotencyHeader(value: string): string {
  if (value.length === 0 || value.length > 128 || /[^\x21-\x7e]/u.test(value)) {
    throw new ApiProtocolError("idempotency key must contain 1 through 128 visible ASCII bytes");
  }
  return value;
}

function verifyNumericEtag(response: Response, expected: number, path: string): void {
  if (response.headers.get("etag") !== `"${expected}"`) {
    throw new ApiProtocolError(`API response ${path} ETag does not match its revision`);
  }
}

async function noContentRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
): Promise<void> {
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "DELETE",
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204) {
    throw new ApiProtocolError(`API response ${path} must use status 204`);
  }
}

async function rosterMutation<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method: "POST" | "PUT" | "DELETE";
    readonly body?: unknown;
    readonly headers?: Readonly<Record<string, string>>;
  },
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {
    accept: "application/json",
    ...options.headers,
  };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method,
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return { body: decoder(await boundedResponseJson(response, path)), response };
}

export function createEnrollmentClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): CourseRosterClient {
  return {
    redeemCourseInvitation: (invitationToken) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/course-invitations/redeem",
        decodeClaimedCourseInvitation,
        { method: "POST", body: { invitationToken } },
      ),
    listCourseRoster: (courseId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}/roster${
          cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`
        }`,
        decodeCourseRosterPage,
      ),
    inviteCourseMember: async (
      courseId,
      email,
      rosterId,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["inviteCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations`;
      return (
        await rosterMutation(fetchImplementation, basePath, path, decodeCourseInvitationAccepted, {
          method: "POST",
          headers: { "idempotency-key": idempotencyHeader(idempotencyKey) },
          body: { email, rosterId },
        })
      ).body;
    },
    revokeCourseInvitation: async (
      courseId,
      invitationId,
      rosterRevision,
    ): ReturnType<CourseRosterClient["revokeCourseInvitation"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations/${encodedId(invitationId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterRevisionResult,
        {
          method: "DELETE",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    revokeCourseMember: async (
      courseId,
      memberId,
      rosterRevision,
    ): ReturnType<CourseRosterClient["revokeCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/members/${encodedId(memberId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterRevisionResult,
        {
          method: "DELETE",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    replaceCourseEnrollmentPolicy: async (
      courseId,
      policy,
      rosterRevision,
    ): ReturnType<CourseRosterClient["replaceCourseEnrollmentPolicy"]> => {
      const path = `/api/courses/${encodedId(courseId)}/enrollment-policy`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeCourseEnrollmentPolicyResult,
        {
          method: "PUT",
          headers: { "if-match": positiveRevisionHeader(rosterRevision, "roster revision") },
          body: policy,
        },
      );
      verifyNumericEtag(result.response, result.body.rosterRevision, path);
      return result.body;
    },
    previewRosterImport: async (
      courseId,
      csv,
      rosterRevision,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["previewRosterImport"]> => {
      if (csv.size <= 0) throw new ApiProtocolError("Roster CSV is empty");
      if (csv.size > MAX_ROSTER_CSV_BYTES) throw new ApiProtocolError("Roster CSV exceeds 1 MiB");
      const path = `/api/courses/${encodedId(courseId)}/roster-imports/preview`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "text/csv; charset=utf-8",
          "idempotency-key": idempotencyHeader(idempotencyKey),
          "if-match": positiveRevisionHeader(rosterRevision, "roster revision"),
        },
        body: csv,
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const preview = decodeRosterImportPreview(await boundedResponseJson(response, path));
      verifyNumericEtag(response, preview.importRevision, path);
      return preview;
    },
    commitRosterImport: async (
      courseId,
      preview,
      rowNumbers,
      idempotencyKey,
    ): ReturnType<CourseRosterClient["commitRosterImport"]> => {
      const path = `/api/courses/${encodedId(courseId)}/roster-imports/${encodedId(preview.importId)}/commit`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeRosterImportCommitResult,
        {
          method: "POST",
          headers: {
            "idempotency-key": idempotencyHeader(idempotencyKey),
            "if-match": positiveRevisionHeader(preview.importRevision, "roster import revision"),
          },
          body: { rowNumbers },
        },
      );
      if (result.body.importId !== preview.importId) {
        throw new ApiProtocolError("Roster import commit does not match its preview");
      }
      verifyNumericEtag(result.response, result.body.importRevision, path);
      return result.body;
    },
  };
}

export function readyRosterRows(preview: RosterImportPreview): ReadonlyArray<number> {
  return preview.rows.filter((row) => row.status === "readyToInvite").map((row) => row.rowNumber);
}

export function newIdempotencyKey(): string {
  if (!("randomUUID" in crypto)) {
    throw new ApiProtocolError("This browser cannot create an idempotency key");
  }
  return crypto.randomUUID();
}
