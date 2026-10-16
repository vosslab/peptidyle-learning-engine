// Same-origin transport for the Student's global Attempt History.

import type { ApiClient } from "../client";
import type {
  StudentAttemptHistoryFacetField,
  StudentAttemptHistoryFacetPage,
  StudentAttemptHistoryQuery,
  StudentAttemptHistoryClient,
  StudentAttemptHistoryPage,
} from "../student_course_attempt_history";
import {
  decodeStudentAttemptHistoryFacetPage,
  decodeStudentAttemptHistoryPage,
} from "../decoders/student_course_attempt_history";
import { parseCourseInstanceId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function historyPath(
  request: StudentAttemptHistoryQuery,
  pageSize: number,
  cursor?: string,
  facet?: { readonly field: StudentAttemptHistoryFacetField; readonly search: string },
): string {
  if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > (facet ? 50 : 100)) {
    throw new ApiProtocolError("Attempt History page size must be between 1 and 100");
  }
  if (cursor !== undefined && (cursor.length === 0 || cursor.length > 512)) {
    throw new ApiProtocolError("Attempt History cursor is invalid");
  }
  const search = request.search.trim();
  if (
    [...search].length > 200 ||
    request.courseIds.length > 100 ||
    request.courseIds.some((course) => parseCourseInstanceId(course) === null)
  ) {
    throw new ApiProtocolError("Attempt History query is invalid");
  }
  const query = new URLSearchParams({
    pageSize: String(pageSize),
    search,
    state: request.state,
    sort: request.sort,
  });
  if (request.courseIds.length > 0) query.set("courses", request.courseIds.join(","));
  if (cursor !== undefined) query.set("cursor", cursor);
  if (facet !== undefined) {
    query.set("field", facet.field);
    query.set("choiceSearch", facet.search);
  }
  const base =
    facet === undefined
      ? "/api/student/assessment-attempts"
      : "/api/student/assessment-attempts/facets";
  return `${base}?${query}`;
}

export function createStudentAttemptHistoryClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof StudentAttemptHistoryClient> {
  return {
    listStudentAttemptHistory: async (
      query,
      pageSize = 50,
      cursor,
    ): Promise<StudentAttemptHistoryPage> => {
      const path = historyPath(query, pageSize, cursor);
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeStudentAttemptHistoryPage(await boundedResponseJson(response, path), "response");
    },
    listStudentAttemptHistoryFacetChoices: async (
      query,
      field,
      choiceSearch,
      cursor,
    ): Promise<StudentAttemptHistoryFacetPage> => {
      if ([...choiceSearch].length > 100) {
        throw new ApiProtocolError("Attempt History facet search is invalid");
      }
      const path = historyPath(query, 50, cursor, { field, search: choiceSearch.trim() });
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      return decodeStudentAttemptHistoryFacetPage(
        await boundedResponseJson(response, path),
        "response",
      );
    },
  };
}
