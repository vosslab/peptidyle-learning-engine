// Strict decoder for one Question Pool's optional support texts.

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import { DecodeError, decodeRecord, decodeSafeInteger } from "../decoder";
import type {
  QuestionPoolPleManagedSupport,
  QuestionPoolSupportSave,
} from "../question_pool_support";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";

const MAX_SUPPORT_CHARACTERS = 4000;
const SUPPORT_FIELDS = [
  "questionPoolId",
  "questionPoolMetadataEditNumber",
  "hint",
  "generalFeedback",
  "workedSolution",
] as const;

function hasControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

function decodeSupportText(value: unknown, path: string): string | null {
  if (value === null) return null;
  if (typeof value !== "string" || value.trim() !== value || [...value].length === 0) {
    throw new DecodeError(path, "absent or trimmed Pool support text");
  }
  if ([...value].length > MAX_SUPPORT_CHARACTERS || hasControlCharacter(value)) {
    throw new DecodeError(path, `Pool support text within ${MAX_SUPPORT_CHARACTERS} characters`);
  }
  return value;
}

function decodeEditNumber(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1) throw new DecodeError(path, "a positive Pool Edit Number");
  return decoded;
}

function decodeSupport(value: unknown, path: string): QuestionPoolPleManagedSupport {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, SUPPORT_FIELDS);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolMetadataEditNumber: decodeEditNumber(
      field(record, "questionPoolMetadataEditNumber", path),
      `${path}.questionPoolMetadataEditNumber`,
    ),
    hint: decodeSupportText(field(record, "hint", path), `${path}.hint`),
    generalFeedback: decodeSupportText(
      field(record, "generalFeedback", path),
      `${path}.generalFeedback`,
    ),
    workedSolution: decodeSupportText(
      field(record, "workedSolution", path),
      `${path}.workedSolution`,
    ),
  };
}

/** ASVS 1.5.2 and 2.2.1: allow only the Pool identity, Edit Number, and three texts. */
export function decodeQuestionPoolSupport(
  value: unknown,
  path = "response",
): QuestionPoolPleManagedSupport {
  return decodeSupport(value, path);
}

function wireText(value: string | null): string | null {
  if (value === null) return null;
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

/** ASVS 2.2.1: blank text is absent, and a member Question id is not a field. */
export function validateQuestionPoolSupportSave(
  input: QuestionPoolSupportSave,
): QuestionPoolPleManagedSupport {
  return decodeSupport(
    {
      questionPoolId: input.questionPoolId,
      questionPoolMetadataEditNumber: input.questionPoolMetadataEditNumber,
      hint: wireText(input.hint),
      generalFeedback: wireText(input.generalFeedback),
      workedSolution: wireText(input.workedSolution),
    },
    "request",
  );
}

export function supportPath(questionPoolId: QuestionPoolId): string {
  return `/api/question-pools/${encodeURIComponent(questionPoolId)}/ple-managed-support`;
}
