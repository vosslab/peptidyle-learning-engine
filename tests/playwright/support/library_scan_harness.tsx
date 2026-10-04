import { MemoryRouter, Route } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import { NO_QUESTION_LIBRARY_FACET_TRUNCATION } from "../../../src/pages/library_page_model";
import type { QuestionLibraryBrowseRow } from "../../../src/pages/library_page_model";
import { LibraryBrowseRows } from "../../../src/pages/library_browse_rows";

const rows: readonly QuestionLibraryBrowseRow[] = [
  {
    displayId: "ABCD-XEFG",
    publishedQuestionRevisionTuple: { publishedQuestionId: "ABCD-XEFG", revisionNumber: 1 },
    questionTitle: "Enzyme kinetics",
    summary: "Compare rate facts",
    bloom: null,
    disciplineName: "Biology",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authorNames: ["Ada Lovelace"],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  },
  {
    displayId: "7K3M-79QP",
    publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
    questionTitle: "Inhibitor binding",
    summary: "Compare binding facts",
    bloom: null,
    disciplineName: "Chemistry",
    disciplineIsRetired: false,
    questionFormat: "pleQuestionJson",
    authorNames: ["Marie Curie"],
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
  return (
    <LibraryBrowseRows
      mayMutateLibrary={false}
      displayedRows={() => rows}
      selectedIds={() => new Set()}
      editorBusy={() => false}
      browseState={() => ({
        kind: "ready",
        rows,
        aggregates: [],
        nextCursor: null,
        facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
      })}
      setLibraryWindow={() => undefined}
      onScroll={() => undefined}
      onRetry={() => undefined}
      onUpdateSelection={() => undefined}
      onSelectLoaded={() => undefined}
      onClearSelection={() => undefined}
      onOpenMetadataEditor={() => undefined}
      returnTokenFor={() => "00000000-0000-4000-8000-000000000001"}
      onSaveReturnState={() => undefined}
    />
  );
}
