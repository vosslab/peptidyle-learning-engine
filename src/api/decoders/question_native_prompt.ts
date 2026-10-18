// Bounded answer-free Native prompt projection shared by API and browser rows.

import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import { decodeNullable } from "../decoder";
import { decodeQuestionContentBlock } from "./question_model";
import { decodeBoundedArray, MAX_QUESTION_DETAILS_STATIC_PROMPT_BLOCKS } from "./shared";

/** ASVS 1.5.2: reuse the strict content-block allowlist and existing prompt bound. */
export function decodeQuestionNativePrompt(
  value: unknown,
  path: string,
): QuestionContentBlock[] | null {
  return decodeNullable(value, path, (blocks, blocksPath) =>
    decodeBoundedArray(
      blocks,
      blocksPath,
      MAX_QUESTION_DETAILS_STATIC_PROMPT_BLOCKS,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
  );
}
