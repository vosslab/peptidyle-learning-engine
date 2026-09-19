// question_json_editor_page.tsx - private instructor surface for ple-question-json authoring.

import { Show, batch, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { parseReviewedQuestionAuthorship } from "../../api/question_authorship";
import {
  initialPleQuestionJsonEditorState,
  reducePleQuestionJsonEditor,
  reorderChoices,
  validatePleQuestionJsonSource,
  type PleQuestionJsonEditorAction,
  type PleQuestionJsonEditorState,
} from "./question_json_editor_model";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "./question_json_editor_styles";
import { parseNumericLiteral } from "./question_json_numeric_model";
import type { PleQuestionJsonPreviewProps } from "./question_json_preview";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import {
  PleQuestionJsonEditorWorkspace,
  type PleQuestionJsonPublishReview,
} from "./question_json_editor_workspace";
import { PleQuestionJsonStaleConflictError } from "./question_json_repository";
import { PleQuestionJsonConflictError } from "./question_json_client";
import { setPleQuestionJsonHotspotAsset } from "./question_json_hotspot_model";
import { PleQuestionGeneralFeedbackConflictError } from "./question_general_feedback_client";
import type { PleQuestionJsonDocument, PleQuestionJsonOrderingItem } from "./question_json_source";
import type { PleQuestionJsonInstructorAnswerCheck } from "./question_json_preview";

export type {
  PleQuestionJsonDraftDisplayState,
  PleQuestionJsonEditorPageProps,
} from "./question_json_editor_types";

type Review = PleQuestionJsonPublishReview;

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
      items: items.filter((item): item is PleQuestionJsonOrderingItem => item !== undefined),
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
 * A purpose-built author surface. Student preview is a local answer-free PLE Question JSON Public Preview; this component
 * does not write Draft Question Content to URLs, browser storage, or diagnostics.
 */
export function PleQuestionJsonEditorPage(props: PleQuestionJsonEditorPageProps): JSX.Element {
  function hotspotDraftAsset(): PleQuestionJsonPreviewProps["hotspotDraftAsset"] {
    const draftQuestion = props.draftQuestion;
    const client = props.assetClient;
    if (draftQuestion === undefined || client === undefined) return undefined;
    return {
      draftQuestion,
      assetUrl: (asset) =>
        new URL(
          client.assetPreviewPath(draftQuestion, asset.questionAsset),
          window.location.origin,
        ),
    };
  }
  const [state, setState] = createSignal<PleQuestionJsonEditorState>(
    initialPleQuestionJsonEditorState(),
  );
  const [latestRevision, setLatestRevision] = createSignal(props.initial.revision);
  const [review, setReview] = createSignal<Review | null>(null);
  const [authorshipText, setAuthorshipText] = createSignal("");
  const [disciplineUuid, setDisciplineUuid] = createSignal<string | null>(null);
  const [subjectUuid, setSubjectUuid] = createSignal<string | null>(null);
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [publishedSummary, setPublishedSummary] = createSignal<QuestionSummary>();
  const [status, setStatus] = createSignal<string | null>(null);
  const [showInstructorCheck, setShowInstructorCheck] = createSignal(false);
  const [assetUploading, setAssetUploading] = createSignal(false);
  const [hotspotPending, setHotspotPending] = createSignal(false);
  const [hotspotLiteralsValid, setHotspotLiteralsValid] = createSignal(true);
  const [generalFeedback, setGeneralFeedback] = createSignal<string | null>(
    props.initialGeneralFeedback.generalFeedback,
  );
  const [savedGeneralFeedback, setSavedGeneralFeedback] = createSignal<string | null>(
    props.initialGeneralFeedback.generalFeedback,
  );
  const [generalFeedbackRevision, setGeneralFeedbackRevision] = createSignal(
    props.initialGeneralFeedback.revision,
  );
  const [generalFeedbackSaving, setGeneralFeedbackSaving] = createSignal(false);
  let heading: HTMLHeadingElement | null = null;
  let authorshipInput: HTMLTextAreaElement | undefined;
  let headingFocusDelivered = false;

  // The draft accessor is the sole render-time source. Each reducer transition updates this
  // draft editor state together with workflow state, so a rendered response editor never captures a stale
  // Question Response Format branch.
  const [source, setSource] = createSignal<PleQuestionJsonDocument | null>(null);
  // Numeric source values are numbers. This local literal is intentionally separate so partially
  // typed values such as "6.02e" remain visible without replacing the last valid source value.
  const [numericAnswerLiteral, setNumericAnswerLiteral] = createSignal("0");
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
      assetUploading() ||
      generalFeedbackSaving() ||
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
      !hotspotPending() &&
      hotspotLiteralsValid() &&
      !assetUploading() &&
      !hasUnsavedGeneralFeedback()
    );
  };
  const isSaved = (): boolean => {
    const current = state();
    return (
      ((current.kind === "ready" && current.status === "clean") ||
        current.kind === "publishReview" ||
        current.kind === "publishing") &&
      numericLiteralError() === undefined &&
      !hotspotPending() &&
      hotspotLiteralsValid() &&
      !assetUploading() &&
      !hasUnsavedGeneralFeedback()
    );
  };
  const hasUnsavedGeneralFeedback = (): boolean => generalFeedback() !== savedGeneralFeedback();
  const sourceHasUnsavedChanges = (): boolean => {
    const current = state();
    return current.kind === "ready" && current.status === "dirty";
  };
  const canEditGeneralFeedback = (): boolean => {
    const current = state();
    return current.kind === "ready" && current.status === "clean" && !isLocked();
  };

  createEffect(() => {
    props.onDraftDisplayStateChange?.({
      revision: latestRevision(),
      dirty:
        hasLocalDraftChanges(state()) ||
        hasUnsavedGeneralFeedback() ||
        hotspotPending() ||
        !hotspotLiteralsValid() ||
        assetUploading(),
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

  onMount(() => {
    transition({ kind: "loaded", source: props.initial.source });
  });

  function applyEdit(next: PleQuestionJsonDocument): void {
    if (isLocked()) return;
    setShowInstructorCheck(false);
    setReview(null);
    setStatus(null);
    transition({ kind: "edit", source: next });
  }

  function updateNumericAnswerLiteral(literal: string): void {
    setNumericAnswerLiteral(literal);
    const current = source();
    if (current === null || current.response.kind !== "numeric") return;
    const answer = parseNumericLiteral(literal);
    if (answer === null) return;
    applyEdit({ ...current, response: { ...current.response, answer } });
  }

  async function uploadAsset(file: File, signal: AbortSignal): Promise<void> {
    const client = props.assetClient;
    if (client === undefined || isLocked()) {
      setStatus("Image upload is unavailable. Your draft is unchanged.");
      return;
    }
    setAssetUploading(true);
    setStatus("Uploading private image...");
    try {
      const asset = await client.uploadAsset(props.draftQuestion, file, latestRevision(), signal);
      if (signal.aborted) {
        setStatus("Upload canceled. Your previous draft is unchanged.");
        return;
      }
      setReview(null);
      setShowInstructorCheck(false);
      transition({ kind: "edit", source: setPleQuestionJsonHotspotAsset(currentSource(), asset) });
      setStatus("Image uploaded. Edit the description and regions, then save before publishing.");
    } catch (error: unknown) {
      if (signal.aborted) {
        setStatus("Upload canceled. Your previous draft is unchanged.");
      } else if (error instanceof PleQuestionJsonConflictError) {
        transition({ kind: "assetConflict" });
        setStatus("A newer draft exists. Your local edits and previous image are retained.");
      } else {
        setStatus(
          authorSafeMessage(error, "Image upload failed. Your previous draft is unchanged."),
        );
      }
    } finally {
      setAssetUploading(false);
    }
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
      const result = await props.repository.save(props.draftQuestion, current);
      setLatestRevision(result.revision);
      setGeneralFeedbackRevision(result.revision);
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

  async function reload(preserveLocal = false): Promise<void> {
    if (!isConflict()) return;
    const local = source();
    transition({ kind: "reloadStarted" });
    setStatus("Loading the newest private draft...");
    try {
      const [newest, newestGeneralFeedback] = await Promise.all([
        props.repository.reload(props.draftQuestion),
        props.generalFeedbackClient.load(props.draftQuestion),
      ]);
      setLatestRevision(newest.revision);
      setGeneralFeedback(newestGeneralFeedback.generalFeedback);
      setSavedGeneralFeedback(newestGeneralFeedback.generalFeedback);
      setGeneralFeedbackRevision(newestGeneralFeedback.revision);
      setReview(null);
      setShowInstructorCheck(false);
      transition({ kind: "reloadSucceeded", source: newest.source });
      if (preserveLocal && local !== null) transition({ kind: "edit", source: local });
      if (!preserveLocal) {
        setHotspotPending(false);
        setHotspotLiteralsValid(true);
      }
      setStatus(
        preserveLocal
          ? "Your local edits are restored over the newest saved draft. Review and save them before publishing."
          : "Loaded the newest saved draft. Review it before editing.",
      );
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

  async function saveGeneralFeedback(): Promise<void> {
    if (isLocked() || sourceHasUnsavedChanges() || !hasUnsavedGeneralFeedback()) {
      return;
    }
    setGeneralFeedbackSaving(true);
    setStatus("Saving general feedback...");
    try {
      const result = await props.generalFeedbackClient.save(
        props.draftQuestion,
        generalFeedback(),
        generalFeedbackRevision(),
      );
      setSavedGeneralFeedback(generalFeedback());
      setGeneralFeedbackRevision(result.revision);
      setLatestRevision(result.revision);
      props.repository.synchronizeRevision(props.draftQuestion, result.revision);
      setStatus("General feedback saved. It remains separate from backend interaction feedback.");
    } catch (error: unknown) {
      if (error instanceof PleQuestionGeneralFeedbackConflictError) {
        const localSource = source();
        if (localSource !== null) {
          setShowInstructorCheck(false);
          setState({ kind: "conflict", localSource });
          setSource(localSource);
        }
        setStatus("A newer draft exists. Reload it before saving general feedback.");
      } else {
        setStatus(authorSafeMessage(error, "General feedback could not be saved."));
      }
    } finally {
      setGeneralFeedbackSaving(false);
    }
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

  function openPublishReview(): void {
    if (!isSaved() || isLocked()) {
      setStatus("Save a valid private draft before reviewing publication.");
      return;
    }
    const nextReview: Review = {
      revision: latestRevision(),
      baseQuestion: "newQuestion",
      questionTitle: currentSource().questionTitle,
      changed: ["PLE Question JSON source"],
    };
    setReview(nextReview);
    transition({ kind: "reviewOpened", review: "Publication review is ready." });
    setStatus(null);
  }

  async function publish(): Promise<void> {
    if (state().kind !== "publishReview" || !isSaved() || isLocked()) return;
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
      const discipline = disciplineUuid();
      const subject = subjectUuid();
      if (discipline === null || subject === null)
        throw new Error("Select a Discipline and Subject before publishing.");
      const summary = await props.repository.publish(props.draftQuestion, {
        authorship,
        disciplineUuid: discipline,
        subjectUuid: subject,
        topicUuid: topicUuid(),
        subtopicUuid: subtopicUuid(),
      });
      setPublishedSummary(summary);
      transition({
        kind: "publishSucceeded",
        reference: `/library/${encodeURIComponent(summary.questionId)}`,
      });
      setStatus("Publication complete. Open the published Question to inspect it.");
      requestAnimationFrame(() => heading?.focus());
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
        <p class="eyebrow">
          {publishedReference(state()) ? "Publication complete" : "Private instructor authoring"}
        </p>
        <h1 ref={(node) => (heading = node)} tabindex="-1">
          {publishedReference(state()) ? "Question published" : "PLE Question JSON"}
        </h1>
        <p>
          {publishedReference(state())
            ? "Your authoring work is complete. This confirmation identifies the Question now available in the Question Library."
            : "Build a clear student question, save it privately, then review and publish it when it is ready."}
        </p>
      </header>
      <Show when={status()}>
        {(message) => (
          <p
            role="status"
            aria-label={publishedReference(state()) ? "Publication status" : "Private draft status"}
          >
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
          <button type="button" class="primary-action" onClick={() => void reload(true)}>
            Keep local edits and load newest edit number
          </button>
          <button type="button" class="primary-action" onClick={() => void reload()}>
            Discard local edits and reload newest draft
          </button>
        </section>
      </Show>
      <PleQuestionJsonEditorWorkspace
        source={source}
        currentSource={currentSource}
        errors={errors}
        isLocked={isLocked}
        canSave={canSave}
        isSaved={isSaved}
        canEditGeneralFeedback={canEditGeneralFeedback}
        hasUnsavedGeneralFeedback={hasUnsavedGeneralFeedback}
        numericAnswerLiteral={numericAnswerLiteral}
        hotspotPending={hotspotPending}
        generalFeedback={generalFeedback}
        generalFeedbackSaving={generalFeedbackSaving}
        showInstructorCheck={showInstructorCheck}
        review={review}
        authorshipText={authorshipText}
        disciplineUuid={disciplineUuid}
        subjectUuid={subjectUuid}
        topicUuid={topicUuid}
        subtopicUuid={subtopicUuid}
        publishedReference={() => publishedReference(state())}
        publishedSummary={publishedSummary}
        hotspotDraftAsset={hotspotDraftAsset}
        instructorAnswerCheck={(draft) => answerCheck(draft) ?? undefined}
        classificationClient={props.classificationClient}
        responseValidator={props.responseValidator}
        assetPreviewPath={(asset) =>
          asset === ""
            ? ""
            : (props.assetClient?.assetPreviewPath(props.draftQuestion, asset) ?? "")
        }
        onEdit={applyEdit}
        onNumericAnswerLiteralChange={updateNumericAnswerLiteral}
        onMoveChoice={moveChoice}
        onStatus={setStatus}
        onHotspotPendingChange={setHotspotPending}
        onHotspotLiteralValidityChange={setHotspotLiteralsValid}
        onUpload={uploadAsset}
        onGeneralFeedbackChange={setGeneralFeedback}
        onSaveGeneralFeedback={() => void saveGeneralFeedback()}
        onSave={() => void save()}
        onInspectInstructorAnswer={inspectInstructorAnswer}
        onOpenPublishReview={openPublishReview}
        onAuthorshipTextChange={setAuthorshipText}
        onAuthorshipInput={(element) => {
          authorshipInput = element;
        }}
        onDisciplineChange={setDisciplineUuid}
        onSubjectChange={setSubjectUuid}
        onTopicChange={setTopicUuid}
        onSubtopicChange={setSubtopicUuid}
        onPublish={() => void publish()}
      />
    </main>
  );
}
