// problem_detail_page.tsx - safe current-question catalog detail and lineage projection.

import { A, createAsync, useParams } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { CopyableProblemId } from "../components/copyable_problem_id";
import { parseProblemRouteReference } from "../navigation/public_route";

export function ProblemDetailPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const detail = createAsync(() => {
    const problemReference = params["problemRef"];
    if (problemReference === undefined || parseProblemRouteReference(problemReference) === null) {
      throw new Error("The Question ID address is incomplete.");
    }
    return runtime.client
      .resolveCatalogProblem(problemReference)
      .then((summary) => runtime.queries.catalogDetail(summary.problem, summary.version));
  });
  return (
    <section class="page problem-detail-page" data-route-surface="problemDetail">
      <A class="quiet-link" href="/library">
        Return to problem library
      </A>
      <Suspense
        fallback={
          <p class="loading-state" role="status">
            Loading question...
          </p>
        }
      >
        <Show
          when={detail()}
          fallback={
            <section class="route-error" role="alert">
              <h1>Question unavailable</h1>
              <p>Return to the library and try again.</p>
            </section>
          }
        >
          {(record) => (
            <article>
              <p class="eyebrow">Published question</p>
              <h1>{record().summary.metadata.title}</h1>
              <CopyableProblemId displayId={record().summary.questionId} />
              <p>{`Backend: ${record().summary.backend}`}</p>
              <p>
                {record().statistics === "unavailable"
                  ? "Anonymous learning statistics are not available yet."
                  : ""}
              </p>
              <section aria-label="Problem prompt">
                <For each={record().prompt}>
                  {(block) => (
                    <p>
                      {block.kind === "text"
                        ? block.markdown
                        : block.kind === "math"
                          ? block.description
                          : block.kind === "image"
                            ? block.description
                            : block.kind === "code"
                              ? block.source
                              : block.description}
                    </p>
                  )}
                </For>
              </section>
              <Show when={record().summary.derivedFrom}>
                {(_origin) => (
                  <p>
                    <span>Source lineage is available from the problem library.</span>
                  </p>
                )}
              </Show>
            </article>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
