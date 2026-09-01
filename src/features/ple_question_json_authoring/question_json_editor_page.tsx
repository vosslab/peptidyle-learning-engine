// question_json_editor_page.tsx - private instructor surface for ple-question-json authoring.

import { For, Show, batch, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { parseReviewedQuestionAuthorship } from "../../api/question_authorship";
import { PleQuestionJsonFeedbackFields } from "./question_json_feedback_fields";
import { PleQuestionJsonHintField } from "./question_json_hint_field";
import { hotspotSourceFromAsset } from "./question_json_hotspot_editor_model";
import {
  initialPleQuestionJsonEditorState,
  reducePleQuestionJsonEditor,
  reorderChoices,
  setQuestionAttemptLimit,
  setPleQuestionJsonPoints,
  setPleQuestionJsonPrompt,
  setPleQuestionJsonTitle,
  setLanguage,
  setQuestionDescription,
  setQuestionCitation,
  setQuestionLicense,
  setQuestionHint,
  setOutcomeFeedback,
  setTags,
  setClassifications,
  setQuestionAttemptTimeLimit,
  validatePleQuestionJsonSource,
  type PleQuestionJsonEditorAction,
  type PleQuestionJsonEditorState,
} from "./question_json_editor_model";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "./question_json_editor_styles";
import { PleQuestionJsonMetadataFields } from "./question_json_metadata_fields";
import { parseNumericLiteral } from "./question_json_numeric_model";
import { PleQuestionJsonPolicyFields } from "./question_json_policy_fields";
import { pleQuestionJsonPublicPreview } from "./question_json_public_preview";
import { PleQuestionJsonPreview } from "./question_json_preview";
import { PleQuestionJsonResponseFields } from "./question_json_response_fields";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import { PleQuestionJsonStaleConflictError } from "./question_json_repository";
import type { PleQuestionJsonAssetDescriptor } from "./question_json_asset_client";
import type {
  PleQuestionJsonHotspotResponse,
  PleQuestionJsonDocument,
} from "./question_json_source";
import type { PleQuestionJsonInstructorAnswerCheck } from "./question_json_preview";

export type {
  PleQuestionJsonDraftDisplayState,
  PleQuestionJsonEditorPageProps,
} from "./question_json_editor_types";

type Review = {
  readonly revision: string;
  readonly baseQuestion: "newQuestion";
  readonly title: string;
  readonly changed: ReadonlyArray<string>;
};

function authorSafeMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return fallback;
}

function sourceFrom(state: PleQuestionJsonEditorState): PleQuestionJsonDocument | null {
  if (state.kind === "conflict" || state.kind === "reloading") return state.localSource;
  if (state.kind === "error") return state.source;
  if (state.kind === "loading" || state.kind === "published") return null;
  return state.source;
}

function errorMessage(state: PleQuestionJsonEditorState): string | null {
  return state.kind === "error" ? state.message : null;
}

function publishedReference(state: PleQuestionJsonEditorState): string | null {
  return state.kind === "published" ? state.reference : null;
}

function fieldErrors(source: PleQuestionJsonDocument | null): Readonly<Record<string, string>> {
  if (source === null) return {};
  const validation = validatePleQuestionJsonSource(source);
  return Object.fromEntries(validation.issues.map((issue) => [issue.field, issue.message]));
}

