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
  pageNumber: number,
  pageSize: number,
  facet?: {
    readonly field: StudentAttemptHistoryFacetField;
    readonly search: string;
    readonly cursor?: string;
  },
): string {
  if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > (facet ? 50 : 100)) {
    throw new ApiProtocolError("Attempt History page size must be between 1 and 100");
  }
  if (!Number.isSafeInteger(pageNumber) || pageNumber < 1) {
    throw new ApiProtocolError("Attempt History page number is invalid");
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
    ...(facet === undefined ? { pageNumber: String(pageNumber), pageSize: String(pageSize) } : {}),
    search,
    state: request.state,
    sort: request.sort,
  });
  if (request.courseIds.length > 0) query.set("courses", request.courseIds.join(","));
  if (facet !== undefined) {
    query.set("field", facet.field);
    query.set("choiceSearch", facet.search);
    if (facet.cursor !== undefined) query.set("cursor", facet.cursor);
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
      pageNumber = 1,
      pageSize = 50,
    ): Promise<StudentAttemptHistoryPage> => {
      const path = historyPath(query, pageNumber, pageSize);
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
      const path = historyPath(query, 1, 50, {
        field,
        search: choiceSearch.trim(),
        ...(cursor === undefined ? {} : { cursor }),
      });
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
