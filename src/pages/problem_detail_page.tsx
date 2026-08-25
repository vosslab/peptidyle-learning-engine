// problem_detail_page.tsx - safe current-question catalog detail and lineage projection.

import { A, createAsync, useParams } from "@solidjs/router";
import { Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { QuestionPromptRenderer } from "../components/question_renderer";
import { parseProblemRouteReference } from "../navigation/public_route";
import { CatalogStatisticsPanel, CatalogUsagePanel } from "./catalog_statistics_panel";
import "./problem_detail_page.css";

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
      .then((summary) => runtime.queries.catalogDetail(summary.questionId));
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
              <CopyableQuestionId displayId={record().summary.questionId} />
              <p aria-label="Published by">By {record().summary.byline.names.join(", ")}</p>
              <p>{`Backend: ${record().summary.backend}`}</p>
              <Show when={record().prompt.kind === "generatedExample"}>
                <aside class="catalog-generated-example" aria-label="Generated example">
                  <strong>Generated example</strong>
                  <p>
                    This example uses resolved values for catalog viewing. Assigned versions may use
                    different values.
                  </p>
                </aside>
              </Show>
              <section aria-label="Problem prompt">
                <QuestionPromptRenderer
                  blocks={record().prompt.blocks}
                  assetUrl={(asset) =>
                    new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                  }
                />
              </section>
              <CatalogStatisticsPanel evidence={record().evidence} />
              <CatalogUsagePanel usage={record().usage} />
            </article>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
