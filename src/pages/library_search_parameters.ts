// URL handoff between browse and search retains text, hierarchy, Tags and other filters.

import {
  appendLibraryClassificationParameters,
  LIBRARY_CLASSIFICATION_UUID_FIELDS,
  parseLibraryClassificationParameters,
} from "../api/library_classification_filter";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  type QuestionLibraryBrowseQuery,
} from "./library_page_model";

/** Explicit recovery removes only the rejected hierarchy, retaining unrelated URL state. */
export function clearLibraryClassificationSearch(search: string): string {
  const parameters = new URLSearchParams(search);
  for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) parameters.delete(field);
  parameters.delete("cross_discipline");
  const serialized = parameters.toString();
  return serialized === "" ? "" : `?${serialized}`;
}

function boundedValues(parameters: URLSearchParams, name: string): Array<string> {
  return parameters
    .getAll(name)
    .filter((value) => value.trim().length > 0 && Array.from(value).length <= 256);
}

export function searchHandoffQuery(search: string): QuestionLibraryBrowseQuery {
  const parameters = new URLSearchParams(search);
  return {
    ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    ...parseLibraryClassificationParameters(parameters),
    search: boundedValues(parameters, "search")[0] ?? "",
    subjects: boundedValues(parameters, "subjects"),
    topics: boundedValues(parameters, "topics"),
    authorName: boundedValues(parameters, "authorName")[0] ?? null,
    backend: boundedValues(parameters, "backend")[0] ?? null,
    tag: boundedValues(parameters, "tag")[0] ?? null,
    questionType: boundedValues(parameters, "questionType")[0] ?? null,
    capability: boundedValues(parameters, "capability")[0] ?? null,
    questionLicense: boundedValues(parameters, "questionLicense")[0] ?? null,
    usedInMyCourses: boundedValues(parameters, "usedInMyCourses")[0] ?? null,
  };
}

export function hasExactBrowseFilters(query: QuestionLibraryBrowseQuery): boolean {
  return (
    query.discipline_uuid !== null ||
    query.subjects.length > 0 ||
    query.topics.length > 0 ||
    query.tag !== null ||
    query.questionType !== null
  );
}

export function searchWithinResultsPath(query: QuestionLibraryBrowseQuery): string {
  const parameters = new URLSearchParams();
  appendLibraryClassificationParameters(parameters, query);
  for (const subject of query.subjects) parameters.append("subjects", subject);
  for (const topic of query.topics) parameters.append("topics", topic);
  for (const field of [
    "authorName",
    "backend",
    "tag",
    "questionType",
    "capability",
    "questionLicense",
    "usedInMyCourses",
  ] as const) {
    const value = query[field];
    if (value !== null) parameters.set(field, value);
  }
  if (query.search !== "") parameters.set("search", query.search);
  const serialized = parameters.toString();
  return serialized.length === 0 ? "/library" : `/library?${serialized}`;
}
