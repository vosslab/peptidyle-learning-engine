// question_draft_editor_page.tsx - route composition for one private Draft Question.

import { A, useLocation, useParams } from "@solidjs/router";
import {
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
  type Setter,
} from "solid-js";

import { createPleQuestionJsonClient } from "../features/ple_question_json_authoring/question_json_client";
import { useApplicationApi } from "../api/application_api";
import {
  PleQuestionGeneralFeedbackConflictError,
  createPleQuestionGeneralFeedbackClient,
  optionalPleManagedSupportText,
  questionTypeUpdateForBackend,
  type PleQuestionGeneralFeedbackClient,
  type PleQuestionGeneralFeedbackRead,
} from "../features/ple_question_json_authoring/question_general_feedback_client";
import type { PleQuestionJsonRecordMetadata } from "../features/ple_question_json_authoring/question_json_defaults";
import { PleQuestionJsonMetadataFields } from "../features/ple_question_json_authoring/question_json_metadata_fields";
import { DraftWebworkPublicationControl } from "../features/ple_question_json_authoring/draft_webwork_publication_control";
import {
  createDraftQuestionMutationQueue,
  isDraftSnapshotSaved,
} from "../features/ple_question_json_authoring/question_publication_review_model";
import {
  createQuestionDraftAutosave,
  type QuestionDraftAutosaveState,
} from "../features/ple_question_json_authoring/question_draft_autosave";
import type { QuestionType } from "../../generated/api/QuestionType";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import { PleQuestionJsonEditorPage } from "../features/ple_question_json_authoring/question_json_editor_page";
import type { PleQuestionJsonClient } from "../features/ple_question_json_authoring/question_json_client";
import type { ContentClassificationClient } from "../api/content_classification";
import { createPleQuestionJsonRepository } from "../features/ple_question_json_authoring/question_json_repository";
import {
  createDraftQuestionPreviewClient,
  type DraftQuestionPreviewClient,
  type DraftSourceRead,
} from "../features/question_draft_preview/draft_preview_client";
import { WebworkDraftSourceEditor } from "../features/question_draft_preview/webwork_draft_source_editor";
import { DraftCorrectionPublishControl } from "../features/question_draft_preview/draft_correction_publish_control";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { decodePositiveQuestionRevisionNumber, decodeQuestionId } from "../api/decoders/shared";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "../features/ple_question_json_authoring/question_json_editor_styles";
import { parseDraftQuestionId, type DraftQuestionRouteId } from "../navigation/public_route";
import { useWasmFacade } from "../wasm/context";
import { PageFrame } from "../components/page_frame";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../ribbon/route_scope_context";

const WEBWORK_QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const satisfies ReadonlyArray<QuestionType>;

function authorSafeMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return fallback;
}

function correctionTargetFromSearch(search: string): PublishedQuestionRevisionTuple | undefined {
  const query = new URLSearchParams(search);
  const questionIds = query.getAll("correctionQuestionId");
  const revisionNumbers = query.getAll("correctionRevisionNumber");
  if (questionIds.length === 0 && revisionNumbers.length === 0) return undefined;
  if (questionIds.length !== 1 || revisionNumbers.length !== 1) return undefined;
  const rawRevision = revisionNumbers[0];
  const rawQuestionId = questionIds[0];
  if (
    rawRevision === undefined ||
    rawQuestionId === undefined ||
    !/^[1-9][0-9]*$/u.test(rawRevision)
  )
    return undefined;
  try {
    return {
      publishedQuestionId: decodeQuestionId(rawQuestionId, "correctionQuestionId"),
      revisionNumber: decodePositiveQuestionRevisionNumber(
        Number(rawRevision),
        "correctionRevisionNumber",
      ),
    };
  } catch {
    return undefined;
  }
}

function correctionContextRequested(search: string): boolean {
  const query = new URLSearchParams(search);
  return query.has("correctionQuestionId") || query.has("correctionRevisionNumber");
}

