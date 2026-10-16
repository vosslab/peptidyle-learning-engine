// Search state and browser mapping for one Instructor's private Starred collection.

import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import type { RecordFilterChoices } from "../components/record_list/record_fields";
import type { RecordPageSize } from "../components/record_list/record_page_controls";
import type { RecordContent } from "../components/record_list/record_list";
import type { SearchDefinition, SearchPage } from "../features/search/search_session";
import type { ApiClient } from "../api/client";
import type { StarredQuestionSort } from "../api/question_star";
import {
  libraryFacetAggregates,
  libraryObjectSearchRequest,
  questionLibraryRowFromResult,
} from "../api/question_library_repository";
import {
  decodeLibrarySearchPage,
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  normalizeQuestionLibraryBrowseQuery,
  type LibraryObjectQuestionRow,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryFilterCounts,
} from "./library_page_model";
import { questionLibraryContent } from "./question_library_search_definition";

export interface StarredBrowseQuery extends QuestionLibraryBrowseQuery {
  readonly starredSort: StarredQuestionSort;
}

export interface StarredQuestionRow extends LibraryObjectQuestionRow {
  readonly starredAtMicros: number;
}

export const INITIAL_STARRED_QUERY: StarredBrowseQuery = {
  ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  kind: "questions",
  questions: "all",
  sort: "titleAscending",
  starredSort: "starredDateDescending",
};

export function normalizedStarredQuery(query: StarredBrowseQuery): StarredBrowseQuery {
  return {
    ...normalizeQuestionLibraryBrowseQuery({
      ...query,
      kind: "questions",
      questions: "all",
      sort: "titleAscending",
    }),
    starredSort: query.starredSort,
  };
}

export function starredQuestionContent(
  row: StarredQuestionRow,
  onUnstar: (questionId: string) => void,
): RecordContent {
  const content = questionLibraryContent(row);
  const date = new Date(Math.floor(row.starredAtMicros / 1000));
  return {
    ...content,
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open",
        href: `/library/${encodeURIComponent(row.displayId)}?revisionNumber=${row.publishedQuestionRevisionTuple.revisionNumber}`,
      },
      {
        id: "unstar",
        kind: "command",
        label: "Unstar",
        onClick: () => onUnstar(row.displayId),
      },
    ],
    details: [
      ...content.details,
      {
        kind: "time",
        label: "Starred",
        value: date.toLocaleString(),
        dateTime: date.toISOString(),
      },
    ],
  };
}

/** The private endpoint uses the public Library filter vocabulary and fixed server scope. */
export function starredSearchRepository(client: ApiClient): {
  search: (
    query: QuestionLibraryBrowseQuery,
    cursor: string | null,
    pageSize?: RecordPageSize,
  ) => Promise<ReturnType<typeof decodeLibrarySearchPage>>;
  listFacetChoices: (
    field: LibraryFacetField,
    query: QuestionLibraryBrowseQuery,
    search: string,
    cursor: string | null,
  ) => Promise<RecordFilterChoices>;
} {
  return {
    async search(
      query: QuestionLibraryBrowseQuery,
      cursor: string | null,
      pageSize: RecordPageSize = 50,
    ): Promise<ReturnType<typeof decodeLibrarySearchPage>> {
      const starredQuery = query as StarredBrowseQuery;
      const request = libraryObjectSearchRequest(
        { ...starredQuery, kind: "questions", questions: "all", sort: "titleAscending" },
        cursor,
        "any",
        pageSize,
      );
      const page = await client.listStarredQuestions(request, starredQuery.starredSort);
      const rawRows = page.items.map((item) =>
        questionLibraryRowFromResult({ kind: "question", ...item }),
      );
      const decoded = decodeLibrarySearchPage({
        items: rawRows,
        nextCursor: page.nextCursor,
        aggregates: libraryFacetAggregates(page),
        facetTruncation: {
          authorNames: page.facets.authorNamesTruncated,
          tags: page.facets.tagsTruncated,
          subjects: page.facets.subjectsTruncated,
          topics: page.facets.topicsTruncated,
        },
      });
      return {
        ...decoded,
        items: decoded.items.map((row, index) => ({
          ...row,
          starredAtMicros: page.items[index]!.starredAtMicros,
        })),
      };
    },
    async listFacetChoices(
      field: LibraryFacetField,
      query: QuestionLibraryBrowseQuery,
      search: string,
      cursor: string | null,
    ): Promise<RecordFilterChoices> {
      const starredQuery = query as StarredBrowseQuery;
      const request = libraryObjectSearchRequest(
        { ...starredQuery, kind: "questions", questions: "all", sort: "titleAscending" },
        cursor,
        "any",
        50,
      );
      return client.listStarredFacetChoices(field, request, starredQuery.starredSort, search);
    },
  };
}

export function starredSearchDefinition(
  client: ApiClient,
  onUnstar: (questionId: string) => void,
): SearchDefinition<StarredBrowseQuery, StarredQuestionRow, QuestionLibraryFilterCounts> {
  const repository = starredSearchRepository(client);
  return {
    initialQuery: INITIAL_STARRED_QUERY,
    cleanup: normalizedStarredQuery,
    isEligible: () => true,
    getText: (query) => query.search,
    setText: (query, search) => ({ ...query, search }),
    rowId: (row) => row.displayId,
    content: (row) => starredQuestionContent(row, onUnstar),
    fetchPage: async (
      query,
      cursor,
      pageSize,
    ): Promise<SearchPage<StarredQuestionRow, QuestionLibraryFilterCounts>> => {
      const page = await repository.search(query, cursor, pageSize);
      if (page.items.some((row) => row.kind !== "question"))
        throw new Error("Starred page contained a Pool");
      return {
        items: page.items as ReadonlyArray<StarredQuestionRow>,
        nextCursor: page.nextCursor,
        filterCounts: { aggregates: page.aggregates, facetTruncation: page.facetTruncation },
      };
    },
  };
}
