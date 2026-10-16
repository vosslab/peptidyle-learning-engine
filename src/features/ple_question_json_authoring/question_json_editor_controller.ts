// Native Question draft workflow: editing, autosave, recovery, uploads, and publication.

import {
  batch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type Setter,
} from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { parseReviewedQuestionAuthorship } from "../../api/question_authorship";
import {
  initialPleQuestionJsonEditorState,
  reducePleQuestionJsonEditor,
  reorderChoices,
  sourcesEqual,
  validatePleQuestionJsonSource,
  type PleQuestionJsonEditorAction,
  type PleQuestionJsonEditorState,
} from "./question_json_editor_model";
import { parseNumericLiteral } from "./question_json_numeric_model";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import type { PleQuestionJsonPublishReview } from "./question_json_editor_workspace";
import { PleQuestionJsonStaleConflictError } from "./question_json_repository";
import { PleQuestionJsonConflictError } from "./question_json_client";
import {
  createQuestionDraftAutosave,
  type QuestionDraftAutosaveState,
} from "./question_draft_autosave";
import { setPleQuestionJsonHotspotImage } from "./question_json_hotspot_model";
import { PleQuestionGeneralFeedbackConflictError } from "./question_general_feedback_client";
import type { PleQuestionJsonDocument } from "./question_json_source";
import type { PleQuestionJsonRecordMetadata } from "./question_json_defaults";
import { questionJsonAnswerCheck } from "./question_json_answer_check";
import {
  isPublicationReviewDirty,
  type PublicationReviewSnapshot,
} from "./question_publication_review_model";

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

function fieldErrors(source: PleQuestionJsonDocument | null): Readonly<Record<string, string>> {
  if (source === null) return {};
  const validation = validatePleQuestionJsonSource(source);
  return Object.fromEntries(validation.issues.map((issue) => [issue.field, issue.message]));
}

function hasLocalDraftChanges(state: PleQuestionJsonEditorState): boolean {
  if (state.kind === "ready") return state.status !== "clean";
  return ["loading", "conflict", "reloading", "error", "published"].includes(state.kind);
}

type QuestionJsonEditorPresentation = {
  readonly publishQuestionTitle: (title: string) => void;
  readonly clearQuestionTitle: () => void;
  readonly focusHeading: () => void;
  readonly focusAuthorship: () => void;
};

type QuestionJsonEditorController = {
  readonly source: Accessor<PleQuestionJsonDocument | null>;
  readonly metadata: Accessor<PleQuestionJsonRecordMetadata>;
  readonly hasUnsavedMetadata: Accessor<boolean>;
  readonly metadataSaving: Accessor<boolean>;
  readonly currentSource: () => PleQuestionJsonDocument;
  readonly errors: () => Readonly<Record<string, string>>;
  readonly isLocked: Accessor<boolean>;
  readonly isConflict: Accessor<boolean>;
  readonly canSave: Accessor<boolean>;
  readonly isSaved: Accessor<boolean>;
  readonly isSavedDraft: Accessor<boolean>;
  readonly canEditGeneralFeedback: Accessor<boolean>;
  readonly hasUnsavedGeneralFeedback: Accessor<boolean>;
  readonly numericAnswerLiteral: Accessor<string>;
  readonly hotspotPending: Accessor<boolean>;
  readonly generalFeedback: Accessor<string | null>;
  readonly hint: Accessor<string | null>;
  readonly workedSolution: Accessor<string | null>;
  readonly generalFeedbackSaving: Accessor<boolean>;
  readonly showInstructorCheck: Accessor<boolean>;
  readonly review: Accessor<Review | null>;
  readonly authorshipText: Accessor<string>;
  readonly disciplineUuid: Accessor<string | null>;
  readonly subjectUuid: Accessor<string | null>;
  readonly topicUuid: Accessor<string | null>;
  readonly subtopicUuid: Accessor<string | null>;
  readonly publishedSummary: Accessor<QuestionSummary | undefined>;
  readonly latestDraftQuestionEditNumber: Accessor<string>;
  readonly status: Accessor<string | null>;
  readonly errorMessage: Accessor<string | null>;
  readonly publishedQuestionId: Accessor<string | null>;
  readonly saveBeforeLeaving: () => Promise<boolean>;
  readonly dismissError: () => void;
  readonly reload: (preserveLocal?: boolean) => Promise<void>;
  readonly applyEdit: (source: PleQuestionJsonDocument) => void;
  readonly applyMetadataEdit: (metadata: PleQuestionJsonRecordMetadata) => void;
  readonly saveQuestionMetadata: () => Promise<void>;
  readonly updateNumericAnswerLiteral: (literal: string) => void;
  readonly moveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly setStatus: Setter<string | null>;
  readonly setHotspotPending: Setter<boolean>;
  readonly setHotspotLiteralsValid: Setter<boolean>;
  readonly uploadQuestionImage: (file: File, signal: AbortSignal) => Promise<void>;
  readonly applyGeneralFeedbackEdit: Setter<string | null>;
  readonly applyHintEdit: Setter<string | null>;
  readonly applyWorkedSolutionEdit: Setter<string | null>;
  readonly saveGeneralFeedback: () => Promise<void>;
  readonly save: () => Promise<void>;
  readonly inspectInstructorAnswer: () => void;
  readonly openPublishReview: () => void;
  readonly applyAuthorshipTextEdit: Setter<string>;
  readonly selfAuthorPosition: Accessor<number | null>;
  readonly setSelfAuthorPosition: (position: number | null) => void;
  readonly setDisciplineUuid: (uuid: string | null) => void;
  readonly setSubjectUuid: (uuid: string | null) => void;
  readonly setTopicUuid: (uuid: string | null) => void;
  readonly setSubtopicUuid: (uuid: string | null) => void;
  readonly reviewDirty: Accessor<boolean>;
  readonly discardReviewChanges: () => void;
  readonly publish: () => Promise<void>;
};

