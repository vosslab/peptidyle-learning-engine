// library_object_detail_page.tsx - one direct Library URL for Question and Pool detail.

import { useParams } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { parseQuestionRouteId } from "../navigation/public_route";
import { QuestionDetailPage } from "./question_detail_page";
import { QuestionPoolDetail } from "./question_pool_detail";

/** Resolves the authorized Library object kind before rendering its existing detail surface. */
export function LibraryObjectDetailPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const [resolved, { refetch }] = createResource(
    () => params["questionId"],
    async (publicId) => {
      if (publicId === undefined || parseQuestionRouteId(publicId) === null) {
        throw new Error("The Library object ID address is incomplete.");
      }
      return { publicId, kind: await applicationApi.client.getLibraryObjectKind(publicId) };
    },
  );

  return (
    <Show
      when={resolved.loading}
      fallback={
        <Show
          when={resolved.error === undefined && resolved()}
          keyed
          fallback={
            <PageFrame routeSurface="questionDetail" title="Library object unavailable">
              <section class="route-error" role="alert">
                <h2>Library object unavailable</h2>
                <p>Return to the library and try again.</p>
                <button type="button" onClick={() => void refetch()}>
                  Retry Library detail
                </button>
              </section>
            </PageFrame>
          }
        >
          {(object) => (
            <Show
              when={object.kind.kind === "question"}
              fallback={<QuestionPoolDetail poolId={object.publicId} />}
            >
              <QuestionDetailPage />
            </Show>
          )}
        </Show>
      }
    >
      <PageFrame routeSurface="questionDetail" title="Question Library">
        <p class="loading-state" role="status">
          Loading Library detail...
        </p>
      </PageFrame>
    </Show>
  );
}
