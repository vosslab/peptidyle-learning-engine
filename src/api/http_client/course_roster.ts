// Strict same-origin transport for M9 Course Roster routes.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { LiveCourseRosterClient } from "../course_roster";
import {
  decodeClaimedCourseInvitation,
  decodeCourseRoster,
  decodeCourseRosterImportInput,
} from "../decoders/course_roster";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function courseRosterPath(course: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/roster`;
}

async function rosterJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: { readonly method?: "GET" | "POST"; readonly body?: unknown; readonly status?: number } = {},
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    body: options.body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.status !== undefined && response.status !== options.status) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.status}`);
  }
  return decoder(await boundedResponseJson(response, path), "response");
}

/** Composes M9 separately from the legacy generic Course Roster client. */
export function createLiveCourseRosterClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveCourseRosterClient> {
  return {
    getLiveCourseRoster: (course) =>
      rosterJson(fetchImplementation, basePath, courseRosterPath(course), decodeCourseRoster),
    importLiveCourseRoster: (course, input) =>
      rosterJson(fetchImplementation, basePath, courseRosterPath(course), decodeCourseRoster, {
        method: "POST",
        body: decodeCourseRosterImportInput(input),
        status: 201,
      }),
    claimLiveCourseInvitation: (course) =>
      rosterJson(
        fetchImplementation,
        basePath,
        `${courseRosterPath(course)}/claim`,
        decodeClaimedCourseInvitation,
        { method: "POST" },
      ),
    revokeLiveCourseRosterEntry: async (course, rosterId) => {
      if (!/^[A-Za-z0-9._-]{1,64}$/u.test(rosterId)) {
        throw new ApiProtocolError("Course roster identifier must be canonical");
      }
      const path = `${courseRosterPath(course)}/${encodeURIComponent(rosterId)}/revoke`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, { method: "POST" });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 204) {
        throw new ApiProtocolError(`API response ${path} must use status 204`);
      }
    },
  };
}
