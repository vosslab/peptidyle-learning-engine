// Course roster and invitation transport.

import type { CourseRosterChangeNumber } from "../../../generated/api/CourseRosterChangeNumber";
import type { CourseRosterClient, RosterImportPreview } from "../enrollment";
import {
  decodeClaimedCourseInvitation,
  decodeCourseInvitationEmailRule,
  decodeCourseInvitationAccepted,
  decodeCourseRosterPage,
  decodeRosterImportCommitResult,
  decodeRosterImportPreview,
  decodeCourseRosterChangeNumberResult,
} from "../enrollment";
import { ApiProtocolError, ApiRequestError } from "./error";
import { encodedId, requestJson, requestPath, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_ROSTER_CSV_BYTES = 1_048_576;

function rosterChangeNumberHeader(
  rosterChangeNumber: CourseRosterChangeNumber,
  name: string,
): string {
  if (
    !/^[1-9][0-9]{0,18}$/u.test(rosterChangeNumber) ||
    BigInt(rosterChangeNumber) > 9_223_372_036_854_775_807n
  ) {
    throw new ApiProtocolError(`${name} must be a canonical positive PostgreSQL bigint decimal`);
  }
  return `"${rosterChangeNumber}"`;
}

function positiveRevisionHeader(revision: number, name: string): string {
  if (!Number.isSafeInteger(revision) || revision <= 0) {
    throw new ApiProtocolError(`${name} must be a positive safe integer`);
  }
  return `"${revision}"`;
}

function verifyRosterChangeNumberEtag(
  response: Response,
  expected: CourseRosterChangeNumber,
  path: string,
): void {
  if (
    response.headers.get("etag") !==
    rosterChangeNumberHeader(expected, "response roster change number")
  ) {
    throw new ApiProtocolError(`API response ${path} ETag does not match its roster change number`);
  }
}

function verifyNumericEtag(response: Response, expected: number, path: string): void {
  if (response.headers.get("etag") !== `"${expected}"`) {
    throw new ApiProtocolError(`API response ${path} ETag does not match its revision`);
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

export function createCourseRosterClient(
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
    ): ReturnType<CourseRosterClient["inviteCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations`;
      return (
        await rosterMutation(fetchImplementation, basePath, path, decodeCourseInvitationAccepted, {
          method: "POST",
          body: { email, rosterId },
        })
      ).body;
    },
    revokeCourseInvitation: async (
      courseId,
      invitationId,
      rosterChangeNumber,
    ): ReturnType<CourseRosterClient["revokeCourseInvitation"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitations/${encodedId(invitationId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeCourseRosterChangeNumberResult,
        {
          method: "DELETE",
          headers: {
            "if-match": rosterChangeNumberHeader(rosterChangeNumber, "roster change number"),
          },
        },
      );
      verifyRosterChangeNumberEtag(result.response, result.body.rosterChangeNumber, path);
      return result.body;
    },
    revokeCourseMember: async (
      courseId,
      memberId,
      rosterChangeNumber,
    ): ReturnType<CourseRosterClient["revokeCourseMember"]> => {
      const path = `/api/courses/${encodedId(courseId)}/members/${encodedId(memberId)}`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeCourseRosterChangeNumberResult,
        {
          method: "DELETE",
          headers: {
            "if-match": rosterChangeNumberHeader(rosterChangeNumber, "roster change number"),
          },
        },
      );
      verifyRosterChangeNumberEtag(result.response, result.body.rosterChangeNumber, path);
      return result.body;
    },
    replaceCourseInvitationEmailRule: async (
      courseId,
      policy,
      rosterChangeNumber,
    ): ReturnType<CourseRosterClient["replaceCourseInvitationEmailRule"]> => {
      const path = `/api/courses/${encodedId(courseId)}/invitation-email-rule`;
      const result = await rosterMutation(
        fetchImplementation,
        basePath,
        path,
        decodeCourseInvitationEmailRule,
        {
          method: "PUT",
          headers: {
            "if-match": rosterChangeNumberHeader(rosterChangeNumber, "roster change number"),
          },
          body: policy,
        },
      );
      verifyRosterChangeNumberEtag(result.response, result.body.rosterChangeNumber, path);
      return result.body;
    },
    previewRosterImport: async (
      courseId,
      csv,
      rosterChangeNumber,
    ): ReturnType<CourseRosterClient["previewRosterImport"]> => {
      if (csv.size <= 0) throw new ApiProtocolError("Roster CSV is empty");
      if (csv.size > MAX_ROSTER_CSV_BYTES) throw new ApiProtocolError("Roster CSV exceeds 1 MiB");
      const path = `/api/courses/${encodedId(courseId)}/roster-imports/preview`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "text/csv; charset=utf-8",
          "if-match": rosterChangeNumberHeader(rosterChangeNumber, "roster change number"),
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
  return preview.rows.filter((row) => row.result === "readyToInvite").map((row) => row.rowNumber);
}
