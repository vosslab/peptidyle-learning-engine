// question_detail_page.tsx - safe current Question Details and Question Revision lineage View.

import { A, createAsync, useLocation, useNavigate, useParams } from "@solidjs/router";
import {
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  Show,
  Suspense,
  type JSX,
} from "solid-js";

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionRevisionNumber } from "../../generated/api/QuestionRevisionNumber";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import type {
  LoadedQuestionLineage,
  QuestionAvailabilityClient,
} from "../api/question_availability";
import { ApiRequestError } from "../api/http_client/error";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { CopyableQuestionId } from "../components/copyable_question_id";
import { PageFrame } from "../components/page_frame";
import { BloomClassificationText } from "../components/bloom_classification";
import { OpaqueWebworkPreviewFrame } from "../components/opaque_webwork_preview_frame";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import { QuestionMetadataEditor } from "../features/question_metadata/question_metadata_editor";
import { QuestionWatchControl } from "../components/question_watch_control";
import { QuestionStarControl } from "../components/question_star_control";
import { InstructorProfileLink } from "../components/instructor_profile_link";
import { QuestionPromptRenderer } from "../components/question_renderer";
import { QuestionResponsePreviewControl } from "../components/question_response_preview";
import { parseQuestionRouteId } from "../navigation/public_route";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../ribbon/route_scope_context";
import { QuestionStatisticsPanel, QuestionUsePanel } from "./question_statistics_panel";
import "./question_detail_page.css";

function webworkFormatLabel(value: string | null): string | null {
  if (value === "webworkPg") return "PG";
  if (value === "webworkPgml") return "PGML";
  return null;
}

type QuestionLineageLoad =
  { readonly kind: "ready"; readonly value: LoadedQuestionLineage } | { readonly kind: "error" };

type ArchiveNotice = {
  readonly kind: "status" | "alert";
  readonly text: string;
};

const QUESTION_REVISION_QUERY_PARAMETER = "revisionNumber";

