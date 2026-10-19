// The Instructor's private Starred Question search and actions.

import { createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../../api/application_api";
import { PageFrame } from "../../components/page_frame/page_frame";
import type {
  RecordField,
  DataSheetQuery,
  RecordSort,
} from "../../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import type { LibraryFacetField } from "../../../generated/api/LibraryFacetField";
import type { StarredQuestionSort } from "../../api/question_star";
import { libraryRecordQuery } from "../../features/question_library/library_record_filters";
import {
  fixedKindLibrarySearchFields,
  type LibraryRecordSortKey,
} from "../../features/question_library/library_search_fields";
import {
  starredSearchDefinition,
  starredSearchRepository,
  type StarredQuestionRow,
} from "./starred_search_definition";

type StarredSortKey = LibraryRecordSortKey | "starredDate";

const STARRED_SORTS: ReadonlyArray<{
  readonly value: StarredQuestionSort;
  readonly sort: RecordSort<StarredSortKey>;
}> = [
  { value: "starredDateDescending", sort: { key: "starredDate", direction: "descending" } },
  { value: "starredDateAscending", sort: { key: "starredDate", direction: "ascending" } },
  { value: "titleAscending", sort: { key: "title", direction: "ascending" } },
  { value: "titleDescending", sort: { key: "title", direction: "descending" } },
  { value: "disciplineAscending", sort: { key: "discipline", direction: "ascending" } },
  { value: "disciplineDescending", sort: { key: "discipline", direction: "descending" } },
  { value: "questionTypeAscending", sort: { key: "questionType", direction: "ascending" } },
  { value: "questionTypeDescending", sort: { key: "questionType", direction: "descending" } },
  { value: "publishedNewest", sort: { key: "published", direction: "descending" } },
];

const starredFields: ReadonlyArray<
  RecordField<StarredQuestionRow, StarredSortKey, LibraryFacetField>
> = [
  ...fixedKindLibrarySearchFields,
  {
    id: "starredDate",
    label: "Starred",
    sortKey: "starredDate",
    value: (row) => new Date(Math.floor(row.starredAtMicros / 1000)).toLocaleDateString(),
    defaultColumn: { priority: 1, width: "10rem" },
  },
];

/** Whole-collection Starred discovery with private Open and Unstar actions. */
export function StarredQuestionsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [busyQuestionId, setBusyQuestionId] = createSignal<string | null>(null);
  const searchState = createSearchState(
    starredSearchDefinition(runtime.client, (questionId) => void unstar(questionId)),
    {
      runOnMount: true,
    },
  );
  async function unstar(questionId: string): Promise<void> {
    if (busyQuestionId() !== null) return;
    setBusyQuestionId(questionId);
    setActionError(null);
    try {
      await runtime.client.setQuestionStar(questionId, false);
      await searchState.open(searchState.query());
    } catch {
      setActionError("Could not Unstar this Question. Try again.");
    } finally {
      setBusyQuestionId(null);
    }
  }
  const repository = starredSearchRepository(runtime.client);
  const baseQuery = libraryRecordQuery({
    query: searchState.query,
    apply: async (change) =>
      searchState.apply((current) => {
        const next = typeof change === "function" ? change(current) : change;
        return next === undefined ? undefined : { ...current, ...next };
      }),
    sort: async (change) =>
      searchState.sort((current) => {
        const next = typeof change === "function" ? change(current) : change;
        return next === undefined ? undefined : { ...current, ...next };
      }),
    replacementRevision: searchState.replacementRevision,
    repository,
    classificationClient: runtime.client,
    onError: setActionError,
  });
  const recordQuery: DataSheetQuery<StarredSortKey, LibraryFacetField> = {
    filters: baseQuery.filters,
    sort: {
      get value() {
        return (
          STARRED_SORTS.find((entry) => entry.value === searchState.query().starredSort)?.sort ??
          null
        );
      },
      onChange: (sort) => {
        const selected = STARRED_SORTS.find(
          (entry) => entry.sort.key === sort.key && entry.sort.direction === sort.direction,
        );
        if (selected !== undefined)
          void searchState.sort((current) => ({ ...current, starredSort: selected.value }));
      },
      choices: [
        { label: "Recently published", sort: { key: "published", direction: "descending" } },
      ],
    },
  };

  return (
    <PageFrame routeSurface="starredQuestions">
      <SearchBlock
        layout="above"
        state={searchState}
        controls={
          <>
            <SearchControls
              state={searchState}
              textLabel="Search Starred Questions"
              textPlaceholder="Title or concept"
            />
            {actionError() && <p role="alert">{actionError()}</p>}
          </>
        }
        collection={{
          ariaLabel: "Starred Questions",
          fields: starredFields,
          query: recordQuery,
          emptyState: {
            title: "No Starred Questions match these filters",
            message: "Star a Published Question to keep it in this personal collection.",
          },
        }}
      />
    </PageFrame>
  );
}
