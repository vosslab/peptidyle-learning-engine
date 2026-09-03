// Question preview and policy decoders.

import type { QuestionAttemptLimit } from "../../../generated/api/QuestionAttemptLimit";
import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { QuestionAttemptTimeLimit } from "../../../generated/api/QuestionAttemptTimeLimit";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
} from "../decoder";
import { decodeIdentifier, field, kind, requireOnlyFields } from "./shared";
import {
  decodeQuestionContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  questionResponseFormatSupportsType,
  decodeResponseSelectionRule,
} from "./question_response_format";

export {
  decodeQuestionContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  decodeResponseSelectionRule,
  questionResponseFormatSupportsType,
};

/** Strict key-free static PLE Question JSON Draft Question Preview. */
export function decodeKeyFreeDraftPreview(
  value: unknown,
  path = "wasmPreview",
): {
  workspace: string;
  title: string;
  prompt: ReadonlyArray<QuestionContentBlock>;
  response: QuestionResponseFormat;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "title", "prompt", "response"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeQuestionContentBlock(block, blockPath, true),
    ),
    response: decodeQuestionResponseFormat(
      field(record, "response", path),
      `${path}.response`,
      true,
    ),
  };
}

export function decodeQuestionAttemptLimit(
  value: unknown,
  path: string,
  strict = false,
): QuestionAttemptLimit {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["maxAttempts"]);
  }
  const decoded = {
    maxAttempts: decodeNullable(
      field(record, "maxAttempts", path),
      `${path}.maxAttempts`,
      decodePositiveInteger,
    ),
  } satisfies QuestionAttemptLimit;
  return decoded;
}

export function decodeQuestionAttemptTimeLimit(
  value: unknown,
  path: string,
  strict = false,
): QuestionAttemptTimeLimit {
  const record = decodeRecord(value, path);
  const timing = kind(record, path);
  switch (timing) {
    case "unlimited":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: timing };
    case "limited": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "seconds", "graceSeconds"]);
      }
      const decoded = {
        kind: timing,
        seconds: decodePositiveInteger(field(record, "seconds", path), `${path}.seconds`),
        graceSeconds: decodeNonnegativeInteger(
          field(record, "graceSeconds", path),
          `${path}.graceSeconds`,
        ),
      } satisfies QuestionAttemptTimeLimit;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known timing policy");
  }
}
