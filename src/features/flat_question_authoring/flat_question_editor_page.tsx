// flat_question_editor_page.tsx - private instructor surface for flat-question authoring.

import { For, Show, batch, createEffect, createSignal, onMount, type JSX } from "solid-js";

import type { PublicationScope } from "../../../generated/api/PublicationScope";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import { parseReviewedPublicByline } from "../../api/public_byline";
import { FlatChoiceList } from "./flat_choice_list";
import { FlatFeedbackFields } from "./flat_feedback_fields";
import { hotspotSourceFromAsset } from "./flat_hotspot_editor_model";
import { FlatQuestionAdvancedResponseFields } from "./flat_question_advanced_response_fields";
import {
  addChoice,
  addMatchingPair,
  initialFlatQuestionEditorState,
  reduceFlatQuestionEditor,
  removeChoice,
  removeMatchingPair,
  reorderMatchingItems,
  reorderChoices,
  setAttemptPolicy,
  setChoiceFeedback,
  setChoiceText,
  setCorrectChoice,
  setFlatQuestionPoints,
  setFlatQuestionPrompt,
  setFlatQuestionTitle,
  setLanguage,
  setMatchingItemText,
  setMatchingPair,
  setLicense,
  setOutcomeFeedback,
  setTags,
  setTaxonomy,
  setTimingPolicy,
  setFlatQuestionResponseKind,
  validateFlatQuestionSource,
  type FlatQuestionEditorAction,
  type FlatQuestionEditorState,
} from "./flat_question_editor_model";
import { FLAT_QUESTION_EDITOR_STYLES } from "./flat_question_editor_styles";
import { FlatMetadataFields } from "./flat_metadata_fields";
import { FlatMatchingEditor } from "./flat_matching_editor";
import { parseNumericLiteral } from "./flat_numeric_model";
import { FlatPolicyFields } from "./flat_policy_fields";
import { flatQuestionPublicPreview } from "./flat_question_public_preview";
import { FlatQuestionPreview } from "./flat_question_preview";
import type { FlatQuestionEditorPageProps } from "./flat_question_editor_types";
import { FlatQuestionStaleConflictError } from "./flat_question_repository";
import type {
  FlatQuestionAssetClient,
  FlatQuestionAssetDescriptor,
} from "./flat_question_asset_client";
import type { FlatQuestionHotspotResponse, FlatQuestionSourceV2 } from "./flat_question_source";
import type { FlatQuestionInstructorAnswerCheck } from "./flat_question_preview";

export type {
  FlatQuestionDraftDisplayState,
  FlatQuestionEditorPageProps,
} from "./flat_question_editor_types";

type Review = {
  readonly revision: string;
  readonly baseline: "newQuestion";
  readonly title: string;
  readonly changed: ReadonlyArray<string>;
};

function authorSafeMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0 && error.message.length < 240) {
    return error.message;
  }
  return fallback;
}

function sourceFrom(state: FlatQuestionEditorState): FlatQuestionSourceV2 | null {
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

function fieldErrors(source: FlatQuestionSourceV2 | null): Readonly<Record<string, string>> {
  if (source === null) return {};
  const validation = validateFlatQuestionSource(source);
  return Object.fromEntries(validation.issues.map((issue) => [issue.field, issue.message]));
}

function answerCheck(source: FlatQuestionSourceV2): FlatQuestionInstructorAnswerCheck | null {
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

function singleChoiceResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "singleChoice" }> | null {
  return source.response.kind === "singleChoice" ? source.response : null;
}

function matchingResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "matching" }> | null {
  return source.response.kind === "matching" ? source.response : null;
}

function isEditableResponseKind(
  value: string,
): value is Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot"> {
  return (
    value === "singleChoice" ||
    value === "multipleAnswer" ||
    value === "fillIn" ||
    value === "multiFillIn" ||
    value === "numeric" ||
    value === "matching" ||
    value === "ordering"
  );
}

