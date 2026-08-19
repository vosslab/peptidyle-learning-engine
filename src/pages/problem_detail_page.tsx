// problem_detail_page.tsx - safe current-question catalog detail and lineage projection.

import { A, createAsync, useParams } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap, type SessionBootstrapState } from "../auth/session_context";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { parseProblemRouteReference } from "../navigation/public_route";
import { rolesMayAccessRoute } from "../route_contract";
import { CatalogStatisticsPanel } from "./catalog_statistics_panel";

function mayReadCatalog(state: SessionBootstrapState): boolean {
  return (
    state.kind === "authenticated" && rolesMayAccessRoute("problemDetail", state.session.user.roles)
  );
}

function CatalogDetailDenied(): JSX.Element {
  return (
    <section class="page" data-route-surface="catalogDetailDenied" aria-live="polite">
      <p class="eyebrow">Published question library</p>
      <h1>Problem library is not available for this account</h1>
      <p>Your learning space remains available. Ask an instructor for library access.</p>
    </section>
  );
}

function CatalogDetailPage(): JSX.Element {
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
              <CatalogStatisticsPanel statistics={record().statistics} />
            </article>
          )}
        </Show>
      </Suspense>
    </section>
  );
}

/** Guards catalog detail transport with the same role contract as the library route. */
export function ProblemDetailPage(): JSX.Element {
  const session = useSessionBootstrap();
  return (
    <Show when={mayReadCatalog(session.state())} fallback={<CatalogDetailDenied />}>
      <CatalogDetailPage />
    </Show>
  );
}
