import { MemoryRouter, Route } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import { SearchResults } from "../../../src/features/search/search_results";
import { createSearchState } from "../../../src/features/search/search_state";
import { questionLibraryContent } from "../../../src/pages/question_library_search_definition";
import type { QuestionLibraryBrowseRow } from "../../../src/pages/library_page_model";

const rows: readonly QuestionLibraryBrowseRow[] = [
  {
    kind: "question",
    displayId: "ABCD-XEFG",
    publishedQuestionRevisionTuple: { publishedQuestionId: "ABCD-XEFG", revisionNumber: 1 },
    questionTitle: "Enzyme kinetics",
    summary: "Compare rate facts",
    bloom: null,
    disciplineName: "Biology",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authors: [{ displayName: "Ada Lovelace", accountId: null }],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  },
  {
    kind: "question",
    displayId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
    questionTitle: "Inhibitor binding",
    summary: "Compare binding facts",
    bloom: null,
    disciplineName: "Chemistry",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authors: [{ displayName: "Marie Curie", accountId: null }],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  },
];

export function mountLibraryScan(target: HTMLElement): void {
  render(
    () => (
      <MemoryRouter>
        <Route path="*" component={LibraryScan} />
      </MemoryRouter>
    ),
    target,
  );
}

function LibraryScan(): JSX.Element {
  const state = createSearchState(
    {
      initialQuery: undefined,
      cleanup: () => undefined,
      getText: () => "",
      setText: () => undefined,
      rowId: (row: QuestionLibraryBrowseRow) => row.displayId,
      content: questionLibraryContent,
      fetchPage: () => Promise.resolve({ items: rows, nextCursor: null, filterCounts: undefined }),
    },
    { runOnMount: true },
  );
  return (
    <SearchResults
      state={state}
      ariaLabel="Question Library results"
      emptyState={{ title: "No results", message: "No Question Library rows are available." }}
    />
  );
}
