// flat_question_preview.tsx - learner-equivalent, answer-free local preview with author-only answer check.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { FlatQuestionPublicPreview } from "./flat_question_public_preview";

export interface FlatQuestionInstructorAnswerCheck {
  readonly correctChoiceId: string;
  readonly correctChoiceText: string;
  readonly correctFeedback: string | null;
  readonly incorrectFeedback: string | null;
}

export interface FlatQuestionPreviewProps {
  /** This projection contains no correct answer and is safe for the learner-equivalent preview. */
  readonly preview: FlatQuestionPublicPreview;
  /** The page exposes this only after the author explicitly asks to inspect answer material. */
  readonly instructorAnswerCheck?: FlatQuestionInstructorAnswerCheck;
}

/**
 * Mirrors the learner's one-choice response interaction without importing author source into a
 * learner component. The answer check remains an explicitly named author-only panel.
 */
export function FlatQuestionPreview(props: FlatQuestionPreviewProps): JSX.Element {
  const [selectedChoice, setSelectedChoice] = createSignal<string | null>(null);
  const groupName = "flat-question-student-preview-choice";
  return (
    <section
      class="flat-question-authoring__preview"
      aria-labelledby="flat-student-preview-heading"
    >
      <h3 id="flat-student-preview-heading">Student preview</h3>
      <p class="flat-question-authoring__help">
        This is answer-free. Selecting an answer does not grade, save, or send a request.
      </p>
      <article aria-labelledby="flat-preview-title">
        <h4 id="flat-preview-title">{props.preview.title}</h4>
        <p>{props.preview.prompt}</p>
        <fieldset>
          <legend>Choose one response</legend>
          <For each={props.preview.choices}>
            {(choice, index) => (
              <label class="flat-question-authoring__preview-choice">
                <input
                  type="radio"
                  name={groupName}
                  value={choice.id}
                  checked={selectedChoice() === choice.id}
                  onChange={() => setSelectedChoice(choice.id)}
                />
                <span>
                  <strong aria-hidden="true">{String.fromCharCode(65 + index())}.</strong>{" "}
                  {choice.text}
                </span>
              </label>
            )}
          </For>
        </fieldset>
      </article>
      <Show when={props.instructorAnswerCheck}>
        {(check) => (
          <aside
            class="flat-question-authoring__instructor-check"
            aria-labelledby="flat-instructor-check-heading"
          >
            <h4 id="flat-instructor-check-heading">Instructor answer check</h4>
            <p>This protected authoring panel is not part of the student preview.</p>
            <p>
              <strong>Correct choice:</strong> {check().correctChoiceText}
            </p>
            <Show when={check().correctFeedback !== null}>
              <p>
                <strong>Correct-answer feedback:</strong> {check().correctFeedback}
              </p>
            </Show>
            <Show when={check().incorrectFeedback !== null}>
              <p>
                <strong>Incorrect-answer feedback:</strong> {check().incorrectFeedback}
              </p>
            </Show>
          </aside>
        )}
      </Show>
    </section>
  );
}
