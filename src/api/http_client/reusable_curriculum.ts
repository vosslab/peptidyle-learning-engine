// Strict same-origin transport for instructor reusable curricula.

import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { CreateBlueprintCourseDefinitionInput } from "../../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReplaceBlueprintCourseDefinitionInput } from "../../../generated/api/ReplaceBlueprintCourseDefinitionInput";
import type { ApiClient } from "../client";
import type { CursorPage } from "../contracts";
import {
  decodeBlueprintCoursePage,
  decodeBlueprintCourseView,
  decodeCreateBlueprintCourseDefinitionInput,
  decodeBlueprintCourseReference,
  decodeReplaceBlueprintCourseDefinitionInput,
} from "../decoders/reusable_curriculum";
import type {
  ReusableCurriculumClient,
  ReusableCurriculumEtag,
  RevisionedBlueprintCourse,
} from "../reusable_curriculum";
import { ApiProtocolError, ApiRequestError, ReusableCurriculumConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;

function pagePath(path: string, cursor: string | undefined, pageSize: number | undefined): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError("curriculum page size must be an integer from 1 through 100");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", cursor);
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

function parseStrongEtag(value: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong positive numeric validator`);
  }
  return value;
}

function requireMatchingEtag(response: Response, revision: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || parseStrongEtag(etag, path) !== `"${revision}"`) {
    throw new ApiProtocolError(`API response ${path} ETag must match its revision`);
  }
  return etag;
}

function requestRevision(value: ReusableCurriculumEtag, path: string): string {
  return parseStrongEtag(value, `${path} If-Match`);
}

function blueprintPath(value: BlueprintCourseReference): string {
  const reference = decodeBlueprintCourseReference(value, "blueprint");
  return `/api/course-blueprints/${encodeURIComponent(reference)}`;
}

async function curriculumJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: ReusableCurriculumEtag;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) headers["if-match"] = requestRevision(options.etag, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ReusableCurriculumConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  }
  const body = decoder(await boundedResponseJson(response, path), "response");
  return { body, response };
}

async function deleteCurriculum(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  etag: ReusableCurriculumEtag,
): Promise<void> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "DELETE",
    headers: { "if-match": requestRevision(etag, path) },
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new ReusableCurriculumConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204 || (await response.text()).length !== 0) {
    throw new ApiProtocolError(`API response ${path} must use an empty 204 response`);
  }
}

/** Creates the complete reusable-curriculum capability without coupling it to a screen model. */
export function createReusableCurriculumClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof ReusableCurriculumClient> {
  return {
    listBlueprintCourses: async (
      cursor,
      pageSize,
    ): Promise<CursorPage<BlueprintCourseSummaryView>> => {
      const path = pagePath("/api/course-blueprints", cursor, pageSize);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCoursePage,
      );
      return result.body;
    },
    getBlueprintCourse: async (reference): Promise<RevisionedBlueprintCourse> => {
      const path = blueprintPath(reference);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
      );
      return {
        blueprintCourse: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    createBlueprintCourse: async (
      definition: CreateBlueprintCourseDefinitionInput,
    ): Promise<RevisionedBlueprintCourse> => {
      const path = "/api/course-blueprints";
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "POST",
          body: decodeCreateBlueprintCourseDefinitionInput(definition),
          expectedStatus: 201,
        },
      );
      return {
        blueprintCourse: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    replaceBlueprintCourse: async (
      reference,
      definition: ReplaceBlueprintCourseDefinitionInput,
      etag,
    ): Promise<RevisionedBlueprintCourse> => {
      const path = blueprintPath(reference);
      const result = await curriculumJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "PUT",
          body: decodeReplaceBlueprintCourseDefinitionInput(definition),
          etag,
          expectedStatus: 200,
        },
      );
      return {
        blueprintCourse: result.body,
        etag: requireMatchingEtag(result.response, result.body.revision, path),
      };
    },
    deleteBlueprintCourse: (reference, etag) =>
      deleteCurriculum(fetchImplementation, basePath, blueprintPath(reference), etag),
  };
}
