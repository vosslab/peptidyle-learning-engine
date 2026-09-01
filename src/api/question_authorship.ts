// Browser boundary for the reviewed Question Authorship wire contract.

import type { QuestionAuthorship } from "../../generated/api/QuestionAuthorship";
import { DecodeError, decodeArray, decodeRecord, decodeString } from "./decoder";
import { field, requireOnlyFields } from "./decoders/shared";

const MAX_QUESTION_AUTHORS = 16;
const MAX_QUESTION_AUTHOR_DISPLAY_NAME_SCALARS = 120;
const CONTROL_CHARACTER = /[\p{Cc}]/u;

function isValidQuestionAuthorDisplayName(displayName: string): boolean {
  return (
    displayName.length > 0 &&
    displayName === displayName.trim() &&
    !CONTROL_CHARACTER.test(displayName) &&
    Array.from(displayName).length <= MAX_QUESTION_AUTHOR_DISPLAY_NAME_SCALARS
  );
}

/** Returns reviewed Question Authorship or null for locally correctable author input. */
export function parseReviewedQuestionAuthorship(text: string): QuestionAuthorship | null {
  const displayNames = text.split("\n").map((name) => name.trim());
  if (
    displayNames.length === 0 ||
    displayNames.length > MAX_QUESTION_AUTHORS ||
    displayNames.some((displayName) => !isValidQuestionAuthorDisplayName(displayName)) ||
    new Set(displayNames).size !== displayNames.length
  ) {
    return null;
  }
  return { authors: displayNames.map((displayName) => ({ displayName })) };
}

/** Validates a publication command before it crosses the HTTP boundary. */
export function isQuestionAuthorship(value: QuestionAuthorship): boolean {
  const displayNames = value.authors.map((author) => author.displayName);
  return (
    displayNames.length > 0 &&
    displayNames.length <= MAX_QUESTION_AUTHORS &&
    displayNames.every(isValidQuestionAuthorDisplayName) &&
    new Set(displayNames).size === displayNames.length
  );
}

/** Strictly decodes the generated Question Authorship wire shape. */
export function decodeQuestionAuthorship(value: unknown, path: string): QuestionAuthorship {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authors"]);
  const authors = decodeArray(
    field(record, "authors", path),
    `${path}.authors`,
    (entry, entryPath) => {
      const author = decodeRecord(entry, entryPath);
      requireOnlyFields(author, entryPath, ["displayName"]);
      return {
        displayName: decodeString(
          field(author, "displayName", entryPath),
          `${entryPath}.displayName`,
        ),
      };
    },
  );
  if (!isQuestionAuthorship({ authors })) {
    throw new DecodeError(path, "one to sixteen distinct reviewed Question Authors");
  }
  return { authors };
}
