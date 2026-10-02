// my_questions_page.tsx - Instructor list of their own Published Questions.

import { Show, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { RecordPageControls } from "../components/record_list/record_page_controls";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import type { QuestionLibraryBrowseRow } from "./library_page_model";
import {
  FIRST_MY_QUESTIONS_POSITION,
  loadMyQuestions,
  myQuestionsNextPosition,
  myQuestionsPageSizePosition,
  myQuestionsPreviousPosition,
  type MyQuestionsPosition,
} from "./my_questions_model";

function questionHref(questionId: string): string {
  return buildRoutePath("questionDetail", { questionId }) ?? "/library";
}

function questionContent(row: QuestionLibraryBrowseRow): RecordContent {
  return {
    title: row.questionTitle,
    description: row.summary.length === 0 ? undefined : row.summary,
    details: [{ kind: "questionId", questionTitle: row.questionTitle, displayId: row.displayId }],
    actions: [
      {
        id: "open-question",
        kind: "link",
        label: "Open question",
        href: questionHref(row.displayId),
        primary: true,
      },
    ],
  };
}

function listFailed(value: unknown): value is Error {
  return value instanceof Error;
}

/** Shows the signed-in Instructor's Published Questions, one server page at a time. */
export function MyQuestionsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [position, setPosition] = createSignal<MyQuestionsPosition>(FIRST_MY_QUESTIONS_POSITION);
  const [page, { refetch }] = createResource(position, (requested: MyQuestionsPosition) =>
    loadMyQuestions(runtime.client, {
      cursor: requested.inputCursor,
      pageSize: requested.pageSize,
    }),
  );
  const failed = (): boolean => listFailed(page.error);
  const rows = (): ReadonlyArray<QuestionLibraryBrowseRow> =>
    failed() ? [] : (page()?.items ?? []);
  const continuation = (): string | null => {
    if (page.loading || failed()) return null;
    const cursor = page()?.nextCursor;
    return typeof cursor === "string" && cursor.length > 0 ? cursor : null;
  };
  const listState = (): RecordListState => {
    if (page.loading) return { kind: "loading", label: "Loading My Questions..." };
    if (failed()) {
      return {
        kind: "error",
        title: "My Questions could not load",
        message: "Check your connection and try again.",
        retry: (): void => void refetch(),
      };
    }
    return { kind: "ready" };
  };

  function showNextPage(): void {
    const cursor = continuation();
    if (cursor === null) return;
    setPosition(myQuestionsNextPosition(position(), cursor));
  }

  function showPreviousPage(): void {
    if (page.loading) return;
    const previous = myQuestionsPreviousPosition(position());
    if (previous !== null) setPosition(previous);
  }

  return (
    <PageFrame
      routeSurface="myQuestions"
      headingId="my-questions-heading"
      eyebrow="Questions"
      title="My Questions"
      lede="Published Questions you authored. Open one to review or manage it."
    >
      <RecordList
        ariaLabel="My Published Questions"
        emptyState={{
          title: "No Published Questions yet",
          message: "Questions you publish appear here so you can find and open them.",
        }}
        content={questionContent}
        recordId={(row) => row.displayId}
        rows={rows()}
        state={listState()}
      />
      <RecordPageControls
        ariaLabel="My Published Question pages"
        hasPrevious={position().previousCursors.length > 0}
        hasNext={continuation() !== null}
        loading={page.loading}
        disabled={failed()}
        onPrevious={showPreviousPage}
        onNext={showNextPage}
        pageSize={position().pageSize}
        onPageSizeChange={(pageSize) => setPosition(myQuestionsPageSizePosition(pageSize))}
      />
      <Show when={!page.loading && !failed() && rows().length === 0}>
        <p>
          <a class="primary-link" href="/authoring/drafts">
            Open My Draft Questions
          </a>
        </p>
      </Show>
    </PageFrame>
  );
}
