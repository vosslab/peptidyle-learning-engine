// question_draft_editor_page.tsx - route composition for one private Draft Question.

import { A, useParams } from "@solidjs/router";
import { Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import { createPleQuestionJsonClient } from "../features/ple_question_json_authoring/question_json_client";
import { useApplicationApi } from "../api/application_api";
import {
  PleQuestionGeneralFeedbackConflictError,
  createPleQuestionGeneralFeedbackClient,
  type PleQuestionGeneralFeedbackClient,
  type PleQuestionGeneralFeedbackRead,
} from "../features/ple_question_json_authoring/question_general_feedback_client";
import { PleQuestionJsonEditorPage } from "../features/ple_question_json_authoring/question_json_editor_page";
import { createPleQuestionJsonRepository } from "../features/ple_question_json_authoring/question_json_repository";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "../features/ple_question_json_authoring/question_json_editor_styles";
import { parseDraftQuestionReference } from "../navigation/public_route";
import { useWasmFacade } from "../wasm/context";
import type { DraftQuestionReference } from "../../generated/api/DraftQuestionReference";

function authorSafeMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return fallback;
}

/**
 * Keeps PLE-managed general feedback available for a private Draft whose source belongs to a
 * backend that this page deliberately does not parse or edit.
 */
function GeneralFeedbackOnlyPage(props: {
  readonly draftQuestion: DraftQuestionReference;
  readonly initial: PleQuestionGeneralFeedbackRead;
  readonly client: PleQuestionGeneralFeedbackClient;
}): JSX.Element {
  const [generalFeedback, setGeneralFeedback] = createSignal(props.initial.generalFeedback);
  const [savedGeneralFeedback, setSavedGeneralFeedback] = createSignal(
    props.initial.generalFeedback,
  );
  const [revision, setRevision] = createSignal(props.initial.revision);
  const [saving, setSaving] = createSignal(false);
  const [status, setStatus] = createSignal<string | null>(null);
  const dirty = (): boolean => generalFeedback() !== savedGeneralFeedback();

  async function reload(): Promise<void> {
    setSaving(true);
    setStatus("Loading the newest general feedback...");
    try {
      const newest = await props.client.load(props.draftQuestion);
      setGeneralFeedback(newest.generalFeedback);
      setSavedGeneralFeedback(newest.generalFeedback);
      setRevision(newest.revision);
      setStatus("Loaded the newest saved general feedback.");
    } catch (error: unknown) {
      setStatus(authorSafeMessage(error, "General feedback could not be loaded."));
    } finally {
      setSaving(false);
    }
  }

  async function save(): Promise<void> {
    if (saving() || !dirty()) return;
    setSaving(true);
    setStatus("Saving general feedback...");
    try {
      const saved = await props.client.save(props.draftQuestion, generalFeedback(), revision());
      setSavedGeneralFeedback(generalFeedback());
      setRevision(saved.revision);
      setStatus("General feedback saved. It remains separate from backend interaction feedback.");
    } catch (error: unknown) {
      if (error instanceof PleQuestionGeneralFeedbackConflictError) {
        setStatus("A newer draft exists. Reload it before saving general feedback.");
      } else {
        setStatus(authorSafeMessage(error, "General feedback could not be saved."));
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <main
      class="page ple-question-json-authoring"
      data-route-surface="questionDraftGeneralFeedback"
    >
      <style>{PLE_QUESTION_JSON_EDITOR_STYLES}</style>
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1>General Feedback</h1>
        <p>
          This Draft's source is managed by its Question Backend and is not editable on this page.
          You can still maintain PLE-managed general feedback below.
        </p>
      </header>
      <Show when={status()}>{(message) => <p role="status">{message()}</p>}</Show>
      <section class="editor-panel" aria-labelledby="general-feedback-heading">
        <h2 id="general-feedback-heading">General Feedback</h2>
        <p class="ple-question-json-authoring__help">
          This plain authored text is separate from backend source and transient feedback generated
          during a backend interaction.
        </p>
        <label class="ple-question-json-authoring__field">
          <span>General Feedback (optional)</span>
          <textarea
            value={generalFeedback() ?? ""}
            disabled={saving()}
            aria-describedby="draft-general-feedback-help"
            onInput={(event) =>
              setGeneralFeedback(
                event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
              )
            }
          />
          <span id="draft-general-feedback-help" class="ple-question-json-authoring__help">
            This text is not copied from or written into the Question Backend source.
          </span>
        </label>
        <div class="editor-actions">
          <button
            type="button"
            class="primary-action"
            disabled={saving() || !dirty()}
            onClick={() => void save()}
          >
            {saving() ? "Saving general feedback..." : "Save general feedback"}
          </button>
          <button
            type="button"
            class="quiet-action"
            disabled={saving()}
            onClick={() => void reload()}
          >
            Reload saved feedback
          </button>
        </div>
      </section>
      <A class="quiet-link" href="/authoring/drafts">
        Return to My Question Drafts
      </A>
    </main>
  );
}

/** Loads the private PLE Question JSON editor only after the route grammar accepts `D-...`. */
export function QuestionDraftEditorPage(): JSX.Element {
  const params = useParams();
  const wasm = useWasmFacade();
  const client = createPleQuestionJsonClient();
  const classificationClient = useApplicationApi().client;
  const generalFeedbackClient = createPleQuestionGeneralFeedbackClient();
  const repository = createPleQuestionJsonRepository(client);
  const reference = createMemo(() => parseDraftQuestionReference(params.draftQuestionRef ?? ""));
  const [initial, { refetch }] = createResource(
    reference,
    async (draftQuestion) => await repository.load(draftQuestion),
  );
  const [initialGeneralFeedback, { refetch: refetchGeneralFeedback }] = createResource(
    reference,
    async (draftQuestion) => await generalFeedbackClient.load(draftQuestion),
  );
  const initialLoadFailed = createMemo(
    () => initial.error !== undefined || initialGeneralFeedback.error !== undefined,
  );
  const loadedEditor = createMemo(() => {
    const source = initial();
    const generalFeedback = initialGeneralFeedback();
    return source === undefined || generalFeedback === undefined
      ? undefined
      : { source, generalFeedback };
  });
  const metadataOnlyEditor = createMemo(() => {
    const generalFeedback = initialGeneralFeedback();
    return initial.error !== undefined && generalFeedback !== undefined
      ? generalFeedback
      : undefined;
  });

  return (
    <Show
      when={reference()}
      fallback={
        <main class="page route-error" data-route-surface="questionDraftEditorInvalid" role="alert">
          <h1>Draft Question not found</h1>
          <A class="primary-link" href="/authoring/drafts">
            Return to My Question Drafts
          </A>
        </main>
      }
    >
      <Show
        when={loadedEditor()}
        fallback={
          <Show
            when={metadataOnlyEditor()}
            fallback={
              <main class="page" data-route-surface="questionDraftEditorLoading">
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
                        void refetch();
                        void refetchGeneralFeedback();
                      }}
                    >
                      Retry
                    </button>
                  </section>
                </Show>
              </main>
            }
          >
            {(generalFeedback) => (
              <GeneralFeedbackOnlyPage
                draftQuestion={reference()!}
                initial={generalFeedback()}
                client={generalFeedbackClient}
              />
            )}
          </Show>
        }
      >
        {(loaded) => (
          <PleQuestionJsonEditorPage
            draftQuestion={reference()!}
            initial={loaded().source}
            initialGeneralFeedback={loaded().generalFeedback}
            generalFeedbackClient={generalFeedbackClient}
            repository={repository}
            classificationClient={classificationClient}
            responseValidator={wasm}
          />
        )}
      </Show>
    </Show>
  );
}
