// Instructor workflow for creating one reusable Published Question Pool.

import { For, Show, createSignal, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { QuestionPoolCreationClient } from "../api/question_pool_creation";
import { normalizeQuestionIdSyntax } from "../question_id";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import {
  QuestionPicker,
  questionLibraryPickerRepository,
  questionLibraryPickerSources,
  type QuestionPickerSelection,
} from "../features/question_picker";

type CreationState = "choosing" | "reviewing" | "creating" | "created";

export interface QuestionPoolCreateDialogProps {
  readonly questionPoolClient: QuestionPoolCreationClient;
  readonly questionLibrary: QuestionLibraryBrowseRepository;
  readonly getQuestionDetails: (questionId: QuestionId) => Promise<QuestionDetails>;
  readonly onClose: () => void;
}

function canonicalQuestionId(value: string): QuestionId {
  const questionId = normalizeQuestionIdSyntax(value);
  if (questionId === null || questionId !== value) {
    throw new Error("The selected Question is no longer a canonical Published Question.");
  }
  return questionId;
}

async function latestSelectedRevisions(
  selection: QuestionPickerSelection,
  getQuestionDetails: QuestionPoolCreateDialogProps["getQuestionDetails"],
): Promise<ReadonlyArray<QuestionRevisionReference>> {
  return await Promise.all(
    selection.questions.map(async (selected) => {
      const questionId = canonicalQuestionId(selected.questionId);
      const detail = await getQuestionDetails(questionId);
      const reference = detail.summary.latestQuestionRevision;
      if (reference.questionId !== questionId) {
        throw new Error("The selected Question did not resolve to its current published Revision.");
      }
      return reference;
    }),
  );
}

/** Keeps ordered picker selection local until the Instructor explicitly attests and creates. */
export function QuestionPoolCreateDialog(props: QuestionPoolCreateDialogProps): JSX.Element {
  const [state, setState] = createSignal<CreationState>("choosing");
  const [selection, setSelection] = createSignal<QuestionPickerSelection>();
  const [attested, setAttested] = createSignal(false);
  const [error, setError] = createSignal<string>();
  const [createdPool, setCreatedPool] = createSignal<{
    readonly questionPoolId: string;
    readonly revisionNumber: number;
  }>();
  const pickerRepository = questionLibraryPickerRepository(
    props.questionLibrary,
    props.questionLibrary,
  );
  const pickerSources = questionLibraryPickerSources(false);

  function chooseAgain(): void {
    setAttested(false);
    setError(undefined);
    setState("choosing");
  }

  function acceptSelection(next: QuestionPickerSelection): void {
    setSelection(next);
    setAttested(false);
    setError(undefined);
    setState("reviewing");
  }

  async function createPool(): Promise<void> {
    const selected = selection();
    if (selected === undefined || !attested() || state() === "creating") return;
    setState("creating");
    setError(undefined);
    try {
      const members = await latestSelectedRevisions(selected, props.getQuestionDetails);
      const created = await props.questionPoolClient.createQuestionPool({
        members,
        interchangeabilityAttested: true,
      });
      setCreatedPool(created);
      setState("created");
    } catch {
      setError(
        "Question Pool could not be created. Your ordered selection is still here; review it and try again.",
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
          maximumSelection={MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY}
          title="Choose published Questions for this Pool"
          instructions="Select interchangeable published Questions and arrange their order before you attest to creating a reusable Pool."
          confirmLabel="Review selected Questions"
          trigger={undefined}
          onConfirm={acceptSelection}
          onCancel={props.onClose}
        />
      </Show>
      <Show when={state() === "choosing" ? undefined : selection()}>
        {(selected) => (
          <>
            <p class="eyebrow">Reusable published content</p>
            <h2 id="question-pool-create-heading">Create Question Pool</h2>
            <p>
              Create a reusable published Pool from these Questions. Their current Revisions are
              resolved when you create the Pool.
            </p>
            <ol>
              <For each={selected().questions}>
                {(question) => (
                  <li>
                    <strong>{question.row.questionTitle}</strong> ({question.questionId})
                  </li>
                )}
              </For>
            </ol>
            <Show when={state() !== "created"}>
              <label>
                <input
                  type="checkbox"
                  checked={attested()}
                  disabled={state() === "creating"}
                  onChange={(event) => setAttested(event.currentTarget.checked)}
                />{" "}
                I attest that these Published Questions are interchangeable for this Pool.
              </label>
              <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
              <p>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={state() === "creating"}
                  onClick={chooseAgain}
                >
                  Choose different Questions
                </button>{" "}
                <button
                  class="primary-action"
                  type="button"
                  disabled={!attested() || state() === "creating"}
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
                    {created().questionPoolId}, Revision {created().revisionNumber}
                  </p>
                  <button class="quiet-action" type="button" onClick={props.onClose}>
                    Close
                  </button>
                </section>
              )}
            </Show>
          </>
        )}
      </Show>
    </>
  );
}
