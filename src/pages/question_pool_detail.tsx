// question_pool_detail.tsx - direct Library detail for one reusable published Question Pool.

import { A } from "@solidjs/router";
import {
  Show,
  createEffect,
  createResource,
  createSignal,
  on,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolMemberView } from "../../generated/api/QuestionPoolMemberView";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import {
  BloomClassificationEditor,
  BloomClassificationText,
} from "../components/bloom_classification";
import { PageFrame } from "../components/page_frame";
import { QuestionPoolStarControl } from "../components/question_pool_star_control";
import { QuestionPoolSupportEditor } from "../components/question_pool_support_editor";
import { QuestionPoolWatchControl } from "../components/question_pool_watch_control";
import { RecordSequence } from "../components/record_list/record_sequence";
import type { RecordContent } from "../components/record_list/record_list";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../ribbon/route_scope_context";
import { backendLabel, questionTypeLabel } from "./library_page_helpers";

function poolMemberContent(member: QuestionPoolMemberView): RecordContent {
  const revision = member.publishedQuestionRevisionTuple;
  return {
    title: member.question.question_library.summary.metadata.questionTitle,
    details: [
      { kind: "text", label: "Published Question ID", value: revision.publishedQuestionId },
      { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
      {
        kind: "text",
        label: "Question License",
        value: member.question.question_library.summary.metadata.questionLicense ?? "Unavailable",
      },
      {
        kind: "questionAuthors",
        label: "Authors",
        authors: member.question.question_library.summary.authorship.authors,
      },
    ],
    actions: [],
  };
}

/** Direct, current-detail view for a Pool whose kind was resolved by the Library route. */
export function QuestionPoolDetail(props: { readonly poolId: QuestionPoolId }): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  const [correctedBloom, setCorrectedBloom] = createSignal<BloomClassificationView>();
  const [detail, { refetch }] = createResource(
    () => props.poolId,
    (poolId) => applicationApi.client.getQuestionPool(poolId),
  );
  const mayMutateLibrary = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  };
  createEffect(
    on(
      () => props.poolId,
      () => setCorrectedBloom(undefined),
    ),
  );
  function publishLoadedPoolTitle(): void {
    if (publication === undefined) return;
    const title = detail.error === undefined ? detail()?.metadata.title : undefined;
    if (title === undefined || title.length === 0) {
      clearRouteScopeLabels(publication);
      return;
    }
    publishRouteScopeLabels(publication, { libraryObjectTitle: title });
  }
  onMount(() => {
    publication = routeScopePublication();
    publishLoadedPoolTitle();
  });
  createEffect(() => {
    publishLoadedPoolTitle();
  });
  onCleanup(() => {
    if (publication !== undefined) clearRouteScopeLabels(publication);
  });

  return (
    <PageFrame
      routeSurface="questionDetail"
      title={(detail.error === undefined ? detail()?.metadata.title : undefined) ?? "Question Pool"}
      eyebrow="Question Library"
    >
      <A class="quiet-link" href="/library/browse">
        Return to Browse Question Library
      </A>
      <Show when={detail.loading}>
        <p class="loading-state" role="status">
          Loading Question Pool...
        </p>
      </Show>
      <Show when={detail.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Question Pool unavailable</h2>
          <p>Return to the library and try again.</p>
          <button type="button" onClick={() => void refetch()}>
            Retry Question Pool
          </button>
        </section>
      </Show>
      <Show when={!detail.loading && detail.error === undefined && detail()}>
        {(value) => (
          <article aria-label="Question Pool detail">
            <p class="eyebrow">Published Question Pool</p>
            <p>{value().metadata.description}</p>
            <dl class="question-detail-metadata">
              <div>
                <dt>Discipline</dt>
                <dd>
                  {value().metadata.disciplineName}
                  <Show when={value().metadata.disciplineIsRetired}> (retired)</Show>
                </dd>
              </div>
              <div>
                <dt>Question Type</dt>
                <dd>{questionTypeLabel(value().questionType)}</dd>
              </div>
              <div>
                <dt>Backend</dt>
                <dd>{backendLabel(value().backend)}</dd>
              </div>
              <div>
                <dt>Pool ID</dt>
                <dd>{value().questionPoolId}</dd>
              </div>
              <div>
                <dt>Edit</dt>
                <dd>{value().questionPoolEditNumber}</dd>
              </div>
              <div>
                <dt>Calculated Pool License</dt>
                <dd>{value().license}</dd>
              </div>
              <div>
                <dt>Members</dt>
                <dd>{value().members.length}</dd>
              </div>
              <div>
                <dt>Tags</dt>
                <dd>{value().metadata.tags.join(", ") || "None"}</dd>
              </div>
              <Show when={correctedBloom() ?? value().bloom}>
                {(bloom) => (
                  <div>
                    <dt>Bloom Classification</dt>
                    <dd>
                      <BloomClassificationText bloom={bloom()} />
                    </dd>
                  </div>
                )}
              </Show>
            </dl>
            <Show when={mayMutateLibrary()}>
              <QuestionPoolSupportEditor
                client={applicationApi.client}
                questionPoolId={value().questionPoolId}
              />
            </Show>
            <Show when={mayMutateLibrary() && (correctedBloom() ?? value().bloom)}>
              {(bloom) => (
                <BloomClassificationEditor
                  targetName="Question Pool"
                  contentMarkerKind="Edit"
                  contentMarkerNumber={value().questionPoolEditNumber}
                  bloom={bloom()}
                  save={(request) =>
                    applicationApi.client
                      .correctQuestionPoolBloom(value().questionPoolId, request)
                      .then((receipt) => receipt.bloom)
                  }
                  loadCurrent={() =>
                    applicationApi.client.getQuestionPool(value().questionPoolId).then((loaded) => {
                      if (loaded.bloom === null) {
                        throw new Error("Bloom Classification is not assigned.");
                      }
                      return loaded.bloom;
                    })
                  }
                  onCurrent={setCorrectedBloom}
                  onConflictCurrent={setCorrectedBloom}
                  onAccepted={() => undefined}
                />
              )}
            </Show>
            <Show when={mayMutateLibrary()}>
              <div class="question-detail-support-actions">
                <QuestionPoolStarControl poolId={value().questionPoolId} />
                <QuestionPoolWatchControl poolId={value().questionPoolId} />
              </div>
            </Show>
            <section aria-labelledby="pool-members-heading">
              <h2 id="pool-members-heading">Exact Question Revisions</h2>
              <RecordSequence
                rows={value().members}
                content={poolMemberContent}
                recordId={(member) =>
                  `${member.publishedQuestionRevisionTuple.publishedQuestionId}:${member.publishedQuestionRevisionTuple.revisionNumber}`
                }
                state={{ kind: "ready" }}
                ariaLabel="Ordered exact Question Revisions"
                emptyState={{ title: "No Question Revisions are in this Pool." }}
              />
            </section>
          </article>
        )}
      </Show>
    </PageFrame>
  );
}