export function createQuestionJsonEditorController(
  props: PleQuestionJsonEditorPageProps,
  presentation: QuestionJsonEditorPresentation,
): QuestionJsonEditorController {
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
  const [reviewEdited, setReviewEdited] = createSignal(false);
  const [selfAuthorPosition, setSelfAuthorPosition] = createSignal<number | null>(null);
  createEffect(() => {
    if (!reviewEdited()) {
      setSelfAuthorPosition(null);
      setAuthorshipText(props.initialGeneralFeedback.authors.join("\n"));
    }
  });
  const [disciplineUuid, setDisciplineUuid] = createSignal<string | null>(null);
  const [subjectUuid, setSubjectUuid] = createSignal<string | null>(null);
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [reviewBaseline, setReviewBaseline] = createSignal<PublicationReviewSnapshot>({
    authorshipText: props.initialGeneralFeedback.authors.join("\n"),
    selfAuthorPosition: null,
    disciplineUuid: null,
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
  });
  createEffect(() => {
    const authorshipText = props.initialGeneralFeedback.authors.join("\n");
    if (!reviewEdited() && reviewBaseline().authorshipText !== authorshipText)
      setReviewBaseline({ ...reviewBaseline(), authorshipText });
  });
  const [publishedSummary, setPublishedSummary] = createSignal<QuestionSummary>();
  const reviewSnapshot = (): PublicationReviewSnapshot => ({
    authorshipText: authorshipText(),
    selfAuthorPosition: selfAuthorPosition(),
    disciplineUuid: disciplineUuid(),
    subjectUuid: subjectUuid(),
    topicUuid: topicUuid(),
    subtopicUuid: subtopicUuid(),
  });
  const reviewDirty = (): boolean =>
    publishedSummary() === undefined &&
    isPublicationReviewDirty(reviewSnapshot(), reviewBaseline());
  function discardReviewChanges(): void {
    const baseline = reviewBaseline();
    batch(() => {
      setAuthorshipText(baseline.authorshipText);
      setSelfAuthorPosition(baseline.selfAuthorPosition);
      setDisciplineUuid(baseline.disciplineUuid);
      setSubjectUuid(baseline.subjectUuid);
      setTopicUuid(baseline.topicUuid);
      setSubtopicUuid(baseline.subtopicUuid);
      setReviewEdited(false);
    });
  }
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
      presentation.publishQuestionTitle(snapshot.metadata.questionTitle);
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
        current.kind === "publishing" ||
        (current.kind === "error" &&
          current.resume === "publishReview" &&
          current.source !== null &&
          current.savedSource !== null &&
          sourcesEqual(current.source, current.savedSource))) &&
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

  onMount(() => {
    transition({ kind: "loaded", source: props.initial.source });
    presentation.publishQuestionTitle(metadata().questionTitle);
  });
  onCleanup(() => {
    autosave.dispose();
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
      presentation.publishQuestionTitle(newestGeneralFeedback.metadata.questionTitle);
      if (preserveLocal && local !== null) transition({ kind: "edit", source: local });
      if (preserveLocal) {
        setMetadata(localMetadata);
        setGeneralFeedback(localGeneralFeedback);
        setHint(localHint);
        setWorkedSolution(localWorkedSolution);
        presentation.publishQuestionTitle(localMetadata.questionTitle);
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
      queueMicrotask(presentation.focusHeading);
    } catch (error: unknown) {
      presentation.clearQuestionTitle();
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
    const check = questionJsonAnswerCheck(current);
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
      requestAnimationFrame(presentation.focusAuthorship);
      return;
    }
    const selectedPosition = props.allowSelfAuthorship ? selfAuthorPosition() : null;
    transition({ kind: "publishStarted" });
    setStatus("Publishing a new Question ID...");
    try {
      const discipline = disciplineUuid();
      const subject = subjectUuid();
      if (discipline === null || subject === null)
        throw new Error("Select a Discipline and Subject before publishing.");
      const summary = await props.repository.publish(props.draftQuestion, {
        authorship,
        selfAuthorPosition: selectedPosition,
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
      requestAnimationFrame(presentation.focusHeading);
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

  function applyAuthorshipTextEdit(value: Parameters<Setter<string>>[0]): string {
    setReviewEdited(true);
    setSelfAuthorPosition(null);
    return setAuthorshipText(value);
  }

  function changeReviewValue<T>(current: Accessor<T>, set: Setter<T>, value: T): void {
    if (current() !== value) setReviewEdited(true);
    set(value as Parameters<Setter<T>>[0]);
  }

  const errorMessage = (): string | null => {
    const current = state();
    return current.kind === "error" ? current.message : null;
  };
  const publishedQuestionId = (): string | null => {
    const current = state();
    return current.kind === "published" ? current.libraryPath : null;
  };

  return {
    source,
    metadata,
    hasUnsavedMetadata,
    metadataSaving,
    currentSource,
    errors,
    isLocked,
    isConflict,
    canSave,
    isSaved,
    isSavedDraft,
    canEditGeneralFeedback,
    hasUnsavedGeneralFeedback,
    numericAnswerLiteral,
    hotspotPending,
    generalFeedback,
    hint,
    workedSolution,
    generalFeedbackSaving,
    showInstructorCheck,
    review,
    authorshipText,
    reviewDirty,
    discardReviewChanges,
    disciplineUuid,
    subjectUuid,
    topicUuid,
    subtopicUuid,
    publishedSummary,
    latestDraftQuestionEditNumber,
    status,
    errorMessage,
    publishedQuestionId,
    saveBeforeLeaving,
    dismissError,
    reload,
    applyEdit,
    applyMetadataEdit,
    saveQuestionMetadata,
    updateNumericAnswerLiteral,
    moveChoice,
    setStatus,
    setHotspotPending,
    setHotspotLiteralsValid,
    uploadQuestionImage,
    applyGeneralFeedbackEdit,
    applyHintEdit,
    applyWorkedSolutionEdit,
    saveGeneralFeedback,
    save,
    inspectInstructorAnswer,
    openPublishReview,
    applyAuthorshipTextEdit,
    selfAuthorPosition,
    setSelfAuthorPosition: (position) =>
      changeReviewValue(selfAuthorPosition, setSelfAuthorPosition, position),
    setDisciplineUuid: (uuid) => changeReviewValue(disciplineUuid, setDisciplineUuid, uuid),
    setSubjectUuid: (uuid) => changeReviewValue(subjectUuid, setSubjectUuid, uuid),
    setTopicUuid: (uuid) => changeReviewValue(topicUuid, setTopicUuid, uuid),
    setSubtopicUuid: (uuid) => changeReviewValue(subtopicUuid, setSubtopicUuid, uuid),
    publish,
  };
}
