// Instructor workflow for creating one reusable Published Question Pool.

import { createMemo, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

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
} from "../features/question_picker";
import {
  questionPoolEligibilityForQuestion,
  questionPoolMemberTuples,
  questionPoolStartingEligibility,
  QuestionPoolSelectionError,
  type QuestionPoolStartingQuestion,
  type QuestionPoolStartingContext,
} from "./question_pool_create_model";
import { QuestionPoolAvailableQuestions } from "./question_pool_available_questions";
import { questionNativePreview } from "./question_native_preview";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import {
  comparePoolMemberFields,
  poolMemberRowId,
  questionPoolMemberFields,
} from "./question_pool_member_fields";
import type { QuestionPoolMemberDraft } from "./question_pool_members_model";
import type { LibraryObjectQuestionRow } from "../pages/library_page_model";
import {
  UnsavedChangesGuard,
  confirmUnsavedChanges,
  trackEditorSave,
} from "./unsaved_changes_guard";
import "./question_pool_create_dialog.css";

type CreationState = "choosing" | "reviewing" | "creating" | "created";
type PoolCreationMember = QuestionPoolMemberDraft & {
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
  readonly classificationClient: ContentClassificationClient;
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
  const [adding, setAdding] = createSignal(false);
  const creationGuardScope = Symbol("Question Pool creation");
  const [createdPool, setCreatedPool] = createSignal<{
    readonly questionPoolId: string;
    readonly questionPoolEditNumber: number;
  }>();
  const dirty = (): boolean =>
    state() !== "created" &&
    (trimPoolDraft(title()).length > 0 ||
      trimPoolDraft(description()).length > 0 ||
      (selection()?.questions.length ?? 0) > 0);
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

  const memberSource = createMemo(() => [
    ...startingQuestions.map((question): PoolCreationMember => ({
      publishedQuestionRevisionTuple: question.publishedQuestionRevisionTuple,
      questionTitle: question.questionTitle,
      nativePrompt: null,
      questionLicense: null,
      revisionIsFixed: true,
    })),
    ...(selection()?.questions ?? []).map((question): PoolCreationMember => ({
      publishedQuestionRevisionTuple: question.row.publishedQuestionRevisionTuple,
      questionTitle: question.row.questionTitle,
      nativePrompt: question.row.nativePrompt,
      questionLicense: question.row.questionLicense,
      revisionIsFixed: false,
    })),
  ]);
  const memberFields = questionPoolMemberFields<PoolCreationMember>();
  const memberCount = (): number => startingQuestions.length + (selection()?.questions.length ?? 0);
  const memberSearch = createLocalCompleteSearch(
    { rows: memberSource },
    {
      fields: memberFields,
      rowId: poolMemberRowId,
      keyword: (member) =>
        `${member.questionTitle} ${member.publishedQuestionRevisionTuple.publishedQuestionId}`,
      compare: comparePoolMemberFields,
      content: (member) => ({
        title: member.questionTitle,
        preview: questionNativePreview(member),
        details: [
          {
            kind: "questionId",
            questionTitle: member.questionTitle,
            displayId: member.publishedQuestionRevisionTuple.publishedQuestionId,
          },
          {
            kind: "text",
            label: "Revision",
            value: String(member.publishedQuestionRevisionTuple.revisionNumber),
          },
        ],
        actions: [
          {
            id: "remove",
            kind: "command",
            label: "Remove",
            disabled:
              member.revisionIsFixed || memberCount() <= 1 || adding() || state() !== "reviewing",
            onClick: (): void => {
              if (
                member.revisionIsFixed ||
                memberCount() <= 1 ||
                adding() ||
                state() !== "reviewing"
              )
                return;
              const id = member.publishedQuestionRevisionTuple.publishedQuestionId;
              setSelection((current) =>
                current === undefined
                  ? current
                  : {
                      questionIds: current.questionIds.filter((value) => value !== id),
                      questions: current.questions.filter((question) => question.questionId !== id),
                    },
              );
              setError(undefined);
            },
          },
        ],
      }),
    },
  );

  async function addQuestion(row: LibraryObjectQuestionRow): Promise<boolean> {
    const currentEligibility = eligibility();
    if (
      currentEligibility === undefined ||
      adding() ||
      state() !== "reviewing" ||
      memberCount() >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
    )
      return false;
    const id = row.displayId;
    if (
      startingQuestions.some(
        (question) => question.publishedQuestionRevisionTuple.publishedQuestionId === id,
      ) ||
      selection()?.questionIds.includes(id)
    )
      return false;
    setAdding(true);
    setError(undefined);
    try {
      const addition = { questionIds: [id], questions: [{ questionId: id, row }] };
      // ASVS 2.2.1/2.2.2: validate the exact selected revision; Create repeats checks server-side.
      await questionPoolMemberTuples(
        addition,
        props.loadQuestionInspection,
        props.getCurrentQuestionSharedMetadata,
        currentEligibility,
      );
      if (disposed) return false;
      setSelection((current) =>
        current === undefined
          ? current
          : {
              questionIds: [...current.questionIds, id],
              questions: [...current.questions, ...addition.questions],
            },
      );
      return true;
    } catch {
      if (!disposed)
        setError(
          "This Question no longer meets the Pool requirements. Your selected Questions and entered work are still here.",
        );
      return false;
    } finally {
      if (!disposed) setAdding(false);
    }
  }

  async function createPool(): Promise<boolean> {
    const selected = selection();
    if (selected === undefined || state() !== "reviewing" || adding()) return false;
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
          description: "Your selected Questions, Pool Title, and Description will be discarded.",
        }}
      />
      <Show when={state() === "choosing"}>
        <QuestionPicker
          repository={basePickerRepository}
          sources={pickerSources}
          loadQuestionInspection={props.loadQuestionInspection}
          questionRevisionPreviewDocumentUrl={props.questionRevisionPreviewDocumentUrl}
          questionImageUrl={props.questionImageUrl}
          mode="one"
          maximumSelection={1}
          title="Choose published Questions for this Pool"
          instructions="Choose the first published Question to establish this Pool's Discipline, Subject, Type, and Backend."
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
                      The first Question, {selected().questions[0]?.row.questionTitle}, establishes
                      this Pool's Discipline and Subject. Every additional Question must share both.
                      The server checks this when you create the Pool.
                    </p>
                  }
                >
                  {(context) => (
                    <p>
                      The starting Questions establish this Pool's {context().disciplineName}{" "}
                      Discipline and {context().subjectName} Subject. Every additional Question must
                      share both.
                    </p>
                  )}
                </Show>
              </fieldset>
            </Show>
            <div class="question-pool-create-review-grid">
              <section aria-labelledby="question-pool-selected-heading">
                <h3 id="question-pool-selected-heading">Selected Questions</h3>
                <SearchBlock
                  layout="above"
                  state={memberSearch.state}
                  controls={
                    <SearchControls
                      state={memberSearch.state}
                      textLabel="Search selected Pool Questions"
                    />
                  }
                  collection={{
                    ariaLabel: "Selected Pool Questions",
                    fields: memberFields,
                    query: memberSearch.query,
                    emptyState: { title: "No selected Questions match this search." },
                  }}
                />
              </section>
              <Show when={state() !== "created" && eligibility()}>
                {(currentEligibility) => (
                  <section aria-labelledby="question-pool-create-available-heading">
                    <h3 id="question-pool-create-available-heading">Library Questions</h3>
                    <QuestionPoolAvailableQuestions
                      repository={props.questionLibrary}
                      classificationClient={props.classificationClient}
                      eligibility={currentEligibility()}
                      includedIds={
                        new Set([
                          ...startingQuestions.map(
                            (question) =>
                              question.publishedQuestionRevisionTuple.publishedQuestionId,
                          ),
                          ...(selection()?.questionIds ?? []),
                        ])
                      }
                      disabled={
                        state() !== "reviewing" ||
                        adding() ||
                        memberCount() >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
                      }
                      onAdd={addQuestion}
                    />
                  </section>
                )}
              </Show>
            </div>
            <Show when={state() !== "created"}>
              <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
              <p class="question-pool-create-actions">
                <button class="quiet-action" type="button" onClick={() => void closeReview()}>
                  Cancel
                </button>{" "}
                <button
                  class="primary-action"
                  type="button"
                  disabled={
                    trimPoolDraft(title()).length === 0 ||
                    trimPoolDraft(description()).length === 0 ||
                    state() === "creating" ||
                    adding()
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
