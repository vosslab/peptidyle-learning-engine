// Same-origin transport for the focused Gradebook projection.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { LiveDemoGradebook, LiveDemoGradebookClient } from "../live_gradebook";
import { decodeLiveDemoGradebook } from "../decoders/live_gradebook";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function gradebookPath(course: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || Number(course.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/gradebook`;
}

/** Composes only the registered answer-free Gradebook handler. */
export function createLiveDemoGradebookClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LiveDemoGradebookClient> {
  return {
    getLiveDemoGradebook: async (course): Promise<LiveDemoGradebook> => {
      const path = gradebookPath(course);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeLiveDemoGradebook(await boundedResponseJson(response, path), "response");
    },
  };
}
