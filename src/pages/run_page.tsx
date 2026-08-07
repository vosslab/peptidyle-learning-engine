// run_page.tsx - reference question/response vertical slice over the typed client.

import { createAsync, useNavigate, useParams } from "@solidjs/router";
import { ErrorBoundary, For, Show, Suspense, type JSX } from "solid-js";

import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import { useApiRuntime } from "../api/runtime";
import { MultipleChoiceResponse } from "../components/multiple_choice_response";
import { useWasmFacade } from "../wasm/context";

function PromptBlock(props: { readonly block: ContentBlock }): JSX.Element {
  return (
    <Show
      when={props.block.kind === "text" ? props.block.markdown : undefined}
      fallback={
        <Show when={props.block.kind === "image" ? props.block.description : undefined}>
          {(description) => <p class="visual-note">Figure evidence: {description()}</p>}
        </Show>
      }
    >
      {(markdown) => <p>{markdown()}</p>}
    </Show>
  );
}

function multipleChoiceDefinition(
  definition: ResponseDefinition,
): Extract<ResponseDefinition, { kind: "multipleChoice" }> | undefined {
  return definition.kind === "multipleChoice" ? definition : undefined;
}

export function RunPage(): JSX.Element {
  const runtime = useApiRuntime();
  const validator = useWasmFacade();
  const navigate = useNavigate();
  const params = useParams();
  const runScreen = createAsync(() => {
    const runId = params["runId"];
    if (runId === undefined) {
      return Promise.reject(new Error("Run route is missing runId"));
    }
    return runtime.queries.runScreen(runId);
  });

  return (
    <section class="page run-page" data-route-surface="runAttempt">
      <Suspense fallback={<p class="loading-state">Loading the current question...</p>}>
        <Show when={runScreen()}>
          {(screen) => (
            <>
              <header class="run-header">
                <div>
                  <p class="eyebrow">Practice run {screen().run.runNumber}</p>
                  <h1>{screen().question.metadata.title}</h1>
                </div>
                <span class="calm-status">Untimed</span>
              </header>
              <article class="question-card">
                <div class="prompt-copy">
                  <For each={screen().question.prompt}>
                    {(block) => <PromptBlock block={block} />}
                  </For>
                </div>
                <ErrorBoundary
                  fallback={(error, reset) => (
                    <div class="inline-error" role="alert">
                      <p>The response controls could not load. Your run is still safe.</p>
                      <button class="quiet-action" type="button" onClick={reset}>
                        Try response controls again
                      </button>
                      <span class="visually-hidden">{String(error)}</span>
                    </div>
                  )}
                >
                  <Show
                    when={multipleChoiceDefinition(screen().question.response)}
                    fallback={
                      <p class="inline-error" role="alert">
                        This reference screen currently supports multiple-choice responses only.
                      </p>
                    }
                  >
                    {(definition) => (
                      <MultipleChoiceResponse
                        attemptId={screen().attempt.id}
                        definition={definition()}
                        validator={validator}
                        onSubmit={async (response: StudentResponse) => {
                          await runtime.client.submitResponse(
                            screen().attempt.id,
                            response,
                            crypto.randomUUID(),
                          );
                        }}
                        onEscape={() =>
                          navigate(
                            `/courses/${screen().course.id}/assignments/${screen().assignment.id}`,
                          )
                        }
                      />
                    )}
                  </Show>
                </ErrorBoundary>
              </article>
            </>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
