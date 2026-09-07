import type { ApiClient } from "../client";
import type { CourseRosterEntry } from "../course_roster";
import type { SupportCapabilityClient } from "../support_roster";
import { decodeCourseRoster } from "../decoders/course_roster";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export function createSupportCapabilityClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof SupportCapabilityClient> {
  return {
    readSupportCourseRoster: async (capabilityId): Promise<ReadonlyArray<CourseRosterEntry>> => {
      if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(capabilityId))
        throw new ApiProtocolError("Support capability identity must be canonical");
      const path = `/api/support-capabilities/${capabilityId}/course-roster`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeCourseRoster(await boundedResponseJson(response, path));
    },
  };
}
