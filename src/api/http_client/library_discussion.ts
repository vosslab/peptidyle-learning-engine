// Same-origin transport for retained Library Object discussions and notices.

import type { PublishedQuestionId } from "../../../generated/api/PublishedQuestionId";
import type { ApiClient } from "../client";
import { decodeLibraryDiscussionView } from "../decoders/library_discussion";
import type {
  LibraryDiscussionClient,
  LibraryDiscussionView,
  LibraryObjectDiscussionKind,
} from "../library_discussion";
import { ApiRequestError } from "./error";
import { encodedId, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function objectPath(kind: LibraryObjectDiscussionKind, publicId: PublishedQuestionId): string {
  return `/api/library-objects/${kind}/${encodedId(publicId)}`;
}

async function mutate(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  method: "POST" | "PUT",
  body?: unknown,
): Promise<void> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, { method, body });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204) throw new Error(`API response ${path} must be empty`);
}

export function createLibraryDiscussionClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof LibraryDiscussionClient> {
  return {
    getLibraryDiscussion: async (kind, publicId): Promise<LibraryDiscussionView> => {
      const path = `${objectPath(kind, publicId)}/discussions`;
      const response = await requestSameOrigin(fetchImplementation, basePath, path);
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeLibraryDiscussionView(await boundedResponseJson(response, path));
    },
    createImprovementThread: (kind, publicId, body) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/improvement-threads`,
        "POST",
        { body },
      ),
    replyToImprovementThread: (kind, publicId, threadId, body) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/improvement-threads/${encodedId(threadId)}/posts`,
        "POST",
        { body },
      ),
    editOwnImprovementPost: (kind, publicId, postId, body) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/improvement-posts/${encodedId(postId)}`,
        "PUT",
        { body },
      ),
    setImprovementThreadResolved: (kind, publicId, threadId, resolved) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/improvement-threads/${encodedId(threadId)}/state`,
        "PUT",
        { resolved },
      ),
    createImpactNotice: (kind, publicId, affectedRevisionNumber, body) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/impact-notices`,
        "POST",
        { affectedRevisionNumber, body },
      ),
    updateImpactNotice: (kind, publicId, impactNoticeId, affectedRevisionNumber, body) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/impact-notices/${encodedId(impactNoticeId)}`,
        "PUT",
        { affectedRevisionNumber, body },
      ),
    cancelImpactNotice: (kind, publicId, impactNoticeId) =>
      mutate(
        fetchImplementation,
        basePath,
        `${objectPath(kind, publicId)}/impact-notices/${encodedId(impactNoticeId)}/cancel`,
        "PUT",
      ),
  };
}
