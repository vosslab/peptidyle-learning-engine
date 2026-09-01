// editor_page.tsx - key-free workspace editor with explicit runtime boundaries.

import { ErrorBoundary, For, Show, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { Capability } from "../../generated/api/Capability";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { QuestionSeed } from "../../generated/api/QuestionSeed";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { QuestionAuthorship } from "../../generated/api/QuestionAuthorship";
import { QuestionRenderer } from "../components/question_renderer";
import { QuestionResponseControl } from "../components/question_response_controls/question_response_control";
import { ContentBlockList } from "../components/feedback_panel";
import { WorkspaceConflictError } from "../api/http_client";
import { parseReviewedQuestionAuthorship } from "../api/question_authorship";
import type { WasmFacade } from "../wasm/index";
import { useWasmFacade } from "../wasm/context";
import { createEditorPreviewFacade } from "./editor_preview_facade";
import {
  InstructorPreviewConflictError,
  type InstructorPreviewPresentation,
} from "./editor_instructor_preview";
import {
  capabilityLabel,
  type DraftCapabilityViolation,
  type EditorDraft,
  type EditorDraftDisplayState,
  type EditorPreview,
  type EditorRepository,
  type PreviewFacade,
  type QuestionPublicationReview,
  type DraftQuestionSummary,
} from "./editor_page_model";
import { EDITOR_PAGE_STYLES } from "./editor_page_styles";

type PageState =
  | { readonly kind: "loading" }
  | {
      readonly kind: "ready";
      readonly drafts: ReadonlyArray<DraftQuestionSummary>;
      readonly nextCursor: string | null;
      readonly draft: EditorDraft;
    }
  | { readonly kind: "empty" }
  | { readonly kind: "error"; readonly message: string };

type PreviewState =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly preview: EditorPreview }
  | { readonly kind: "error"; readonly message: string };

type InstructorPreviewState =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "available"; readonly presentation: InstructorPreviewPresentation }
  | { readonly kind: "unavailable"; readonly message: string }
  | { readonly kind: "error"; readonly message: string };

type PublishState =
  | { readonly kind: "idle" }
  | { readonly kind: "loadingDiff" }
  | {
      readonly kind: "confirm";
      readonly review: QuestionPublicationReview;
      readonly message: string | null;
    }
  | { readonly kind: "publishing" }
  | { readonly kind: "published"; readonly questionId: string }
  | { readonly kind: "error"; readonly message: string };

const EDITOR_CAPABILITIES: ReadonlyArray<Capability> = [
  "algorithmicGeneration",
  "hints",
  "questionAttemptTimeLimit",
  "offlinePreview",
];

function initialSeed(): QuestionSeed {
  return 101;
}

function policySummary(draft: EditorDraft): string {
  const attempts =
    draft.questionAttemptLimit.maxAttempts === null
      ? "Unlimited response attempts"
      : `${draft.questionAttemptLimit.maxAttempts} response attempt(s)`;
  const timing =
    draft.questionAttemptTimeLimit.kind === "unlimited"
      ? "unlimited"
      : `${draft.questionAttemptTimeLimit.seconds} seconds with ${draft.questionAttemptTimeLimit.graceSeconds} seconds grace`;
  return `${attempts}; ${timing}.`;
}

function safeAssetUrl(asset: string): URL {
  return new URL(`/api/assets/${asset}`, globalThis.location.origin);
}

function firstTextPrompt(draft: EditorDraft): string {
  const block = draft.prompt.find((promptBlock) => promptBlock.kind === "text");
  return block?.kind === "text" ? block.markdown : "";
}

/** This compact field edits only the first prose block; diagrams, math, code, and tables persist. */
export function replaceFirstTextPrompt(draft: EditorDraft, markdown: string): EditorDraft {
  const index = draft.prompt.findIndex((promptBlock) => promptBlock.kind === "text");
  const prompt: ReadonlyArray<QuestionContentBlock> =
    index === -1
      ? [{ kind: "text", markdown }, ...draft.prompt]
      : draft.prompt.map((block, blockIndex) =>
          blockIndex === index ? { kind: "text", markdown } : block,
        );
  return { ...draft, prompt };
}

