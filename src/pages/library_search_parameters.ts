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
    bloomCognitiveProcesses(parameters);
  } catch {
    parameters.delete("bloomCognitiveProcesses");
  }
  try {
    bloomKnowledgeDimensions(parameters);
  } catch {
    parameters.delete("bloomKnowledgeDimensions");
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
  switch (value) {
    case "titleAscending":
    case "titleDescending":
    case "kindAscending":
    case "kindDescending":
    case "disciplineAscending":
    case "disciplineDescending":
    case "questionTypeAscending":
    case "questionTypeDescending":
    case "publishedNewest":
      return value;
    default:
      throw new Error("Question Library sort is invalid");
  }
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

function bloomCognitiveProcesses(
  parameters: URLSearchParams,
): QuestionLibraryBrowseQuery["bloomCognitiveProcesses"] {
  const values = parameters.getAll("bloomCognitiveProcesses");
  if (values.some((value) => !isBloomCognitiveProcess(value)))
    throw new Error("Bloom Cognitive Process is invalid");
  return values as QuestionLibraryBrowseQuery["bloomCognitiveProcesses"];
}

function bloomKnowledgeDimensions(
  parameters: URLSearchParams,
): QuestionLibraryBrowseQuery["bloomKnowledgeDimensions"] {
  const values = parameters.getAll("bloomKnowledgeDimensions");
  if (values.some((value) => !isBloomKnowledgeDimension(value)))
    throw new Error("Bloom Knowledge Dimension is invalid");
  return values as QuestionLibraryBrowseQuery["bloomKnowledgeDimensions"];
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
    bloomCognitiveProcesses: bloomCognitiveProcesses(parameters),
    bloomKnowledgeDimensions: bloomKnowledgeDimensions(parameters),
    authorNames: boundedValues(parameters, "authorNames"),
    backends: boundedValues(parameters, "backends"),
    tags: boundedValues(parameters, "tags"),
    questionTypes: boundedValues(parameters, "questionTypes"),
    capabilities: boundedValues(parameters, "capabilities"),
    questionLicenses: boundedValues(parameters, "questionLicenses"),
    sort: librarySort(parameters),
  };
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
    "authorNames",
    "backends",
    "tags",
    "questionTypes",
    "capabilities",
    "questionLicenses",
    "bloomCognitiveProcesses",
    "bloomKnowledgeDimensions",
  ] as const) {
    for (const value of query[field]) parameters.append(field, value);
  }
  if (query.search !== "") parameters.set("search", query.search);
  if (query.sort !== EMPTY_QUESTION_LIBRARY_BROWSE_QUERY.sort) {
    parameters.set("sort", query.sort);
  }
  const serialized = parameters.toString();
  return serialized.length === 0 ? "/library" : `/library?${serialized}`;
}
