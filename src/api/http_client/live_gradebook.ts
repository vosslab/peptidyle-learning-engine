// Same-origin transport for the focused Gradebook projection.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type { CourseGradebook, CourseGradebookClient } from "../live_gradebook";
import { decodeCourseGradebook } from "../decoders/live_gradebook";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

function gradebookPath(course: CourseInstanceId): string {
  if (parseCourseInstanceId(course) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(course)}/gradebook`;
}

/** Composes only the registered answer-free Gradebook handler. */
export function createCourseGradebookClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof CourseGradebookClient> {
  return {
    downloadCourseGradebook: async (course, format): Promise<Blob> => {
      const path = `${gradebookPath(course)}/export?format=${format}`;
      if (format !== "csv" && format !== "tsv") {
        throw new ApiProtocolError("Gradebook export format must be csv or tsv");
      }
      const mediaType =
        format === "csv" ? "text/csv; charset=utf-8" : "text/tab-separated-values; charset=utf-8";
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        headers: { accept: mediaType },
      });
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      requireNoStore(response, path);
      // ASVS 3.2.1/5.4.1/14.3.2: accept only the protected attachment contract.
      if (
        response.headers.get("content-type") !== mediaType ||
        response.headers.get("content-disposition") !==
          `attachment; filename="ple_${course}_grades.${format}"` ||
        response.headers.get("x-content-type-options") !== "nosniff"
      ) {
        throw new ApiProtocolError(
          `API response ${path} must use the Gradebook attachment contract`,
        );
      }
      return response.blob();
    },
    getCourseGradebook: async (course): Promise<CourseGradebook> => {
      const path = gradebookPath(course);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeCourseGradebook(await boundedResponseJson(response, path), "response");
    },
  };
}