export interface EditorPageProps {
  readonly repository: EditorRepository;
  readonly previewFacade: PreviewFacade;
  readonly responseValidator: WasmFacade;
  readonly initialWorkspace?: WorkspaceId;
  /** Live routing may open a selected draft without changing the editor's runtime boundaries. */
  readonly onOpenDraft?: (draft: DraftQuestionSummary) => void;
  /** The live workspace list can start a complete PLE Question JSON draft. */
  readonly onCreatePleQuestionJson?: () => Promise<void>;
  /** Reports the strong revision and local-change state represented by this editor. */
  readonly onDraftDisplayStateChange?: (state: EditorDraftDisplayState | null) => void;
  /** Prevents edits while QTI conversion is replacing and refetching this draft. */
  readonly replacementPending?: boolean;
}

/** Production composition: editor preview always crosses the key-free WASM facade. */
export function WasmEditorPage(
  props: Omit<EditorPageProps, "previewFacade" | "responseValidator">,
): JSX.Element {
  const wasm = useWasmFacade();
  return (
    <EditorPage
      {...props}
      previewFacade={createEditorPreviewFacade(wasm)}
      responseValidator={wasm}
    />
  );
}

/**
 * The instructor workflow keeps repository, preview, and response-validation boundaries explicit
 * at the production composition seam.
 */
