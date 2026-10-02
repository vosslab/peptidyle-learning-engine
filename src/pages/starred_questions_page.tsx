// starred_questions_page.tsx - the Instructor's personal Starred Question collection.

import { A } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { StarredQuestionSummary } from "../api/question_star";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { RecordPageControls } from "../components/record_list/record_page_controls";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import {
  FIRST_STARRED_QUESTIONS_POSITION,
  loadStarredQuestions,
  starredQuestionsNextPosition,
  starredQuestionsPageSizePosition,
  starredQuestionsPreviousPosition,
  type StarredQuestionsPosition,
} from "./starred_questions_model";

function questionHref(questionId: string): string {
  return buildRoutePath("questionDetail", { questionId }) ?? "/library";
}

function questionContent(row: StarredQuestionSummary): RecordContent {
  return {
    title: row.questionTitle,
    details: [{ kind: "questionId", questionTitle: row.questionTitle, displayId: row.questionId }],
    actions: [
      {
        id: "open-question",
        kind: "link",
        label: "Open question",
        href: questionHref(row.questionId),
        primary: true,
      },
    ],
  };
}

function listFailed(value: unknown): value is Error {
  return value instanceof Error;
}

/** Shows the signed-in Instructor's Starred Questions, newest first. */
export function StarredQuestionsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [position, setPosition] = createSignal<StarredQuestionsPosition>(
    FIRST_STARRED_QUESTIONS_POSITION,
  );
  const [page, { refetch }] = createResource(position, (requested) =>
    loadStarredQuestions(runtime.client, requested),
  );
  const failed = (): boolean => listFailed(page.error);
  const rows = (): ReadonlyArray<StarredQuestionSummary> => page()?.items ?? [];
  const continuation = (): string | null => {
    const cursor = page()?.nextCursor;
    return !page.loading && !failed() && typeof cursor === "string" && cursor.length > 0
      ? cursor
      : null;
  };
  const listState = (): RecordListState => {
    if (page.loading) return { kind: "loading", label: "Loading Starred Questions..." };
    if (failed()) {
      return {
        kind: "error",
        title: "Starred Questions could not load",
        message: "Check your connection and try again.",
        retry: (): void => void refetch(),
      };
    }
    return { kind: "ready" };
  };

  return (
    <PageFrame
      routeSurface="starredQuestions"
      headingId="starred-questions-heading"
      eyebrow="Questions"
      title="Starred"
      lede="Published Questions you starred, newest first. Open one when you want it handy."
    >
      <RecordList
        ariaLabel="Starred Questions"
        emptyState={{
          title: "No Starred Questions yet",
          message: "Star a Published Question and it appears in this personal collection.",
        }}
        content={questionContent}
        recordId={(row) => row.questionId}
        rows={rows()}
        state={listState()}
      />
      <RecordPageControls
        ariaLabel="Starred Question pages"
        hasPrevious={position().previousCursors.length > 0}
        hasNext={continuation() !== null}
        loading={page.loading}
        disabled={failed()}
        onPrevious={() => {
          const previous = starredQuestionsPreviousPosition(position());
          if (previous !== null) setPosition(previous);
        }}
        onNext={() => {
          const cursor = continuation();
          if (cursor !== null) setPosition(starredQuestionsNextPosition(position(), cursor));
        }}
        pageSize={position().pageSize}
        onPageSizeChange={(pageSize) => setPosition(starredQuestionsPageSizePosition(pageSize))}
      />
      <Show when={!page.loading && !failed() && rows().length === 0}>
        <p>
          <A class="primary-link" href="/library">
            Search Question Library
          </A>
        </p>
      </Show>
    </PageFrame>
  );
}
