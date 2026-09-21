// Instructor workflow for creating one reusable Published Question Pool.

import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import { decodeQuestionPoolText } from "../api/decoders/question_pool_library";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import {
  QuestionPicker,
  questionLibraryPickerRepository,
  questionLibraryPickerSources,
  type QuestionPickerSelection,
} from "../features/question_picker";
import {
  questionPoolMemberTuples,
  questionPoolSourcePickerRepository,
  type QuestionPoolStartingQuestion,
} from "./question_pool_create_model";
import "./question_pool_create_dialog.css";

type CreationState = "choosing" | "reviewing" | "creating" | "created";

// Match PostgreSQL btrim and Rust trim_matches(' '), not JavaScript Unicode trim.
function trimPoolDraft(value: string): string {
  return value.replace(/^ +| +$/gu, "");
}

export interface QuestionPoolCreateDialogProps {
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly questionLibrary: QuestionLibraryBrowseRepository;
  readonly getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>;
  /** Exact Published Question that fixes source-bound Pool membership and classification. */
  readonly startingQuestion?: QuestionPoolStartingQuestion;
  readonly onTaskPhaseChange: (active: boolean) => void;
  readonly onClose: () => void;
}

/** Keeps ordered picker selection local until the Instructor explicitly attests and creates. */
export function QuestionPoolCreateDialog(props: QuestionPoolCreateDialogProps): JSX.Element {
  const sourceBound = props.startingQuestion !== undefined;
  const [state, setState] = createSignal<CreationState>(sourceBound ? "reviewing" : "choosing");
  const [selection, setSelection] = createSignal<QuestionPickerSelection | undefined>(
    sourceBound ? { questionIds: [], questions: [] } : undefined,
  );
  const [attested, setAttested] = createSignal(false);
  const [title, setTitle] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [error, setError] = createSignal<string>();
  const [createdPool, setCreatedPool] = createSignal<{
    readonly questionPoolId: string;
    readonly questionPoolEditNumber: number;
  }>();
  let reviewHeading: HTMLHeadingElement | undefined;
  const pickerRepository =
    props.startingQuestion === undefined
      ? questionLibraryPickerRepository(props.questionLibrary, props.questionLibrary)
      : questionPoolSourcePickerRepository(props.questionLibrary, props.startingQuestion);
  const pickerSources = questionLibraryPickerSources(false);

  onMount(() => {
    if (!sourceBound) return;
    props.onTaskPhaseChange(true);
    queueMicrotask(() => reviewHeading?.focus());
  });

  function chooseAgain(): void {
    setAttested(false);
    setError(undefined);
    setState("choosing");
    props.onTaskPhaseChange(false);
  }

  function cancelSelection(): void {
    if (!sourceBound) {
      props.onClose();
      return;
    }
    setState("reviewing");
    props.onTaskPhaseChange(true);
    queueMicrotask(() => reviewHeading?.focus());
  }

  function acceptSelection(next: QuestionPickerSelection): void {
    setSelection(next);
    setAttested(false);
    setError(undefined);
    setState("reviewing");
    props.onTaskPhaseChange(true);
    queueMicrotask(() => reviewHeading?.focus());
  }

  async function createPool(): Promise<void> {
    const selected = selection();
    if (selected === undefined || !attested() || state() === "creating") return;
    try {
      decodeQuestionPoolText(trimPoolDraft(title()), "Title", 512);
      decodeQuestionPoolText(trimPoolDraft(description()), "Description", 4000);
    } catch {
      setError(
        "Enter a Title (1-512 characters) and Description (1-4000 characters), without control characters.",
      );
      return;
    }
    setState("creating");
    setError(undefined);
    try {
      const members = await questionPoolMemberTuples(
        selected,
        props.getQuestionDetails,
        props.startingQuestion,
      );
      const created = await props.questionPoolClient.createQuestionPool({
        title: trimPoolDraft(title()),
        description: trimPoolDraft(description()),
        members,
        interchangeabilityAttested: true,
      });
      setCreatedPool(created);
      setState("created");
    } catch {
      setError(
        "Question Pool could not be created. Your metadata and ordered selection are still here. Check that every Question has the same Discipline and Subject, then try again.",
      );
      setState("reviewing");
    }
  }

  return (
    <>
      <Show when={state() === "choosing"}>
        <QuestionPicker
          repository={pickerRepository}
          sources={pickerSources}
          mode="many"
          maximumSelection={
            sourceBound
              ? MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY - 1
              : MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
          }
          initialSelection={selection()}
          title={
            sourceBound
              ? "Add published Questions to this Pool"
              : "Choose published Questions for this Pool"
          }
          instructions={
            props.startingQuestion === undefined
              ? "Select interchangeable published Questions and arrange their order before you attest to creating a reusable Pool."
              : `Results begin filtered to ${props.startingQuestion.disciplineName} / ${props.startingQuestion.subjectName}. The starting Question stays first.`
          }
          confirmLabel="Review selected Questions"
          trigger={undefined}
          onConfirm={acceptSelection}
          onCancel={cancelSelection}
        />
      </Show>
      <Show when={state() === "choosing" ? undefined : selection()}>
        {(selected) => (
          <section
            class="question-pool-create-review"
            aria-labelledby="question-pool-create-heading"
          >
            <p class="eyebrow">Reusable published content</p>
            <h1
              id="question-pool-create-heading"
              ref={(element) => (reviewHeading = element)}
              tabindex="-1"
            >
              Create Question Pool
            </h1>
            <p class="question-pool-create-introduction">
              {sourceBound
                ? "The starting Published Question is fixed first. Add interchangeable Questions or create this reusable Pool with its starting Question."
                : "Review the ordered Questions and describe the reusable Pool. Their current Revisions are resolved when you create it."}
            </p>
            <div class="question-pool-create-review-grid">
              <section aria-labelledby="question-pool-selected-heading">
                <h3 id="question-pool-selected-heading">Selected Questions in order</h3>
                <ol class="question-pool-member-list">
                  <Show when={props.startingQuestion}>
                    {(starting) => (
                      <li>
                        <strong>{starting().questionTitle}</strong> (
                        {starting().publishedQuestionRevisionTuple.publishedQuestionId}, Revision{" "}
                        {starting().publishedQuestionRevisionTuple.revisionNumber})
                      </li>
                    )}
                  </Show>
                  <For each={selected().questions}>
                    {(question) => (
                      <li>
                        <strong>{question.row.questionTitle}</strong> ({question.questionId})
                      </li>
                    )}
                  </For>
                </ol>
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
                    <Show when={props.startingQuestion}>
                      {(starting) => (
                        <dl class="question-pool-starting-classification">
                          <div>
                            <dt>Discipline</dt>
                            <dd>{starting().disciplineName}</dd>
                          </div>
                          <div>
                            <dt>Subject</dt>
                            <dd>{starting().subjectName}</dd>
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
                    when={props.startingQuestion}
                    fallback={
                      <p>
                        The first Question, {selected().questions[0]?.row.questionTitle},
                        establishes this Pool's Discipline and Subject. Every additional Question
                        must share both. The server checks this when you create the Pool.
                      </p>
                    }
                  >
                    {(starting) => (
                      <p>
                        <strong>{starting().questionTitle}</strong> establishes this Pool's{" "}
                        {starting().disciplineName} Discipline and {starting().subjectName} Subject.
                        Every additional Question must share both.
                      </p>
                    )}
                  </Show>
                </fieldset>
              </Show>
            </div>
            <Show when={state() !== "created"}>
              <label class="question-pool-attestation">
                <input
                  type="checkbox"
                  checked={attested()}
                  disabled={state() === "creating"}
                  onChange={(event) => setAttested(event.currentTarget.checked)}
                />{" "}
                I attest that these Published Questions assess the same intended learning and can
                reasonably substitute for one another. This lets PLE select among them without
                changing what the Pool assesses.
              </label>
              <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
              <p class="question-pool-create-actions">
                <button
                  class="quiet-action"
                  type="button"
                  disabled={state() === "creating"}
                  onClick={chooseAgain}
                >
                  {sourceBound ? "Add or change Questions" : "Choose different Questions"}
                </button>{" "}
                <button
                  class="primary-action"
                  type="button"
                  disabled={
                    !attested() ||
                    trimPoolDraft(title()).length === 0 ||
                    trimPoolDraft(description()).length === 0 ||
                    state() === "creating"
                  }
                  onClick={() => void createPool()}
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
