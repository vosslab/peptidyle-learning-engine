// question_draft_editor_page.tsx - route composition for one private Draft Question.

import { A, useParams } from "@solidjs/router";
import { Show, createMemo, createResource, type JSX } from "solid-js";

import { createPleQuestionJsonClient } from "../features/ple_question_json_authoring/question_json_client";
import { PleQuestionJsonEditorPage } from "../features/ple_question_json_authoring/question_json_editor_page";
import { createPleQuestionJsonRepository } from "../features/ple_question_json_authoring/question_json_repository";
import { parseDraftQuestionReference } from "../navigation/public_route";
import { useWasmFacade } from "../wasm/context";

/** Loads the private PLE Question JSON editor only after the route grammar accepts `D-...`. */
export function QuestionDraftEditorPage(): JSX.Element {
  const params = useParams();
  const wasm = useWasmFacade();
  const client = createPleQuestionJsonClient();
  const repository = createPleQuestionJsonRepository(client);
  const reference = createMemo(() => parseDraftQuestionReference(params.draftQuestionRef ?? ""));
  const [initial, { refetch }] = createResource(reference, async (draftQuestion) =>
    await repository.load(draftQuestion),
  );

  return (
    <Show
      when={reference()}
      fallback={
        <main class="page route-error" data-route-surface="questionDraftEditorInvalid" role="alert">
          <h1>Draft Question not found</h1>
          <A class="primary-link" href="/authoring/drafts">Return to My Question Drafts</A>
        </main>
      }
    >
      <Show
        when={initial()}
        fallback={
          <main class="page" data-route-surface="questionDraftEditorLoading">
            <Show
              when={initial.error}
              fallback={<p class="calm-status" role="status">Loading private Draft Question...</p>}
            >
              <section class="inline-error" role="alert">
                <p>This Draft Question is unavailable.</p>
                <button class="quiet-action" type="button" onClick={() => void refetch()}>Retry</button>
              </section>
            </Show>
          </main>
        }
      >
        {(loaded) => (
          <PleQuestionJsonEditorPage
            draftQuestion={reference()!}
            initial={loaded()}
            repository={repository}
            responseValidator={wasm}
          />
        )}
      </Show>
    </Show>
  );
}
