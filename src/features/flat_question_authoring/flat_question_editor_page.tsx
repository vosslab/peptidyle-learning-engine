// flat_question_editor_page.tsx - private instructor surface for flat-question authoring.

import { For, Show, createEffect, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { PublicationScope } from "../../../generated/api/PublicationScope";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { ApiClient } from "../../api/client";
import { FlatChoiceList } from "./flat_choice_list";
import { FlatFeedbackFields } from "./flat_feedback_fields";
import {
  addChoice,
  initialFlatQuestionEditorState,
  reduceFlatQuestionEditor,
  removeChoice,
  reorderChoices,
  setAttemptPolicy,
  setChoiceFeedback,
  setChoiceText,
  setCorrectChoice,
  setFlatQuestionPoints,
  setFlatQuestionPrompt,
  setFlatQuestionTitle,
  setLanguage,
  setLicense,
  setOutcomeFeedback,
  setTags,
  setTaxonomy,
  setTimingPolicy,
  validateFlatQuestionSource,
  type FlatQuestionEditorState,
} from "./flat_question_editor_model";
import { FLAT_QUESTION_EDITOR_STYLES } from "./flat_question_editor_styles";
import { FlatMetadataFields } from "./flat_metadata_fields";
import { FlatPolicyFields } from "./flat_policy_fields";
import { flatQuestionPublicPreview } from "./flat_question_public_preview";
import { FlatQuestionPreview } from "./flat_question_preview";
import {
  FlatQuestionStaleConflictError,
  type FlatQuestionRepository,
} from "./flat_question_repository";
import type { FlatQuestionRead } from "./flat_question_client";
import type { FlatQuestionSourceV1 } from "./flat_question_source";

export interface FlatQuestionEditorPageProps {
  readonly workspace: WorkspaceId;
  readonly initial: FlatQuestionRead;
  readonly repository: FlatQuestionRepository;
  /** The ordinary browser client supplies only answer-free publication review data. */
  readonly api: Pick<ApiClient, "validateWorkspacePublication" | "getWorkspacePublicationDiff">;
  /** Same-route QTI conversion may move focus into the newly replaced draft. */
  readonly focusHeadingOnMount?: boolean;
  /** Clears the route's one-shot focus request after the unlocked heading receives it. */
  readonly onHeadingFocusDelivered?: () => void;
  /** Reports the exact saved revision and whether this editor has local changes. */
  readonly onDraftDisplayStateChange?: (state: FlatQuestionDraftDisplayState) => void;
  /** Prevents edits while QTI conversion is replacing and refetching this draft. */
  readonly replacementPending?: boolean;
}

export interface FlatQuestionDraftDisplayState {
  readonly revision: string;
  readonly dirty: boolean;
}

type Review = {
  readonly revision: string;
  readonly priorVersion: string | null;
  readonly title: string;
  readonly changed: ReadonlyArray<string>;
};

function authorSafeMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return fallback;
}

function sourceFrom(state: FlatQuestionEditorState): FlatQuestionSourceV1 | null {
  if (state.kind === "conflict" || state.kind === "reloading") return state.localSource;
  if (state.kind === "error") return state.source;
  if (state.kind === "loading" || state.kind === "published") return null;
  return state.source;
}

function errorMessage(state: FlatQuestionEditorState): string | null {
  return state.kind === "error" ? state.message : null;
}

function publishedReference(state: FlatQuestionEditorState): string | null {
  return state.kind === "published" ? state.reference : null;
}

function fieldErrors(source: FlatQuestionSourceV1 | null): Readonly<Record<string, string>> {
  if (source === null) return {};
  const validation = validateFlatQuestionSource(source);
  return Object.fromEntries(validation.issues.map((issue) => [issue.field, issue.message]));
}

function answerCheck(source: FlatQuestionSourceV1): {
  readonly correctChoiceId: string;
  readonly correctChoiceText: string;
  readonly correctFeedback: string | null;
  readonly incorrectFeedback: string | null;
} | null {
  const correct = source.choices.find((choice) => choice.id === source.correctChoice);
  if (correct === undefined) return null;
  return {
    correctChoiceId: correct.id,
    correctChoiceText: correct.text,
    correctFeedback: source.feedback.correct,
    incorrectFeedback: source.feedback.incorrect,
  };
}