function QuestionForkControl(props: {
  readonly sourceRevisionTuple: PublishedQuestionRevisionTuple;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const [forking, setForking] = createSignal(false);
  const [error, setError] = createSignal("");
  let action:
    | { readonly sourceRevisionTuple: PublishedQuestionRevisionTuple; readonly key: string }
    | undefined;
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  async function forkPublishedQuestion(): Promise<void> {
    const sourceRevisionTuple = props.sourceRevisionTuple;
    if (forking()) return;
    setForking(true);
    setError("");
    try {
      // ASVS 2.3.1: an uncertain retry keeps this exact source Revision and opaque request key.
      if (
        action === undefined ||
        action.sourceRevisionTuple.publishedQuestionId !==
          sourceRevisionTuple.publishedQuestionId ||
        action.sourceRevisionTuple.revisionNumber !== sourceRevisionTuple.revisionNumber
      ) {
        action = { sourceRevisionTuple, key: crypto.randomUUID() };
      }
      const fork = await applicationApi.client.forkPublishedQuestion(
        sourceRevisionTuple,
        action.key,
      );
      if (disposed) return;
      // ASVS 1.2.2: only the exact server-returned private Draft UUID selects this local route.
      navigate(`/authoring/drafts/${encodeURIComponent(fork.draftQuestion)}`);
    } catch {
      // ASVS 16.5.1, 16.5.3: no response body or transport detail reaches the Instructor.
      if (!disposed) {
        setError(
          "The private Draft could not be confirmed. Retry to confirm the same fork request.",
        );
      }
    } finally {
      if (!disposed) setForking(false);
    }
  }

  return (
    <section
      class="question-fork-control"
      aria-label="Fork this Published Question"
      aria-busy={forking()}
    >
      <h2>Fork this Published Question</h2>
      <p>
        Fork Revision {props.sourceRevisionTuple.revisionNumber} into your own private Draft
        Question. It must pass publication validation before it can join the Question Library.
      </p>
      <button type="button" disabled={forking()} onClick={() => void forkPublishedQuestion()}>
        {forking()
          ? "Creating private Draft..."
          : error()
            ? "Retry fork creation"
            : "Fork Question"}
      </button>
      <Show when={forking()}>
        <p role="status">Creating your private Draft Question...</p>
      </Show>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
    </section>
  );
}

function QuestionPoolFromQuestionControl(props: { readonly detail: QuestionDetails }): JSX.Element {
  const applicationApi = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(applicationApi.client);
  const [createOpen, setCreateOpen] = createSignal(false);
  let trigger: HTMLButtonElement | undefined;

  function closeCreation(): void {
    setCreateOpen(false);
    queueMicrotask(() => trigger?.focus());
  }

  return (
    <Show
      when={createOpen()}
      fallback={
        <section
          class="question-pool-from-question-control"
          aria-label="Create Pool from this Published Question"
        >
          <h2>Create Pool from Question</h2>
          <p>
            Start a reusable Question Pool with this exact Revision first and its current Discipline
            and Subject.
          </p>
          <button
            ref={(element) => (trigger = element)}
            type="button"
            onClick={() => setCreateOpen(true)}
          >
            Create Pool from Question
          </button>
        </section>
      }
    >
      <QuestionPoolCreateDialog
        questionPoolClient={applicationApi.client}
        questionLibrary={questionLibrary}
        getQuestionDetails={applicationApi.client.getQuestionDetails}
        getCurrentQuestionSharedMetadata={applicationApi.client.getCurrentQuestionSharedMetadata}
        startingQuestion={{
          publishedQuestionRevisionTuple: props.detail.summary.publishedQuestionRevisionTuple,
          questionTitle: props.detail.summary.metadata.questionTitle,
          disciplineName: props.detail.disciplineName,
          subjectName: props.detail.subjectName,
          questionType: props.detail.summary.questionType,
          backend: props.detail.summary.backend,
        }}
        onTaskPhaseChange={() => undefined}
        onClose={closeCreation}
      />
    </Show>
  );
}

function questionRevisionNumberFromSearch(search: string): QuestionRevisionNumber | undefined {
  const values = new URLSearchParams(search).getAll(QUESTION_REVISION_QUERY_PARAMETER);
  if (values.length === 0) return undefined;
  const [value] = values;
  if (value === undefined || values.length !== 1 || !/^[1-9][0-9]*$/u.test(value))
    throw new Error("The Question Revision address is incomplete.");
  const revisionNumber = Number(value);
  if (!Number.isSafeInteger(revisionNumber) || revisionNumber > 4_294_967_295)
    throw new Error("The Question Revision address is incomplete.");
  return revisionNumber;
}

function archiveFailureMessage(error: unknown): string {
  if (!(error instanceof ApiRequestError))
    return "Published Question could not archive. Try again.";
  if (error.status === 412)
    return "Question availability changed elsewhere. Review the current state and try again.";
  if (error.status === 409) return "This Question is no longer available to archive.";
  if (error.status === 422) return "Enter the current Published Question title exactly.";
  if ([401, 403, 404].includes(error.status))
    return "This Published Question is unavailable for this action.";
  return "Published Question could not archive. Try again.";
}

export interface QuestionArchiveControlProps {
  readonly client: Pick<
    QuestionAvailabilityClient,
    "getQuestionLineage" | "archiveQuestion" | "restoreQuestion"
  >;
  readonly questionId: PublishedQuestionId;
  /** Renders a related action only while the loaded Published Question remains available. */
  readonly renderAvailableAction?: () => JSX.Element;
  /** Owner and Sysadmin content correction for the current available Revision. */
  readonly renderCorrectionAction?: (
    sourceRevisionTuple: PublishedQuestionRevisionTuple,
  ) => JSX.Element;
}

export function QuestionArchiveControl(props: QuestionArchiveControlProps): JSX.Element {
  const [lineage, { mutate: mutateLineage, refetch: refetchLineage }] = createResource(
    () => props.questionId,
    async (questionId): Promise<QuestionLineageLoad> => {
      try {
        return { kind: "ready", value: await props.client.getQuestionLineage(questionId) };
      } catch {
        return { kind: "error" };
      }
    },
  );
  const [archiveOpen, setArchiveOpen] = createSignal(false);
  const [archiveConfirmation, setArchiveConfirmation] = createSignal("");
  const [archiving, setArchiving] = createSignal(false);
  const [archiveNotice, setArchiveNotice] = createSignal<ArchiveNotice>();
  const [recoveryUnavailable, setRecoveryUnavailable] = createSignal(false);
  const readyLineage = (): LoadedQuestionLineage | undefined => {
    const loaded = lineage();
    return loaded?.kind === "ready" ? loaded.value : undefined;
  };
  const archiveLineage = (): LoadedQuestionLineage | undefined => {
    return readyLineage();
  };
  const availableLineage = (): LoadedQuestionLineage | undefined => {
    const current = readyLineage();
    return current?.summary.availability.availability === "available" ? current : undefined;
  };

  function cancelArchive(): void {
    setArchiveOpen(false);
    setArchiveConfirmation("");
    setArchiveNotice(undefined);
    setRecoveryUnavailable(false);
  }

  async function archivePublishedQuestion(): Promise<void> {
    const currentLineage = lineage();
    if (
      currentLineage?.kind !== "ready" ||
      !currentLineage.value.viewerMayArchive ||
      currentLineage.value.summary.questionId !== props.questionId ||
      currentLineage.value.summary.availability.availability !== "available" ||
      archiveConfirmation() !== currentLineage.value.summary.metadata.questionTitle ||
      archiving()
    )
      return;

    setArchiving(true);
    setArchiveNotice(undefined);
    try {
      // ASVS 8.2.1, 8.2.2, 8.3.1: this UI gate is only an affordance. The
      // server reauthorizes the current Instructor, exact Question, and owner.
      const transition = await props.client.archiveQuestion(
        props.questionId,
        archiveConfirmation(),
        currentLineage.value.questionAvailabilityEditNumber,
      );
      mutateLineage({
        kind: "ready",
        value: {
          summary: {
            ...currentLineage.value.summary,
            availability: { availability: transition.availability },
          },
          viewerMayArchive: currentLineage.value.viewerMayArchive,
          viewerMayEditMetadata: currentLineage.value.viewerMayEditMetadata,
          questionAvailabilityEditNumber: transition.questionAvailabilityEditNumber,
        },
      });
      setArchiveOpen(false);
      setArchiveConfirmation("");
      setArchiveNotice({
        kind: "status",
        text: "Published Question archived. It no longer appears in normal Question Library discovery. Existing exact Revisions and Student Work are unchanged.",
      });
    } catch (error: unknown) {
      // ASVS 16.5.1, 16.5.3: preserve a fail-closed action and expose only
      // status-classified recovery copy, never a response body or internal detail.
      if (error instanceof ApiRequestError && [404, 409, 412, 422].includes(error.status)) {
        const refreshed = await refetchLineage();
        if (refreshed?.kind !== "ready") {
          setRecoveryUnavailable(true);
          setArchiveOpen(false);
          setArchiveConfirmation("");
          setArchiveNotice({
            kind: "status",
            text: "Current Question availability could not be loaded. Return to the Question Library to continue.",
          });
        } else if (refreshed.value.summary.availability.availability === "archived") {
          setRecoveryUnavailable(false);
          setArchiveOpen(false);
          setArchiveConfirmation("");
          setArchiveNotice({
            kind: "status",
            text: "This Published Question is already archived and absent from normal Question Library discovery.",
          });
        } else {
          setRecoveryUnavailable(false);
          if (
            error.status === 422 ||
            refreshed.value.summary.metadata.questionTitle !==
              currentLineage.value.summary.metadata.questionTitle
          ) {
            setArchiveConfirmation("");
            setArchiveNotice({
              kind: "alert",
              text: "Question title changed. Enter the current Published Question title exactly.",
            });
          } else {
            setArchiveNotice({ kind: "alert", text: archiveFailureMessage(error) });
          }
        }
      } else {
        setArchiveNotice({ kind: "alert", text: archiveFailureMessage(error) });
      }
    } finally {
      setArchiving(false);
    }
  }

  async function restorePublishedQuestion(): Promise<void> {
    const current = lineage();
    if (
      current?.kind !== "ready" ||
      current.value.summary.availability.availability !== "archived" ||
      archiving()
    )
      return;
    setArchiving(true);
    setArchiveNotice(undefined);
    try {
      const transition = await props.client.restoreQuestion(
        props.questionId,
        current.value.questionAvailabilityEditNumber,
      );
      mutateLineage({
        kind: "ready",
        value: {
          ...current.value,
          summary: {
            ...current.value.summary,
            availability: { availability: transition.availability },
          },
          questionAvailabilityEditNumber: transition.questionAvailabilityEditNumber,
        },
      });
      setArchiveNotice({
        kind: "status",
        text: "Published Question restored to normal availability.",
      });
    } catch {
      setArchiveNotice({
        kind: "alert",
        text: "Published Question could not be restored. Reload and try again.",
      });
    } finally {
      setArchiving(false);
    }
  }

  return (
    <>
      <Show when={lineage()?.kind === "error" && !recoveryUnavailable()}>
        <p class="question-archive-unavailable" role="status">
          Archive controls are unavailable. Reload the page to try again.
        </p>
      </Show>
      <Show when={archiveLineage()}>
        {(currentLineage) => (
          <Show
            when={currentLineage().summary.availability.availability === "available"}
            fallback={
              <section class="question-archive-status" aria-label="Archived Published Question">
                <p role="status">
                  This Published Question is archived and read-only. It no longer appears in normal
                  Question Library discovery; existing references remain intact.
                </p>
                <Show when={currentLineage().viewerMayArchive}>
                  <button
                    type="button"
                    disabled={archiving()}
                    onClick={() => void restorePublishedQuestion()}
                  >
                    {archiving() ? "Restoring..." : "Restore Published Question"}
                  </button>
                </Show>
              </section>
            }
          >
            <Show when={currentLineage().viewerMayArchive}>
              <Show
                when={archiveOpen()}
                fallback={
                  <section class="question-archive-action" aria-label="Question availability">
                    <button
                      type="button"
                      class="question-archive-open"
                      onClick={() => {
                        setArchiveOpen(true);
                        setArchiveNotice(undefined);
                        setRecoveryUnavailable(false);
                      }}
                    >
                      Archive Published Question
                    </button>
                  </section>
                }
              >
                <aside
                  class="question-archive-danger-zone"
                  aria-labelledby="question-archive-heading"
                >
                  <h2 id="question-archive-heading">Danger Zone: Archive Published Question</h2>
                  <p>
                    Archiving removes this Published Question from normal Question Library
                    discovery. Existing exact Revisions and Student Work remain unchanged.
                  </p>
                  <label for="question-archive-confirmation">
                    Type <strong>{currentLineage().summary.metadata.questionTitle}</strong> to
                    confirm
                  </label>
                  <input
                    id="question-archive-confirmation"
                    autocomplete="off"
                    value={archiveConfirmation()}
                    disabled={archiving()}
                    onInput={(event) => setArchiveConfirmation(event.currentTarget.value)}
                  />
                  <div class="question-archive-actions">
                    <button
                      type="button"
                      class="question-archive-confirm"
                      disabled={
                        archiving() ||
                        archiveConfirmation() !== currentLineage().summary.metadata.questionTitle
                      }
                      onClick={() => void archivePublishedQuestion()}
                    >
                      {archiving() ? "Archiving..." : "Archive Published Question"}
                    </button>
                    <button type="button" disabled={archiving()} onClick={cancelArchive}>
                      Cancel
                    </button>
                  </div>
                </aside>
              </Show>
            </Show>
          </Show>
        )}
      </Show>
      <Show when={archiveNotice()}>
        {(notice) => (
          <p class="question-archive-notice" role={notice().kind} aria-live="polite">
            {notice().text}
          </p>
        )}
      </Show>
      <Show when={availableLineage()}>
        {(currentLineage) => (
          <>
            {props.renderAvailableAction?.()}
            <Show when={currentLineage().viewerMayEditMetadata}>
              {props.renderCorrectionAction?.(
                currentLineage().summary.publishedQuestionRevisionTuple,
              )}
            </Show>
          </>
        )}
      </Show>
    </>
  );
}

function QuestionCorrectionDraftControl(props: {
  readonly sourceRevisionTuple: PublishedQuestionRevisionTuple;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal("");
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  async function openCorrectionDraft(): Promise<void> {
    if (creating()) return;
    setCreating(true);
    setError("");
    try {
      const sourceRevisionTuple = props.sourceRevisionTuple;
      const created = await applicationApi.client.createCorrectionDraft(sourceRevisionTuple);
      if (disposed) return;
      const query = new URLSearchParams({
        correctionQuestionId: sourceRevisionTuple.publishedQuestionId,
        correctionRevisionNumber: String(sourceRevisionTuple.revisionNumber),
      });
      navigate(`/authoring/drafts/${encodeURIComponent(created.draftQuestion)}?${query}`);
    } catch {
      if (!disposed)
        setError("A correction Draft could not be opened. Reload the Question and try again.");
    } finally {
      if (!disposed) setCreating(false);
    }
  }

  return (
    <section class="question-correction-control" aria-label="Correct Published Question content">
      <h2>Correct Published Question content</h2>
      <p>
        Open the current source and support in a separate private Draft. Publish the correction
        through the existing Question Revision operation.
      </p>
      <button type="button" disabled={creating()} onClick={() => void openCorrectionDraft()}>
        {creating() ? "Opening correction Draft..." : "Open correction Draft"}
      </button>
      <Show when={creating()}>
        <p role="status">Copying the current Published Question into a private Draft...</p>
      </Show>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
    </section>
  );
}

export function QuestionDetailPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  const params = useParams();
  const location = useLocation();
  const mayMutateLibrary = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  };
  const [metadataReload, setMetadataReload] = createSignal(0);
  const detail = createAsync((): Promise<QuestionDetails> => {
    metadataReload();
    const questionId = params["questionId"];
    if (questionId === undefined || parseQuestionRouteId(questionId) === null) {
      throw new Error("The Question ID address is incomplete.");
    }
    const revisionNumber = questionRevisionNumberFromSearch(location.search);
    if (revisionNumber !== undefined)
      return applicationApi.client.getQuestionRevision({
        publishedQuestionId: questionId,
        revisionNumber,
      });
    return applicationApi.client
      .resolveQuestion(questionId)
      .then((summary) =>
        applicationApi.client.getQuestionRevision(summary.publishedQuestionRevisionTuple),
      );
  });
  function publishLoadedQuestionTitle(): void {
    if (publication === undefined) return;
    const title = detail()?.summary.metadata.questionTitle;
    if (title === undefined || title.length === 0) {
      clearRouteScopeLabels(publication);
      return;
    }
    publishRouteScopeLabels(publication, { libraryObjectTitle: title });
  }
  onMount(() => {
    publication = routeScopePublication();
    publishLoadedQuestionTitle();
  });
  createEffect(() => {
    publishLoadedQuestionTitle();
  });
  onCleanup(() => {
    if (publication !== undefined) clearRouteScopeLabels(publication);
  });
  return (
    <PageFrame
      routeSurface="questionDetail"
      title={detail()?.summary.metadata.questionTitle ?? "Question"}
      eyebrow="Question Library"
    >
      <A class="quiet-link" href="/library/browse">
        Return to Browse Question Library
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
              <h2>Question unavailable</h2>
              <p>Return to the library and try again.</p>
            </section>
          }
        >
          {(record) => (
            <article>
              <p class="eyebrow">Published question</p>
              <section aria-label="Question prompt">
                <Show
                  when={record().summary.backend === "webwork"}
                  fallback={
                    <QuestionPromptRenderer
                      blocks={record().prompt.blocks}
                      publishedQuestionRevisionTuple={
                        record().summary.publishedQuestionRevisionTuple
                      }
                      questionImageUrl={(asset) =>
                        new URL(
                          applicationApi.client.questionImageUrl(
                            record().summary.publishedQuestionRevisionTuple,
                            asset.questionImageAssetId,
                          ),
                          window.location.origin,
                        )
                      }
                    />
                  }
                >
                  <OpaqueWebworkPreviewFrame
                    class="question-library-webwork-preview"
                    src={applicationApi.client.questionRevisionPreviewDocumentUrl(
                      record().summary.publishedQuestionRevisionTuple,
                    )}
                    title={`Generated example for ${record().summary.metadata.questionTitle}, Revision ${record().summary.publishedQuestionRevisionTuple.revisionNumber}`}
                  />
                </Show>
              </section>
              <Show when={record().responsePreview}>
                {(preview) => (
                  <QuestionResponsePreviewControl
                    preview={preview()}
                    publishedQuestionRevisionTuple={record().summary.publishedQuestionRevisionTuple}
                    questionImageUrl={(asset) =>
                      new URL(
                        applicationApi.client.questionImageUrl(
                          record().summary.publishedQuestionRevisionTuple,
                          asset.questionImageAssetId,
                        ),
                        window.location.origin,
                      )
                    }
                  />
                )}
              </Show>
              <section class="question-detail-description" aria-label="Question Description">
                <h2>Question Description</h2>
                <p>{record().summary.metadata.questionDescription}</p>
              </section>
              <QuestionMetadataEditor
                detail={record()}
                onSaved={() => setMetadataReload((count) => count + 1)}
              />
              <section class="question-detail-support" aria-label="Question details and actions">
                <CopyableQuestionId
                  questionTitle={record().summary.metadata.questionTitle}
                  displayId={record().summary.questionId}
                />
                <dl class="question-detail-metadata">
                  <div>
                    <dt>Authors</dt>
                    <dd>
                      {record().summary.authorship.authors.map((author, index) => (
                        <>
                          {index > 0 ? ", " : ""}
                          <Show when={author.accountId} fallback={author.displayName}>
                            {(accountId) => (
                              <InstructorProfileLink
                                accountId={accountId()}
                                displayName={author.displayName}
                              />
                            )}
                          </Show>
                        </>
                      ))}
                    </dd>
                  </div>
                  <div>
                    <dt>Backend</dt>
                    <dd>{record().summary.backend}</dd>
                  </div>
                  <div>
                    <dt>Discipline</dt>
                    <dd>
                      {record().disciplineName}
                      <Show when={record().disciplineIsRetired}> (retired)</Show>
                    </dd>
                  </div>
                  <div>
                    <dt>Subject</dt>
                    <dd>{record().subjectName}</dd>
                  </div>
                  <Show when={webworkFormatLabel(record().summary.questionFormat)}>
                    {(format) => (
                      <div>
                        <dt>Format</dt>
                        <dd>{format()}</dd>
                      </div>
                    )}
                  </Show>
                  <div>
                    <dt>Revision</dt>
                    <dd>{record().summary.publishedQuestionRevisionTuple.revisionNumber}</dd>
                  </div>
                  <div>
                    <dt>Bloom Classification</dt>
                    <dd>
                      <BloomClassificationText bloom={record().summary.bloom} />
                    </dd>
                  </div>
                </dl>
                <Show
                  when={
                    record().summary.backend === "webwork" ||
                    record().prompt.kind === "generatedExample"
                  }
                >
                  <aside class="question-library-generated-example" aria-label="Generated example">
                    <strong>Generated example</strong>
                    <p>
                      This example uses resolved values for Question Library viewing. Assigned
                      versions may use different values.
                    </p>
                  </aside>
                </Show>
                <Show when={mayMutateLibrary()}>
                  <div class="question-detail-support-actions">
                    <QuestionStarControl questionId={record().summary.questionId} />
                    <QuestionWatchControl questionId={record().summary.questionId} />
                  </div>
                </Show>
              </section>
              <QuestionStatisticsPanel evidence={record().evidence} />
              <QuestionUsePanel usage={record().usage} />
              <QuestionArchiveControl
                client={applicationApi.client}
                questionId={record().summary.questionId}
                renderAvailableAction={() => (
                  <Show when={mayMutateLibrary()}>
                    <QuestionPoolFromQuestionControl detail={record()} />
                  </Show>
                )}
                renderCorrectionAction={(sourceRevisionTuple) => (
                  <QuestionCorrectionDraftControl sourceRevisionTuple={sourceRevisionTuple} />
                )}
              />
              <Show when={mayMutateLibrary()}>
                <QuestionForkControl
                  sourceRevisionTuple={record().summary.publishedQuestionRevisionTuple}
                />
              </Show>
            </article>
          )}
        </Show>
      </Suspense>
    </PageFrame>
  );
}
