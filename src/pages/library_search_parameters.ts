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
import {
  isBloomCognitiveProcess,
  isBloomKnowledgeDimension,
} from "../api/decoders/bloom_classification";
import { validateCanonicalPublicId } from "../question_id";

/** Removes only rejected strict URL state while retaining valid Library options. */
export function recoverLibrarySearch(search: string): string {
  const parameters = new URLSearchParams(search);
  try {
    parseLibraryClassificationParameters(parameters);
  } catch {
    for (const field of LIBRARY_CLASSIFICATION_UUID_FIELDS) parameters.delete(field);
    parameters.delete("cross_discipline");
  }
  try {
    librarySort(parameters);
  } catch {
    parameters.delete("sort");
  }
  parameters.delete("membership");
  for (const parse of [libraryKind, publishedQuestionFilter, libraryOwner]) {
    try {
      parse(parameters);
    } catch {
      parameters.delete(
        parse === libraryKind
          ? "kind"
          : parse === publishedQuestionFilter
            ? "questions"
            : "ownerAccountId",
      );
    }
  }
  try {
    bloomCognitiveProcess(parameters);
  } catch {
    parameters.delete("bloomCognitiveProcess");
  }
  try {
    bloomKnowledgeDimension(parameters);
  } catch {
    parameters.delete("bloomKnowledgeDimension");
  }
  const serialized = parameters.toString();
  return serialized === "" ? "" : `?${serialized}`;
}

function boundedValues(parameters: URLSearchParams, name: string): Array<string> {
  return parameters
    .getAll(name)
    .filter((value) => value.trim().length > 0 && Array.from(value).length <= 256);
}

function librarySort(parameters: URLSearchParams): QuestionLibraryBrowseQuery["sort"] {
  // ASVS 2.2.1: reject unknown URL state rather than widening it into a server request.
  const values = parameters.getAll("sort");
  if (values.length > 1) throw new Error("Question Library sort must appear at most once");
  const value = values[0] ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.sort;
  if (value !== "titleAscending" && value !== "publishedNewest") {
    throw new Error("Question Library sort is invalid");
  }
  return value;
}

function oneParameter(parameters: URLSearchParams, name: string): string | undefined {
  const values = parameters.getAll(name);
  if (values.length > 1) throw new Error(`${name} must appear at most once`);
  return values[0];
}

function libraryKind(parameters: URLSearchParams): QuestionLibraryBrowseQuery["kind"] {
  const value = oneParameter(parameters, "kind") ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.kind;
  if (value !== "both" && value !== "questions" && value !== "pools") {
    throw new Error("Question Library kind is invalid");
  }
  return value;
}

function publishedQuestionFilter(
  parameters: URLSearchParams,
): QuestionLibraryBrowseQuery["questions"] {
  const value =
    oneParameter(parameters, "questions") ?? EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.questions;
  if (value !== "inNoPool" && value !== "all")
    throw new Error("Published Question filter is invalid");
  return value;
}

function libraryOwner(parameters: URLSearchParams): string | null {
  const value = oneParameter(parameters, "ownerAccountId");
  if (value === undefined) return null;
  if (validateCanonicalPublicId("account", value) === null) {
    throw new Error("Question Library owner is invalid");
  }
  return value;
}

function bloomCognitiveProcess(
  parameters: URLSearchParams,
): QuestionLibraryBrowseQuery["bloomCognitiveProcess"] {
  const values = parameters.getAll("bloomCognitiveProcess");
  if (values.length > 1) throw new Error("Bloom Cognitive Process must appear at most once");
  const value = values[0];
  if (value === undefined) return null;
  if (!isBloomCognitiveProcess(value)) throw new Error("Bloom Cognitive Process is invalid");
  return value;
}

function bloomKnowledgeDimension(
  parameters: URLSearchParams,
): QuestionLibraryBrowseQuery["bloomKnowledgeDimension"] {
  const values = parameters.getAll("bloomKnowledgeDimension");
  if (values.length > 1) throw new Error("Bloom Knowledge Dimension must appear at most once");
  const value = values[0];
  if (value === undefined) return null;
  if (!isBloomKnowledgeDimension(value)) throw new Error("Bloom Knowledge Dimension is invalid");
  return value;
}

export function searchHandoffQuery(search: string): QuestionLibraryBrowseQuery {
  const parameters = new URLSearchParams(search);
  return {
    ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    ...parseLibraryClassificationParameters(parameters),
    kind: libraryKind(parameters),
    questions: publishedQuestionFilter(parameters),
    ownerAccountId: libraryOwner(parameters),
    search: boundedValues(parameters, "search")[0] ?? "",
    subjects: boundedValues(parameters, "subjects"),
    topics: boundedValues(parameters, "topics"),
    bloomCognitiveProcess: bloomCognitiveProcess(parameters),
    bloomKnowledgeDimension: bloomKnowledgeDimension(parameters),
    authorName: boundedValues(parameters, "authorName")[0] ?? null,
    backend: boundedValues(parameters, "backend")[0] ?? null,
    tag: boundedValues(parameters, "tag")[0] ?? null,
    questionType: boundedValues(parameters, "questionType")[0] ?? null,
    capability: boundedValues(parameters, "capability")[0] ?? null,
    questionLicense: boundedValues(parameters, "questionLicense")[0] ?? null,
    sort: librarySort(parameters),
  };
}

export function hasExactBrowseFilters(query: QuestionLibraryBrowseQuery): boolean {
  return (
    query.discipline_uuid !== null ||
    query.subjects.length > 0 ||
    query.topics.length > 0 ||
    query.tag !== null ||
    query.questionType !== null ||
    query.bloomCognitiveProcess !== null ||
    query.bloomKnowledgeDimension !== null
  );
}

export function searchWithinResultsPath(query: QuestionLibraryBrowseQuery): string {
  const parameters = new URLSearchParams();
  appendLibraryClassificationParameters(parameters, query);
  if (query.kind !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.kind) parameters.set("kind", query.kind);
  if (query.questions !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.questions) {
    parameters.set("questions", query.questions);
  }
  if (query.ownerAccountId !== null) parameters.set("ownerAccountId", query.ownerAccountId);
  for (const subject of query.subjects) parameters.append("subjects", subject);
  for (const topic of query.topics) parameters.append("topics", topic);
  for (const field of [
    "authorName",
    "backend",
    "tag",
    "questionType",
    "capability",
    "questionLicense",
    "bloomCognitiveProcess",
    "bloomKnowledgeDimension",
  ] as const) {
    const value = query[field];
    if (value !== null) parameters.set(field, value);
  }
  if (query.search !== "") parameters.set("search", query.search);
  if (query.sort !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.sort) {
    parameters.set("sort", query.sort);
  }
  const serialized = parameters.toString();
  return serialized.length === 0 ? "/library" : `/library?${serialized}`;
}
