// question_json_editor_page.tsx - private instructor surface for ple-question-json authoring.

import {
  Show,
  batch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
  type Setter,
} from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { parseReviewedQuestionAuthorship } from "../../api/question_authorship";
import { PageFrame } from "../../components/page_frame";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
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
import {
  createQuestionDraftAutosave,
  type QuestionDraftAutosaveState,
} from "./question_draft_autosave";
import { setPleQuestionJsonHotspotImage } from "./question_json_hotspot_model";
import { PleQuestionGeneralFeedbackConflictError } from "./question_general_feedback_client";
import type { PleQuestionJsonDocument, PleQuestionJsonOrderingItem } from "./question_json_source";
import type { PleQuestionJsonRecordMetadata } from "./question_json_defaults";
import { DraftPreviewPanel } from "../question_draft_preview/draft_preview_panel";
import { DraftCorrectionPublishControl } from "../question_draft_preview/draft_correction_publish_control";
import type { PleQuestionJsonInstructorAnswerCheck } from "./question_json_preview";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../../ribbon/route_scope_context";

export type {
  PleQuestionJsonDraftDisplayState,
  PleQuestionJsonEditorPageProps,
} from "./question_json_editor_types";

type Review = PleQuestionJsonPublishReview;

type DraftSnapshot = {
  readonly source: PleQuestionJsonDocument;
  readonly metadata: PleQuestionJsonRecordMetadata;
  readonly generalFeedback: string | null;
  readonly hint: string | null;
  readonly workedSolution: string | null;
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

function publishedQuestionId(state: PleQuestionJsonEditorState): string | null {
  return state.kind === "published" ? state.libraryPath : null;
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
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  function publishQuestionTitle(title: string): void {
    if (publication !== undefined) publishRouteScopeLabels(publication, { questionTitle: title });
  }
  function hotspotDraftQuestionImage(): PleQuestionJsonPreviewProps["hotspotDraftQuestionImage"] {
    const draftQuestion = props.draftQuestion;
    const client = props.questionImageClient;
    if (draftQuestion === undefined || client === undefined) return undefined;
    return {
      draftQuestion,
      questionImageUrl: (asset) =>
        new URL(
          client.questionImagePreviewPath(draftQuestion, asset.questionImageAssetId),
          window.location.origin,
        ),
    };
  }
  const [state, setState] = createSignal<PleQuestionJsonEditorState>(
    initialPleQuestionJsonEditorState(),
  );
  const [latestDraftQuestionEditNumber, setLatestDraftQuestionEditNumber] = createSignal(
    props.initial.draftQuestionEditNumber,
  );
  const [review, setReview] = createSignal<Review | null>(null);
  const [authorshipText, setAuthorshipText] = createSignal(
    props.initialGeneralFeedback.authors.join("\n"),
  );
  const [authorshipEdited, setAuthorshipEdited] = createSignal(false);
  createEffect(() => {
    if (!authorshipEdited()) {
      setAuthorshipText(props.initialGeneralFeedback.authors.join("\n"));
    }
  });
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
  const [hint, setHint] = createSignal<string | null>(props.initialGeneralFeedback.hint);
  const [savedHint, setSavedHint] = createSignal<string | null>(props.initialGeneralFeedback.hint);
  const [workedSolution, setWorkedSolution] = createSignal<string | null>(
    props.initialGeneralFeedback.workedSolution,
  );
  const [savedWorkedSolution, setSavedWorkedSolution] = createSignal<string | null>(
    props.initialGeneralFeedback.workedSolution,
  );
  const [generalFeedbackSaving, setGeneralFeedbackSaving] = createSignal(false);
  const headingId = "ple-question-json-authoring-heading";
  let authorshipInput: HTMLTextAreaElement | undefined;
  let headingFocusDelivered = false;

  function focusHeading(): void {
    const heading = document.getElementById(headingId) as HTMLHeadingElement | null;
    if (heading === null) return;
    heading.tabIndex = -1;
    heading.focus();
  }

  // The draft accessor is the sole render-time source. Each reducer transition updates this
  // draft editor state together with workflow state, so a rendered response editor never captures a stale
  // Question Response Format branch.
  const [source, setSource] = createSignal<PleQuestionJsonDocument | null>(null);
  const [metadata, setMetadata] = createSignal<PleQuestionJsonRecordMetadata>(
    props.initialGeneralFeedback.metadata,
  );
  const [savedMetadata, setSavedMetadata] = createSignal<PleQuestionJsonRecordMetadata>(
    props.initialGeneralFeedback.metadata,
  );
  const [autosaveState, setAutosaveState] = createSignal<QuestionDraftAutosaveState>({
    status: "saved",
    generation: 0,
    acknowledgedGeneration: 0,
  });
  let lastAcknowledgedSource = props.initial.source;
  const autosave = createQuestionDraftAutosave<DraftSnapshot>({
    save: persistDraftSnapshot,
    debounceMs: 500,
    onStateChange: autosaveStatusChanged,
  });
  const [metadataSaving, setMetadataSaving] = createSignal(false);
  // Numeric source values are numbers. This local literal is intentionally separate so partially
  // typed values such as "6.02e" remain visible without replacing the last valid source value.
  const [numericAnswerLiteral, setNumericAnswerLiteral] = createSignal("0");
  let displayedResponseKind: PleQuestionJsonDocument["response"]["kind"] | null = null;
  function currentSource(): PleQuestionJsonDocument {
    const draft = source();
    if (draft === null) throw new Error("The private draft is unavailable.");
    return draft;
  }
  function draftSnapshot(nextSource = currentSource()): DraftSnapshot {
    return {
      source: nextSource,
      metadata: metadata(),
      generalFeedback: generalFeedback(),
      hint: hint(),
      workedSolution: workedSolution(),
    };
  }
  function queueDraftSnapshot(nextSource = currentSource()): void {
    autosave.edit(draftSnapshot(nextSource));
  }
  function autosaveStatusChanged(next: QuestionDraftAutosaveState): void {
    setAutosaveState(next);
    if (next.status === "unsaved") setStatus("Draft changes are waiting to save...");
    else if (next.status === "saving") setStatus("Saving private draft...");
    else if (next.status === "saved" && next.generation > 0) {
      const unfinishedInput =
        numericLiteralError() !== undefined || hotspotPending() || !hotspotLiteralsValid();
      if (unfinishedInput) {
        setStatus("The unfinished field edit is still unsaved.");
      } else if (assetUploading()) {
        setStatus("The image upload is still in progress.");
      } else {
        setStatus("Private draft saved. It is not published.");
      }
    } else if (next.status === "error") {
      if (
        next.error instanceof PleQuestionJsonStaleConflictError ||
        next.error instanceof PleQuestionGeneralFeedbackConflictError
      ) {
        setShowInstructorCheck(false);
        transition({ kind: "saveConflict" });
        setStatus("A newer draft exists. Your local edits are still shown below.");
      } else {
        setStatus(authorSafeMessage(next.error, "The private draft could not be saved."));
      }
    }
  }
  async function persistDraftSnapshot(snapshot: DraftSnapshot): Promise<void> {
    if (JSON.stringify(snapshot.source) !== JSON.stringify(lastAcknowledgedSource)) {
      const result = await props.repository.save(props.draftQuestion, snapshot.source);
      setLatestDraftQuestionEditNumber(result.draftQuestionEditNumber);
      lastAcknowledgedSource = snapshot.source;
      transition({ kind: "autosaveAcknowledged", source: snapshot.source });
    }

    const metadataChanged = JSON.stringify(snapshot.metadata) !== JSON.stringify(savedMetadata());
    const supportChanged =
      snapshot.generalFeedback !== savedGeneralFeedback() ||
      snapshot.hint !== savedHint() ||
      snapshot.workedSolution !== savedWorkedSolution();
    if (metadataChanged || supportChanged) {
      const result = await props.generalFeedbackClient.save(
        props.draftQuestion,
        snapshot.metadata,
        {
          generalFeedback: snapshot.generalFeedback,
          hint: snapshot.hint,
          workedSolution: snapshot.workedSolution,
        },
        latestDraftQuestionEditNumber(),
      );
      setLatestDraftQuestionEditNumber(result.draftQuestionEditNumber);
      props.repository.synchronizeDraftQuestionEditNumber(
        props.draftQuestion,
        result.draftQuestionEditNumber,
      );
      setSavedMetadata(snapshot.metadata);
      setSavedGeneralFeedback(snapshot.generalFeedback);
      setSavedHint(snapshot.hint);
      setSavedWorkedSolution(snapshot.workedSolution);
      publishQuestionTitle(snapshot.metadata.questionTitle);
    }
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
      if (
        (action.kind === "loaded" || action.kind === "reloadSucceeded") &&
        nextSource?.response.kind === "numeric"
      ) {
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
      metadataSaving() ||
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
      !hotspotPending() &&
      hotspotLiteralsValid() &&
      !assetUploading()
    );
  };
  const hasUnsavedMetadata = (): boolean =>
    JSON.stringify(metadata()) !== JSON.stringify(savedMetadata());
  const isSavedDraft = (): boolean => {
    const current = state();
    return (
      ((current.kind === "ready" && current.status === "clean") ||
        current.kind === "publishReview" ||
        current.kind === "publishing") &&
      numericLiteralError() === undefined &&
      !hotspotPending() &&
      hotspotLiteralsValid() &&
      !assetUploading() &&
      autosaveState().status === "saved" &&
      !hasUnsavedGeneralFeedback() &&
      !hasUnsavedMetadata()
    );
  };
  const isSaved = (): boolean => state().kind === "published" || isSavedDraft();
  async function saveBeforeLeaving(): Promise<boolean> {
    try {
      await autosave.flush();
      return isSaved();
    } catch {
      return false;
    }
  }
  const hasUnsavedGeneralFeedback = (): boolean =>
    generalFeedback() !== savedGeneralFeedback() ||
    hint() !== savedHint() ||
    workedSolution() !== savedWorkedSolution();
  const canEditGeneralFeedback = (): boolean => {
    const current = state();
    return current.kind === "ready" && current.status === "clean" && !isLocked();
  };

  createEffect(() => {
    props.onDraftDisplayStateChange?.({
      draftQuestionEditNumber: latestDraftQuestionEditNumber(),
      dirty:
        hasLocalDraftChanges(state()) ||
        hasUnsavedMetadata() ||
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
      if (document.getElementById(headingId) !== null) {
        focusHeading();
        props.onHeadingFocusDelivered?.();
      }
    });
  });

  onMount(() => {
    publication = routeScopePublication();
    transition({ kind: "loaded", source: props.initial.source });
    publishQuestionTitle(metadata().questionTitle);
  });
  onCleanup(() => {
    autosave.dispose();
    if (publication !== undefined) clearRouteScopeLabels(publication);
  });

  function applyEdit(next: PleQuestionJsonDocument): void {
    if (isLocked()) return;
    setShowInstructorCheck(false);
    setReview(null);
    setStatus(null);
    transition({ kind: "edit", source: next });
    queueDraftSnapshot(next);
  }

  function applyGeneralFeedbackEdit(value: Parameters<Setter<string | null>>[0]): string | null {
    const next = typeof value === "function" ? value(generalFeedback()) : value;
    setGeneralFeedback(next);
    queueDraftSnapshot();
    return next;
  }

  function applyHintEdit(value: Parameters<Setter<string | null>>[0]): string | null {
    const next = typeof value === "function" ? value(hint()) : value;
    setHint(next);
    queueDraftSnapshot();
    return next;
  }

  function applyWorkedSolutionEdit(value: Parameters<Setter<string | null>>[0]): string | null {
    const next = typeof value === "function" ? value(workedSolution()) : value;
    setWorkedSolution(next);
    queueDraftSnapshot();
    return next;
  }

  function applyMetadataEdit(next: PleQuestionJsonRecordMetadata): void {
    if (isLocked()) return;
    setMetadata(next);
    queueDraftSnapshot();
    setReview(null);
    setShowInstructorCheck(false);
    if (state().kind === "publishReview") transition({ kind: "edit", source: currentSource() });
  }

  function updateNumericAnswerLiteral(literal: string): void {
    setNumericAnswerLiteral(literal);
    const current = source();
    if (current === null || current.response.kind !== "numeric") return;
    const answer = parseNumericLiteral(literal);
    if (answer === null) {
      queueDraftSnapshot(current);
      return;
    }
    applyEdit({ ...current, response: { ...current.response, answer } });
  }

  async function uploadQuestionImage(file: File, signal: AbortSignal): Promise<void> {
    const client = props.questionImageClient;
    if (client === undefined || isLocked()) {
      setStatus("Image upload is unavailable. Your draft is unchanged.");
      return;
    }
    try {
      await autosave.flush();
      setAssetUploading(true);
      setStatus("Uploading private image...");
      const asset = await client.uploadQuestionImage(
        props.draftQuestion,
        file,
        latestDraftQuestionEditNumber(),
        signal,
      );
      if (signal.aborted) {
        setStatus("Upload canceled. Your previous draft is unchanged.");
        return;
      }
      setReview(null);
      setShowInstructorCheck(false);
      transition({ kind: "edit", source: setPleQuestionJsonHotspotImage(currentSource(), asset) });
      queueDraftSnapshot();
      setStatus("Image uploaded. Your Draft changes will save automatically.");
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
    if (current === null || isLocked()) return;
    try {
      await autosave.flush();
    } catch (error: unknown) {
      setStatus(authorSafeMessage(error, "The private draft could not be saved."));
    }
  }

  async function reload(preserveLocal = false): Promise<void> {
    if (!isConflict()) return;
    const local = source();
    const localMetadata = metadata();
    const localGeneralFeedback = generalFeedback();
    const localHint = hint();
    const localWorkedSolution = workedSolution();
    transition({ kind: "reloadStarted" });
    setStatus("Loading the newest private draft...");
    try {
      const [newest, newestGeneralFeedback] = await Promise.all([
        props.repository.reload(props.draftQuestion),
        props.generalFeedbackClient.load(props.draftQuestion),
      ]);
      setLatestDraftQuestionEditNumber(newest.draftQuestionEditNumber);
      setMetadata(newestGeneralFeedback.metadata);
      setSavedMetadata(newestGeneralFeedback.metadata);
      setGeneralFeedback(newestGeneralFeedback.generalFeedback);
      setSavedGeneralFeedback(newestGeneralFeedback.generalFeedback);
      setHint(newestGeneralFeedback.hint);
      setSavedHint(newestGeneralFeedback.hint);
      setWorkedSolution(newestGeneralFeedback.workedSolution);
      setSavedWorkedSolution(newestGeneralFeedback.workedSolution);
      setReview(null);
      setShowInstructorCheck(false);
      lastAcknowledgedSource = newest.source;
      transition({ kind: "reloadSucceeded", source: newest.source });
      publishQuestionTitle(newestGeneralFeedback.metadata.questionTitle);
      if (preserveLocal && local !== null) transition({ kind: "edit", source: local });
      if (preserveLocal) {
        setMetadata(localMetadata);
        setGeneralFeedback(localGeneralFeedback);
        setHint(localHint);
        setWorkedSolution(localWorkedSolution);
        publishQuestionTitle(localMetadata.questionTitle);
        queueDraftSnapshot(local ?? newest.source);
      } else {
        autosave.reset(draftSnapshot(newest.source));
      }
      if (!preserveLocal) {
        setHotspotPending(false);
        setHotspotLiteralsValid(true);
      }
      setStatus(
        preserveLocal
          ? "Your local edits are restored over the newest saved draft. Review and save them before publishing."
          : "Loaded the newest saved draft. Review it before editing.",
      );
      queueMicrotask(focusHeading);
    } catch (error: unknown) {
      if (publication !== undefined) clearRouteScopeLabels(publication);
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
    if (isLocked() || !hasUnsavedGeneralFeedback()) return;
    setGeneralFeedbackSaving(true);
    try {
      await autosave.flush();
    } catch (error: unknown) {
      setStatus(authorSafeMessage(error, "General feedback could not be saved."));
    } finally {
      setGeneralFeedbackSaving(false);
    }
  }

  async function saveQuestionMetadata(): Promise<void> {
    if (isLocked() || metadataSaving() || !hasUnsavedMetadata()) return;
    setMetadataSaving(true);
    try {
      await autosave.flush();
      setReview(null);
      if (state().kind === "publishReview") transition({ kind: "edit", source: currentSource() });
    } catch (error: unknown) {
      setStatus(authorSafeMessage(error, "Question metadata could not be saved."));
    } finally {
      setMetadataSaving(false);
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
    if (!validatePleQuestionJsonSource(currentSource()).valid) {
      setStatus("Complete the highlighted Question details before reviewing publication.");
      return;
    }
    const nextReview: Review = {
      draftQuestionEditNumber: latestDraftQuestionEditNumber(),
      baseQuestion: "newQuestion",
      questionTitle: metadata().questionTitle,
      changed: ["PLE Question JSON source", "Question record metadata"],
    };
    setReview(nextReview);
    transition({ kind: "reviewOpened", review: "Publication review is ready." });
    setStatus(null);
  }

  async function publish(): Promise<void> {
    if (state().kind !== "publishReview" || !isSaved() || isLocked()) return;
    const activeReview = review();
    if (
      activeReview === null ||
      activeReview.draftQuestionEditNumber !== latestDraftQuestionEditNumber()
    ) {
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
        libraryPath: `/library/${encodeURIComponent(summary.questionId)}`,
      });
      setStatus("Publication complete. Open the published Question to inspect it.");
      requestAnimationFrame(focusHeading);
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
    <div
      class="ple-question-json-authoring"
      inert={props.replacementPending === true}
      aria-busy={props.replacementPending === true}
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
      <PageFrame
        eyebrow={
          publishedQuestionId(state()) ? "Publication complete" : "Private instructor authoring"
        }
        title={publishedQuestionId(state()) ? "Question published" : "PLE Question JSON"}
        lede={
          publishedQuestionId(state())
            ? "Your authoring work is complete. This confirmation identifies the Question now available in the Question Library."
            : "Build a clear student question, save it privately, then review and publish it when it is ready."
        }
        routeSurface="pleQuestionJsonEditor"
        headingId={headingId}
      >
        <Show when={status()}>
          {(message) => (
            <p
              role="status"
              aria-label={
                publishedQuestionId(state()) ? "Publication status" : "Private draft status"
              }
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
          metadata={metadata}
          metadataDirty={hasUnsavedMetadata}
          metadataSaving={metadataSaving}
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
          hint={hint}
          workedSolution={workedSolution}
          generalFeedbackSaving={generalFeedbackSaving}
          showInstructorCheck={showInstructorCheck}
          review={review}
          authorshipText={authorshipText}
          disciplineUuid={disciplineUuid}
          subjectUuid={subjectUuid}
          topicUuid={topicUuid}
          subtopicUuid={subtopicUuid}
          publishedQuestionId={() => publishedQuestionId(state())}
          publishedSummary={publishedSummary}
          hotspotDraftQuestionImage={hotspotDraftQuestionImage}
          instructorAnswerCheck={(draft) => answerCheck(draft) ?? undefined}
          classificationClient={props.classificationClient}
          responseValidator={props.responseValidator}
          draftPreviewPanel={
            <DraftPreviewPanel
              draftQuestion={props.draftQuestion}
              draftQuestionEditNumber={latestDraftQuestionEditNumber()}
              binding={{
                backend: "ple",
                format: "pleQuestionJson",
                mediaType: "application/vnd.peptidyle.question+json",
                webworkPgPath: null,
              }}
              isSaved={isSaved()}
              responseValidator={props.responseValidator}
              hotspotDraftQuestionImage={hotspotDraftQuestionImage()}
            />
          }
          questionImagePreviewPath={(asset) =>
            asset === ""
              ? ""
              : (props.questionImageClient?.questionImagePreviewPath(props.draftQuestion, asset) ??
                "")
          }
          onEdit={applyEdit}
          onMetadataChange={applyMetadataEdit}
          onSaveMetadata={() => void saveQuestionMetadata()}
          onNumericAnswerLiteralChange={updateNumericAnswerLiteral}
          onMoveChoice={moveChoice}
          onStatus={setStatus}
          onHotspotPendingChange={setHotspotPending}
          onHotspotLiteralValidityChange={setHotspotLiteralsValid}
          onUpload={uploadQuestionImage}
          onGeneralFeedbackChange={applyGeneralFeedbackEdit}
          onHintChange={applyHintEdit}
          onWorkedSolutionChange={applyWorkedSolutionEdit}
          onSaveGeneralFeedback={() => void saveGeneralFeedback()}
          onSave={() => void save()}
          onInspectInstructorAnswer={inspectInstructorAnswer}
          onOpenPublishReview={openPublishReview}
          onAuthorshipTextChange={(value) => {
            setAuthorshipEdited(true);
            setAuthorshipText(value);
          }}
          onAuthorshipInput={(element) => {
            authorshipInput = element;
          }}
          onDisciplineChange={setDisciplineUuid}
          onSubjectChange={setSubjectUuid}
          onTopicChange={setTopicUuid}
          onSubtopicChange={setSubtopicUuid}
          onPublish={() => void publish()}
        />
        <Show when={props.correctionParent}>
          {(parent) => (
            <DraftCorrectionPublishControl
              draftQuestion={props.draftQuestion}
              parent={parent()}
              draftQuestionEditNumber={latestDraftQuestionEditNumber()}
              isSaved={isSavedDraft()}
            />
          )}
        </Show>
      </PageFrame>
    </div>
  );
}
