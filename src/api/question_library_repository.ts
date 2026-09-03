// question_library_repository.ts - converts the generated Question Library search contract for the Library UI.

import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { QuestionSearchAuthorship } from "../../generated/api/QuestionSearchAuthorship";
import type { Capability } from "../../generated/api/Capability";
import type { QuestionLicense } from "../../generated/api/QuestionLicense";
import type { QuestionType } from "../../generated/api/QuestionType";
import type { ApiClient } from "./client";
import type {
  QuestionSearchQuery,
  QuestionLibraryRepository,
  QuestionSearchFacetAggregate,
  QuestionSearchPage,
} from "../pages/library_page_model";

const QUESTION_LIBRARY_PAGE_SIZE = 50;
const CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "questionAttemptTimeLimit",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
const QUESTION_LICENSES = [
  "CC0-1.0",
  "CC-BY-4.0",
  "CC-BY-SA-4.0",
] as const satisfies ReadonlyArray<QuestionLicense>;
const BACKENDS = ["ple", "webwork", "qti", "imathas"] as const;
const QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const satisfies ReadonlyArray<QuestionType>;

function selectedCapability(value: string | null): Array<Capability> {
  if (value === null) {
    return [];
  }
  const selected = CAPABILITIES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Question Library capability selection is invalid");
  }
  return [selected];
}

function selectedQuestionLicense(value: string | null): Array<QuestionLicense> {
  if (value === null) {
    return [];
  }
  const selected = QUESTION_LICENSES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Question Library Question License selection is invalid");
  }
  return [selected];
}

function selectedBackend(value: string | null): QuestionSearchRequest["backends"] {
  if (value === null) return [];
  const selected = BACKENDS.find((candidate) => candidate === value);
  if (selected === undefined) throw new Error("Question Library backend selection is invalid");
  return [selected];
}

function selectedQuestionType(value: string | null): QuestionSearchRequest["question_types"] {
  if (value === null) return [];
  const selected = QUESTION_TYPES.find((candidate) => candidate === value);
  if (selected === undefined)
    throw new Error("Question Library Question Type selection is invalid");
  return [selected];
}

function selectedPublicText(value: string | null): Array<string> {
  return value === null ? [] : [value];
}

function evidenceFilter(value: string | null): QuestionSearchRequest["evidence"] {
  return value === "available" || value === "unavailable" ? value : "any";
}

function facets(
  page: Awaited<ReturnType<ApiClient["searchQuestionLibrary"]>>,
): ReadonlyArray<QuestionSearchFacetAggregate> {
  return [
    ...page.facets.authorNames.map((facet) => ({
      facet: "authorName" as const,
      value: facet.authorName,
      count: facet.count,
    })),
    ...page.facets.backends.map((facet) => ({
      facet: "backend" as const,
      value: facet.backend,
      count: facet.count,
    })),
    ...page.facets.tags.map((facet) => ({
      facet: "tag" as const,
      value: facet.tag,
      count: facet.count,
    })),
    ...page.facets.questionTypes.map((facet) => ({
      facet: "questionType" as const,
      value: facet.questionType,
      count: facet.count,
    })),
    ...page.facets.capabilities.map((facet) => ({
      facet: "capability" as const,
      value: facet.capability,
      count: facet.count,
    })),
    ...page.facets.questionLicenses.map((facet) => ({
      facet: "questionLicense" as const,
      value: facet.questionLicense,
      count: facet.count,
    })),
    { facet: "evidence" as const, value: "available", count: page.facets.evidence.available },
    {
      facet: "evidence" as const,
      value: "unavailable",
      count: page.facets.evidence.unavailable,
    },
    { facet: "usedInMyCourses" as const, value: "used", count: page.facets.usedInMyCourses.used },
  ];
}

/** Builds the one closed Question Library search request used by Library and source-aware pickers. */
export function questionSearchRequest(
  query: QuestionSearchQuery,
  cursor: string | null,
  authorship: QuestionSearchAuthorship = "any",
): QuestionSearchRequest {
  return {
    text: query.search === "" ? null : query.search,
    author_names: selectedPublicText(query.authorName),
    backends: selectedBackend(query.backend),
    tags: selectedPublicText(query.tag),
    question_types: selectedQuestionType(query.questionType),
    capabilities: selectedCapability(query.capability),
    question_licenses: selectedQuestionLicense(query.questionLicense),
    evidence: evidenceFilter(query.evidence),
    used_in_my_courses: query.usedInMyCourses === "used" ? "used" : "any",
    authorship,
    cursor,
    page_size: QUESTION_LIBRARY_PAGE_SIZE,
  };
}

/** The only production bridge from the generated client into the virtual Question Library surface. */
export function createQuestionLibraryRepository(
  client: ApiClient,
  authorship: QuestionSearchAuthorship = "any",
): QuestionLibraryRepository {
  return {
    async search(query: QuestionSearchQuery, cursor: string | null): Promise<unknown> {
      const search = questionSearchRequest(query, cursor, authorship);
      const page = await client.searchQuestionLibrary(search);
      return {
        items: page.items.map((item) => ({
          displayId: item.summary.questionId,
          title: item.summary.metadata.title,
          summary: item.summary.metadata.questionDescription,
          authorNames: item.summary.authorship.authors.map((author) => author.displayName),
          capabilities: item.summary.capabilities,
          questionLicense: item.summary.metadata.questionLicense,
          evidence:
            item.evidence.state === "available"
              ? {
                  state: "available" as const,
                  observedCourseCount: item.evidence.observedCourseCount,
                  independentLearnerObservationCount:
                    item.evidence.independentLearnerObservationCount,
                  difficultyIndex: item.evidence.difficultyIndex,
                  discriminationIndex: item.evidence.discriminationIndex ?? undefined,
                }
              : { state: "insufficientEvidence" as const },
        })),
        nextCursor: page.nextCursor,
        aggregates: facets(page),
      } satisfies QuestionSearchPage;
    },
  };
}
