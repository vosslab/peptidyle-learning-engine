// question_detail_page.tsx - safe current Question Details and Question Revision lineage View.

import {
  A,
  createAsync,
  useLocation,
  useNavigate,
  useParams,
  useSearchParams,
} from "@solidjs/router";
import {
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  Show,
  Suspense,
  type JSX,
} from "solid-js";

import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionNumber } from "../../generated/api/QuestionRevisionNumber";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import type {
  LoadedQuestionLineage,
  QuestionAvailabilityClient,
} from "../api/question_availability";
import { ApiRequestError } from "../api/http_client/error";
import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { CopyableQuestionId } from "../components/copyable_question_id";
import {
  BloomClassificationEditor,
  BloomClassificationText,
} from "../components/bloom_classification";
import { OpaqueWebworkPreviewFrame } from "../components/opaque_webwork_preview_frame";
import { QuestionPoolCreateDialog } from "../components/question_pool_create_dialog";
import { QuestionWatchControl } from "../components/question_watch_control";
import { LibraryDiscussionPanel } from "../components/library_discussion_panel";
import { QuestionStarControl } from "../components/question_star_control";
import { QuestionPromptRenderer } from "../components/question_renderer";
import { QuestionResponsePreviewControl } from "../components/question_response_preview";
import { parseQuestionRouteReference } from "../navigation/public_route";
import {
  parseQuestionLibraryReturnToken,
  questionLibraryReturnPath,
  refreshQuestionLibraryReturnState,
  QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER,
} from "./library_page_model";
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

const QUESTION_REVISION_QUERY_PARAMETER = "revision";

function QuestionForkControl(props: { readonly source: QuestionRevisionReference }): JSX.Element {
  const applicationApi = useApplicationApi();
  const navigate = useNavigate();
  const [forking, setForking] = createSignal(false);
  const [error, setError] = createSignal("");
  let action: { readonly source: QuestionRevisionReference; readonly key: string } | undefined;
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  async function forkPublishedQuestion(): Promise<void> {
    const source = props.source;
    if (forking()) return;
    setForking(true);
    setError("");
    try {
      // ASVS 2.3.1: an uncertain retry keeps this exact source Revision and opaque request key.
      if (
        action === undefined ||
        action.source.questionId !== source.questionId ||
        action.source.revisionNumber !== source.revisionNumber
      ) {
        action = { source, key: crypto.randomUUID() };
      }
      const fork = await applicationApi.client.forkPublishedQuestion(source, action.key);
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
        Fork Revision {props.source.revisionNumber} into your own private Draft Question. It must
        pass publication validation before it can join the Question Library.
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
        startingQuestion={{
          questionRevision: props.detail.summary.questionRevision,
          questionTitle: props.detail.summary.metadata.questionTitle,
          disciplineName: props.detail.disciplineName,
          subjectName: props.detail.subjectName,
        }}
        onTaskPhaseChange={() => undefined}
        onClose={closeCreation}
      />
    </Show>
  );
}

function questionRevisionFromSearch(search: string): QuestionRevisionNumber | undefined {
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
  readonly client: Pick<QuestionAvailabilityClient, "getQuestionLineage" | "archiveQuestion">;
  readonly questionId: QuestionId;
  /** Renders a related action only while the loaded Published Question remains available. */
  readonly renderAvailableAction?: () => JSX.Element;
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
    const current = readyLineage();
    return current?.viewerMayArchive === true ? current : undefined;
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
        currentLineage.value.availabilityEtag,
      );
      mutateLineage({
        kind: "ready",
        value: {
          summary: {
            ...currentLineage.value.summary,
            availability: { availability: transition.availability },
          },
          viewerMayArchive: currentLineage.value.viewerMayArchive,
          availabilityEtag: transition.etag,
        },
      });
      setArchiveOpen(false);
      setArchiveConfirmation("");
      setArchiveNotice({
        kind: "status",
        text: "Published Question archived. It is no longer available for new selection. Existing exact Revisions and Student Work are unchanged.",
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
            text: "This Published Question is already archived and unavailable for new selection.",
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
              <p class="question-archive-status" role="status">
                This Published Question is archived and unavailable for new selection.
              </p>
            }
          >
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
                  Archiving removes this Published Question from shared Question Library discovery
                  and new selection. Existing exact Revisions and Student Work remain unchanged.
                </p>
                <label for="question-archive-confirmation">
                  Type <strong>{currentLineage().summary.metadata.questionTitle}</strong> to confirm
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
        )}
      </Show>
      <Show when={archiveNotice()}>
        {(notice) => (
          <p class="question-archive-notice" role={notice().kind} aria-live="polite">
            {notice().text}
          </p>
        )}
      </Show>
      <Show when={availableLineage()}>{(_currentLineage) => props.renderAvailableAction?.()}</Show>
    </>
  );
}