function FlatQuestionResponseFields(props: {
  readonly source: () => FlatQuestionSourceV2;
  readonly fieldErrors: Readonly<Record<string, string>>;
  readonly disabled: boolean;
  readonly numericAnswerLiteral: () => string;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onEdit: (source: FlatQuestionSourceV2) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly onStatus: (message: string) => void;
  readonly selectedKind: () => FlatQuestionSourceV2["response"]["kind"];
  readonly hotspotResponse: () => FlatQuestionHotspotResponse | null;
  readonly pendingHotspotDescription: () => string;
  readonly assetClient: FlatQuestionAssetClient | undefined;
  readonly workspace: WorkspaceId;
  readonly onChooseHotspot: () => void;
  readonly onSelectHotspotAsset: (asset: FlatQuestionAssetDescriptor) => void;
  readonly onPendingHotspotDescriptionChange: (description: string) => void;
  readonly onChooseOrdinaryFormat: () => void;
}): JSX.Element {
  const responseKind = props.selectedKind;
  function chooseFormat(kind: Exclude<FlatQuestionSourceV2["response"]["kind"], "hotspot">): void {
    props.onEdit(setFlatQuestionResponseKind(props.source(), kind));
  }
  return (
    <>
      <label class="flat-question-authoring__field">
        <span>Question format</span>
        <select
          value={responseKind()}
          disabled={props.disabled}
          onChange={(event) => {
            const kind = event.currentTarget.value;
            if (isEditableResponseKind(kind)) {
              props.onChooseOrdinaryFormat();
              chooseFormat(kind);
            } else if (kind === "hotspot") props.onChooseHotspot();
          }}
        >
          <option value="singleChoice">Multiple choice (one answer)</option>
          <option value="multipleAnswer">Multiple answer (select all)</option>
          <option value="fillIn">Fill in the blank</option>
          <option value="multiFillIn">Multiple fill in the blank</option>
          <option value="numeric">Numerical entry</option>
          <option value="matching">Matching pairs</option>
          <option value="ordering">Ordered list</option>
          <option value="hotspot">Image hotspot</option>
        </select>
        <span class="flat-question-authoring__help">
          Choose the learner task first. Changing the format starts a valid private draft for that
          format. Image hotspot starts with a verified image and learner-facing description.
        </span>
      </label>
      <Show when={responseKind() === "singleChoice"}>
        {(_isSingleChoice) => (
          <FlatChoiceList
            choices={singleChoiceResponse(props.source())?.choices ?? []}
            correctChoice={singleChoiceResponse(props.source())?.correctChoice ?? ""}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onChoiceChange={(id, patch) => {
              const next =
                patch.text === undefined
                  ? setChoiceFeedback(props.source(), id, patch.feedback ?? null)
                  : setChoiceText(props.source(), id, patch.text);
              if (next.changed) props.onEdit(next.source);
            }}
            onCorrectChoiceChange={(id) => {
              const next = setCorrectChoice(props.source(), id);
              if (next.changed) props.onEdit(next.source);
            }}
            onAddChoice={() => {
              const next = addChoice(props.source());
              if (next.changed) props.onEdit(next.source);
            }}
            onRemoveChoice={(id) => {
              const next = removeChoice(props.source(), id);
              if (next.changed) props.onEdit(next.source);
            }}
            onMoveChoice={props.onMoveChoice}
          />
        )}
      </Show>
      <Show when={responseKind() === "matching"}>
        {(_isMatching) => (
          <FlatMatchingEditor
            prompts={matchingResponse(props.source())?.prompts ?? []}
            choices={matchingResponse(props.source())?.choices ?? []}
            matches={matchingResponse(props.source())?.matches ?? []}
            fieldErrors={props.fieldErrors}
            disabled={props.disabled}
            onPromptTextChange={(id, text) => {
              const next = setMatchingItemText(props.source(), "prompts", id, text);
              if (next.changed) props.onEdit(next.source);
            }}
            onChoiceTextChange={(id, text) => {
              const next = setMatchingItemText(props.source(), "choices", id, text);
              if (next.changed) props.onEdit(next.source);
            }}
            onMatchChange={(prompt, choice) => {
              const next = setMatchingPair(props.source(), prompt, choice);
              if (next.changed) props.onEdit(next.source);
            }}
            onAddPair={() => {
              const next = addMatchingPair(props.source());
              if (next.changed) props.onEdit(next.source);
            }}
            onRemovePair={(prompt) => {
              const next = removeMatchingPair(props.source(), prompt);
              if (next.changed) props.onEdit(next.source);
              else if (next.error !== null) props.onStatus(next.error);
            }}
            onMoveItem={(side, id, direction) => {
              const response = matchingResponse(props.source());
              if (response === null) return;
              const items = response[side];
              const index = items.findIndex((item) => item.id === id);
              const other = direction === "earlier" ? index - 1 : index + 1;
              if (index < 0 || other < 0 || other >= items.length) return;
              const ordered = items.map((item) => item.id);
              const displaced = ordered[other];
              if (displaced === undefined) return;
              ordered[other] = id;
              ordered[index] = displaced;
              const next = reorderMatchingItems(props.source(), side, ordered);
              if (next.changed) props.onEdit(next.source);
              else if (next.error !== null) props.onStatus(next.error);
            }}
            onStatus={props.onStatus}
          />
        )}
      </Show>
      <FlatQuestionAdvancedResponseFields
        source={props.source}
        fieldErrors={props.fieldErrors}
        disabled={props.disabled}
        numericAnswerLiteral={props.numericAnswerLiteral}
        onNumericAnswerLiteralChange={props.onNumericAnswerLiteralChange}
        onEdit={props.onEdit}
        onStatus={props.onStatus}
        selectedKind={responseKind}
        hotspotResponse={props.hotspotResponse}
        pendingHotspotDescription={props.pendingHotspotDescription}
        assetClient={props.assetClient}
        workspace={props.workspace}
        onSelectHotspotAsset={props.onSelectHotspotAsset}
        onPendingHotspotDescriptionChange={props.onPendingHotspotDescriptionChange}
      />
    </>
  );
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
  const [bylineText, setBylineText] = createSignal("");
  const [status, setStatus] = createSignal<string | null>(null);
  const [showInstructorCheck, setShowInstructorCheck] = createSignal(false);
  let heading: HTMLHeadingElement | null = null;
  let bylineInput: HTMLTextAreaElement | undefined;
  let reviewRequestGeneration = 0;
  let headingFocusDelivered = false;

  // The draft accessor is the sole render-time source. Each reducer transition updates this
  // projection together with workflow state, so a mounted response editor never captures a stale
  // response-family branch.
  const [source, setSource] = createSignal<FlatQuestionSourceV2 | null>(null);
  // Numeric source values are numbers. This local literal is intentionally separate so partially
  // typed values such as "6.02e" remain visible without replacing the last valid source value.
  const [numericAnswerLiteral, setNumericAnswerLiteral] = createSignal("0");
  // HOTSPOT remains local until a verified descriptor and a useful learner description can make
  // a valid persisted source. This prevents a placeholder asset from entering a draft.
  const [hotspotPending, setHotspotPending] = createSignal(false);
  const [pendingHotspotAsset, setPendingHotspotAsset] =
    createSignal<FlatQuestionAssetDescriptor | null>(null);
  const [pendingHotspotDescription, setPendingHotspotDescription] = createSignal("");
  let displayedResponseKind: FlatQuestionSourceV2["response"]["kind"] | null = null;
  function currentSource(): FlatQuestionSourceV2 {
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
  function transition(action: FlatQuestionEditorAction): void {
    const next = reduceFlatQuestionEditor(state(), action);
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

  function applyEdit(next: FlatQuestionSourceV2): void {
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

  function hotspotResponse(): FlatQuestionHotspotResponse | null {
    const current = source();
    return current?.response.kind === "hotspot" ? current.response : null;
  }

  function selectHotspotAsset(asset: FlatQuestionAssetDescriptor): void {
    const current = source();
    if (current === null || isLocked()) return;
    setPendingHotspotAsset(asset);
    const description = pendingHotspotDescription();
    if (description.trim() === "") {
      setStatus(
        "Describe the image for learners, then the verified image will become this hotspot draft.",
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
    const validation = validateFlatQuestionSource(current);
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
      if (error instanceof FlatQuestionStaleConflictError) {
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
        baseline: diff.baseline,
        title: diff.current.title,
        changed: diff.changed,
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
    const byline = parseReviewedPublicByline(bylineText());
    if (byline === null) {
      setStatus("Provide one to sixteen distinct reviewed author names before publishing.");
      requestAnimationFrame(() => bylineInput?.focus());
      return;
    }
    transition({ kind: "publishStarted" });
    setStatus("Publishing a new Question ID...");
    try {
      await props.repository.publish(props.workspace, { scope: scope(), byline });
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
      class="page flat-question-authoring"
      data-route-surface="flatQuestionEditor"
      inert={props.replacementPending === true}
      aria-busy={props.replacementPending === true}
    >
      <style>{FLAT_QUESTION_EDITOR_STYLES}</style>
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1 ref={(node) => (heading = node)} tabindex="-1">
          Flat question
        </h1>
        <p>
          Build a clear learner question, save it privately, then review and publish it when it is
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
              <label class="flat-question-authoring__field">
                <span>Question title</span>
                <input
                  value={currentSource().title}
                  disabled={isLocked()}
                  aria-invalid={errors()["title"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setFlatQuestionTitle(currentSource(), event.currentTarget.value))
                  }
                />
              </label>
              <label class="flat-question-authoring__field">
                <span>Learner-facing prompt</span>
                <textarea
                  value={currentSource().prompt}
                  disabled={isLocked()}
                  aria-invalid={errors()["prompt"] !== undefined}
                  onInput={(event) =>
                    applyEdit(setFlatQuestionPrompt(currentSource(), event.currentTarget.value))
                  }
                />
              </label>
              <FlatQuestionResponseFields
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
              <FlatFeedbackFields
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
              <FlatPolicyFields
                points={currentSource().points}
                attemptPolicy={currentSource().attemptPolicy}
                timingPolicy={currentSource().timingPolicy}
                fieldErrors={errors()}
                disabled={isLocked()}
                onPointsChange={(points) =>
                  applyEdit(setFlatQuestionPoints(currentSource(), points))
                }
                onAttemptPolicyChange={(policy) =>
                  applyEdit(setAttemptPolicy(currentSource(), policy))
                }
                onTimingPolicyChange={(policy) =>
                  applyEdit(setTimingPolicy(currentSource(), policy))
                }
              />
              <FlatMetadataFields
                tags={currentSource().tags}
                taxonomy={currentSource().taxonomy}
                license={currentSource().license}
                language={currentSource().language}
                fieldErrors={errors()}
                disabled={isLocked()}
                onTagsChange={(tags) => applyEdit(setTags(currentSource(), tags))}
                onTaxonomyChange={(taxonomy) => applyEdit(setTaxonomy(currentSource(), taxonomy))}
                onLicenseChange={(license) => applyEdit(setLicense(currentSource(), license))}
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
                      Student preview appears after the verified image and learner description form
                      a complete hotspot draft.
                    </p>
                  }
                >
                  <For each={[currentSource()]}>
                    {(draft) => (
                      <FlatQuestionPreview
                        preview={flatQuestionPublicPreview(draft)}
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
              <section class="editor-panel" aria-labelledby="flat-publish-heading">
                <h2 id="flat-publish-heading">Publish review</h2>
                <p>Review the saved content before publishing a new Question ID.</p>
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
                        This publication creates a new Question ID. Existing assignments keep their
                        assigned questions until an instructor deliberately replaces an item.
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
                      <label class="flat-question-authoring__field">
                        <span>Reviewed public byline</span>
                        <textarea
                          ref={(element) => {
                            bylineInput = element;
                          }}
                          value={bylineText()}
                          onInput={(event) => setBylineText(event.currentTarget.value)}
                          aria-describedby="flat-byline-help"
                          disabled={isLocked()}
                        />
                        <span id="flat-byline-help" class="flat-question-authoring__help">
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
            <a href={reference()}>Open question library</a>
          </section>
        )}
      </Show>
    </main>
  );
}