function isPublicationScope(value: string): value is PublicationScope {
  return value === "institution" || value === "public";
}

function hasLocalDraftChanges(state: FlatQuestionEditorState): boolean {
  if (state.kind === "ready") return state.status !== "clean";
  return ["loading", "conflict", "reloading", "error", "published"].includes(state.kind);
}

/**
 * A purpose-built author surface. Learner preview is a local answer-free projection; this component
 * does not write source material to URLs, storage, or diagnostics.
 */
export function FlatQuestionEditorPage(props: FlatQuestionEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<FlatQuestionEditorState>(initialFlatQuestionEditorState());
  const [latestRevision, setLatestRevision] = createSignal(props.initial.revision);
  const [review, setReview] = createSignal<Review | null>(null);
  const [reviewLoading, setReviewLoading] = createSignal(false);
  const [scope, setScope] = createSignal<PublicationScope>("institution");
  const [status, setStatus] = createSignal<string | null>(null);
  const [showInstructorCheck, setShowInstructorCheck] = createSignal(false);
  let heading: HTMLHeadingElement | null = null;
  let reviewRequestGeneration = 0;
  let headingFocusDelivered = false;

  const source = createMemo(() => sourceFrom(state()));
  const errors = createMemo(() => fieldErrors(source()));
  const isBusy = (): boolean => {
    const current = state();
    return (
      current.kind === "reloading" ||
      current.kind === "publishing" ||
      (current.kind === "ready" && current.status === "saving")
    );
  };
  const isConflict = (): boolean => state().kind === "conflict";
  const isLocked = (): boolean =>
    isBusy() || isConflict() || state().kind === "error" || props.replacementPending === true;
  const canSave = (): boolean => {
    const current = state();
    return current.kind === "ready" && current.status === "dirty";
  };
  const isSaved = (): boolean => {
    const current = state();
    return (
      (current.kind === "ready" && current.status === "clean") ||
      current.kind === "publishReview" ||
      current.kind === "publishing"
    );
  };

  createEffect(() => {
    props.onDraftDisplayStateChange?.({
      revision: latestRevision(),
      dirty: hasLocalDraftChanges(state()),
    });
  });

  createEffect(() => {
    if (
      headingFocusDelivered ||
      props.focusHeadingOnMount !== true ||
      props.replacementPending === true
    ) {
      return;
    }
    headingFocusDelivered = true;
    queueMicrotask(() => {
      if (props.replacementPending === true) {
        headingFocusDelivered = false;
        return;
      }
      if (heading !== null) {
        heading.focus();
        props.onHeadingFocusDelivered?.();
      }
    });
  });

  function cancelPendingReview(): void {
    reviewRequestGeneration += 1;
    setReviewLoading(false);
  }

  function reviewRequestIsCurrent(generation: number, revision: string): boolean {
    return generation === reviewRequestGeneration && latestRevision() === revision && isSaved();
  }

  onMount(() => {
    setState(reduceFlatQuestionEditor(state(), { kind: "loaded", source: props.initial.source }));
  });

  function applyEdit(next: FlatQuestionSourceV1): void {
    if (isLocked()) return;
    cancelPendingReview();
    setShowInstructorCheck(false);
    setReview(null);
    setStatus(null);
    setState(reduceFlatQuestionEditor(state(), { kind: "edit", source: next }));
  }

  async function save(): Promise<void> {
    const current = source();
    if (current === null || !canSave() || isLocked()) return;
    const validation = validateFlatQuestionSource(current);
    if (!validation.valid) {
      setStatus("Correct the highlighted question details before saving.");
      return;
    }
    setStatus("Saving private draft...");
    setState(reduceFlatQuestionEditor(state(), { kind: "saveStarted" }));
    try {
      const result = await props.repository.save(props.workspace, current);
      setLatestRevision(result.revision);
      setState(reduceFlatQuestionEditor(state(), { kind: "saveSucceeded" }));
      setStatus("Private draft saved. It is not published.");
    } catch (error: unknown) {
      if (error instanceof FlatQuestionStaleConflictError) {
        setShowInstructorCheck(false);
        setState(reduceFlatQuestionEditor(state(), { kind: "saveConflict" }));
        setStatus("A newer draft exists. Your local edits are still shown below.");
        return;
      }
      setState(
        reduceFlatQuestionEditor(state(), {
          kind: "saveFailed",
          message: authorSafeMessage(error, "The private draft could not be saved."),
        }),
      );
    }
  }

  async function reload(): Promise<void> {
    if (!isConflict()) return;
    setState(reduceFlatQuestionEditor(state(), { kind: "reloadStarted" }));
    setStatus("Loading the newest private draft...");
    try {
      const newest = await props.repository.reload(props.workspace);
      setLatestRevision(newest.revision);
      setReview(null);
      setShowInstructorCheck(false);
      setState(
        reduceFlatQuestionEditor(state(), { kind: "reloadSucceeded", source: newest.source }),
      );
      setStatus("Loaded the newest saved draft. Review it before editing.");
      queueMicrotask(() => heading?.focus());
    } catch (error: unknown) {
      const message = authorSafeMessage(error, "The newest draft could not load.");
      setState(
        reduceFlatQuestionEditor(state(), {
          kind: "reloadFailed",
          message,
        }),
      );
      setStatus(`${message} Your local edits are still shown below.`);
    }
  }

  function dismissError(): void {
    setStatus(null);
    setState(reduceFlatQuestionEditor(state(), { kind: "dismissError" }));
  }

  function inspectInstructorAnswer(): void {
    const current = source();
    if (current === null || !isSaved()) {
      setStatus("Save the current private draft before checking the instructor answer.");
      return;
    }
    const check = answerCheck(current);
    if (check === null) {
      setStatus("Select one of the listed choices as the correct answer first.");
      return;
    }
    setShowInstructorCheck(true);
    setStatus("Instructor answer check is visible only in this private authoring page.");
  }

  async function openPublishReview(): Promise<void> {
    if (!isSaved() || isLocked() || reviewLoading()) {
      setStatus("Save a valid private draft before reviewing publication.");
      return;
    }
    const revision = latestRevision();
    const generation = reviewRequestGeneration + 1;
    reviewRequestGeneration = generation;
    setReviewLoading(true);
    setStatus("Checking publication readiness...");
    try {
      const validation = await props.api.validateWorkspacePublication(props.workspace);
      if (!reviewRequestIsCurrent(generation, revision)) return;
      if (validation.kind === "readinessFailure") {
        setStatus(validation.message);
        return;
      }
      if (validation.revision !== revision) {
        setStatus("The saved draft changed. Reload it before publishing.");
        return;
      }
      const diff = await props.api.getWorkspacePublicationDiff(props.workspace);
      if (!reviewRequestIsCurrent(generation, revision)) return;
      if (diff.revision !== revision) {
        setStatus(
          "The saved draft changed while its review was loading. Reload it before publishing.",
        );
        return;
      }
      const nextReview: Review = {
        revision: diff.revision,
        priorVersion: diff.prior?.version ?? null,
        title: diff.current.title,
        changed: diff.changed,
      };
      setReview(nextReview);
      setState(
        reduceFlatQuestionEditor(state(), {
          kind: "reviewOpened",
          review: "Publication review is ready.",
        }),
      );
      setStatus(null);
    } catch (error: unknown) {
      if (!reviewRequestIsCurrent(generation, revision)) return;
      setStatus(
        authorSafeMessage(error, "Publication review could not load. Your draft remains editable."),
      );
    } finally {
      if (generation === reviewRequestGeneration) setReviewLoading(false);
    }
  }

  async function publish(): Promise<void> {
    if (state().kind !== "publishReview" || isLocked()) return;
    const activeReview = review();
    if (activeReview === null || activeReview.revision !== latestRevision()) {
      setStatus("Refresh the publication review before publishing.");
      return;
    }
    setState(reduceFlatQuestionEditor(state(), { kind: "publishStarted" }));
    setStatus("Publishing immutable question version...");
    try {
      const result = await props.repository.publish(props.workspace, scope());
      setState(
        reduceFlatQuestionEditor(state(), {
          kind: "publishSucceeded",
          reference: `/library/${result.problem}/versions/${result.version}`,
        }),
      );
      setStatus("Published an immutable question version.");
    } catch (error: unknown) {
      setState(
        reduceFlatQuestionEditor(state(), {
          kind: "publishFailed",
          message: authorSafeMessage(
            error,
            "Publication could not finish. Your draft remains editable.",
          ),
        }),
      );
    }
  }

  function moveChoice(choiceId: string, direction: "up" | "down"): void {
    const current = source();
    if (current === null) return;
    const index = current.choices.findIndex((choice) => choice.id === choiceId);
    const other = direction === "up" ? index - 1 : index + 1;
    if (index < 0 || other < 0 || other >= current.choices.length) return;
    const ids = current.choices.map((choice) => choice.id);
    const displaced = ids[other];
    if (displaced === undefined) return;
    ids[other] = choiceId;
    ids[index] = displaced;
    const result = reorderChoices(current, ids);
    if (result.changed) applyEdit(result.source);
  }

  return (
    <main
      class="page flat-question-authoring"
      data-route-surface="flatQuestionEditor"
      inert={props.replacementPending === true}
      aria-busy={props.replacementPending === true}
    >
      <style>{FLAT_QUESTION_EDITOR_STYLES}</style>
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1 ref={(node) => (heading = node)} tabindex="-1">
          Flat single-choice question
        </h1>
        <p>
          Build a clear learner question, save it privately, then review and publish an immutable
          version when it is ready.
        </p>
      </header>
      <Show when={status()}>{(message) => <p role="status">{message()}</p>}</Show>
      <Show when={errorMessage(state())}>
        {(message) => (
          <section class="flat-question-authoring__error" role="alert">
            <p>{message()}</p>
            <button type="button" class="quiet-action" onClick={dismissError}>
              Dismiss
            </button>
          </section>
        )}
      </Show>
      <Show when={isConflict()}>
        <section class="flat-question-authoring__error" role="alert">
          <p>A newer saved draft exists. Your local version remains visible for comparison.</p>
          <button type="button" class="primary-action" onClick={() => void reload()}>
            Reload newest draft
          </button>
        </section>
      </Show>
      <Show when={source()}>
        {(current) => (
          <div class="editor-grid">
            <section class="editor-panel">
              <label class="flat-question-authoring__field">
                <span>Question title</span>
                <input
                  value={current().title}
                  disabled={isLocked()}
                  aria-invalid={errors()["title"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setFlatQuestionTitle(current(), event.currentTarget.value))
                  }
                />
              </label>
              <label class="flat-question-authoring__field">
                <span>Learner-facing prompt</span>
                <textarea
                  value={current().prompt}
                  disabled={isLocked()}
                  aria-invalid={errors()["prompt"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setFlatQuestionPrompt(current(), event.currentTarget.value))
                  }
                />
              </label>
              <FlatChoiceList
                choices={current().choices}
                correctChoice={current().correctChoice}
                fieldErrors={errors()}
                disabled={isLocked()}
                onChoiceChange={(id, patch) => {
                  const next =
                    patch.text === undefined
                      ? setChoiceFeedback(current(), id, patch.feedback ?? null)
                      : setChoiceText(current(), id, patch.text);
                  if (next.changed) applyEdit(next.source);
                }}
                onCorrectChoiceChange={(id) => {
                  const next = setCorrectChoice(current(), id);
                  if (next.changed) applyEdit(next.source);
                }}
                onAddChoice={() => {
                  const next = addChoice(current());
                  if (next.changed) applyEdit(next.source);
                }}
                onRemoveChoice={(id) => {
                  const next = removeChoice(current(), id);
                  if (next.changed) applyEdit(next.source);
                }}
                onMoveChoice={moveChoice}
              />
              <FlatFeedbackFields
                value={current().feedback}
                fieldErrors={errors()}
                disabled={isLocked()}
                onChange={(patch) =>
                  applyEdit(setOutcomeFeedback(current(), { ...current().feedback, ...patch }))
                }
              />
              <FlatPolicyFields
                points={current().points}
                attemptPolicy={current().attemptPolicy}
                timingPolicy={current().timingPolicy}
                fieldErrors={errors()}
                disabled={isLocked()}
                onPointsChange={(points) => applyEdit(setFlatQuestionPoints(current(), points))}
                onAttemptPolicyChange={(policy) => applyEdit(setAttemptPolicy(current(), policy))}
                onTimingPolicyChange={(policy) => applyEdit(setTimingPolicy(current(), policy))}
              />
              <FlatMetadataFields
                tags={current().tags}
                taxonomy={current().taxonomy}
                license={current().license}
                language={current().language}
                fieldErrors={errors()}
                disabled={isLocked()}
                onTagsChange={(tags) => applyEdit(setTags(current(), tags))}
                onTaxonomyChange={(taxonomy) => applyEdit(setTaxonomy(current(), taxonomy))}
                onLicenseChange={(license) => applyEdit(setLicense(current(), license))}
                onLanguageChange={(language) => applyEdit(setLanguage(current(), language))}
              />
              <div class="editor-actions">
                <button
                  type="button"
                  class="primary-action"
                  disabled={!canSave() || isLocked()}
                  onClick={() => void save()}
                >
                  Save private draft
                </button>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={!isSaved() || isLocked()}
                  onClick={inspectInstructorAnswer}
                >
                  Check instructor answer
                </button>
              </div>
            </section>
            <aside class="editor-preview">
              <section class="editor-panel">
                <FlatQuestionPreview
                  preview={flatQuestionPublicPreview(current())}
                  instructorAnswerCheck={
                    showInstructorCheck() && isSaved()
                      ? (answerCheck(current()) ?? undefined)
                      : undefined
                  }
                />
              </section>
              <section class="editor-panel" aria-labelledby="flat-publish-heading">
                <h2 id="flat-publish-heading">Publish review</h2>
                <p>
                  Publication creates an immutable version. Student preview never sends a request.
                </p>
                <Show when={review() === null}>
                  <button
                    type="button"
                    class="primary-action"
                    disabled={!isSaved() || isLocked() || reviewLoading()}
                    onClick={() => void openPublishReview()}
                  >
                    {reviewLoading()
                      ? "Checking publication readiness..."
                      : "Review publication changes"}
                  </button>
                </Show>
                <Show when={review()}>
                  {(activeReview) => (
                    <div class="flat-question-authoring__review">
                      <p>
                        <strong>Question:</strong> {activeReview().title}
                      </p>
                      <p>
                        <strong>Previous version:</strong>{" "}
                        {activeReview().priorVersion ?? "First published version"}
                      </p>
                      <h3>Changed sections</h3>
                      <ul>
                        <For each={activeReview().changed}>{(section) => <li>{section}</li>}</For>
                      </ul>
                      <label class="flat-question-authoring__field">
                        <span>Publication scope</span>
                        <select
                          value={scope()}
                          onChange={(event) => {
                            const nextScope = event.currentTarget.value;
                            if (isPublicationScope(nextScope)) setScope(nextScope);
                          }}
                          disabled={isLocked()}
                        >
                          <option value="institution">Institution</option>
                          <option value="public">Public</option>
                        </select>
                      </label>
                      <p>Confirming publishes this saved private draft as an immutable version.</p>
                      <button
                        type="button"
                        class="primary-action"
                        disabled={isLocked()}
                        onClick={() => void publish()}
                      >
                        Confirm and publish
                      </button>
                    </div>
                  )}
                </Show>
              </section>
            </aside>
          </div>
        )}
      </Show>
      <Show when={publishedReference(state())}>
        {(reference) => (
          <section class="editor-panel" role="status">
            <h2>Published</h2>
            <a href={reference()}>Open published version</a>
          </section>
        )}
      </Show>
    </main>
  );
}