type DraftMetadataAutosaveSnapshot = {
  readonly metadata: PleQuestionJsonRecordMetadata;
  readonly generalFeedback: string | null;
  readonly hint: string | null;
  readonly workedSolution: string | null;
  readonly questionType: QuestionType | null;
};

/** Keeps backend source editing separate from PLE-managed support fields. */
export function GeneralFeedbackOnlyPage(props: {
  readonly draftQuestion: DraftQuestionRouteId;
  readonly initial: PleQuestionGeneralFeedbackRead;
  readonly source: DraftSourceRead;
  readonly client: PleQuestionGeneralFeedbackClient;
  readonly correctionClient: DraftQuestionPreviewClient;
  readonly correctionParent?: PublishedQuestionRevisionTuple;
  readonly classificationClient: ContentClassificationClient;
  readonly publishDraft: PleQuestionJsonClient["publish"];
  readonly allowNewLineagePublication: boolean;
  readonly renderSourceEditor?: (state: {
    readonly draftQuestionEditNumber: Accessor<string>;
    readonly setDraftQuestionEditNumber: Setter<string>;
    readonly onSavedStateChange: (saved: boolean) => void;
    readonly registerReload: (reload: (() => Promise<DraftSourceRead>) | undefined) => void;
    readonly registerFlush: (flush: (() => Promise<void>) | undefined) => void;
    readonly serializeMutation: <T>(operation: () => Promise<T>) => Promise<T>;
    readonly isDraftSaved: Accessor<boolean>;
    readonly reloadDraft: () => Promise<void>;
  }) => JSX.Element;
}): JSX.Element {
  const [generalFeedback, setGeneralFeedback] = createSignal(props.initial.generalFeedback);
  const [authors, setAuthors] = createSignal(props.initial.authors);
  const [hint, setHint] = createSignal(props.initial.hint);
  const [workedSolution, setWorkedSolution] = createSignal(props.initial.workedSolution);
  const [metadata, setMetadata] = createSignal(props.initial.metadata);
  const [questionType, setQuestionType] = createSignal<QuestionType | null>(
    props.initial.questionType,
  );
  const initialMetadataSnapshot: DraftMetadataAutosaveSnapshot = {
    metadata: props.initial.metadata,
    generalFeedback: props.initial.generalFeedback,
    hint: props.initial.hint,
    workedSolution: props.initial.workedSolution,
    questionType: props.initial.questionType,
  };
  const [savedMetadataSnapshot, setSavedMetadataSnapshot] =
    createSignal<DraftMetadataAutosaveSnapshot>(initialMetadataSnapshot);
  const [metadataAutosaveState, setMetadataAutosaveState] =
    createSignal<QuestionDraftAutosaveState>({
      status: "saved",
      generation: 0,
      acknowledgedGeneration: 0,
    });
  const [draftQuestionEditNumber, setDraftQuestionEditNumber] = createSignal(
    props.initial.draftQuestionEditNumber,
  );
  const [saving, setSaving] = createSignal(false);
  const [sourceSaved, setSourceSaved] = createSignal(false);
  const [snapshotAligned, setSnapshotAligned] = createSignal(true);
  const [status, setStatus] = createSignal<string | null>(null);
  let sourceReload: (() => Promise<DraftSourceRead>) | undefined;
  let sourceFlush: (() => Promise<void>) | undefined;
  // ASVS 2.3.1, 15.4.1: one queue sequences source and metadata CAS writes by Edit Number.
  const serializeMutation = createDraftQuestionMutationQueue();
  const isWebwork = props.source.binding.backend === "webwork";
  function metadataSnapshot(): DraftMetadataAutosaveSnapshot {
    return {
      metadata: metadata(),
      generalFeedback: generalFeedback(),
      hint: hint(),
      workedSolution: workedSolution(),
      questionType: questionType(),
    };
  }

  function metadataSnapshotsMatch(
    left: DraftMetadataAutosaveSnapshot,
    right: DraftMetadataAutosaveSnapshot,
  ): boolean {
    return JSON.stringify(left) === JSON.stringify(right);
  }

  const metadataDirty = (): boolean =>
    !metadataSnapshotsMatch(metadataSnapshot(), savedMetadataSnapshot());
  const autosave = createQuestionDraftAutosave<DraftMetadataAutosaveSnapshot>({
    debounceMs: 500,
    onStateChange: (state) => {
      setMetadataAutosaveState(state);
      if (state.status === "unsaved") {
        setStatus("Draft metadata and support changes are waiting to save...");
      } else if (state.status === "saving") {
        setStatus("Saving Draft metadata and support...");
      } else if (state.status === "error") {
        if (state.error instanceof PleQuestionGeneralFeedbackConflictError)
          setSnapshotAligned(false);
        setStatus(
          `Draft metadata and support save failed: ${authorSafeMessage(state.error, "Retry the save or reload the Draft.")}`,
        );
      } else if (state.status === "saved") {
        setStatus(null);
      }
    },
    save: async (snapshot) => {
      const saved = await serializeMutation(() =>
        props.client.save(
          props.draftQuestion,
          snapshot.metadata,
          {
            generalFeedback: snapshot.generalFeedback,
            hint: snapshot.hint,
            workedSolution: snapshot.workedSolution,
          },
          draftQuestionEditNumber(),
          questionTypeUpdateForBackend(props.source.binding.backend, snapshot.questionType),
        ),
      );
      setSavedMetadataSnapshot(snapshot);
      setDraftQuestionEditNumber(saved.draftQuestionEditNumber);
      setSnapshotAligned(true);
    },
  });
  onCleanup(() => autosave.dispose());

  function scheduleMetadataAutosave(): void {
    if (!isWebwork || !snapshotAligned()) return;
    const snapshot = metadataSnapshot();
    if (
      metadataSnapshotsMatch(snapshot, savedMetadataSnapshot()) &&
      metadataAutosaveState().status !== "saving"
    ) {
      autosave.reset(snapshot);
      return;
    }
    autosave.edit(snapshot);
  }

  function isMetadataPending(): boolean {
    return metadataDirty() || (isWebwork && metadataAutosaveState().status !== "saved");
  }

  const isSaved = (): boolean =>
    isDraftSnapshotSaved(sourceSaved(), isMetadataPending(), saving() || !snapshotAligned());

  function registerReload(reloadSource: (() => Promise<DraftSourceRead>) | undefined): void {
    sourceReload = reloadSource;
  }

  function registerSourceFlush(flushSource: (() => Promise<void>) | undefined): void {
    sourceFlush = flushSource;
  }

  async function saveBeforeLeaving(): Promise<boolean> {
    try {
      await sourceFlush?.();
      if (isWebwork) await autosave.flush();
      else if (metadataDirty()) await saveMetadataManually();
      return isSaved();
    } catch {
      return false;
    }
  }

  async function saveMetadataManually(): Promise<void> {
    if (saving() || isWebwork || !metadataDirty() || !sourceSaved() || !snapshotAligned()) return;
    setSaving(true);
    setStatus("Saving Question metadata and support...");
    const snapshot = metadataSnapshot();
    try {
      const saved = await serializeMutation(() =>
        props.client.save(
          props.draftQuestion,
          snapshot.metadata,
          {
            generalFeedback: snapshot.generalFeedback,
            hint: snapshot.hint,
            workedSolution: snapshot.workedSolution,
          },
          draftQuestionEditNumber(),
          questionTypeUpdateForBackend(props.source.binding.backend, snapshot.questionType),
        ),
      );
      setSavedMetadataSnapshot(snapshot);
      setDraftQuestionEditNumber(saved.draftQuestionEditNumber);
      setSnapshotAligned(true);
      setStatus(null);
    } catch (error: unknown) {
      if (error instanceof PleQuestionGeneralFeedbackConflictError) setSnapshotAligned(false);
      setStatus(authorSafeMessage(error, "Question metadata and support could not be saved."));
    } finally {
      setSaving(false);
    }
  }

  async function reload(): Promise<void> {
    if (saving() || metadataAutosaveState().status === "saving") return;
    const resumeAutosaveAfterReloadFailure =
      isWebwork &&
      snapshotAligned() &&
      !(metadataAutosaveState().error instanceof PleQuestionGeneralFeedbackConflictError);
    autosave.reset(savedMetadataSnapshot());
    setSaving(true);
    setSnapshotAligned(false);
    setStatus("Loading the newest saved Draft...");
    try {
      if (sourceReload === undefined) throw new Error("Draft source reload is unavailable.");
      await serializeMutation(async () => {
        for (let attempt = 0; attempt < 3; attempt += 1) {
          const newestSource = await sourceReload?.();
          if (newestSource === undefined) throw new Error("Draft source reload is unavailable.");
          const newestMetadata = await props.client.load(props.draftQuestion);
          if (newestSource.draftQuestionEditNumber !== newestMetadata.draftQuestionEditNumber)
            continue;
          const newestMetadataSnapshot: DraftMetadataAutosaveSnapshot = {
            metadata: newestMetadata.metadata,
            generalFeedback: newestMetadata.generalFeedback,
            hint: newestMetadata.hint,
            workedSolution: newestMetadata.workedSolution,
            questionType: newestMetadata.questionType,
          };
          setMetadata(newestMetadataSnapshot.metadata);
          setQuestionType(newestMetadataSnapshot.questionType);
          setGeneralFeedback(newestMetadataSnapshot.generalFeedback);
          setAuthors(newestMetadata.authors);
          setHint(newestMetadataSnapshot.hint);
          setWorkedSolution(newestMetadataSnapshot.workedSolution);
          setSavedMetadataSnapshot(newestMetadataSnapshot);
          autosave.reset(newestMetadataSnapshot);
          setDraftQuestionEditNumber(newestMetadata.draftQuestionEditNumber);
          setSnapshotAligned(true);
          setStatus("Loaded the newest saved source, metadata, and support.");
          return;
        }
        throw new Error("Draft source and metadata kept changing. Retry the reload.");
      });
    } catch (error: unknown) {
      setStatus(authorSafeMessage(error, "The saved Draft could not be reloaded."));
      if (resumeAutosaveAfterReloadFailure && metadataDirty()) autosave.edit(metadataSnapshot());
    } finally {
      setSaving(false);
    }
  }

  return (
    <PageFrame
      routeSurface="questionDraftGeneralFeedback"
      eyebrow="Private instructor authoring"
      title="Draft Question editor"
      lede="Edit the exact backend source and the separate Question record metadata. Saved, preview, and publication use one acknowledged Draft snapshot."
    >
      <UnsavedChangesGuard
        autoSaveOnLeave
        dirty={() => !isSaved()}
        save={saveBeforeLeaving}
        copy={{
          heading: "Draft changes could not be saved",
          description: "Your latest Draft changes must save before you leave this editor.",
          saveActionLabel: "Retry save and continue",
          savingActionLabel: "Saving Draft...",
          saveFailureMessage:
            "Your Draft is still open. Resolve the save error, retry, or stay and keep editing.",
        }}
      />
      <style>{PLE_QUESTION_JSON_EDITOR_STYLES}</style>
      <Show when={status()}>{(message) => <p role="status">{message()}</p>}</Show>
      {props.renderSourceEditor?.({
        draftQuestionEditNumber,
        setDraftQuestionEditNumber,
        onSavedStateChange: setSourceSaved,
        registerReload,
        registerFlush: registerSourceFlush,
        serializeMutation,
        isDraftSaved: isSaved,
        reloadDraft: reload,
      })}
      <p
        id="draft-saved-gate"
        data-testid="draft-saved-gate"
        data-state={
          saving() || metadataAutosaveState().status === "saving"
            ? "saving"
            : isSaved()
              ? "saved"
              : "unsaved"
        }
        role="status"
        aria-live="polite"
      >
        {saving() || metadataAutosaveState().status === "saving"
          ? "Saving Draft changes..."
          : isSaved()
            ? `Saved as Draft Edit ${draftQuestionEditNumber()}.`
            : !snapshotAligned()
              ? "Reload the complete Draft before preview or publication."
              : "Draft source, metadata, or support has unsaved changes."}
      </p>
      <section class="editor-panel" aria-labelledby="draft-question-metadata-heading">
        <h2 id="draft-question-metadata-heading">Question record metadata</h2>
        <PleQuestionJsonMetadataFields
          metadata={metadata()}
          disabled={saving()}
          onMetadataChange={(next: PleQuestionJsonRecordMetadata) => {
            setMetadata(next);
            scheduleMetadataAutosave();
          }}
        />
        <Show when={isWebwork}>
          <label class="ple-question-json-authoring__field">
            <span>Question Type</span>
            <select
              data-testid="draft-webwork-question-type"
              value={questionType() ?? ""}
              disabled={saving()}
              onChange={(event) => {
                const selected = WEBWORK_QUESTION_TYPES.find(
                  (value) => value === event.currentTarget.value,
                );
                setQuestionType(selected ?? null);
                scheduleMetadataAutosave();
              }}
            >
              <option value="">Select a WebWork Type before publication</option>
              {WEBWORK_QUESTION_TYPES.map((value) => (
                <option value={value}>{value}</option>
              ))}
            </select>
          </label>
        </Show>
      </section>
      <section class="editor-panel" aria-labelledby="general-feedback-heading">
        <h2 id="general-feedback-heading">PLE-managed support</h2>
        <p class="ple-question-json-authoring__help">
          These optional texts are stored with this Draft and copied onto the next Published
          Question Revision. They are separate from backend source and from transient feedback
          generated during a backend interaction.
        </p>
        <label class="ple-question-json-authoring__field">
          <span>Hint (optional)</span>
          <textarea
            value={hint() ?? ""}
            disabled={saving()}
            onInput={(event) => {
              setHint(optionalPleManagedSupportText(event.currentTarget.value));
              scheduleMetadataAutosave();
            }}
          />
        </label>
        <label class="ple-question-json-authoring__field">
          <span>General Feedback (optional)</span>
          <textarea
            value={generalFeedback() ?? ""}
            disabled={saving()}
            aria-describedby="draft-general-feedback-help"
            onInput={(event) => {
              setGeneralFeedback(optionalPleManagedSupportText(event.currentTarget.value));
              scheduleMetadataAutosave();
            }}
          />
          <span id="draft-general-feedback-help" class="ple-question-json-authoring__help">
            This text is not copied from or written into the Question Backend source.
          </span>
        </label>
        <label class="ple-question-json-authoring__field">
          <span>Worked Solution (optional)</span>
          <textarea
            value={workedSolution() ?? ""}
            disabled={saving()}
            onInput={(event) => {
              setWorkedSolution(optionalPleManagedSupportText(event.currentTarget.value));
              scheduleMetadataAutosave();
            }}
          />
        </label>
        <div class="editor-actions">
          <Show when={!isWebwork}>
            <button
              type="button"
              class="primary-action"
              disabled={saving() || !metadataDirty() || !sourceSaved() || !snapshotAligned()}
              onClick={() => void saveMetadataManually()}
            >
              {saving() ? "Saving metadata and support..." : "Save metadata and support"}
            </button>
          </Show>
          <Show when={metadataAutosaveState().status === "error" && snapshotAligned()}>
            <button
              type="button"
              class="primary-action"
              disabled={saving()}
              onClick={() => void autosave.flush().catch(() => undefined)}
            >
              Retry metadata and support save
            </button>
          </Show>
          <button
            type="button"
            class="quiet-action"
            disabled={saving() || metadataAutosaveState().status === "saving"}
            onClick={() => void reload()}
          >
            Discard local edits and reload Draft
          </button>
        </div>
      </section>
      <Show when={isWebwork && props.allowNewLineagePublication}>
        <DraftWebworkPublicationControl
          draftQuestion={props.draftQuestion}
          questionTitle={() => metadata().questionTitle}
          authors={authors}
          questionType={questionType}
          draftQuestionEditNumber={draftQuestionEditNumber}
          isSaved={isSaved}
          classificationClient={props.classificationClient}
          publish={props.publishDraft}
        />
      </Show>
      <Show when={props.correctionParent}>
        {(parent) => (
          <DraftCorrectionPublishControl
            draftQuestion={props.draftQuestion}
            parent={parent()}
            draftQuestionEditNumber={draftQuestionEditNumber()}
            isSaved={isSaved()}
            client={props.correctionClient}
          />
        )}
      </Show>
      <A class="quiet-link" href="/authoring/drafts">
        Return to My Question Drafts
      </A>
    </PageFrame>
  );
}

