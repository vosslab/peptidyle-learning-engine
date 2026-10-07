// question_pool_detail.tsx - direct Library detail for one reusable published Question Pool.

import { A, useNavigate } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolMemberView } from "../../generated/api/QuestionPoolMemberView";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import { useApplicationApi } from "../api/application_api";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useSessionBootstrap } from "../auth/session_context";
import { BloomClassificationText } from "../components/bloom_classification";
import { InstructorProfileLink } from "../components/instructor_profile_link";
import { PageFrame } from "../components/page_frame";
import { QuestionPoolMetadataEditor } from "../components/question_pool_metadata_editor";
import { QuestionPoolMembersEditor } from "../components/question_pool_members_editor";
import { mayEditQuestionPoolMembers } from "../components/question_pool_members_model";
import { QuestionPoolStarControl } from "../components/question_pool_star_control";
import { QuestionPoolSupportEditor } from "../components/question_pool_support_editor";
import { QuestionPoolWatchControl } from "../components/question_pool_watch_control";
import { RecordSortControl } from "../components/record_list/record_sort_control";
import { RecordTable, type RecordTableColumn } from "../components/record_list/record_table";
import {
  questionPoolMemberSortOptions,
  sortQuestionPoolMembers,
  type QuestionPoolMemberSort,
} from "../components/record_list/question_pool_member_sort";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../ribbon/route_scope_context";
import { backendLabel, questionTypeLabel } from "./library_page_helpers";
import { QuestionStatisticsPanel } from "./question_statistics_panel";

/** Direct, current-detail view for a Pool whose kind was resolved by the Library route. */
export function QuestionPoolDetail(props: { readonly poolId: QuestionPoolId }): JSX.Element {
  const applicationApi = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(applicationApi.client);
  const session = useSessionBootstrap();
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  const [detail, { refetch }] = createResource(
    () => props.poolId,
    (poolId) => applicationApi.client.getQuestionPool(poolId),
  );
  const [memberSort, setMemberSort] = createSignal<QuestionPoolMemberSort>("as-loaded");
  const sortedMembers = createMemo(() =>
    sortQuestionPoolMembers(detail()?.members ?? [], memberSort()),
  );
  const memberColumns: ReadonlyArray<RecordTableColumn<QuestionPoolMemberView>> = [
    {
      id: "published-question-id",
      header: "Published Question ID",
      cell: (member) => member.publishedQuestionRevisionTuple.publishedQuestionId,
    },
    {
      id: "revision",
      header: "Revision",
      cell: (member) => String(member.publishedQuestionRevisionTuple.revisionNumber),
    },
    {
      id: "question-license",
      header: "Question License",
      cell: (member) =>
        member.question.question_library.summary.metadata.questionLicense ?? "Unavailable",
    },
    {
      id: "authors",
      header: "Authors",
      cell: (member) => (
        <For each={member.question.question_library.summary.authorship.authors}>
          {(author, index) => (
            <>
              {index() > 0 ? ", " : ""}
              <Show when={author.accountId} fallback={author.displayName}>
                {(accountId) => (
                  <InstructorProfileLink accountId={accountId()} displayName={author.displayName} />
                )}
              </Show>
            </>
          )}
        </For>
      ),
    },
  ];
  const mayMutateLibrary = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  };
  const mayViewPoolSupport = (): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" &&
      (state.session.account.userRole === "instructor" ||
        state.session.account.userRole === "sysadmin")
    );
  };
  const mayEditPoolMembers = (pool: QuestionPoolView): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" &&
      mayEditQuestionPoolMembers(
        state.session.account.userRole,
        state.session.account.id,
        pool.ownerAccountId,
      )
    );
  };
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
                <dt>Pool Edit Number</dt>
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
              <div>
                <dt>Bloom Classification</dt>
                <dd>
                  <BloomClassificationText bloom={value().bloom} />
                </dd>
              </div>
            </dl>
            <QuestionStatisticsPanel evidence={value().evidence} />
            <QuestionPoolMetadataEditor
              questionPoolId={value().questionPoolId}
              pool={value()}
              canEdit={value().canEditMetadata}
              onSaved={() => void refetch()}
            />
            <Show when={mayViewPoolSupport()}>
              <QuestionPoolSupportEditor
                client={applicationApi.client}
                questionPoolId={value().questionPoolId}
                canEdit={value().canEditMetadata}
              />
            </Show>
            <Show when={mayMutateLibrary()}>
              <div class="question-detail-support-actions">
                <QuestionPoolForkControl poolId={value().questionPoolId} />
                <QuestionPoolStarControl poolId={value().questionPoolId} />
                <QuestionPoolWatchControl poolId={value().questionPoolId} />
              </div>
            </Show>
            <Show when={mayEditPoolMembers(value())}>
              <QuestionPoolMembersEditor
                pool={value()}
                client={applicationApi.client}
                questionLibrary={questionLibrary}
                getQuestionDetails={applicationApi.client.getQuestionDetails}
                getCurrentQuestionSharedMetadata={
                  applicationApi.client.getCurrentQuestionSharedMetadata
                }
                reloadPool={async () => await refetch()}
                onSaved={() => void refetch()}
              />
            </Show>
            <section aria-labelledby="pool-members-heading">
              <h2 id="pool-members-heading">Questions in this Pool</h2>
              <RecordSortControl
                label="Sort Pool Questions"
                options={questionPoolMemberSortOptions}
                value={memberSort()}
                onChange={setMemberSort}
              />
              <RecordTable
                rows={sortedMembers()}
                columns={memberColumns}
                rowId={(member) =>
                  `${member.publishedQuestionRevisionTuple.publishedQuestionId}:${member.publishedQuestionRevisionTuple.revisionNumber}`
                }
                state={{ kind: "ready" }}
                rowHeader={{
                  id: "question-title",
                  header: "Question Title",
                  content: (member) =>
                    member.question.question_library.summary.metadata.questionTitle,
                }}
                ariaLabel="Questions in this Pool"
                emptyState={{ title: "No Question Revisions are in this Pool." }}
              />
            </section>
          </article>
        )}
      </Show>
    </PageFrame>
  );
}

function QuestionPoolForkControl(props: { readonly poolId: QuestionPoolId }): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const [forking, setForking] = createSignal(false);
  const [error, setError] = createSignal("");
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  async function forkQuestionPool(): Promise<void> {
    if (forking()) return;
    setForking(true);
    setError("");
    try {
      const fork = await applicationApi.client.forkQuestionPool(props.poolId);
      if (disposed) return;
      navigate(`/library/${encodeURIComponent(fork.questionPoolId)}`);
    } catch {
      if (!disposed) setError("The Question Pool could not be forked. Try again.");
    } finally {
      if (!disposed) setForking(false);
    }
  }

  return (
    <section aria-label="Fork this Question Pool" aria-busy={forking()}>
      <button type="button" disabled={forking()} onClick={() => void forkQuestionPool()}>
        {forking() ? "Forking Pool..." : error() ? "Retry Fork" : "Fork"}
      </button>
      <Show when={forking()}>
        <span role="status">Forking Question Pool...</span>
      </Show>
      <Show when={error()}>
        <span role="alert">{error()}</span>
      </Show>
    </section>
  );
}
