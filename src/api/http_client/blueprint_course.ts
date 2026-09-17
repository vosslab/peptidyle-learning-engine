// Strict same-origin transport for Blueprint lineage metadata and immutable Revisions.

import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintPoolMembersView } from "../../../generated/api/BlueprintPoolMembersView";
import { decodeBlueprintPoolMembersView } from "../decoders/blueprint_pool_members";
import {
  decodeBlueprintComparisonView,
  decodeCanonicalBlueprintCourse,
} from "../decoders/blueprint_comparison";
import { decodeBlueprintHistoryPageView } from "../decoders/blueprint_history";
import { decodeUuid } from "../decoder";
import { decodeCourseClassification } from "../decoders/course_classification";
import { decodeCursor, decodeQuestionId } from "../decoders/shared";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseSaveResponse } from "../../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintMetadataState } from "../../../generated/api/BlueprintMetadataState";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintHistoryPageView } from "../../../generated/api/BlueprintHistoryPageView";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintKnownForkView } from "../../../generated/api/BlueprintKnownForkView";
import type { BlueprintForkApplyResponse } from "../../../generated/api/BlueprintForkApplyResponse";
import type { CanonicalBlueprintCourse } from "../../../generated/api/CanonicalBlueprintCourse";
import {
  decodeBlueprintForkApplyRequest,
  decodeBlueprintForkApplyResponse,
} from "../decoders/blueprint_fork_apply";
import type { ApiClient } from "../client";
import type { CursorPage } from "../contracts";
import {
  decodeBlueprintCoursePage,
  decodeKnownBlueprintForks,
  decodeBlueprintCourseReference,
  decodeBlueprintCourseSaveResponse,
  decodeBlueprintCourseView,
  decodeBlueprintMetadataState,
  decodeBlueprintRevision,
  decodeBlueprintRevisionView,
  decodeCreateBlueprintCourseInput,
  decodeRenameBlueprintCourseInput,
  decodeReplaceBlueprintCourseContentInput,
} from "../decoders/blueprint_course";
import type {
  BlueprintCourseClient,
  BlueprintCourseClassificationSearch,
  BlueprintIdempotencyKey,
  BlueprintMetadataTransition,
  LoadedBlueprintCourse,
  BlueprintRevisionEtag,
} from "../blueprint_course";
import { ApiProtocolError, ApiRequestError, BlueprintCourseConflictError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const MAX_PAGE_SIZE = 100;
const MAX_IDEMPOTENCY_KEY_BYTES = 128;
const MAX_BLUEPRINT_RESPONSE_CHARACTERS = 16 * 1_024 * 1_024;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

function pagePath(
  path: string,
  cursor: string | undefined,
  pageSize: number | undefined,
  includeArchived: boolean,
  searchQuery?: string,
  publicOnly = false,
  promotedOnly = false,
  classification?: BlueprintCourseClassificationSearch,
): string {
  if (
    pageSize !== undefined &&
    (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > MAX_PAGE_SIZE)
  ) {
    throw new ApiProtocolError("Blueprint Course page size must be an integer from 1 through 100");
  }
  // ASVS 2.2.1, 2.2.2: mirror service limits for useful client feedback; server remains authoritative.
  if (
    searchQuery !== undefined &&
    (Array.from(searchQuery).length > 256 || /\p{Cc}/u.test(searchQuery))
  ) {
    throw new ApiProtocolError(
      "Blueprint Course search must be at most 256 characters without control characters",
    );
  }
  if (publicOnly && includeArchived) {
    throw new ApiProtocolError("Public Blueprint Course search cannot include Archived courses");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", cursor);
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  if (includeArchived) query.set("includeArchived", "true");
  // ASVS 1.2.2: encode literal search text as a query value, never URL syntax.
  if (searchQuery !== undefined) query.set("query", searchQuery);
  if (publicOnly) query.set("publicOnly", "true");
  if (promotedOnly) query.set("promotedOnly", "true");
  if (classification !== undefined) {
    // ASVS 2.2.1-2.2.3: validate UUIDs and complete chains; service validates real parents.
    const { disciplineUuid, subjectUuid, topicUuid, subtopicUuid, crossDiscipline } =
      classification;
    if (
      typeof crossDiscipline !== "boolean" ||
      (subjectUuid !== null && disciplineUuid === null) ||
      (topicUuid !== null && subjectUuid === null) ||
      (subtopicUuid !== null && topicUuid === null) ||
      (crossDiscipline && (disciplineUuid === null || subjectUuid === null))
    )
      throw new ApiProtocolError(
        "Blueprint classification search requires a complete parent chain",
      );
    for (const [key, value] of Object.entries({
      disciplineUuid,
      subjectUuid,
      topicUuid,
      subtopicUuid,
    })) {
      // ASVS 1.2.2: validated identities still use query-context encoding.
      if (value !== null) query.set(key, decodeUuid(value, key));
    }
    if (crossDiscipline) query.set("crossDiscipline", "true");
  }
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `${path}${suffix}`;
}

function parseRevisionEtag(value: string, path: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value) || BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong positive Revision validator`);
  }
  return value;
}

function parseMetadataEtag(value: string, path: string): string {
  const unquoted = /^"(.+)"$/u.exec(value)?.[1];
  if (unquoted === undefined || !UUID.test(unquoted)) {
    throw new ApiProtocolError(`API ${path} ETag must be one strong opaque metadata validator`);
  }
  return value;
}

function metadataEtag(value: string, path: string): string {
  if (!UUID.test(value)) {
    throw new ApiProtocolError(`API ${path} metadata ETag must be a canonical UUID`);
  }
  return `"${value}"`;
}

function idempotencyKey(value: BlueprintIdempotencyKey, path: string): string {
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

function blueprintPath(value: BlueprintCourseReference): string {
  return `/api/course-blueprints/${encodeURIComponent(decodeBlueprintCourseReference(value, "blueprint"))}`;
}

async function blueprintJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: {
    readonly method?: "GET" | "POST" | "PUT";
    readonly body?: unknown;
    readonly etag?: string;
    readonly parseEtag?: (value: string, path: string) => string;
    readonly idempotencyKey?: BlueprintIdempotencyKey;
    readonly expectedStatus?: 200 | 201;
  } = {},
): Promise<{ readonly body: T; readonly response: Response }> {
  const headers: Record<string, string> = {};
  if (options.etag !== undefined) {
    headers["if-match"] = (options.parseEtag ?? parseRevisionEtag)(
      options.etag,
      `${path} If-Match`,
    );
  }
  if (options.idempotencyKey !== undefined)
    headers["idempotency-key"] = idempotencyKey(options.idempotencyKey, path);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: options.method ?? "GET",
    headers,
    body: options.body,
  });
  requireNoStore(response, path);
  if (response.status === 412) throw new BlueprintCourseConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (options.expectedStatus !== undefined && response.status !== options.expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${options.expectedStatus}`);
  }
  return {
    body: decoder(
      await boundedResponseJson(response, path, MAX_BLUEPRINT_RESPONSE_CHARACTERS),
      "response",
    ),
    response,
  };
}

