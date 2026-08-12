// problem_detail_page.tsx - safe immutable catalog detail and lineage projection.

import { A, createAsync, useParams } from "@solidjs/router";
import { For, Show, Suspense, type JSX } from "solid-js";

import { useApiRuntime } from "../api/runtime";
import { CopyableProblemId } from "../components/copyable_problem_id";

function versionLink(problem: string, version: string): string {
  return `/library/${encodeURIComponent(problem)}/versions/${encodeURIComponent(version)}`;
}

export function ProblemDetailPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const detail = createAsync(() => {
    const problemId = params["problemId"];
    const versionId = params["versionId"];
    if (problemId === undefined || versionId === undefined) {
      throw new Error("The problem version address is incomplete.");
    }
    return runtime.queries.catalogDetail(problemId, versionId);
  });
  return (
    <section class="page problem-detail-page" data-route-surface="problemDetail">
      <A class="quiet-link" href="/library">
        Return to problem library
      </A>
      <Suspense
        fallback={
          <p class="loading-state" role="status">
            Loading immutable problem version...
          </p>
        }
      >
        <Show
          when={detail()}
          fallback={
            <section class="route-error" role="alert">
              <h1>Problem version unavailable</h1>
              <p>Return to the library and try again.</p>
            </section>
          }
        >
          {(record) => (
            <article>
              <p class="eyebrow">Immutable published version</p>
              <h1>{record().summary.metadata.title}</h1>
              <CopyableProblemId
                displayId={`P-${record().summary.publicId}-v${record().summary.versionNumber}`}
              />
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
              <Show when={record().summary.previousVersion}>
                <p>
                  <A
                    href={versionLink(
                      record().summary.problem,
                      record().summary.previousVersion ?? "",
                    )}
                  >
                    View previous version
                  </A>
                </p>
              </Show>
              <Show when={record().summary.derivedFrom}>
                {(origin) => (
                  <p>
                    <A href={versionLink(origin().problem, origin().version)}>
                      View source version
                    </A>
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
