// Strict same-origin transport for Course Instance creation and Teaching Team reads.

import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import { decodeBlueprintCourseView } from "../decoders/blueprint_course";
import type { ApiClient } from "../client";
import type { CourseInstanceClient } from "../course_instance";
import { decodeCourseClassification } from "../decoders/course_classification";
import { decodeRecord, decodeUuid, DecodeError } from "../decoder";
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
import { parseCourseInstanceReference } from "../../navigation/public_route";

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

export function courseInstancePath(reference: CourseInstanceReference): string {
  if (parseCourseInstanceReference(reference) === null) {
    throw new ApiProtocolError("Course Instance reference must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: positively validate, then path-encode route input.
  return `/api/course-instances/${encodeURIComponent(reference)}`;
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
      reference,
      input,
      requestKey,
    ): Promise<Awaited<ReturnType<CourseInstanceClient["createBlueprintFromCourseInstance"]>>> => {
      const path = `${courseInstancePath(reference)}/course-blueprints`;
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
      if (revisionEtag !== `"${blueprintCourse.current_revision.revision}"`) {
        throw new ApiProtocolError(
          `API response ${path} ETag must match its current Blueprint Revision`,
        );
      }
      if (
        blueprintCourse.availability !== "private" ||
        blueprintCourse.read_access !== "blueprint_course_owner" ||
        blueprintCourse.fork_source !== null ||
        blueprintCourse.current_revision.reference !== blueprintCourse.reference ||
        blueprintCourse.current_revision.revision !== "1"
      ) {
        throw new ApiProtocolError(
          "Course-derived Blueprint must be an actor-owned Private root at Revision 1",
        );
      }
      return { blueprintCourse, revisionEtag };
    },
    updateCourseInstanceClassification: async (
      reference,
      classification,
      metadataEtag,
    ): Promise<Awaited<ReturnType<CourseInstanceClient["updateCourseInstanceClassification"]>>> => {
      const path = `${courseInstancePath(reference)}/classification`;
      const validator = decodeUuid(metadataEtag, "metadataEtag");
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
      requireOnlyFields(record, "response", ["classification", "metadataEtag", "changed"]);
      const nextEtag = decodeUuid(
        field(record, "metadataEtag", "response"),
        "response.metadataEtag",
      );
      if (response.headers.get("etag") !== `"${nextEtag}"`)
        throw new ApiProtocolError("Course metadata response ETag must match its validator");
      const changed = field(record, "changed", "response");
      if (typeof changed !== "boolean") throw new DecodeError("response.changed", "a boolean");
      return {
        classification: decodeCourseClassification(
          field(record, "classification", "response"),
          "response.classification",
        ),
        metadataEtag: nextEtag,
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
    getCourseInstance: (reference) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        courseInstancePath(reference),
        decodeCourseInstanceView,
      ),
    getCourseInstanceRouteSummary: (reference) =>
      courseInstanceJson(
        fetchImplementation,
        basePath,
        `${courseInstancePath(reference)}/summary`,
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