/** Loads the private PLE Question JSON editor only after its UUID route grammar accepts the ID. */
export function QuestionDraftEditorPage(): JSX.Element {
  const params = useParams();
  const location = useLocation();
  const wasm = useWasmFacade();
  const client = createPleQuestionJsonClient();
  const classificationClient = useApplicationApi().client;
  const generalFeedbackClient = createPleQuestionGeneralFeedbackClient();
  const draftPreviewClient = createDraftQuestionPreviewClient();
  const repository = createPleQuestionJsonRepository(client);
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  const draftQuestionId = createMemo(() => parseDraftQuestionId(params.draftQuestionId ?? ""));
  const correctionTarget = createMemo(() => correctionTargetFromSearch(location.search));
  const [draftSource, { refetch: refetchDraftSource }] = createResource(
    draftQuestionId,
    async (draftQuestion) => await draftPreviewClient.loadSource(draftQuestion),
  );
  const [initial, { refetch }] = createResource(draftSource, async (read) => {
    const draftQuestion = draftQuestionId();
    if (read?.binding.backend !== "ple" || draftQuestion === null) return undefined;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const [source, generalFeedback] = await Promise.all([
        repository.load(draftQuestion),
        generalFeedbackClient.load(draftQuestion),
      ]);
      if (source.draftQuestionEditNumber === generalFeedback.draftQuestionEditNumber) {
        return { source, generalFeedback };
      }
    }
    throw new Error("Draft source and metadata changed during loading. Reload the Draft.");
  });
  const [textEditorInitial, { refetch: refetchTextEditor }] = createResource(
    draftSource,
    async (read) => {
      const draftQuestion = draftQuestionId();
      if (read === undefined || draftQuestion === null) return undefined;
      for (let attempt = 0; attempt < 3; attempt += 1) {
        const source = await draftPreviewClient.loadSource(draftQuestion);
        const generalFeedback = await generalFeedbackClient.load(draftQuestion);
        if (source.draftQuestionEditNumber === generalFeedback.draftQuestionEditNumber)
          return { source, generalFeedback };
      }
      throw new Error("Draft source and support changed during loading. Reload the Draft.");
    },
  );
  const initialLoadFailed = createMemo(
    () =>
      draftSource.error !== undefined ||
      (draftSource()?.binding.backend === "ple" &&
        initial.error !== undefined &&
        textEditorInitial.error !== undefined) ||
      (textEditorInitial.error !== undefined && draftSource()?.binding.backend === "webwork"),
  );
  const loadedEditor = createMemo(() => initial());
  const metadataOnlyEditor = createMemo(() => {
    const loaded = textEditorInitial();
    const backend = draftSource()?.binding.backend;
    return loaded !== undefined &&
      (backend === "webwork" || (backend === "ple" && initial.error !== undefined))
      ? loaded
      : undefined;
  });

  onMount(() => {
    publication = routeScopePublication();
  });
  createEffect(() => {
    if (metadataOnlyEditor() !== undefined && publication !== undefined) {
      publishRouteScopeLabels(publication, { questionTitle: "Draft Question" });
    } else if (
      publication !== undefined &&
      (initialLoadFailed() || metadataOnlyEditor() !== undefined)
    ) {
      clearRouteScopeLabels(publication);
    }
  });
  onCleanup(() => {
    if (publication !== undefined) clearRouteScopeLabels(publication);
  });

  return (
    <Show
      when={draftQuestionId()}
      fallback={
        <PageFrame
          // Failure card on the content region. PageFrame still owns the stack.
          contentClass="route-error"
          routeSurface="questionDraftEditorInvalid"
          title="Draft Question not found"
        >
          <A class="primary-link" href="/authoring/drafts">
            Return to My Question Drafts
          </A>
        </PageFrame>
      }
    >
      <Show
        when={loadedEditor()}
        fallback={
          <Show
            when={metadataOnlyEditor()}
            fallback={
              <PageFrame routeSurface="questionDraftEditorLoading" title="Draft Question">
                <Show
                  when={initialLoadFailed()}
                  fallback={
                    <p class="calm-status" role="status">
                      Loading private Draft Question...
                    </p>
                  }
                >
                  <section class="inline-error" role="alert">
                    <p>This Draft Question is unavailable.</p>
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => {
                        void refetchDraftSource();
                        void refetch();
                        void refetchTextEditor();
                      }}
                    >
                      Retry
                    </button>
                  </section>
                </Show>
              </PageFrame>
            }
          >
            {(generalFeedback) => (
              <GeneralFeedbackOnlyPage
                draftQuestion={draftQuestionId()!}
                initial={generalFeedback().generalFeedback}
                source={generalFeedback().source}
                client={generalFeedbackClient}
                correctionClient={draftPreviewClient}
                correctionParent={correctionTarget()}
                classificationClient={classificationClient}
                publishDraft={(draftQuestion, request, editNumber) =>
                  client.publish(draftQuestion, request, editNumber)
                }
                allowNewLineagePublication={!correctionContextRequested(location.search)}
                renderSourceEditor={({
                  draftQuestionEditNumber,
                  setDraftQuestionEditNumber,
                  onSavedStateChange,
                  registerReload,
                  registerFlush,
                  serializeMutation,
                  isDraftSaved,
                  reloadDraft,
                }) => (
                  <WebworkDraftSourceEditor
                    draftQuestion={draftQuestionId()!}
                    initial={generalFeedback().source}
                    editNumber={draftQuestionEditNumber}
                    setEditNumber={setDraftQuestionEditNumber}
                    client={draftPreviewClient}
                    responseValidator={wasm}
                    onSavedStateChange={onSavedStateChange}
                    isDraftSaved={isDraftSaved}
                    serializeMutation={serializeMutation}
                    registerReload={registerReload}
                    registerFlush={registerFlush}
                    reloadDraft={reloadDraft}
                  />
                )}
              />
            )}
          </Show>
        }
      >
        {(loaded) => (
          <PleQuestionJsonEditorPage
            draftQuestion={draftQuestionId()!}
            initial={loaded().source}
            initialGeneralFeedback={loaded().generalFeedback}
            generalFeedbackClient={generalFeedbackClient}
            repository={repository}
            questionImageClient={client}
            classificationClient={classificationClient}
            responseValidator={wasm}
            correctionParent={correctionTarget()}
          />
        )}
      </Show>
    </Show>
  );
}