function answerCheck(source: PleQuestionJsonDocument): PleQuestionJsonInstructorAnswerCheck | null {
  const response = source.response;
  if (response.kind === "multipleAnswer") {
    const correctChoiceTexts = response.choices
      .filter((choice) => response.correctChoices.includes(choice.id))
      .map((choice) => choice.text);
    return correctChoiceTexts.length === 0 ? null : { kind: "multipleAnswer", correctChoiceTexts };
  }
  if (response.kind === "fillIn") {
    return { kind: "fillIn", answers: response.answers, matchMode: response.matchMode };
  }
  if (response.kind === "multiFillIn") {
    return {
      kind: "multiFillIn",
      blanks: response.blanks.map((blank) => ({ label: blank.label, answers: blank.answers })),
    };
  }
  if (response.kind === "numeric") {
    return {
      kind: "numeric",
      answer: response.answer,
      tolerance: response.tolerance,
      unit: response.unit,
    };
  }
  if (response.kind === "ordering") {
    const items = response.correctOrder.map((id) => response.items.find((item) => item.id === id));
    if (items.some((item) => item === undefined)) return null;
    return {
      kind: "ordering",
      items: items.filter(
        (item): item is { readonly id: string; readonly text: string } => item !== undefined,
      ),
    };
  }
  if (response.kind === "matching") {
    const prompts = new Map(response.prompts.map((item) => [item.id, item.text]));
    const choices = new Map(response.choices.map((item) => [item.id, item.text]));
    const pairs = response.matches.map((pair) => {
      const prompt = prompts.get(pair.prompt);
      const choice = choices.get(pair.choice);
      return prompt === undefined || choice === undefined ? null : ([prompt, choice] as const);
    });
    if (pairs.some((pair) => pair === null)) return null;
    return {
      kind: "matching",
      pairs: pairs.filter((pair): pair is readonly [string, string] => pair !== null),
    };
  }
  if (response.kind !== "singleChoice") return null;
  const correct = response.choices.find((choice) => choice.id === response.correctChoice);
  if (correct === undefined) return null;
  return {
    kind: "singleChoice",
    correctChoiceText: correct.text,
    correctFeedback: source.feedback.correct,
    incorrectFeedback: source.feedback.incorrect,
  };
}

function hasLocalDraftChanges(state: PleQuestionJsonEditorState): boolean {
  if (state.kind === "ready") return state.status !== "clean";
  return ["loading", "conflict", "reloading", "error", "published"].includes(state.kind);
}

/**
 * A purpose-built author surface. Student preview is a local answer-free projection; this component
 * does not write source material to URLs, storage, or diagnostics.
 */
