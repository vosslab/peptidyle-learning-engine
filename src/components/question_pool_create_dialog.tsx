// Instructor workflow for creating one reusable Published Question Pool.

import { Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { ContentClassificationClient } from "../api/content_classification";
import { decodeQuestionPoolText } from "../api/decoders/question_pool_summary";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import {
  QuestionPicker,
  questionLibraryPickerRepository,
  questionLibraryPickerSources,
  type QuestionPickerEligibility,
  type QuestionPickerProps,
  type QuestionPickerSelection,
  type QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  questionPoolEligibilityForQuestion,
  questionPoolMemberTuples,
  questionPoolStartingEligibility,
  questionPoolSourcePickerRepository,
  QuestionPoolSelectionError,
  type QuestionPoolStartingQuestion,
  type QuestionPoolStartingContext,
} from "./question_pool_create_model";
import { RecordSequence } from "./record_list/record_sequence";
import {
  UnsavedChangesGuard,
  confirmUnsavedChanges,
  trackEditorSave,
} from "./unsaved_changes_guard";
import "./question_pool_create_dialog.css";

type CreationState = "choosing" | "reviewing" | "creating" | "created";
type PoolCreationMember = {
  readonly title: string;
  readonly publishedQuestionId: string;
  readonly revisionNumber: number;
  readonly revisionIsFixed: boolean;
};

// Match PostgreSQL btrim and Rust trim_matches(' '), not JavaScript Unicode trim.
function trimPoolDraft(value: string): string {
  return value.replace(/^ +| +$/gu, "");
}

export interface QuestionPoolCreateDialogProps {
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly questionLibrary: QuestionLibraryBrowseRepository;
  readonly getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>;
  readonly loadQuestionInspection: QuestionPickerProps["loadQuestionInspection"];
  readonly questionRevisionPreviewDocumentUrl: QuestionPickerProps["questionRevisionPreviewDocumentUrl"];
  readonly questionImageUrl: QuestionPickerProps["questionImageUrl"];
  readonly getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"];
  readonly classificationClient: Pick<
    ContentClassificationClient,
    "listDisciplinesIncludingRetired" | "listSubjects"
  >;
  /** Exact Published Question Revisions that fix Pool membership and classification. */
  readonly startingQuestions?: ReadonlyArray<QuestionPoolStartingQuestion>;
  readonly onTaskPhaseChange: (active: boolean) => void;
  readonly onClose: () => void;
}

/** Keeps the exact tuple set local until the Instructor creates the Pool. */
export function QuestionPoolCreateDialog(props: QuestionPoolCreateDialogProps): JSX.Element {
  const startingQuestions = props.startingQuestions ?? [];
  const sourceBound = startingQuestions.length > 0;
  const [state, setState] = createSignal<CreationState>(sourceBound ? "reviewing" : "choosing");
  const [selection, setSelection] = createSignal<QuestionPickerSelection | undefined>(
    sourceBound ? { questionIds: [], questions: [] } : undefined,
  );
  const [eligibility, setEligibility] = createSignal<QuestionPickerEligibility>();
  const [startingContext, setStartingContext] = createSignal<QuestionPoolStartingContext>();
  const [title, setTitle] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [error, setError] = createSignal<string>();
  const creationGuardScope = Symbol("Question Pool creation");
  const [createdPool, setCreatedPool] = createSignal<{
    readonly questionPoolId: string;
    readonly questionPoolEditNumber: number;
  }>();
  const dirty = (): boolean =>
    state() !== "created" &&
    (trimPoolDraft(title()).length > 0 || trimPoolDraft(description()).length > 0);
  let reviewHeading: HTMLHeadingElement | undefined;
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });
  const pickerSources = questionLibraryPickerSources(false);
  const basePickerRepository = questionLibraryPickerRepository(
    props.questionLibrary,
    props.questionLibrary,
  );
  const pickerRepository = (): QuestionPickerSourceRepository =>
    sourceBound && eligibility() !== undefined
      ? questionPoolSourcePickerRepository(props.questionLibrary, eligibility()!)
      : basePickerRepository;

  onMount(() => {
    if (!sourceBound) return;
    props.onTaskPhaseChange(true);
    queueMicrotask(() => reviewHeading?.focus());
    void loadStartingContext().catch((cause: unknown) => {
      if (!disposed)
        setError(
          cause instanceof QuestionPoolSelectionError
            ? cause.message
            : "Pool eligibility could not be loaded. Try again.",
        );
    });
  });

  async function loadStartingContext(): Promise<QuestionPoolStartingContext> {
    const context = await questionPoolStartingEligibility(
      startingQuestions,
      props.getCurrentQuestionSharedMetadata,
      props.classificationClient,
    );
    if (!disposed) {
      setStartingContext(context);
      setEligibility(context.eligibility);
      setError(undefined);
    }
    return context;
  }

  async function chooseAgain(): Promise<void> {
    setError(undefined);
    try {
      if (sourceBound) {
        if (startingQuestions.length >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY) {
          throw new QuestionPoolSelectionError(
            "This Pool already has the maximum number of starting Questions.",
          );
        }
        const nextContext = await loadStartingContext();
        if (disposed) return;
        setEligibility(nextContext.eligibility);
      }
      if (disposed) return;
      setState("choosing");
      props.onTaskPhaseChange(false);
    } catch (cause) {
      if (!disposed)
        setError(
          cause instanceof QuestionPoolSelectionError
            ? cause.message
            : "Pool eligibility could not be loaded. Try again.",
        );
    }
  }

  async function cancelSelection(): Promise<boolean> {
    if (!sourceBound) {
      if (!(await confirmUnsavedChanges(creationGuardScope))) return false;
      props.onClose();
      return true;
    }
    setState("reviewing");
    props.onTaskPhaseChange(true);
    queueMicrotask(() => reviewHeading?.focus());
    return true;
  }

  async function closeReview(): Promise<void> {
    if (await confirmUnsavedChanges(creationGuardScope)) props.onClose();
  }

  async function acceptSelection(next: QuestionPickerSelection): Promise<boolean> {
    try {
      if (!sourceBound && eligibility() === undefined) {
        const first = next.questions[0];
        if (first === undefined) throw new Error("Choose one Question to establish this Pool.");
        const nextEligibility = await questionPoolEligibilityForQuestion(
          first.questionId,
          props.getQuestionDetails,
          props.getCurrentQuestionSharedMetadata,
        );
        if (disposed) return false;
        setEligibility(nextEligibility);
      }
      if (disposed) return false;
      setSelection(next);
      setError(undefined);
      setState("reviewing");
      props.onTaskPhaseChange(true);
      queueMicrotask(() => reviewHeading?.focus());
      return true;
    } catch {
      if (!disposed)
        setError(
          "The selected Question cannot establish Pool eligibility. Choose another published Question.",
        );
      return false;
    }
  }

  async function createPool(): Promise<boolean> {
    const selected = selection();
    if (selected === undefined || state() !== "reviewing") return false;
    try {
      decodeQuestionPoolText(trimPoolDraft(title()), "Title", 512);
      decodeQuestionPoolText(trimPoolDraft(description()), "Description", 4000);
    } catch {
      setError(
        "Enter a Title (1-512 characters) and Description (1-4000 characters), without control characters.",
      );
      return false;
    }
    setState("creating");
    setError(undefined);
    try {
      let currentEligibility = eligibility();
      if (sourceBound) {
        const context = await questionPoolStartingEligibility(
          startingQuestions,
          props.getCurrentQuestionSharedMetadata,
          props.classificationClient,
        );
        currentEligibility = context.eligibility;
        setStartingContext(context);
      } else if (currentEligibility === undefined) {
        const first = selected.questions[0];
        if (first === undefined) {
          throw new Error("Choose one Question to establish this Pool.");
        }
        currentEligibility = await questionPoolEligibilityForQuestion(
          first.questionId,
          props.getQuestionDetails,
          props.getCurrentQuestionSharedMetadata,
        );
      }
      const members = await questionPoolMemberTuples(
        selected,
        props.loadQuestionInspection,
        props.getCurrentQuestionSharedMetadata,
        currentEligibility,
        startingQuestions,
      );
      const created = await props.questionPoolClient.createQuestionPool({
        title: trimPoolDraft(title()),
        description: trimPoolDraft(description()),
        members,
      });
      setCreatedPool(created);
      setState("created");
      return true;
    } catch (cause) {
      // ASVS 16.5.1: show known selection conflicts, not unexpected API internals.
      setError(
        `${cause instanceof QuestionPoolSelectionError ? cause.message : "Question Pool could not be created. Check that every Question remains available and shares the Pool's Discipline, Subject, Type, and Backend."} Your Title, Description, and selected Questions are still here.`,
      );
      setState("reviewing");
      return false;
    }
  }
  const trackedCreate = trackEditorSave(createPool);

  return (
    <>
      <UnsavedChangesGuard
        scope={creationGuardScope}
        dirty={dirty}
        inFlightSave={trackedCreate.inFlightSave}
        copy={{
          heading: "Discard Question Pool creation?",
          description: "Your Pool Title or Description will be discarded.",
        }}
      />
      <Show when={state() === "choosing"}>
        <QuestionPicker
          repository={pickerRepository()}
          sources={pickerSources}
          loadQuestionInspection={props.loadQuestionInspection}
          questionRevisionPreviewDocumentUrl={props.questionRevisionPreviewDocumentUrl}
          questionImageUrl={props.questionImageUrl}
          eligibility={eligibility()}
          mode={sourceBound || eligibility() !== undefined ? "many" : "one"}
          maximumSelection={MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY - startingQuestions.length}
          initialSelection={selection()}
          title={
            sourceBound
              ? "Add published Questions to this Pool"
              : "Choose published Questions for this Pool"
          }
          instructions={
            !sourceBound
              ? "Select published Questions for this reusable Pool."
              : startingContext() === undefined
                ? `The ${startingQuestions.length} starting Questions are already selected.`
                : `Results begin filtered to ${startingContext()!.disciplineName} / ${startingContext()!.subjectName}. The ${startingQuestions.length} starting Questions are already selected.`
          }
          confirmLabel="Review selected Questions"
          trigger={undefined}
          onConfirm={acceptSelection}
          confirmationError={error()}
          onCancel={cancelSelection}
        />
      </Show>
      <Show when={state() === "choosing" ? undefined : selection()}>
        {(selected) => (
          <section
            class="question-pool-create-review"
            aria-labelledby="question-pool-create-heading"
          >
            <h1
              id="question-pool-create-heading"
              ref={(element) => (reviewHeading = element)}
              tabindex="-1"
            >
              Create Question Pool
            </h1>
            <div class="question-pool-create-review-grid">
              <section aria-labelledby="question-pool-selected-heading">
                <h3 id="question-pool-selected-heading">Selected Questions</h3>
                <RecordSequence
                  rows={[
                    ...startingQuestions.map((question) => ({
                      title: question.questionTitle,
                      publishedQuestionId:
                        question.publishedQuestionRevisionTuple.publishedQuestionId,
                      revisionNumber: question.publishedQuestionRevisionTuple.revisionNumber,
                      revisionIsFixed: true,
                    })),
                    ...selected().questions.map((question) => ({
                      title: question.row.questionTitle,
                      publishedQuestionId:
                        question.row.publishedQuestionRevisionTuple.publishedQuestionId,
                      revisionNumber: question.row.publishedQuestionRevisionTuple.revisionNumber,
                      revisionIsFixed: false,
                    })),
                  ].sort((left, right) => left.title.localeCompare(right.title))}
                  content={(member: PoolCreationMember) => ({
                    title: member.title,
                    details: [
                      {
                        kind: "text",
                        label: member.revisionIsFixed
                          ? "Fixed Published Question revision"
                          : "Current selected Question revision (rechecked when creating)",
                        value: `${member.publishedQuestionId}, Revision ${member.revisionNumber}`,
                      },
                    ],
                    actions: [],
                  })}
                  recordId={(member) => `${member.publishedQuestionId}:${member.revisionNumber}`}
                  state={{ kind: "ready" }}
                  ariaLabel="Selected Questions sorted by title"
                  emptyState={{ title: "No Questions are selected." }}
                />
              </section>
              <Show when={state() !== "created"}>
                <fieldset disabled={state() === "creating"}>
                  <legend>Pool metadata</legend>
                  <div class="question-pool-create-title-context">
                    <label class="assessment-editor-field">
                      Pool Title (required)
                      <input
                        required
                        value={title()}
                        onInput={(event) => setTitle(event.currentTarget.value)}
                      />
                    </label>
                    <Show when={startingContext()}>
                      {(context) => (
                        <dl class="question-pool-starting-classification">
                          <div>
                            <dt>Discipline</dt>
                            <dd>{context().disciplineName}</dd>
                          </div>
                          <div>
                            <dt>Subject</dt>
                            <dd>{context().subjectName}</dd>
                          </div>
                        </dl>
                      )}
                    </Show>
                  </div>
                  <label class="assessment-editor-field">
                    Description (required)
                    <textarea
                      required
                      value={description()}
                      onInput={(event) => setDescription(event.currentTarget.value)}
                    />
                  </label>
                  <Show
                    when={startingContext()}
                    fallback={
                      <p>
                        The first Question, {selected().questions[0]?.row.questionTitle},
                        establishes this Pool's Discipline and Subject. Every additional Question
                        must share both. The server checks this when you create the Pool.
                      </p>
                    }
                  >
                    {(context) => (
                      <p>
                        The starting Questions establish this Pool's {context().disciplineName}{" "}
                        Discipline and {context().subjectName} Subject. Every additional Question
                        must share both.
                      </p>
                    )}
                  </Show>
                </fieldset>
              </Show>
            </div>
            <Show when={state() !== "created"}>
              <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
              <p class="question-pool-create-actions">
                <button class="quiet-action" type="button" onClick={() => void closeReview()}>
                  Cancel
                </button>{" "}
                <button
                  class="quiet-action"
                  type="button"
                  disabled={state() === "creating"}
                  onClick={() => void chooseAgain()}
                >
                  {sourceBound ? "Add or change Questions" : "Choose different Questions"}
                </button>{" "}
                <button
                  class="primary-action"
                  type="button"
                  disabled={
                    trimPoolDraft(title()).length === 0 ||
                    trimPoolDraft(description()).length === 0 ||
                    state() === "creating"
                  }
                  onClick={() => void trackedCreate.save()}
                >
                  {state() === "creating"
                    ? "Creating Question Pool..."
                    : "Create published Question Pool"}
                </button>
              </p>
            </Show>
            <Show when={createdPool()}>
              {(created) => (
                <section role="status">
                  <h3>Published Question Pool created</h3>
                  <p>
                    <strong>{trimPoolDraft(title())}</strong>
                  </p>
                  <p>{trimPoolDraft(description())}</p>
                  <p>
                    {created().questionPoolId}, Edit {created().questionPoolEditNumber}
                  </p>
                  <button class="quiet-action" type="button" onClick={props.onClose}>
                    Close
                  </button>
                </section>
              )}
            </Show>
          </section>
        )}
      </Show>
    </>
  );
}
