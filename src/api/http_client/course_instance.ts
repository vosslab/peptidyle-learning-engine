// Strict same-origin transport for Course Instance creation and Teaching Team reads.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import { decodeBlueprintCourseView } from "../decoders/blueprint_course";
import type { ApiClient } from "../client";
import type { CourseInstanceClient } from "../course_instance";
import { decodeCourseClassification } from "../decoders/course_classification";
import { decodeRecord, decodeString, DecodeError } from "../decoder";
import { field, requireOnlyFields } from "../decoders/shared";
import {
  decodeCourseCreationInstructors,
  decodeCourseInstanceList,
  decodeCourseInstanceRouteSummary,
  decodeCourseInstanceView,
  decodeCreateBlueprintFromCourseInstanceInput,
  decodeCreateCourseInstanceInput,
  decodeCreatedCourseInstance,
} from "../decoders/course_instance";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

const MAX_IDEMPOTENCY_KEY_BYTES = 128;

function idempotencyKey(value: string, path: string): string {
  if (
    value.length === 0 ||
    value.length > MAX_IDEMPOTENCY_KEY_BYTES ||
    !Array.from(value).every((character) => {
      const code = character.codePointAt(0);
      return code !== undefined && code >= 0x21 && code <= 0x7e;
    })
  ) {
    throw new ApiProtocolError(
      `API ${path} Idempotency-Key must be 1 through 128 visible ASCII bytes`,
    );
  }
  return value;
}

export function courseInstancePath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: positively validate, then path-encode route input.
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}`;
}

async function courseInstanceJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST";
    readonly body?: unknown;
    readonly status?: 200 | 201;
  } = {},
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

/** Composes this client capability without reusing stale generic Course endpoints. */
export function createCourseInstanceClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof CourseInstanceClient> {
  return {
    createBlueprintFromCourseInstance: async (
      courseInstanceId,
      input,
      requestKey,
    ): Promise<Awaited<ReturnType<CourseInstanceClient["createBlueprintFromCourseInstance"]>>> => {
      const path = `${courseInstancePath(courseInstanceId)}/course-blueprints`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        body: decodeCreateBlueprintFromCourseInstanceInput(input),
        headers: { "idempotency-key": idempotencyKey(requestKey, path) },
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 201)
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      const blueprintCourse = decodeBlueprintCourseView(
        await boundedResponseJson(response, path),
        "response",
      );
      const revisionEtag = response.headers.get("etag");
      if (revisionEtag !== `"${blueprintCourse.current_revision_tuple.revisionNumber}"`) {
        throw new ApiProtocolError(
          `API response ${path} ETag must match its current Blueprint Revision`,
        );
      }
      if (
        blueprintCourse.availability !== "private" ||
        blueprintCourse.read_access !== "blueprint_course_owner" ||
        blueprintCourse.fork_source_tuple !== null ||
        blueprintCourse.current_revision_tuple.blueprintCourseId !== blueprintCourse.id ||
        blueprintCourse.current_revision_tuple.revisionNumber !== "1"
      ) {
        throw new ApiProtocolError(
          "Course-derived Blueprint must be an actor-owned Private root at Revision 1",
        );
      }
      return { blueprintCourse, revisionEtag };
    },
    updateCourseInstanceClassification: async (
      courseInstanceId,
      classification,
      courseEditNumber,
    ): Promise<Awaited<ReturnType<CourseInstanceClient["updateCourseInstanceClassification"]>>> => {
      const path = `${courseInstancePath(courseInstanceId)}/classification`;
      const validator = decodeString(courseEditNumber, "courseEditNumber");
      if (!/^[1-9][0-9]*$/u.test(validator) || BigInt(validator) > 9_223_372_036_854_775_807n) {
        throw new ApiProtocolError("Course If-Match must be a positive Course Edit Number");
      }
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "PUT",
        body: decodeCourseClassification(classification, "request"),
        headers: { "if-match": `"${validator}"` },
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200)
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      const record = decodeRecord(await boundedResponseJson(response, path), "response");
      requireOnlyFields(record, "response", ["classification", "courseEditNumber", "changed"]);
      const nextEditNumber = decodeString(
        field(record, "courseEditNumber", "response"),
        "response.courseEditNumber",
      );
      if (
        !/^[1-9][0-9]*$/u.test(nextEditNumber) ||
        BigInt(nextEditNumber) > 9_223_372_036_854_775_807n
      ) {
        throw new DecodeError("response.courseEditNumber", "a positive Course Edit Number");
      }
      if (response.headers.get("etag") !== `"${nextEditNumber}"`)
        throw new ApiProtocolError("Course metadata response ETag must match its Edit Number");
      const changed = field(record, "changed", "response");
      if (typeof changed !== "boolean") throw new DecodeError("response.changed", "a boolean");
      return {
        classification: decodeCourseClassification(
          field(record, "classification", "response"),
          "response.classification",
        ),
        courseEditNumber: nextEditNumber,
        changed,
      };
    },
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
    getCourseInstance: (courseInstanceId) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        courseInstancePath(courseInstanceId),
        decodeCourseInstanceView,
      ),
    getCourseInstanceRouteSummary: (courseInstanceId) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        `${courseInstancePath(courseInstanceId)}/summary`,
        decodeCourseInstanceRouteSummary,
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