export function EditorPage(props: EditorPageProps): JSX.Element {
  const [page, setPage] = createSignal<PageState>({ kind: "loading" });
  const [preview, setPreview] = createSignal<PreviewState>({ kind: "idle" });
  const [instructorPreview, setInstructorPreview] = createSignal<InstructorPreviewState>({
    kind: "idle",
  });
  const [publish, setPublish] = createSignal<PublishState>({ kind: "idle" });
  const [seedInput, setSeedInput] = createSignal(String(initialSeed()));
  const [requiredCapabilities, setRequiredCapabilities] = createSignal<ReadonlyArray<Capability>>(
    [],
  );
  const [violations, setViolations] = createSignal<ReadonlyArray<DraftCapabilityViolation>>([]);
  const [publicationValidationMessage, setPublicationValidationMessage] = createSignal<
    string | null
  >(null);
  const [saveMessage, setSaveMessage] = createSignal<string | null>(null);
  const [publicationAuthorshipText, setPublicationAuthorshipText] = createSignal("");
  const [staleConflict, setStaleConflict] = createSignal(false);
  const [creatingPleQuestionJson, setCreatingPleQuestionJson] = createSignal(false);
  const [creationMessage, setCreationMessage] = createSignal<string | null>(null);
  const [draftDirty, setDraftDirty] = createSignal(false);
  let publicationAuthorshipInput: HTMLTextAreaElement | undefined;

  const ready = (): Extract<PageState, { readonly kind: "ready" }> | undefined => {
    const value = page();
    return value.kind === "ready" ? value : undefined;
  };

  const pageError = (): Extract<PageState, { readonly kind: "error" }> | undefined => {
    const value = page();
    return value.kind === "error" ? value : undefined;
  };

  const previewError = (): Extract<PreviewState, { readonly kind: "error" }> | undefined => {
    const value = preview();
    return value.kind === "error" ? value : undefined;
  };

  const previewReady = (): Extract<PreviewState, { readonly kind: "ready" }> | undefined => {
    const value = preview();
    return value.kind === "ready" ? value : undefined;
  };

  const instructorPreviewAvailable = ():
    Extract<InstructorPreviewState, { readonly kind: "available" }> | undefined => {
    const value = instructorPreview();
    return value.kind === "available" ? value : undefined;
  };

  const instructorPreviewUnavailable = ():
    Extract<InstructorPreviewState, { readonly kind: "unavailable" }> | undefined => {
    const value = instructorPreview();
    return value.kind === "unavailable" ? value : undefined;
  };

  const instructorPreviewError = ():
    Extract<InstructorPreviewState, { readonly kind: "error" }> | undefined => {
    const value = instructorPreview();
    return value.kind === "error" ? value : undefined;
  };

  const publishConfirm = (): Extract<PublishState, { readonly kind: "confirm" }> | undefined => {
    const value = publish();
    return value.kind === "confirm" ? value : undefined;
  };

  const published = (): Extract<PublishState, { readonly kind: "published" }> | undefined => {
    const value = publish();
    return value.kind === "published" ? value : undefined;
  };

  const publishError = (): Extract<PublishState, { readonly kind: "error" }> | undefined => {
    const value = publish();
    return value.kind === "error" ? value : undefined;
  };

  function seed(): QuestionSeed | null {
    const value = Number(seedInput());
    return Number.isInteger(value) && value >= 0 && value <= 4_294_967_295 ? value : null;
  }

  function replaceDraft(next: EditorDraft, dirty = true): void {
    const current = ready();
    if (current === undefined) return;
    setPage({ ...current, draft: next });
    setDraftDirty(dirty);
    setPreview({ kind: "idle" });
    setInstructorPreview({ kind: "idle" });
    setPublish({ kind: "idle" });
    setSaveMessage(null);
    setPublicationValidationMessage(null);
  }

  async function load(workspace = props.initialWorkspace): Promise<void> {
    setPage({ kind: "loading" });
    try {
      const page = await props.repository.listDrafts();
      const selected = workspace ?? page.items[0]?.workspace;
      if (selected === undefined) {
        setPage({ kind: "empty" });
        return;
      }
      const draft = await props.repository.getDraft(selected);
      setPage({ kind: "ready", drafts: page.items, nextCursor: page.nextCursor, draft });
      setDraftDirty(false);
    } catch (error: unknown) {
      setPage({
        kind: "error",
        message: error instanceof Error ? error.message : "My Question Drafts could not load.",
      });
    }
  }

  async function loadMoreDrafts(): Promise<void> {
    const current = ready();
    if (current?.nextCursor === null || current === undefined) return;
    try {
      const next = await props.repository.listDrafts(current.nextCursor);
      setPage({
        ...current,
        drafts: [...current.drafts, ...next.items],
        nextCursor: next.nextCursor,
      });
    } catch (error: unknown) {
      setSaveMessage(error instanceof Error ? error.message : "More drafts could not load.");
    }
  }

  async function chooseDraft(summary: DraftQuestionSummary): Promise<void> {
    const workspace = summary.workspace;
    if (props.onOpenDraft !== undefined) {
      props.onOpenDraft(summary);
      return;
    }
    const current = ready();
    if (current === undefined || current.draft.workspace === workspace) return;
    try {
      const draft = await props.repository.getDraft(workspace);
      setPage({ ...current, draft });
      setDraftDirty(false);
      setPreview({ kind: "idle" });
      setInstructorPreview({ kind: "idle" });
      setPublish({ kind: "idle" });
      setViolations([]);
    } catch (error: unknown) {
      setPage({
        kind: "error",
        message: error instanceof Error ? error.message : "That draft could not load.",
      });
    }
  }

  async function createPleQuestionJson(): Promise<void> {
    if (props.onCreatePleQuestionJson === undefined || creatingPleQuestionJson()) return;
    setCreatingPleQuestionJson(true);
    setCreationMessage("Creating a private PLE Question JSON draft...");
    try {
      await props.onCreatePleQuestionJson();
    } catch (error: unknown) {
      setCreationMessage(
        error instanceof Error
          ? error.message
          : "The PLE Question JSON draft could not be created.",
      );
    } finally {
      setCreatingPleQuestionJson(false);
    }
  }

  async function saveDraft(): Promise<void> {
    const current = ready();
    if (current === undefined) return;
    setDraftDirty(true);
    setSaveMessage("Saving Draft Question...");
    try {
      const saved = await props.repository.saveDraft(current.draft);
      replaceDraft(saved, false);
      setSaveMessage("Draft saved. It remains a private, unversioned Draft Question.");
      setStaleConflict(false);
    } catch (error: unknown) {
      setStaleConflict(error instanceof WorkspaceConflictError);
      setSaveMessage(error instanceof Error ? error.message : "The draft could not be saved.");
    }
  }

  async function reloadAfterConflict(): Promise<void> {
    const current = ready();
    if (current === undefined || props.repository.reloadDraft === undefined) return;
    setDraftDirty(true);
    try {
      replaceDraft(await props.repository.reloadDraft(current.draft.workspace), false);
      setStaleConflict(false);
      setSaveMessage("Reloaded the newest saved draft. Review it before saving again.");
    } catch (error: unknown) {
      setSaveMessage(error instanceof Error ? error.message : "The newest draft could not reload.");
    }
  }

  async function deleteDraft(): Promise<void> {
    const current = ready();
    if (current === undefined || props.repository.deleteDraft === undefined) return;
    setDraftDirty(true);
    try {
      await props.repository.deleteDraft(current.draft.workspace);
      setSaveMessage("Draft deleted. You can choose another Draft Question in My Question Drafts.");
      await load();
    } catch (error: unknown) {
      setStaleConflict(error instanceof WorkspaceConflictError);
      setSaveMessage(error instanceof Error ? error.message : "The draft could not be deleted.");
    }
  }

  async function renderPreview(): Promise<void> {
    const current = ready();
    const selectedSeed = seed();
    if (current === undefined || selectedSeed === null) {
      setPreview({ kind: "error", message: "Enter a whole-number seed before previewing." });
      return;
    }
    setPreview({ kind: "loading" });
    try {
      const result = await props.previewFacade.preview(current.draft, selectedSeed);
      setPreview({ kind: "ready", preview: result });
    } catch (error: unknown) {
      setPreview({
        kind: "error",
        message: error instanceof Error ? error.message : "The offline preview could not render.",
      });
    }
  }

  async function checkCapabilities(next: ReadonlyArray<Capability>): Promise<void> {
    const current = ready();
    setRequiredCapabilities(next);
    if (current === undefined) return;
    try {
      setViolations(await props.repository.validateCapabilities(current.draft, next));
      setPublicationValidationMessage(null);
    } catch (error: unknown) {
      setPublicationValidationMessage(
        error instanceof Error
          ? error.message
          : "Question Publication Validation could not be checked. Existing capability guidance is still shown.",
      );
    }
  }

  async function requestPublishReview(): Promise<void> {
    const current = ready();
    if (current === undefined) return;
    setPublish({ kind: "loadingDiff" });
    try {
      // A review is meaningful only for a revision that reached the server. Saving here also
      // makes an edited draft explicit rather than silently comparing an older record.
      setDraftDirty(true);
      const saved = await props.repository.saveDraft(current.draft);
      setPage({ ...current, draft: saved });
      setDraftDirty(false);
      setSaveMessage("Draft saved for publication review. It remains private until confirmed.");
      setStaleConflict(false);
      const nextViolations = await props.repository.validateCapabilities(
        saved,
        requiredCapabilities(),
      );
      setViolations(nextViolations);
      setPublicationValidationMessage(null);
      if (nextViolations.length > 0) {
        setPublish({
          kind: "error",
          message: "Publication needs the listed capability changes. Your draft is still open.",
        });
        return;
      }
      setPublish({
        kind: "confirm",
        review: await props.repository.getQuestionPublicationReview(saved),
        message: null,
      });
    } catch (error: unknown) {
      const isConflict = error instanceof WorkspaceConflictError;
      setStaleConflict(isConflict);
      if (!isConflict)
        setPublicationValidationMessage(error instanceof Error ? error.message : null);
      setPublish({
        kind: "error",
        message: isConflict
          ? "Someone saved a newer revision. Reload, save your edits, and review again."
          : error instanceof Error
            ? error.message
            : "The publication comparison is unavailable.",
      });
    }
  }

  async function publishDraft(): Promise<void> {
    const current = ready();
    const review = publishConfirm();
    if (current === undefined || review === undefined) return;
    const authorship = parseReviewedQuestionAuthorship(publicationAuthorshipText());
    if (authorship === null) {
      setPublish({
        ...review,
        message: "Provide one to sixteen distinct reviewed Question Authors.",
      });
      requestAnimationFrame(() => publicationAuthorshipInput?.focus());
      return;
    }
    const request: { readonly authorship: QuestionAuthorship } = { authorship };
    setPublish({ kind: "publishing" });
    try {
      const outcome = await props.repository.publish(
        current.draft,
        request,
        review.review.revision,
      );
      switch (outcome.kind) {
        case "published":
          setPublish({
            kind: "published",
            questionId: outcome.questionId,
          });
          break;
        case "validationFailed":
          setViolations(outcome.violations);
          setPublish({
            ...review,
            message: "Publication needs the listed capability changes. Your draft is still open.",
          });
          break;
        case "error":
          setPublish({ ...review, message: outcome.message });
          break;
      }
    } catch (error: unknown) {
      const isConflict = error instanceof WorkspaceConflictError;
      setStaleConflict(isConflict);
      setPublish(
        isConflict
          ? {
              kind: "error",
              message: "Someone saved a newer revision. Reload, save your edits, and review again.",
            }
          : {
              ...review,
              message:
                error instanceof Error
                  ? error.message
                  : "Publication could not finish. Your draft is still open.",
            },
      );
    }
  }

  async function renderInstructorPreview(): Promise<void> {
    const current = ready();
    const selectedSeed = seed();
    const boundary = props.repository.instructorPreview;
    if (current === undefined || selectedSeed === null) {
      setInstructorPreview({
        kind: "error",
        message: "Enter a whole-number seed before previewing.",
      });
      return;
    }
    if (boundary === undefined) {
      setInstructorPreview({
        kind: "unavailable",
        message: "Instructor answer preview is not available for this Draft Question.",
      });
      return;
    }
    setInstructorPreview({ kind: "loading" });
    try {
      // Saving first gives the protected route one exact persisted revision to derive from. It
      // also preserves local edits if the CAS loses, instead of showing an older answer.
      setDraftDirty(true);
      const saved = await props.repository.saveDraft(current.draft);
      setPage({ ...current, draft: saved });
      setDraftDirty(false);
      setStaleConflict(false);
      const result = await boundary.requestPresentation(saved, selectedSeed);
      if (result.kind === "available") {
        setInstructorPreview({ kind: "available", presentation: result.presentation });
        return;
      }
      setInstructorPreview({
        kind: "unavailable",
        message: `${result.backend} author preview is unavailable: ${result.reason}`,
      });
    } catch (error: unknown) {
      const isConflict =
        error instanceof WorkspaceConflictError || error instanceof InstructorPreviewConflictError;
      setStaleConflict(isConflict);
      setInstructorPreview({
        kind: "error",
        message: isConflict
          ? "Someone saved a newer revision. Reload, save your edits, and try again."
          : error instanceof Error
            ? `${error.message}. Try again.`
            : "The instructor preview could not load. Try again.",
      });
    }
  }

  createEffect(() => {
    const current = ready();
    const revision =
      current === undefined
        ? null
        : (props.repository.displayedRevision?.(current.draft.workspace) ?? null);
    props.onDraftDisplayStateChange?.(revision === null ? null : { revision, dirty: draftDirty() });
  });

  onMount(() => void load());

  return (
    <section
      class="page editor-page"
      data-route-surface="workspaceEditor"
      inert={props.replacementPending === true}
      aria-busy={page().kind === "loading" || props.replacementPending === true}
    >
      <style>{EDITOR_PAGE_STYLES}</style>
      <p class="eyebrow">Instructor workspace</p>
      <h1>Draft, preview, and publish a learning question</h1>
      <p class="page-lede">
        Start with the prompt and response students will see. Preview uses a seed-controlled,
        key-free local variant; an explicit instructor action can request a protected answer
        presentation. Each confirmed publication creates a new Question ID after review.
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {creationMessage() ?? saveMessage() ?? ""}
      </p>

      <Show when={page().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading workspace drafts...
        </p>
      </Show>
      <Show when={page().kind === "empty"}>
        <section class="editor-panel" aria-label="No My Question Drafts">
          <h2>No drafts yet</h2>
          <p>Create a workspace draft to begin with a small Student-facing prompt and response.</p>
          <Show when={props.onCreatePleQuestionJson !== undefined}>
            <button
              class="primary-action"
              type="button"
              disabled={creatingPleQuestionJson()}
              onClick={() => void createPleQuestionJson()}
            >
              {creatingPleQuestionJson() ? "Creating Question..." : "Create Question"}
            </button>
          </Show>
        </section>
      </Show>
      <Show when={pageError()}>
        <section class="route-error" role="alert">
          <h2>Workspace unavailable</h2>
          <p>{pageError()?.message ?? ""}</p>
          <button class="primary-action" type="button" onClick={() => void load()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={ready()}>
        {(current) => (
          <div class="editor-grid">
            <aside class="editor-panel" aria-label="My Question Drafts">
              <h2>Your drafts</h2>
              <Show when={props.onCreatePleQuestionJson !== undefined}>
                <button
                  class="primary-action"
                  type="button"
                  disabled={creatingPleQuestionJson()}
                  onClick={() => void createPleQuestionJson()}
                >
                  {creatingPleQuestionJson() ? "Creating Question..." : "Create Question"}
                </button>
              </Show>
              <ul class="editor-draft-list">
                <For each={current().drafts}>
                  {(draft) => (
                    <li>
                      <button
                        type="button"
                        aria-current={
                          draft.workspace === current().draft.workspace ? "page" : undefined
                        }
                        onClick={() => void chooseDraft(draft)}
                      >
                        <strong>{draft.title}</strong>
                        <br />
                        <small>{draft.questionBackend} workspace draft</small>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
              <Show when={current().nextCursor !== null}>
                <button class="quiet-action" type="button" onClick={() => void loadMoreDrafts()}>
                  Load more drafts
                </button>
              </Show>
              <p class="editor-guidance">Drafts are private until you publish them.</p>
            </aside>

            <div class="editor-preview">
              <section class="editor-panel" aria-labelledby="draft-editor-heading">
                <h2 id="draft-editor-heading">Student-facing draft</h2>
                <label class="editor-field">
                  Question title
                  <input
                    value={current().draft.title}
                    onInput={(event) =>
                      replaceDraft({ ...current().draft, title: event.currentTarget.value })
                    }
                  />
                </label>
                <label class="editor-field">
                  Prompt text
                  <textarea
                    value={firstTextPrompt(current().draft)}
                    onInput={(event) =>
                      replaceDraft(
                        replaceFirstTextPrompt(current().draft, event.currentTarget.value),
                      )
                    }
                  />
                </label>
                <p class="calm-status">Policy: {policySummary(current().draft)}</p>
                <div class="editor-actions">
                  <button class="primary-action" type="button" onClick={() => void saveDraft()}>
                    Save draft
                  </button>
                  <Show when={saveMessage()}>{(message) => <span>{message()}</span>}</Show>
                </div>
                <Show when={staleConflict()}>
                  <div class="inline-error" role="alert">
                    <p>Someone saved a newer revision. Your unsaved edits are still here.</p>
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => void reloadAfterConflict()}
                    >
                      Reload newest draft
                    </button>
                  </div>
                </Show>
                <Show when={props.repository.deleteDraft !== undefined}>
                  <button class="quiet-action" type="button" onClick={() => void deleteDraft()}>
                    Delete this draft
                  </button>
                </Show>
              </section>

              <section class="editor-panel" aria-labelledby="policy-heading">
                <h2 id="policy-heading">Assignment capabilities</h2>
                <p>
                  Choose the capabilities this assignment needs. Every unsupported choice is shown
                  here before publication.
                </p>
                <Show when={props.repository.capabilities?.assignmentValidation === false}>
                  <p class="saved-notice">
                    Assignment capability validation needs its server contract.
                  </p>
                </Show>
                <div class="editor-capabilities">
                  <For each={EDITOR_CAPABILITIES}>
                    {(capability) => (
                      <label class="editor-capability">
                        <input
                          type="checkbox"
                          disabled={props.repository.capabilities?.assignmentValidation === false}
                          checked={requiredCapabilities().includes(capability)}
                          onChange={(event) => {
                            const next = event.currentTarget.checked
                              ? [...requiredCapabilities(), capability]
                              : requiredCapabilities().filter((value) => value !== capability);
                            void checkCapabilities(next);
                          }}
                        />
                        Require {capabilityLabel(capability)}
                      </label>
                    )}
                  </For>
                </div>
                <For each={violations()}>
                  {(violation) => (
                    <p class="editor-violation" role="alert">
                      <strong>{violation.title}</strong> cannot provide{" "}
                      {capabilityLabel(violation.capability)}.
                    </p>
                  )}
                </For>
                <Show when={publicationValidationMessage()}>
                  <p class="inline-error" role="alert">
                    Question Publication Validation: {publicationValidationMessage()}
                  </p>
                </Show>
              </section>

              <section class="editor-panel" aria-labelledby="preview-heading">
                <h2 id="preview-heading">Student preview</h2>
                <p>
                  This uses the same renderer and response controls as students, without sending a
                  request or revealing evaluation material.
                </p>
                <label class="editor-field">
                  Seed
                  <input
                    inputmode="numeric"
                    value={seedInput()}
                    onInput={(event) => setSeedInput(event.currentTarget.value)}
                  />
                </label>
                <div class="editor-actions">
                  <button class="primary-action" type="button" onClick={() => void renderPreview()}>
                    Preview this Question Variant
                  </button>
                </div>
                <Show when={props.repository.capabilities?.instructorPreview === false}>
                  <p class="saved-notice">
                    Instructor answer preview is not available for this workspace.
                  </p>
                </Show>
                <Show when={preview().kind === "loading"}>
                  <p role="status">Generating preview...</p>
                </Show>
                <Show when={previewError()}>
                  <p class="inline-error" role="alert">
                    {previewError()?.message ?? ""}
                  </p>
                </Show>
                <Show when={previewReady()}>
                  {(state) => (
                    <article class="question-card">
                      <ErrorBoundary
                        fallback={(error) => (
                          <p class="inline-error">
                            {error instanceof Error ? error.message : "Preview rendering failed."}
                          </p>
                        )}
                      >
                        <QuestionRenderer
                          presentation={state().preview}
                          assetUrl={(asset) => safeAssetUrl(asset.asset)}
                          onRetry={() => void renderPreview()}
                        />
                      </ErrorBoundary>
                      <QuestionResponseControl
                        attemptId={`preview:${state().preview.workspace}:${state().preview.seed}`}
                        definition={state().preview.response}
                        validator={props.responseValidator}
                        onEscape={() => undefined}
                        onSubmit={() => {
                          setSaveMessage(
                            "Preview checks response format only; it does not grade or record an answer.",
                          );
                          return Promise.resolve({ kind: "accepted" });
                        }}
                      />
                    </article>
                  )}
                </Show>

                <section
                  class="instructor-preview"
                  aria-busy={instructorPreview().kind === "loading"}
                  aria-labelledby="instructor-preview-heading"
                >
                  <h3 id="instructor-preview-heading">Instructor answer preview</h3>
                  <p>
                    Request the server-derived answer presentation for the most recently saved
                    draft. This request is separate from the student preview and is never saved in
                    this browser.
                  </p>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={
                      props.repository.capabilities?.instructorPreview === false ||
                      instructorPreview().kind === "loading"
                    }
                    onClick={() => void renderInstructorPreview()}
                  >
                    {instructorPreview().kind === "error"
                      ? "Retry instructor answer preview"
                      : "Load instructor answer preview"}
                  </button>
                  <Show when={instructorPreview().kind === "loading"}>
                    <p role="status">Loading protected instructor preview...</p>
                  </Show>
                  <Show when={instructorPreviewUnavailable()}>
                    {(state) => (
                      <p class="saved-notice" role="status">
                        {state().message}
                      </p>
                    )}
                  </Show>
                  <Show when={instructorPreviewError()}>
                    {(state) => (
                      <p class="inline-error" role="alert">
                        {state().message}
                      </p>
                    )}
                  </Show>
                  <Show when={instructorPreviewAvailable()}>
                    {(state) => (
                      <article class="question-card instructor-preview__card">
                        <QuestionRenderer
                          presentation={state().presentation}
                          assetUrl={(asset) => safeAssetUrl(asset.asset)}
                          onRetry={() => void renderInstructorPreview()}
                        />
                        <section aria-labelledby="instructor-correct-response-heading">
                          <h4 id="instructor-correct-response-heading">Correct response</h4>
                          <ContentBlockList
                            blocks={state().presentation.questionAnswer}
                            assetUrl={(asset) => safeAssetUrl(asset.asset)}
                          />
                        </section>
                        <Show
                          when={(state().presentation.questionAnswerExplanation?.length ?? 0) > 0}
                        >
                          <section aria-labelledby="instructor-question-answer-explanation-heading">
                            <h4 id="instructor-question-answer-explanation-heading">
                              Answer Explanation
                            </h4>
                            <ContentBlockList
                              blocks={state().presentation.questionAnswerExplanation ?? []}
                              assetUrl={(asset) => safeAssetUrl(asset.asset)}
                            />
                          </section>
                        </Show>
                      </article>
                    )}
                  </Show>
                </section>
              </section>

              <section class="editor-panel" aria-labelledby="publish-heading">
                <h2 id="publish-heading">Publish question</h2>
                <p>
                  Review the exact content before publishing a new question. Your draft remains open
                  if publication is refused.
                </p>
                <Show when={props.repository.capabilities?.publication === false}>
                  <p class="saved-notice">
                    Publication review waits for its server comparison route.
                  </p>
                </Show>
                <Show when={publishConfirm()}>
                  {(state) => (
                    <div aria-live="polite">
                      <h3>Publication changes</h3>
                      <p>
                        This publication creates a new Question ID. Existing assignments keep their
                        assigned questions until an instructor deliberately replaces an item.
                      </p>
                      <p>
                        Publishing saved title: <strong>{state().review.proposedTitle}</strong>
                      </p>
                      <ul class="editor-review">
                        <For each={state().review.sections}>
                          {(section) => (
                            <li>
                              <strong>{section.label}:</strong> {section.before ?? "New"} to{" "}
                              {section.after}
                            </li>
                          )}
                        </For>
                      </ul>
                      <label class="editor-field">
                        Question Authors
                        <textarea
                          ref={(element) => {
                            publicationAuthorshipInput = element;
                          }}
                          value={publicationAuthorshipText()}
                          onInput={(event) =>
                            setPublicationAuthorshipText(event.currentTarget.value)
                          }
                          aria-describedby="workspace-authorship-help"
                        />
                        <span id="workspace-authorship-help">
                          Enter one reviewed name per line. The published attribution never uses
                          account data.
                        </span>
                      </label>
                      <button
                        class="primary-action"
                        type="button"
                        onClick={() => void publishDraft()}
                      >
                        Confirm publication
                      </button>
                      <Show when={state().message !== null}>
                        <p class="inline-error" role="alert">
                          {state().message}
                        </p>
                      </Show>
                    </div>
                  )}
                </Show>
                <Show when={published()}>
                  <p class="saved-notice" role="status">
                    The new Question ID is now available in the <a href="/library">library</a>.
                  </p>
                </Show>
                <Show when={publishError()}>
                  <p class="inline-error" role="alert">
                    {publishError()?.message ?? ""}
                  </p>
                </Show>
                <Show when={publish().kind === "loadingDiff" || publish().kind === "publishing"}>
                  <p role="status">Preparing publication review...</p>
                </Show>
                <Show when={publish().kind === "idle" || publish().kind === "error"}>
                  <div class="editor-actions">
                    <button
                      class="primary-action"
                      type="button"
                      disabled={props.repository.capabilities?.publication === false}
                      onClick={() => void requestPublishReview()}
                    >
                      Review publication changes
                    </button>
                  </div>
                </Show>
              </section>
            </div>
          </div>
        )}
      </Show>
    </section>
  );
}