export function QuestionDetailPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const params = useParams();
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const libraryReturnToken = (): string | null =>
    parseQuestionLibraryReturnToken(searchParams[QUESTION_LIBRARY_RETURN_TOKEN_PARAMETER]);
  const libraryReturnHref = (): string => {
    const token = libraryReturnToken();
    return token === null ? "/library" : questionLibraryReturnPath(token);
  };
  const mayMutateLibrary = (): boolean => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.productRole === "instructor";
  };
  const [correctedBloom, setCorrectedBloom] = createSignal<BloomClassificationView>();
  let correctionTarget = "";
  const detail = createAsync((): Promise<QuestionDetails> => {
    const questionId = params["questionId"];
    if (questionId === undefined || parseQuestionRouteReference(questionId) === null) {
      throw new Error("The Question ID address is incomplete.");
    }
    const revisionNumber = questionRevisionFromSearch(location.search);
    if (revisionNumber !== undefined)
      return applicationApi.client.getQuestionRevision({
        questionId: questionId,
        revisionNumber,
      });
    return applicationApi.client
      .resolveQuestion(questionId)
      .then((summary) => applicationApi.queries.questionDetails(summary.questionId));
  });
  createEffect(() => {
    const reference = detail()?.summary.questionRevision;
    if (reference === undefined) return;
    const key = `${reference.questionId}:${reference.revisionNumber}`;
    if (key === correctionTarget) return;
    correctionTarget = key;
    setCorrectedBloom(undefined);
  });
  return (
    <section class="page" data-route-surface="questionDetail">
      <A class="quiet-link" href={libraryReturnHref()}>
        Return to question library
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
              <h1>{record().summary.metadata.questionTitle}</h1>
              <section class="question-detail-description" aria-label="Question Description">
                <h2>Question Description</h2>
                <p>{record().summary.metadata.questionDescription}</p>
              </section>
              <section aria-label="Question prompt">
                <Show
                  when={record().summary.backend === "webwork"}
                  fallback={
                    <QuestionPromptRenderer
                      blocks={record().prompt.blocks}
                      questionRevision={record().summary.questionRevision}
                      assetUrl={(asset) =>
                        new URL(
                          applicationApi.client.assetUrl(
                            record().summary.questionRevision,
                            asset.questionAsset,
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
                      record().summary.questionRevision,
                    )}
                    title={`Generated example for ${record().summary.metadata.questionTitle}, Revision ${record().summary.questionRevision.revisionNumber}`}
                  />
                </Show>
              </section>
              <Show when={record().responsePreview}>
                {(preview) => (
                  <QuestionResponsePreviewControl
                    preview={preview()}
                    questionRevision={record().summary.questionRevision}
                    assetUrl={(asset) =>
                      new URL(
                        applicationApi.client.assetUrl(
                          record().summary.questionRevision,
                          asset.questionAsset,
                        ),
                        window.location.origin,
                      )
                    }
                  />
                )}
              </Show>
              <section class="question-detail-support" aria-label="Question details and actions">
                <CopyableQuestionId
                  questionTitle={record().summary.metadata.questionTitle}
                  displayId={record().summary.questionId}
                />
                <dl class="question-detail-metadata">
                  <div>
                    <dt>Authors</dt>
                    <dd>
                      {record()
                        .summary.authorship.authors.map((author) => author.displayName)
                        .join(", ")}
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
                    <dd>{record().summary.questionRevision.revisionNumber}</dd>
                  </div>
                  <Show when={correctedBloom() ?? record().summary.bloom}>
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
                <Show when={correctedBloom() ?? record().summary.bloom}>
                  {(bloom) => (
                    <Show when={mayMutateLibrary()}>
                      <BloomClassificationEditor
                        targetName="Question"
                        contentMarkerKind="Revision"
                        contentMarkerNumber={record().summary.questionRevision.revisionNumber}
                        bloom={bloom()}
                        save={(request) =>
                          applicationApi.client
                            .correctQuestionBloom(record().summary.questionRevision, request)
                            .then((receipt) => receipt.bloom)
                        }
                        loadCurrent={() =>
                          applicationApi.client
                            .getQuestionRevision(record().summary.questionRevision)
                            .then((loaded) => {
                              if (loaded.summary.bloom === null) {
                                throw new Error("Bloom Classification is not assigned.");
                              }
                              return loaded.summary.bloom;
                            })
                        }
                        onCurrent={setCorrectedBloom}
                        onConflictCurrent={() =>
                          refreshQuestionLibraryReturnState(libraryReturnToken())
                        }
                        onAccepted={(_bloom, changed) => {
                          if (changed) refreshQuestionLibraryReturnState(libraryReturnToken());
                        }}
                      />
                    </Show>
                  )}
                </Show>
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
              <LibraryDiscussionPanel kind="question" publicId={record().summary.questionId} />
              <Show when={mayMutateLibrary()}>
                <QuestionArchiveControl
                  client={applicationApi.client}
                  questionId={record().summary.questionId}
                  renderAvailableAction={() => (
                    <>
                      <QuestionPoolFromQuestionControl detail={record()} />
                      <QuestionForkControl source={record().summary.questionRevision} />
                    </>
                  )}
                />
              </Show>
            </article>
          )}
        </Show>
      </Suspense>
    </section>
  );
}
