// question_pool_detail.tsx - direct Library detail for one reusable published Question Pool.

import { A, useNavigate } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import type { QuestionPoolId } from "../../../generated/api/QuestionPoolId";
import type { QuestionPoolMemberView } from "../../../generated/api/QuestionPoolMemberView";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import type {
  PoolMemberFilterKey,
  PoolMemberSortKey,
} from "../../components/question_pool_member_fields";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import { useApplicationApi } from "../../api/application_api";
import { createQuestionLibraryRepository } from "../../api/question_library_repository";
import { useSessionBootstrap } from "../../auth/session_context";
import { BloomClassificationText } from "../../components/bloom_classification";
import { InstructorProfileLink } from "../../components/instructor_profile_link";
import { PageFrame } from "../../components/page_frame/page_frame";
import { QuestionPoolMetadataEditor } from "../../components/question_pool_metadata_editor";
import { QuestionPoolMembersEditor } from "../../components/question_pool_members_editor";
import { mayEditQuestionPoolMembers } from "../../components/question_pool_members_model";
import { QuestionPoolStarControl } from "../../components/question_pool_star_control";
import { QuestionPoolSupportEditor } from "../../components/question_pool_support_editor";
import { QuestionPoolWatchControl } from "../../components/question_pool_watch_control";
import { questionNativePreview } from "../../components/question_native_preview";
import type { RecordContent } from "../../components/data_sheet/record_content";
import {
  comparePoolMemberFields,
  poolMemberRowId,
  questionPoolMemberFields,
} from "../../components/question_pool_member_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../../ribbon/route_scope_context";
import { backendLabel, questionTypeLabel } from "../../features/question_library/library_helpers";
import { QuestionStatisticsPanel } from "../../features/question_library/question_statistics_panel";

type PoolMemberSearchRow = {
  readonly member: QuestionPoolMemberView;
  readonly publishedQuestionRevisionTuple: QuestionPoolMemberView["publishedQuestionRevisionTuple"];
  readonly questionTitle: string;
  readonly questionLicense: string | null;
};

const memberFields: ReadonlyArray<
  RecordField<PoolMemberSearchRow, PoolMemberSortKey, PoolMemberFilterKey>
> = [
  ...questionPoolMemberFields<PoolMemberSearchRow>(),
  {
    id: "authors",
    label: "Authors",
    value: (row) =>
      row.member.question.question_library.summary.authorship.authors.map(
        (author) => author.displayName,
      ),
    render: (row) => (
      <For each={row.member.question.question_library.summary.authorship.authors}>
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
    defaultColumn: { priority: 4, width: "15rem" },
  },
];

function memberSearchRow(member: QuestionPoolMemberView): PoolMemberSearchRow {
  const summary = member.question.question_library.summary;
  return {
    member,
    publishedQuestionRevisionTuple: member.publishedQuestionRevisionTuple,
    questionTitle: summary.metadata.questionTitle,
    questionLicense: summary.metadata.questionLicense,
  };
}

function memberContent(row: PoolMemberSearchRow): RecordContent {
  return {
    title: row.questionTitle,
    preview: questionNativePreview({
      publishedQuestionRevisionTuple: row.publishedQuestionRevisionTuple,
      nativePrompt: row.member.question.question_library.nativePrompt,
    }),
    details: [
      {
        kind: "questionId",
        questionTitle: row.questionTitle,
        displayId: row.publishedQuestionRevisionTuple.publishedQuestionId,
      },
      {
        kind: "text",
        label: "Revision",
        value: String(row.publishedQuestionRevisionTuple.revisionNumber),
      },
      { kind: "text", label: "Question License", value: row.questionLicense ?? "Unavailable" },
      {
        kind: "questionAuthors",
        authors: row.member.question.question_library.summary.authorship.authors,
      },
    ],
    actions: [],
  };
}

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
  const currentPool = (): QuestionPoolView | undefined => {
    const pool = detail.latest;
    return pool?.questionPoolId === props.poolId ? pool : undefined;
  };
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
    const title = currentPool()?.metadata.title;
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

  /**
   * The read-only collection is deliberately mounted only for viewers. Pool
   * editors own their draft-backed collection in QuestionPoolMembersEditor.
   */
  function ReadonlyPoolMembers(): JSX.Element {
    const memberSearch = createLocalCompleteSearch(
      {
        rows: () => {
          const pool = detail();
          return pool === undefined ? undefined : pool.members.map(memberSearchRow);
        },
        loading: () => detail.loading,
        error: (): unknown => detail.error,
        refetchSource: refetch,
      },
      {
        fields: memberFields,
        rowId: poolMemberRowId,
        keyword: (row) =>
          `${row.questionTitle} ${row.publishedQuestionRevisionTuple.publishedQuestionId}`,
        compare: comparePoolMemberFields,
        content: memberContent,
      },
    );

    return (
      <section aria-labelledby="pool-members-heading">
        <h2 id="pool-members-heading">Questions in this Pool</h2>
        <SearchBlock
          layout="above"
          controls={<SearchControls state={memberSearch.state} textLabel="Search Pool Questions" />}
          state={memberSearch.state}
          collection={{
            ariaLabel: "Questions in this Pool",
            ...memberSearch.collection,
            emptyState: { title: "No Question Revisions match this search." },
          }}
        />
      </section>
    );
  }

  return (
    <PageFrame routeSurface="questionDetail">
      <A class="quiet-link" href="/library/browse">
        Return to Browse Question Library
      </A>
      <Show when={detail.loading}>
        <p class="loading-state" role="status">
          {currentPool() ? "Refreshing Question Pool..." : "Loading Question Pool..."}
        </p>
      </Show>
      <Show when={detail.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>{currentPool() ? "Question Pool refresh failed" : "Question Pool unavailable"}</h2>
          <p>
            {currentPool()
              ? "The current Pool view is still available. Retry to load the latest changes."
              : "Return to the library and try again."}
          </p>
          <button type="button" onClick={() => void refetch()}>
            Retry Question Pool
          </button>
        </section>
      </Show>
      <Show when={currentPool()}>
        {(value) => (
          <article aria-label="Question Pool detail">
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
            <QuestionStatisticsPanel evidence={value().evidence} onRefresh={refetch} />
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
                classificationClient={applicationApi.client}
                getQuestionRevision={applicationApi.client.getQuestionRevision}
                getCurrentQuestionSharedMetadata={
                  applicationApi.client.getCurrentQuestionSharedMetadata
                }
                reloadPool={async () => await refetch()}
                onSaved={() => void refetch()}
              />
            </Show>
            <Show when={!mayEditPoolMembers(value())}>
              <ReadonlyPoolMembers />
            </Show>
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
