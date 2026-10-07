import {
  Show,
  createEffect,
  createSignal,
  onCleanup,
  type Accessor,
  type JSX,
  type Setter,
} from "solid-js";

import type { WasmFacade } from "../../wasm/index";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import {
  createQuestionDraftAutosave,
  type QuestionDraftAutosaveState,
} from "../ple_question_json_authoring/question_draft_autosave";
import type { DraftQuestionPreviewClient, DraftSourceRead } from "./draft_preview_client";
import { DraftPreviewPanel } from "./draft_preview_panel";

export type WebworkDraftSourceEditorProps = {
  readonly draftQuestion: DraftQuestionRouteId;
  readonly initial: DraftSourceRead;
  readonly editNumber: Accessor<string>;
  readonly setEditNumber: Setter<string>;
  readonly client: DraftQuestionPreviewClient;
  readonly responseValidator: Pick<WasmFacade, "validateResponseFormat">;
  readonly onSavedStateChange?: (saved: boolean) => void;
  readonly isDraftSaved?: Accessor<boolean>;
  readonly serializeMutation?: <T>(operation: () => Promise<T>) => Promise<T>;
  readonly registerReload?: (reload: (() => Promise<DraftSourceRead>) | undefined) => void;
  readonly registerFlush?: (flush: (() => Promise<void>) | undefined) => void;
  readonly reloadDraft: () => Promise<void>;
};

function sameSourceBinding(
  left: DraftSourceRead["binding"],
  right: DraftSourceRead["binding"],
): boolean {
  return (
    left.backend === right.backend &&
    left.format === right.format &&
    left.mediaType === right.mediaType &&
    left.webworkPgPath === right.webworkPgPath
  );
}

function safeErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return "The Draft source could not be saved.";
}

/** Edits exact Draft source text while its selected backend owns parsing and grading. */
export function WebworkDraftSourceEditor(props: WebworkDraftSourceEditorProps): JSX.Element {
  const [source, setSource] = createSignal(props.initial.source);
  const [savedSource, setSavedSource] = createSignal(props.initial.source);
  const [saveMessage, setSaveMessage] = createSignal(
    `Source snapshot is acknowledged at Draft Edit ${props.editNumber()}.`,
  );
  const [autosaveState, setAutosaveState] = createSignal<QuestionDraftAutosaveState>({
    status: "saved",
    generation: 0,
    acknowledgedGeneration: 0,
  });
  const [reloading, setReloading] = createSignal(false);
  const [recoveryError, setRecoveryError] = createSignal<string>();
  const binding = props.initial.binding;
  const sourceFormat =
    binding.backend === "ple"
      ? "Native JSON"
      : binding.format === "webworkPgml"
        ? "WebWork PGML"
        : "WebWork PG";

  const autosave = createQuestionDraftAutosave<string>({
    debounceMs: 500,
    onStateChange: (state) => {
      setAutosaveState(state);
      if (state.status === "unsaved") setSaveMessage("Draft source changes are waiting to save...");
      else if (state.status === "saving") setSaveMessage("Saving Draft source...");
      else if (state.status === "saved") {
        setSaveMessage(`Source snapshot is acknowledged at Draft Edit ${props.editNumber()}.`);
      } else {
        setSaveMessage(`Draft source save failed: ${safeErrorMessage(state.error)}`);
      }
    },
    save: async (nextSource) => {
      const saveSource = async (): Promise<string> => {
        const nextEditNumber = await props.client.saveSource(
          props.draftQuestion,
          binding,
          nextSource,
          props.editNumber(),
        );
        props.setEditNumber(nextEditNumber);
        return nextEditNumber;
      };
      await (props.serializeMutation?.(saveSource) ?? saveSource());
      setSavedSource(nextSource);
    },
  });
  onCleanup(() => autosave.dispose());

  const isSaved = (): boolean => autosaveState().status === "saved" && source() === savedSource();
  createEffect(() => props.onSavedStateChange?.(isSaved()));

  async function reloadSavedSource(): Promise<DraftSourceRead> {
    if (reloading()) throw new Error("Draft source reload is already in progress.");
    setReloading(true);
    setRecoveryError(undefined);
    try {
      const newest = await props.client.loadSource(props.draftQuestion);
      if (!sameSourceBinding(binding, newest.binding)) {
        throw new Error("The registered Draft source binding changed. Local text was kept.");
      }
      props.setEditNumber(newest.draftQuestionEditNumber);
      setSource(newest.source);
      setSavedSource(newest.source);
      autosave.reset(newest.source);
      props.onSavedStateChange?.(true);
      return newest;
    } catch (error: unknown) {
      setRecoveryError(
        error instanceof Error && error.message.length > 0
          ? error.message
          : "The saved Draft source could not be reloaded. Local text was kept.",
      );
      throw error;
    } finally {
      setReloading(false);
    }
  }

  props.registerReload?.(reloadSavedSource);
  onCleanup(() => props.registerReload?.(undefined));
  props.registerFlush?.(async () => {
    await autosave.flush();
    props.onSavedStateChange?.(isSaved());
  });
  onCleanup(() => props.registerFlush?.(undefined));

  function retrySave(): void {
    if (reloading()) return;
    setRecoveryError(undefined);
    void autosave.flush().catch(() => undefined);
  }

  return (
    <>
      <section class="editor-panel draft-source-editor" aria-labelledby="draft-source-heading">
        <h2 id="draft-source-heading">{sourceFormat} source</h2>
        <p class="ple-question-json-authoring__help">
          Edit this private source directly. The selected Question Backend parses, renders, and
          grades the saved Draft.
        </p>
        <label class="ple-question-json-authoring__field">
          <span>Backend source</span>
          <textarea
            class="draft-source-editor__textarea"
            value={source()}
            disabled={reloading()}
            spellcheck={false}
            autocapitalize="off"
            autocomplete="off"
            autocorrect="off"
            aria-describedby="draft-source-save-status"
            onInput={(event) => {
              setSource(event.currentTarget.value);
              autosave.edit(event.currentTarget.value);
            }}
          />
        </label>
        <p
          id="draft-source-save-status"
          role={
            autosaveState().status === "error" || recoveryError() !== undefined ? "alert" : "status"
          }
          aria-live="polite"
        >
          {saveMessage()}
        </p>
        <Show when={autosaveState().status === "error"}>
          <div class="action-row" aria-label="Draft source save recovery">
            <button class="primary-action" type="button" disabled={reloading()} onClick={retrySave}>
              Retry save
            </button>
            <button
              class="quiet-action"
              type="button"
              disabled={reloading()}
              onClick={() => void props.reloadDraft()}
            >
              Discard local edits and reload Draft
            </button>
          </div>
        </Show>
        <Show when={reloading()}>
          <p role="status">Loading the newest saved Draft source...</p>
        </Show>
        <Show when={recoveryError()}>{(message) => <p role="alert">{message()}</p>}</Show>
      </section>
      <DraftPreviewPanel
        draftQuestion={props.draftQuestion}
        draftQuestionEditNumber={props.editNumber()}
        binding={binding}
        isSaved={props.isDraftSaved?.() ?? isSaved()}
        responseValidator={props.responseValidator}
      />
    </>
  );
}
