// Same-origin vocabulary reads plus the narrow Sysadmin Discipline lifecycle.

import type {
  ContentClassificationClient,
  ContentDisciplineAdministrationClient,
  ContentClassificationItem,
  ContentDisciplineRequest,
  ContentSubjectCreation,
} from "../content_classification";
import {
  decodeContentClassificationItem,
  decodeContentDisciplineName,
  decodeContentDisciplineRequest,
  decodeContentDisciplineRequestList,
  decodeClassificationUuid,
  decodeContentClassificationList,
  decodeContentSubjectCreation,
  decodeVocabularyName,
  type ContentClassificationListKey,
} from "../decoders/content_classification";
import type { ApiClient } from "../client";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export function createContentClassificationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): ContentClassificationClient {
  async function list(
    key: ContentClassificationListKey,
    parent?: { readonly field: string; readonly uuid: string },
    pathSegment: string = key,
  ): Promise<ReadonlyArray<ContentClassificationItem>> {
    const query =
      parent === undefined
        ? ""
        : `?${new URLSearchParams({
            [parent.field]: decodeClassificationUuid(parent.uuid, parent.field),
          }).toString()}`;
    const path = `/api/content-classification/${pathSegment}${query}`;
    const response = await requestSameOrigin(fetchImplementation, basePath, path);
    requireNoStore(response, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    return decodeContentClassificationList(await boundedResponseJson(response, path), key);
  }
  return {
    listDisciplines: () => list("disciplines"),
    listDisciplinesIncludingRetired: () => list("disciplines", undefined, "disciplines/discovery"),
    listSubjects: (uuid) => list("subjects", { field: "disciplineUuid", uuid }),
    listTopics: (uuid) => list("topics", { field: "subjectUuid", uuid }),
    listSubtopics: (uuid) => list("subtopics", { field: "topicUuid", uuid }),
    requestContentDiscipline: (name) => requestDiscipline(fetchImplementation, basePath, name),
    createSubject: (name, disciplineUuid) =>
      createSubject(fetchImplementation, basePath, name, disciplineUuid),
    acceptSubjectDiscipline: (subjectUuid, disciplineUuid) =>
      acceptSubjectDiscipline(fetchImplementation, basePath, subjectUuid, disciplineUuid),
    createTopic: (name, subjectUuid) =>
      createVocabularyItem(
        fetchImplementation,
        basePath,
        "/api/content-classification/topics",
        { name: decodeVocabularyName(name, 240, "request.name"), subjectUuid },
        "subjectUuid",
      ),
    createSubtopic: (name, topicUuid) =>
      createVocabularyItem(
        fetchImplementation,
        basePath,
        "/api/content-classification/subtopics",
        { name: decodeVocabularyName(name, 480, "request.name"), topicUuid },
        "topicUuid",
      ),
  };
}

async function createSubject(
  fetchImplementation: ApiFetch,
  basePath: string,
  name: string,
  disciplineUuid: string,
): Promise<ContentSubjectCreation> {
  const path = "/api/content-classification/subjects";
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body: {
      name: decodeVocabularyName(name, 120, "request.name"),
      disciplineUuid: decodeClassificationUuid(disciplineUuid, "disciplineUuid"),
    },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const creation = decodeContentSubjectCreation(await boundedResponseJson(response, path));
  const expected = creation.needsAcceptance ? 200 : 201;
  if (response.status !== expected) {
    throw new ApiProtocolError(`API response ${path} must use status ${expected}`);
  }
  return creation;
}

async function acceptSubjectDiscipline(
  fetchImplementation: ApiFetch,
  basePath: string,
  subjectUuid: string,
  disciplineUuid: string,
): Promise<ContentClassificationItem> {
  const canonicalSubject = decodeClassificationUuid(subjectUuid, "subjectUuid");
  const path = `/api/content-classification/subjects/${encodeURIComponent(canonicalSubject)}/disciplines`;
  return createVocabularyItem(
    fetchImplementation,
    basePath,
    path,
    { disciplineUuid: decodeClassificationUuid(disciplineUuid, "disciplineUuid") },
    "disciplineUuid",
    200,
  );
}

async function createVocabularyItem(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  body: {
    readonly name?: string;
    readonly subjectUuid?: string;
    readonly topicUuid?: string;
    readonly disciplineUuid?: string;
  },
  parentField: "subjectUuid" | "topicUuid" | "disciplineUuid",
  expectedStatus = 201,
): Promise<ContentClassificationItem> {
  const parentUuid = body[parentField];
  if (parentUuid === undefined) {
    throw new ApiProtocolError(`API request ${path} is missing ${parentField}`);
  }
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body: { ...body, [parentField]: decodeClassificationUuid(parentUuid, parentField) },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== expectedStatus) {
    throw new ApiProtocolError(`API response ${path} must use status ${expectedStatus}`);
  }
  return decodeContentClassificationItem(await boundedResponseJson(response, path));
}