export function PleQuestionJsonEditorPage(props: PleQuestionJsonEditorPageProps): JSX.Element {
  const [state, setState] = createSignal<PleQuestionJsonEditorState>(
    initialPleQuestionJsonEditorState(),
  );
  const [latestRevision, setLatestRevision] = createSignal(props.initial.revision);
  const [review, setReview] = createSignal<Review | null>(null);
  const [reviewLoading, setReviewLoading] = createSignal(false);
  const [authorshipText, setAuthorshipText] = createSignal("");
  const [publishedSummary, setPublishedSummary] = createSignal<QuestionSummary>();
  const [status, setStatus] = createSignal<string | null>(null);
  const [showInstructorCheck, setShowInstructorCheck] = createSignal(false);
  let heading: HTMLHeadingElement | null = null;
  let authorshipInput: HTMLTextAreaElement | undefined;
  let reviewRequestGeneration = 0;
  let headingFocusDelivered = false;

  // The draft accessor is the sole render-time source. Each reducer transition updates this
  // projection together with workflow state, so a mounted response editor never captures a stale
  // Question Response Format branch.
  const [source, setSource] = createSignal<PleQuestionJsonDocument | null>(null);
  // Numeric source values are numbers. This local literal is intentionally separate so partially
  // typed values such as "6.02e" remain visible without replacing the last valid source value.
  const [numericAnswerLiteral, setNumericAnswerLiteral] = createSignal("0");
  // HOTSPOT remains local until a verified descriptor and a useful student description can make
  // a valid persisted source. This prevents a placeholder asset from entering a draft.
  const [hotspotPending, setHotspotPending] = createSignal(false);
  const [pendingHotspotAsset, setPendingHotspotAsset] =
    createSignal<PleQuestionJsonAssetDescriptor | null>(null);
  const [pendingHotspotDescription, setPendingHotspotDescription] = createSignal("");
  let displayedResponseKind: PleQuestionJsonDocument["response"]["kind"] | null = null;
  function currentSource(): PleQuestionJsonDocument {
    const draft = source();
    if (draft === null) throw new Error("The private draft is unavailable.");
    return draft;
  }
  const numericLiteralError = (): string | undefined => {
    const current = source();
    if (current?.response.kind !== "numeric") return undefined;
    return parseNumericLiteral(numericAnswerLiteral()) === null
      ? "Finish the numeric value, for example 6.02e23, before saving or reviewing publication."
      : undefined;
  };
  const errors = (): Readonly<Record<string, string>> => {
    const base = fieldErrors(source());
    const numericError = numericLiteralError();
    return numericError === undefined ? base : { ...base, "response.answer": numericError };
  };
  createEffect(() => {
    const next = source();
    const nextKind = next?.response.kind ?? null;
    if (next?.response.kind === "numeric" && displayedResponseKind !== "numeric") {
      setNumericAnswerLiteral(String(next.response.answer));
    }
    displayedResponseKind = nextKind;
  });
  function transition(action: PleQuestionJsonEditorAction): void {
    const next = reducePleQuestionJsonEditor(state(), action);
    batch(() => {
      setState(next);
      setSource(sourceFrom(next));
      const nextSource = sourceFrom(next);
      if (action.kind !== "edit" && nextSource?.response.kind === "numeric") {
        setNumericAnswerLiteral(String(nextSource.response.answer));
      }
    });
  }
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
    return (
      current.kind === "ready" &&
      current.status === "dirty" &&
      numericLiteralError() === undefined &&
      !hotspotPending()
    );
  };
  const isSaved = (): boolean => {
    const current = state();
    return (
      ((current.kind === "ready" && current.status === "clean") ||
        current.kind === "publishReview" ||
        current.kind === "publishing") &&
      numericLiteralError() === undefined &&
      !hotspotPending()
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
    transition({ kind: "loaded", source: props.initial.source });
  });

  function applyEdit(next: PleQuestionJsonDocument): void {
    if (isLocked()) return;
    cancelPendingReview();
    setShowInstructorCheck(false);
    setReview(null);
    setStatus(null);
    transition({ kind: "edit", source: next });
  }

  function selectHotspotFormat(): void {
    if (isLocked()) return;
    setHotspotPending(true);
    setPendingHotspotAsset(null);
    setPendingHotspotDescription("");
    setShowInstructorCheck(false);
    setReview(null);
    setStatus("Choose a verified image and describe it before the hotspot draft can be saved.");
  }

  function selectOrdinaryFormat(): void {
    setHotspotPending(false);
    setPendingHotspotAsset(null);
    setPendingHotspotDescription("");
  }

  function hotspotResponse(): PleQuestionJsonHotspotResponse | null {
    const current = source();
    return current?.response.kind === "hotspot" ? current.response : null;
  }

  function selectHotspotAsset(asset: PleQuestionJsonAssetDescriptor): void {
    const current = source();
    if (current === null || isLocked()) return;
    setPendingHotspotAsset(asset);
    const description = pendingHotspotDescription();
    if (description.trim() === "") {
      setStatus(
        "Describe the image for Students, then the verified image will become this hotspot draft.",
      );
      return;
    }
    applyEdit(hotspotSourceFromAsset(current, asset, description));
    setHotspotPending(false);
    setStatus("Verified image selected. Define the labeled regions before saving.");
  }

  function updatePendingHotspotDescription(description: string): void {
    setPendingHotspotDescription(description);
    const asset = pendingHotspotAsset();
    const current = source();
    if (asset === null || current === null || description.trim() === "" || isLocked()) return;
    applyEdit(hotspotSourceFromAsset(current, asset, description));
    setHotspotPending(false);
    setStatus("Verified image selected. Define the labeled regions before saving.");
  }

  function updateNumericAnswerLiteral(literal: string): void {
    setNumericAnswerLiteral(literal);
    const current = source();
    if (current === null || current.response.kind !== "numeric") return;
    const answer = parseNumericLiteral(literal);
    if (answer === null) return;
    applyEdit({ ...current, response: { ...current.response, answer } });
  }

  async function save(): Promise<void> {
    const current = source();
    if (current === null || !canSave() || isLocked()) return;
    if (numericLiteralError() !== undefined) {
      setStatus("Finish the numeric value before saving.");
      return;
    }
    const validation = validatePleQuestionJsonSource(current);
    if (!validation.valid) {
      setStatus("Correct the highlighted question details before saving.");
      return;
    }
    setStatus("Saving private draft...");
    transition({ kind: "saveStarted" });
    try {
      const result = await props.repository.save(props.workspace, current);
      setLatestRevision(result.revision);
      transition({ kind: "saveSucceeded" });
      setStatus("Private draft saved. It is not published.");
    } catch (error: unknown) {
      if (error instanceof PleQuestionJsonStaleConflictError) {
        setShowInstructorCheck(false);
        transition({ kind: "saveConflict" });
        setStatus("A newer draft exists. Your local edits are still shown below.");
        return;
      }
      transition({
        kind: "saveFailed",
        message: authorSafeMessage(error, "The private draft could not be saved."),
      });
    }
  }

  async function reload(): Promise<void> {
    if (!isConflict()) return;
    transition({ kind: "reloadStarted" });
    setStatus("Loading the newest private draft...");
    try {
      const newest = await props.repository.reload(props.workspace);
      setLatestRevision(newest.revision);
      setReview(null);
      setShowInstructorCheck(false);
      transition({ kind: "reloadSucceeded", source: newest.source });
      setStatus("Loaded the newest saved draft. Review it before editing.");
      queueMicrotask(() => heading?.focus());
    } catch (error: unknown) {
      const message = authorSafeMessage(error, "The newest draft could not load.");
      transition({
        kind: "reloadFailed",
        message,
      });
      setStatus(`${message} Your local edits are still shown below.`);
    }
  }

  function dismissError(): void {
    setStatus(null);
    transition({ kind: "dismissError" });
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
    setStatus("Checking Question Publication Validation...");
    try {
      const validation = await props.api.validateWorkspacePublication(props.workspace);
      if (!reviewRequestIsCurrent(generation, revision)) return;
      if (validation.kind === "questionPublicationValidationUnavailable") {
        setStatus(validation.message);
        return;
      }
      if (validation.revision !== revision) {
        setStatus("The saved draft changed. Reload it before publishing.");
        return;
      }
      const review = await props.api.getQuestionPublicationReview(props.workspace);
      if (!reviewRequestIsCurrent(generation, revision)) return;
      if (review.revision !== revision) {
        setStatus(
          "The saved draft changed while its review was loading. Reload it before publishing.",
        );
        return;
      }
      const nextReview: Review = {
        revision: review.revision,
        baseQuestion: review.baseQuestion,
        title: review.current.title,
        changed: review.changed,
      };
      setReview(nextReview);
      transition({
        kind: "reviewOpened",
        review: "Publication review is ready.",
      });
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
    const authorship = parseReviewedQuestionAuthorship(authorshipText());
    if (authorship === null) {
      setStatus("Provide one to sixteen distinct reviewed Question Authors before publishing.");
      requestAnimationFrame(() => authorshipInput?.focus());
      return;
    }
    transition({ kind: "publishStarted" });
    setStatus("Publishing a new Question ID...");
    try {
      const summary = await props.repository.publish(props.workspace, { authorship });
      setPublishedSummary(summary);
      transition({
        kind: "publishSucceeded",
        reference: "/library",
      });
      setStatus("The new Question ID is published.");
    } catch (error: unknown) {
      transition({
        kind: "publishFailed",
        message: authorSafeMessage(
          error,
          "Publication could not finish. Your draft remains editable.",
        ),
      });
    }
  }

  function moveChoice(choiceId: string, direction: "up" | "down"): void {
    const current = source();
    if (current === null || current.response.kind !== "singleChoice") return;
    const choices = current.response.choices;
    const index = choices.findIndex((choice) => choice.id === choiceId);
    const other = direction === "up" ? index - 1 : index + 1;
    if (index < 0 || other < 0 || other >= choices.length) return;
    const ids = choices.map((choice) => choice.id);
    const displaced = ids[other];
    if (displaced === undefined) return;
    ids[other] = choiceId;
    ids[index] = displaced;
    const result = reorderChoices(current, ids);
    if (result.changed) applyEdit(result.source);
  }

  return (
    <main
      class="page ple-question-json-authoring"
      data-route-surface="pleQuestionJsonEditor"
      inert={props.replacementPending === true}
      aria-busy={props.replacementPending === true}
    >
      <style>{PLE_QUESTION_JSON_EDITOR_STYLES}</style>
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1 ref={(node) => (heading = node)} tabindex="-1">
          PLE Question JSON
        </h1>
        <p>
          Build a clear student question, save it privately, then review and publish it when it is
          ready.
        </p>
      </header>
      <Show when={status()}>
        {(message) => (
          <p role="status" aria-label="Private draft status">
            {message()}
          </p>
        )}
      </Show>
      <Show when={errorMessage(state())}>
        {(message) => (
          <section class="ple-question-json-authoring__error" role="alert">
            <p>{message()}</p>
            <button type="button" class="quiet-action" onClick={dismissError}>
              Dismiss
            </button>
          </section>
        )}
      </Show>
      <Show when={isConflict()}>
        <section class="ple-question-json-authoring__error" role="alert">
          <p>A newer saved draft exists. Your local edits remain visible for comparison.</p>
          <button type="button" class="primary-action" onClick={() => void reload()}>
            Reload newest draft
          </button>
        </section>
      </Show>
      <Show when={source()}>
        {(_draft) => (
          <div class="editor-grid">
            <section class="editor-panel">
              <label class="ple-question-json-authoring__field">
                <span>Question title</span>
                <input
                  value={currentSource().title}
                  disabled={isLocked()}
                  aria-invalid={errors()["title"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setPleQuestionJsonTitle(currentSource(), event.currentTarget.value))
                  }
                />
              </label>
              <label class="ple-question-json-authoring__field">
                <span>Student-facing prompt</span>
                <textarea
                  value={currentSource().prompt}
                  disabled={isLocked()}
                  aria-invalid={errors()["prompt"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setPleQuestionJsonPrompt(currentSource(), event.currentTarget.value))
                  }
                />
              </label>
              <PleQuestionJsonResponseFields
                source={currentSource}
                fieldErrors={errors()}
                disabled={isLocked()}
                numericAnswerLiteral={numericAnswerLiteral}
                onNumericAnswerLiteralChange={updateNumericAnswerLiteral}
                onEdit={applyEdit}
                onMoveChoice={moveChoice}
                onStatus={setStatus}
                selectedKind={() => (hotspotPending() ? "hotspot" : currentSource().response.kind)}
                hotspotResponse={hotspotResponse}
                pendingHotspotDescription={pendingHotspotDescription}
                assetClient={props.assetClient}
                workspace={props.workspace}
                onChooseHotspot={selectHotspotFormat}
                onSelectHotspotAsset={selectHotspotAsset}
                onPendingHotspotDescriptionChange={updatePendingHotspotDescription}
                onChooseOrdinaryFormat={selectOrdinaryFormat}
              />
              <PleQuestionJsonHintField
                value={currentSource().questionHint}
                fieldErrors={errors()}
                disabled={isLocked()}
                onChange={(questionHint) =>
                  applyEdit(setQuestionHint(currentSource(), questionHint))
                }
              />
              <PleQuestionJsonFeedbackFields
                value={currentSource().feedback}
                fieldErrors={errors()}
                disabled={isLocked()}
                onChange={(patch) =>
                  applyEdit(
                    setOutcomeFeedback(currentSource(), {
                      ...currentSource().feedback,
                      ...patch,
                    }),
                  )
                }
              />
              <PleQuestionJsonPolicyFields
                points={currentSource().points}
                questionAttemptLimit={currentSource().questionAttemptLimit}
                questionAttemptTimeLimit={currentSource().questionAttemptTimeLimit}
                fieldErrors={errors()}
                disabled={isLocked()}
                onPointsChange={(points) =>
                  applyEdit(setPleQuestionJsonPoints(currentSource(), points))
                }
                onQuestionAttemptLimitChange={(limit) =>
                  applyEdit(setQuestionAttemptLimit(currentSource(), limit))
                }
                onQuestionAttemptTimeLimitChange={(policy) =>
                  applyEdit(setQuestionAttemptTimeLimit(currentSource(), policy))
                }
              />
              <PleQuestionJsonMetadataFields
                questionDescription={currentSource().questionDescription}
                tags={currentSource().tags}
                classifications={currentSource().classifications}
                questionLicense={currentSource().questionLicense}
                questionCitation={currentSource().questionCitation}
                language={currentSource().language}
                fieldErrors={errors()}
                disabled={isLocked()}
                onQuestionDescriptionChange={(questionDescription) =>
                  applyEdit(setQuestionDescription(currentSource(), questionDescription))
                }
                onTagsChange={(tags) => applyEdit(setTags(currentSource(), tags))}
                onClassificationsChange={(classifications) =>
                  applyEdit(setClassifications(currentSource(), classifications))
                }
                onQuestionLicenseChange={(questionLicense) =>
                  applyEdit(setQuestionLicense(currentSource(), questionLicense))
                }
                onQuestionCitationChange={(questionCitation) =>
                  applyEdit(setQuestionCitation(currentSource(), questionCitation))
                }
                onLanguageChange={(language) => applyEdit(setLanguage(currentSource(), language))}
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
                <Show
                  when={!hotspotPending()}
                  fallback={
                    <p role="status">
                      Student preview appears after the verified image and student description form
                      a complete hotspot draft.
                    </p>
                  }
                >
                  <For each={[currentSource()]}>
                    {(draft) => (
                      <PleQuestionJsonPreview
                        preview={pleQuestionJsonPublicPreview(draft)}
                        validator={props.responseValidator}
                        instructorAnswerCheck={
                          showInstructorCheck() && isSaved()
                            ? (answerCheck(draft) ?? undefined)
                            : undefined
                        }
                      />
                    )}
                  </For>
                </Show>
              </section>
              <section class="editor-panel" aria-labelledby="ple-question-json-publish-heading">
                <h2 id="ple-question-json-publish-heading">Publish review</h2>
                <p>Review the saved content before publishing a new Question ID.</p>
                <Show when={review() === null}>
                  <button
                    type="button"
                    class="primary-action"
                    disabled={!isSaved() || isLocked() || reviewLoading()}
                    onClick={() => void openPublishReview()}
                  >
                    {reviewLoading()
                      ? "Checking Question Publication Validation..."
                      : "Review publication changes"}
                  </button>
                </Show>
                <Show when={review()}>
                  {(activeReview) => (
                    <div class="ple-question-json-authoring__review">
                      <p>
                        <strong>Question:</strong> {activeReview().title}
                      </p>
                      <p>
                        This publication creates a new Question ID. Existing assignments keep their
                        assigned questions until an instructor deliberately replaces an item.
                      </p>
                      <h3>Changed sections</h3>
                      <ul>
                        <For each={activeReview().changed}>{(section) => <li>{section}</li>}</For>
                      </ul>
                      <label class="ple-question-json-authoring__field">
                        <span>Question Authors</span>
                        <textarea
                          ref={(element) => {
                            authorshipInput = element;
                          }}
                          value={authorshipText()}
                          onInput={(event) => setAuthorshipText(event.currentTarget.value)}
                          aria-describedby="ple-question-json-authorship-help"
                          disabled={isLocked()}
                        />
                        <span
                          id="ple-question-json-authorship-help"
                          class="ple-question-json-authoring__help"
                        >
                          Enter one to sixteen distinct names, one per line. This reviewed text, not
                          account information, is published with the question.
                        </span>
                      </label>
                      <p>Confirming publishes this saved private draft with a new Question ID.</p>
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
            <Show when={publishedSummary()} keyed>
              {(summary) => (
                <>
                  {/* ASVS 1.2.1: server-validated Question Library fields render as Solid text, not HTML. */}
                  <p>
                    <strong>Question:</strong> {summary.metadata.title}
                  </p>
                  <p>
                    <strong>Question ID:</strong> <code>{summary.questionId}</code>
                  </p>
                  <p>
                    <strong>Published to:</strong> Question Library
                  </p>
                  <p>
                    <strong>Authors:</strong>{" "}
                    {summary.authorship.authors.map((author) => author.displayName).join(", ")}
                  </p>
                </>
              )}
            </Show>
            <a href={reference()}>Open question library</a>
          </section>
        )}
      </Show>
    </main>
  );
}
