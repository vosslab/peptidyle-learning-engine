// Same-origin vocabulary reads plus the narrow Sysadmin Discipline lifecycle.

import type {
  ContentClassificationClient,
  ContentDisciplineAdministrationClient,
  ContentClassificationItem,
} from "../content_classification";
import {
  decodeContentClassificationItem,
  decodeContentDisciplineName,
  decodeClassificationUuid,
  decodeContentClassificationList,
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
  };
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
  };
}
