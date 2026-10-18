// library_query.ts - Question Library browse query defaults and normalization.

import type { QuestionLibraryBrowseQuery } from "../../api/question_library_projection";
import {
  EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  libraryClassificationFilter,
} from "../../api/library_classification_filter";

export const EMPTY_QUESTION_LIBRARY_BROWSE_QUERY: QuestionLibraryBrowseQuery = {
  ...EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  kind: "both",
  questions: "inNoPool",
  ownerAccountId: null,
  search: "",
  authorNames: [],
  backends: [],
  tags: [],
  subjects: [],
  topics: [],
  bloomCognitiveProcesses: [],
  bloomKnowledgeDimensions: [],
  questionTypes: [],
  capabilities: [],
  questionLicenses: [],
  authorship: "any",
  sort: "titleAscending",
};

function normalizeLibraryTextFilterKeys(values: ReadonlyArray<string>): string[] {
  return [
    ...new Set(values.map((value) => value.trim().replace(/\s+/g, " ").toLowerCase())),
  ].sort();
}

export function normalizeQuestionLibraryBrowseQuery(
  query: QuestionLibraryBrowseQuery,
): QuestionLibraryBrowseQuery {
  const poolsOnly = query.kind === "pools";
  return {
    ...libraryClassificationFilter(query),
    kind: query.kind,
    questions: poolsOnly ? "all" : query.questions,
    ownerAccountId: query.ownerAccountId === null ? null : query.ownerAccountId.trim(),
    search: query.search.trim().replace(/\s+/g, " "),
    authorNames: poolsOnly ? [] : normalizeLibraryTextFilterKeys(query.authorNames),
    backends: query.backends,
    tags: normalizeLibraryTextFilterKeys(query.tags),
    subjects: normalizeLibraryTextFilterKeys(query.subjects),
    topics: normalizeLibraryTextFilterKeys(query.topics),
    bloomCognitiveProcesses: query.bloomCognitiveProcesses,
    bloomKnowledgeDimensions: query.bloomKnowledgeDimensions,
    questionTypes: query.questionTypes,
    capabilities: query.capabilities,
    questionLicenses: query.questionLicenses,
    authorship: poolsOnly ? "any" : query.authorship,
    sort: query.sort,
  };
}