async function requestDiscipline(
  fetchImplementation: ApiFetch,
  basePath: string,
  name: string,
): Promise<ContentDisciplineRequest> {
  const path = "/api/content-classification/discipline-requests";
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body: { name: decodeContentDisciplineName(name) },
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 201) {
    throw new ApiProtocolError(`API response ${path} must use status 201`);
  }
  return decodeContentDisciplineRequest(await boundedResponseJson(response, path));
}

function disciplinePath(uuid: string, action?: "rename" | "retire" | "restore"): string {
  const canonicalUuid = decodeClassificationUuid(uuid, "disciplineUuid");
  const suffix = action === undefined ? "" : `/${action}`;
  return `/api/content-classification/disciplines/${encodeURIComponent(canonicalUuid)}${suffix}`;
}

async function disciplineMutation(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: { readonly body?: { readonly name: string }; readonly created?: boolean } = {},
): Promise<ContentClassificationItem> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "POST",
    body: options.body,
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const expected = options.created ? 201 : 200;
  if (response.status !== expected) {
    throw new ApiProtocolError(`API response ${path} must use status ${expected}`);
  }
  return decodeContentClassificationItem(await boundedResponseJson(response, path));
}

/** Composes the Sysadmin-only mutations independently from ordinary selectors. */
export function createContentDisciplineAdministrationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof ContentDisciplineAdministrationClient> {
  return {
    createDiscipline: (name) =>
      disciplineMutation(fetchImplementation, basePath, "/api/content-classification/disciplines", {
        body: { name: decodeContentDisciplineName(name) },
        created: true,
      }),
    renameDiscipline: (uuid, name) =>
      disciplineMutation(fetchImplementation, basePath, disciplinePath(uuid, "rename"), {
        body: { name: decodeContentDisciplineName(name) },
      }),
    retireDiscipline: (uuid) =>
      disciplineMutation(fetchImplementation, basePath, disciplinePath(uuid, "retire")),
    restoreDiscipline: (uuid) =>
      disciplineMutation(fetchImplementation, basePath, disciplinePath(uuid, "restore")),
    listOpenDisciplineRequests: () => listOpenDisciplineRequests(fetchImplementation, basePath),
    resolveDisciplineRequest: (uuid) =>
      resolveDisciplineRequest(fetchImplementation, basePath, uuid),
    fulfillDisciplineRequest: (uuid) =>
      fulfillDisciplineRequest(fetchImplementation, basePath, uuid),
  };
}

async function listOpenDisciplineRequests(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<ReadonlyArray<ContentDisciplineRequest>> {
  const path = "/api/content-classification/discipline-requests";
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeContentDisciplineRequestList(await boundedResponseJson(response, path));
}

async function resolveDisciplineRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  uuid: string,
): Promise<void> {
  const canonicalUuid = decodeClassificationUuid(uuid, "requestUuid");
  const path = `/api/content-classification/discipline-requests/${encodeURIComponent(canonicalUuid)}/resolve`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method: "POST" });
  requireNoStore(response, path);
  if (response.status !== 204) {
    if (!response.ok) throw new ApiRequestError(response.status, path);
    throw new ApiProtocolError(`API response ${path} must use status 204`);
  }
}

async function fulfillDisciplineRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  uuid: string,
): Promise<ContentClassificationItem> {
  const canonicalUuid = decodeClassificationUuid(uuid, "requestUuid");
  const path = `/api/content-classification/discipline-requests/${encodeURIComponent(canonicalUuid)}/fulfill`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method: "POST" });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 201) {
    throw new ApiProtocolError(`API response ${path} must use status 201`);
  }
  return decodeContentClassificationItem(await boundedResponseJson(response, path));
}
