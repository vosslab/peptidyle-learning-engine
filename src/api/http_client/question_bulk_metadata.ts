// Same-origin transport for whole-set Published Question shared-metadata commands.

import type { QuestionId } from "../../../generated/api/QuestionId";
import type { PublishedQuestionSharedMetadata } from "../../../generated/api/PublishedQuestionSharedMetadata";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import {
  decodeQuestionBulkMetadataCurrent,
  decodeQuestionBulkMetadataResult,
  validateQuestionBulkMetadataRequest,
} from "../decoders/question_bulk_metadata";
import { decodeQuestionId } from "../decoders/shared";
import type { QuestionBulkMetadataClient } from "../question_bulk_metadata";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

const CURRENT_METADATA_PATH = "/api/questions/bulk-metadata/current";
const UPDATE_METADATA_PATH = "/api/questions/bulk-metadata";

// boundedResponseJson counts UTF-16 code units in the pre-parse JSON text. A
// valid serde_json response needs under 18 MiB for 1000 * (64 tags * 120 code
// points at worst two units/escaped characters), two bounded fields, and syntax.
const MAX_BULK_METADATA_RESPONSE_CHARACTERS = 18 * 1_024 * 1_024;

function exactOrderedIdSet(
  expected: ReadonlyArray<QuestionId>,
  actual: ReadonlyArray<{ readonly questionId: QuestionId }>,
  path: string,
): void {
  if (
    actual.length !== expected.length ||
    actual.some((item, index) => item.questionId !== expected[index])
  ) {
    throw new ApiProtocolError(
      `API response ${path} must contain the exact requested Question set`,
    );
  }
}

async function responseJson(response: Response, path: string): Promise<unknown> {
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 4.1.1: the shared response helper verifies JSON Content-Type before parsing.
  return boundedResponseJson(response, path, MAX_BULK_METADATA_RESPONSE_CHARACTERS);
}

/** Composes the closed metadata capability into the ordinary browser client. */
export function createQuestionBulkMetadataClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): QuestionBulkMetadataClient {
  return {
    getCurrentQuestionBulkMetadata: async (
      questionIds,
    ): Promise<ReadonlyArray<PublishedQuestionSharedMetadata>> => {
      if (
        questionIds.length === 0 ||
        questionIds.length > MAX_BULK_QUESTION_METADATA_ITEMS ||
        new Set(questionIds).size !== questionIds.length
      ) {
        throw new ApiProtocolError(
          "Bulk metadata read needs a bounded distinct Question selection",
        );
      }
      const canonicalIds = questionIds.map((questionId, index) =>
        decodeQuestionId(questionId, `request.questionIds[${index}]`),
      );
      const orderedIds = [...canonicalIds].sort();
      if (orderedIds.some((id, index) => id !== questionIds[index])) {
        throw new ApiProtocolError("Bulk metadata read Question IDs must use canonical order");
      }
      const response = await requestSameOrigin(
        fetchImplementation,
        basePath,
        CURRENT_METADATA_PATH,
        { method: "POST", body: { questionIds: canonicalIds } },
      );
      const items = decodeQuestionBulkMetadataCurrent(
        await responseJson(response, CURRENT_METADATA_PATH),
      );
      exactOrderedIdSet(canonicalIds, items, CURRENT_METADATA_PATH);
      return items;
    },
    updateQuestionBulkMetadata: async (
      input,
    ): ReturnType<QuestionBulkMetadataClient["updateQuestionBulkMetadata"]> => {
      // ASVS 2.2.1-2 and 2.3.1-3: validate the closed, ordered CAS command
      // before sending; the trusted server remains the security boundary.
      const request = validateQuestionBulkMetadataRequest(input);
      const response = await requestSameOrigin(
        fetchImplementation,
        basePath,
        UPDATE_METADATA_PATH,
        { method: "POST", body: request },
      );
      const results = decodeQuestionBulkMetadataResult(
        await responseJson(response, UPDATE_METADATA_PATH),
      );
      exactOrderedIdSet(
        request.selection.map((item) => item.questionId),
        results,
        UPDATE_METADATA_PATH,
      );
      return results;
    },
  };
}
