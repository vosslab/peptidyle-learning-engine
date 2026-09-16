// Four authorized same-origin vocabulary reads; no classification mutations.

import type {
  ContentClassificationClient,
  ContentClassificationItem,
} from "../content_classification";
import {
  decodeClassificationUuid,
  decodeContentClassificationList,
  type ContentClassificationListKey,
} from "../decoders/content_classification";
import { ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export function createContentClassificationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): ContentClassificationClient {
  async function list(
    key: ContentClassificationListKey,
    parent?: { readonly field: string; readonly uuid: string },
  ): Promise<ReadonlyArray<ContentClassificationItem>> {
    const query =
      parent === undefined
        ? ""
        : `?${new URLSearchParams({
            [parent.field]: decodeClassificationUuid(parent.uuid, parent.field),
          }).toString()}`;
    const path = `/api/content-classification/${key}${query}`;
    const response = await requestSameOrigin(fetchImplementation, basePath, path);
    requireNoStore(response, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    return decodeContentClassificationList(await boundedResponseJson(response, path), key);
  }
  return {
    listDisciplines: () => list("disciplines"),
    listSubjects: (uuid) => list("subjects", { field: "disciplineUuid", uuid }),
    listTopics: (uuid) => list("topics", { field: "subjectUuid", uuid }),
    listSubtopics: (uuid) => list("subtopics", { field: "topicUuid", uuid }),
  };
}
