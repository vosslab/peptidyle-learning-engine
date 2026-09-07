// Strict same-origin transport for M8 Course Instance creation and teaching-team reads.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { ApiClient } from "../client";
import type { CourseInstanceClient } from "../course_instance";
import {
  decodeCourseCreationInstructors,
  decodeCourseInstanceList,
  decodeCourseInstanceView,
  decodeCreateCourseInstanceInput,
  decodeCreatedCourseInstance,
} from "../decoders/course_instance";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function courseInstancePath(reference: CourseInstanceReference): string {
  if (!/^C-[1-9][0-9]{0,9}$/u.test(reference) || Number(reference.slice(2)) > 2_147_483_647) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  return `/api/course-instances/${encodeURIComponent(reference)}`;
}

async function courseInstanceJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: { readonly method?: "GET" | "POST"; readonly body?: unknown; readonly status?: 200 | 201 } = {},
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

/** Composes the independent M8 client capability without reusing stale generic course endpoints. */
export function createCourseInstanceClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof CourseInstanceClient> {
  return {
    listCourseInstances: () =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        "/api/course-instances",
        decodeCourseInstanceList,
      ),
    createCourseInstance: (input) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        "/api/course-instances",
        decodeCreatedCourseInstance,
        {
          method: "POST",
          body: decodeCreateCourseInstanceInput(input),
          status: 201,
        },
      ),
    getCourseInstance: (reference) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        courseInstancePath(reference),
        decodeCourseInstanceView,
      ),
    listCourseCreationInstructors: () =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        "/api/course-instance-creation/instructors",
        decodeCourseCreationInstructors,
      ),
  };
}
