// Bounded Pool-owned search normalization; classification remains shared.

import { MAX_QUESTION_SEARCH_TAG_FILTERS } from "../../generated/api/MAX_QUESTION_SEARCH_TAG_FILTERS";
import { libraryClassificationFilter } from "./library_classification_filter";
import type { QuestionPoolLibraryFilter } from "./question_pool_library";

export function questionPoolLibraryFilter(
  value: QuestionPoolLibraryFilter,
): QuestionPoolLibraryFilter {
  const classification = libraryClassificationFilter(value);
  const text =
    value.text === null || value.text === undefined
      ? null
      : value.text.trim().split(/\s+/u).join(" ").toLowerCase();
  if (text !== null && Array.from(text).length > 256) {
    throw new Error("Pool search text must contain at most 256 characters");
  }
  const selections = value.tags ?? [];
  if (selections.length > MAX_QUESTION_SEARCH_TAG_FILTERS) {
    throw new Error("Pool search accepts at most 64 Tags");
  }
  const tags = selections.map((tag) => {
    const normalized = tag.trim().split(/\s+/u).join(" ").toLowerCase();
    if (!normalized || Array.from(normalized).length > 256) {
      throw new Error("Each Pool Tag must contain 1 through 256 characters");
    }
    return normalized;
  });
  return { ...classification, text: text || null, tags: [...new Set(tags)].sort() };
}