function requireRevisionEtag(response: Response, revision: string, path: string): string {
  const etag = response.headers.get("etag");
  if (etag === null || parseRevisionEtag(etag, path) !== `"${revision}"`) {
    throw new ApiProtocolError(
      `API response ${path} ETag must match its current Blueprint Revision`,
    );
  }
  return etag;
}

function requireMetadataEtag(
  response: Response,
  state: BlueprintMetadataState,
  path: string,
): string {
  const etag = response.headers.get("etag");
  const expected = metadataEtag(state.metadata_etag, path);
  if (etag === null || parseMetadataEtag(etag, path) !== expected) {
    throw new ApiProtocolError(
      `API response ${path} ETag must match its opaque metadata validator`,
    );
  }
  return etag;
}

function loadedBlueprintCourse(
  body: BlueprintCourseView,
  response: Response,
  path: string,
): LoadedBlueprintCourse {
  return {
    blueprintCourse: body,
    revisionEtag: requireRevisionEtag(response, body.current_revision.revision, path),
  };
}

function metadataTransition(
  metadata: BlueprintMetadataState,
  response: Response,
  path: string,
): BlueprintMetadataTransition {
  return { metadata, metadataEtag: requireMetadataEtag(response, metadata, path) };
}

/** Creates the complete Blueprint Course capability without coupling it to a screen model. */
export function createBlueprintCourseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof BlueprintCourseClient> {
  return {
    exportBlueprintCourse: async (reference): Promise<CanonicalBlueprintCourse> => {
      const path = `${blueprintPath(reference)}/export`;
      return (
        await blueprintJson(fetchImplementation, basePath, path, decodeCanonicalBlueprintCourse, {
          expectedStatus: 200,
        })
      ).body;
    },
    importBlueprintCourse: async (exchange, requestKey): Promise<LoadedBlueprintCourse> => {
      const path = "/api/course-blueprints/import";
      // Decode before dispatch as well as on receipt: imported JSON remains untrusted browser input.
      const body = decodeCanonicalBlueprintCourse(exchange, "request");
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "POST",
          body,
          idempotencyKey: requestKey,
          expectedStatus: 201,
        },
      );
      if (
        result.body.availability !== "private" ||
        result.body.read_access !== "blueprint_course_owner" ||
        result.body.fork_source !== null ||
        result.body.current_revision.revision !== "1"
      )
        throw new ApiProtocolError(
          "Imported Blueprint Course must be an actor-owned Private root at Revision 1",
        );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    listBlueprintHistory: async (
      reference,
      kind = "revisions",
      cursor,
      pageSize = 50,
    ): Promise<BlueprintHistoryPageView> => {
      if (kind !== "revisions" && kind !== "metadata")
        throw new ApiProtocolError("Blueprint history kind must be revisions or metadata");
      const path = `${pagePath(`${blueprintPath(reference)}/history`, cursor === undefined ? undefined : decodeCursor(cursor, "cursor"), pageSize, false)}&kind=${kind}`;
      const body = (
        await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintHistoryPageView, {
          expectedStatus: 200,
        })
      ).body;
      if (
        body.items.length > pageSize ||
        body.items.some(
          (item) => item.kind !== (kind === "revisions" ? "savedRevision" : "metadataChange"),
        )
      )
        throw new ApiProtocolError("Blueprint history must match its requested kind and page size");
      return body;
    },
    forkBlueprintCourse: async (
      reference,
      revision,
      requestKey,
    ): Promise<LoadedBlueprintCourse> => {
      // ASVS 1.2.2, 2.2.1: validate and encode the exact immutable source identity.
      const path = `${blueprintPath(reference)}/revisions/${encodeURIComponent(decodeBlueprintRevision(revision, "revision"))}/fork`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        { method: "POST", idempotencyKey: requestKey, expectedStatus: 201 },
      );
      if (result.body.reference === reference || result.body.availability !== "private") {
        throw new ApiProtocolError(
          "Blueprint fork response must identify an independent Private Blueprint Course",
        );
      }
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    applyBlueprintFork: async (reference, request): Promise<BlueprintForkApplyResponse> => {
      const path = `${blueprintPath(reference)}/fork-update`;
      const body = decodeBlueprintForkApplyRequest(request);
      if (body.expectedFork.reference !== reference)
        throw new ApiProtocolError("Blueprint fork update must target its expected fork reference");
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintForkApplyResponse,
        {
          method: "POST",
          body,
          expectedStatus: 200,
        },
      );
      if (result.body.blueprintRevision.reference !== reference)
        throw new ApiProtocolError(
          "Blueprint fork update response must identify the requested fork",
        );
      requireRevisionEtag(result.response, result.body.blueprintRevision.revision, path);
      return result.body;
    },
    listKnownBlueprintForks: async (reference): Promise<readonly BlueprintKnownForkView[]> => {
      const path = `${blueprintPath(reference)}/forks`;
      return (
        await blueprintJson(fetchImplementation, basePath, path, decodeKnownBlueprintForks, {
          expectedStatus: 200,
        })
      ).body;
    },
    getBlueprintComparison: async (left, right): Promise<BlueprintComparisonView> => {
      const path = `${blueprintPath(left)}/compare/${encodeURIComponent(decodeBlueprintCourseReference(right, "right"))}`;
      const body = (
        await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintComparisonView, {
          expectedStatus: 200,
        })
      ).body;
      if (
        body.left.currentRevision.reference !== left ||
        body.right.currentRevision.reference !== right
      )
        throw new ApiProtocolError("Blueprint comparison must identify the requested pair");
      return body;
    },
    getBlueprintPoolMembers: async (
      reference,
      assessmentReference,
      poolId,
    ): Promise<BlueprintPoolMembersView> => {
      const assessment = decodeUuid(assessmentReference, "assessmentReference");
      const pool = decodeQuestionId(poolId, "poolId");
      const path = `${blueprintPath(reference)}/assessments/${encodeURIComponent(assessment)}/pools/${encodeURIComponent(pool)}/members`;
      const body = (
        await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintPoolMembersView, {
          expectedStatus: 200,
        })
      ).body;
      if (body.questionPoolRevision.questionPoolId !== pool) {
        throw new ApiProtocolError("Blueprint Pool members must identify the requested Pool");
      }
      return body;
    },
    listBlueprintCourses: async (
      cursor,
      pageSize,
      includeArchived = false,
      query,
      publicOnly = false,
      promotedOnly = false,
      classification,
    ): Promise<CursorPage<BlueprintCourseSummaryView>> => {
      const path = pagePath(
        "/api/course-blueprints",
        cursor,
        pageSize,
        includeArchived,
        query,
        publicOnly,
        promotedOnly,
        classification,
      );
      return (await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintCoursePage))
        .body;
    },
    getBlueprintCourse: async (reference): Promise<LoadedBlueprintCourse> => {
      const path = blueprintPath(reference);
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    createBlueprintCourse: async (content, requestKey): Promise<LoadedBlueprintCourse> => {
      const path = "/api/course-blueprints";
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseView,
        {
          method: "POST",
          body: decodeCreateBlueprintCourseInput(content),
          idempotencyKey: requestKey,
          expectedStatus: 201,
        },
      );
      return loadedBlueprintCourse(result.body, result.response, path);
    },
    saveBlueprintCourse: async (
      reference,
      content,
      etag,
      requestKey,
    ): Promise<BlueprintCourseSaveResponse & { readonly revisionEtag: BlueprintRevisionEtag }> => {
      const path = blueprintPath(reference);
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintCourseSaveResponse,
        {
          method: "PUT",
          body: decodeReplaceBlueprintCourseContentInput(content),
          etag,
          parseEtag: parseRevisionEtag,
          idempotencyKey: requestKey,
          expectedStatus: 200,
        },
      );
      return {
        ...result.body,
        revisionEtag: requireRevisionEtag(
          result.response,
          result.body.blueprintCourse.current_revision.revision,
          path,
        ),
      };
    },
    updateBlueprintCourseClassification: async (
      reference,
      classification,
      etag,
    ): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/classification`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "PUT",
          body: decodeCourseClassification(classification, "request"),
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    renameBlueprintCourse: async (reference, names, etag): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/metadata`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "PUT",
          body: decodeRenameBlueprintCourseInput(names),
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    publishBlueprintCourse: async (reference, etag): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/publish`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    getBlueprintRevision: async (reference, revision): Promise<BlueprintRevisionView> => {
      const path = `${blueprintPath(reference)}/revisions/${encodeURIComponent(decodeBlueprintRevision(revision, "revision"))}`;
      return (await blueprintJson(fetchImplementation, basePath, path, decodeBlueprintRevisionView))
        .body;
    },
    archiveBlueprintCourse: async (
      reference,
      confirmationLongName,
      etag,
    ): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/archive`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          body: { confirmationLongName },
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    restoreBlueprintCourse: async (reference, etag): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/restore`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
    returnBlueprintCourseToPrivate: async (
      reference,
      etag,
    ): Promise<BlueprintMetadataTransition> => {
      const path = `${blueprintPath(reference)}/return-to-private`;
      const result = await blueprintJson(
        fetchImplementation,
        basePath,
        path,
        decodeBlueprintMetadataState,
        {
          method: "POST",
          etag,
          parseEtag: parseMetadataEtag,
          expectedStatus: 200,
        },
      );
      return metadataTransition(result.body, result.response, path);
    },
  };
}
