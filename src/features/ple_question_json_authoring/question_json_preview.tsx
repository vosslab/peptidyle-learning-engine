// question_json_preview.tsx - answer-free local preview through the student response runtime.

import { For, Show, type JSX } from "solid-js";

import { QuestionResponseControl } from "../../components/question_response_controls/question_response_control";
import type { WasmFacade } from "../../wasm/index";
import type { PleQuestionJsonPublicPreview } from "./question_json_public_preview";
import type {
  PleQuestionJsonNumericResponseTolerance,
  PleQuestionJsonTextResponseMatchRule,
} from "./question_json_source";

export type PleQuestionJsonInstructorAnswerCheck =
  | {
      readonly kind: "singleChoice";
      readonly correctChoiceText: string;
      readonly correctFeedback: string | null;
      readonly incorrectFeedback: string | null;
    }
  | { readonly kind: "matching"; readonly pairs: ReadonlyArray<readonly [string, string]> }
  | { readonly kind: "multipleAnswer"; readonly correctChoiceTexts: ReadonlyArray<string> }
  | {
      readonly kind: "fillIn";
      readonly answers: ReadonlyArray<string>;
      readonly matchMode: PleQuestionJsonTextResponseMatchRule;
    }
  | {
      readonly kind: "multiFillIn";
      readonly blanks: ReadonlyArray<{
        readonly label: string;
        readonly answers: ReadonlyArray<string>;
      }>;
    }
  | {
      readonly kind: "numeric";
      readonly answer: number;
      readonly tolerance: PleQuestionJsonNumericResponseTolerance;
      readonly unit: string | null;
    }
  | {
      readonly kind: "ordering";
      readonly items: ReadonlyArray<{ readonly id: string; readonly text: string }>;
    };

export interface PleQuestionJsonPreviewProps {
  /** This PLE Question JSON Public Preview contains no correct answer and is safe for the student-equivalent preview. */
  readonly preview: PleQuestionJsonPublicPreview;
  readonly validator: Pick<WasmFacade, "validateResponseFormat">;
  /** The page exposes this only after the author explicitly asks to inspect the Answer Key. */
  readonly instructorAnswerCheck?: PleQuestionJsonInstructorAnswerCheck;
}

function InstructorAnswerCheck(props: {
  readonly check: PleQuestionJsonInstructorAnswerCheck;
}): JSX.Element {
  if (props.check.kind === "matching") {
    return (
      <aside
        class="ple-question-json-authoring__instructor-check"
        aria-labelledby="ple-question-json-instructor-check-heading"
      >
        <h4 id="ple-question-json-instructor-check-heading">Instructor pairing check</h4>
        <p>This protected authoring panel is not part of the student preview.</p>
        <ul>
          <For each={props.check.pairs}>
            {([prompt, choice]) => (
              <li>
                {prompt}: {choice}
              </li>
            )}
          </For>
        </ul>
      </aside>
    );
  }
  if (props.check.kind === "multipleAnswer") {
    return (
      <PrivateAnswerList heading="Instructor answer check" items={props.check.correctChoiceTexts} />
    );
  }
  if (props.check.kind === "fillIn") {
    return (
      <PrivateAnswerList
        heading="Instructor answer check"
        items={props.check.answers}
        detail={`Text matching: ${props.check.matchMode}.`}
      />
    );
  }
  if (props.check.kind === "multiFillIn") {
    return (
      <aside
        class="ple-question-json-authoring__instructor-check"
        aria-labelledby="ple-question-json-instructor-check-heading"
      >
        <h4 id="ple-question-json-instructor-check-heading">Instructor answer check</h4>
        <p>This protected authoring panel is not part of the student preview.</p>
        <ul>
          <For each={props.check.blanks}>
            {(blank) => (
              <li>
                {blank.label}: {blank.answers.join("; ")}
              </li>
            )}
          </For>
        </ul>
      </aside>
    );
  }
  if (props.check.kind === "numeric") {
    const unit = props.check.unit === null ? "" : ` ${props.check.unit}`;
    return (
      <PrivateAnswerList
        heading="Instructor answer check"
        items={[`${props.check.answer}${unit}`]}
      />
    );
  }
  if (props.check.kind === "ordering") {
    return (
      <PrivateAnswerList
        heading="Instructor correct order"
        items={props.check.items.map((item) => item.text)}
      />
    );
  }
  return (
    <aside
      class="ple-question-json-authoring__instructor-check"
      aria-labelledby="ple-question-json-instructor-check-heading"
    >
      <h4 id="ple-question-json-instructor-check-heading">Instructor answer check</h4>
      <p>This protected authoring panel is not part of the student preview.</p>
      <p>
        <strong>Correct choice:</strong> {props.check.correctChoiceText}
      </p>
      <Show when={props.check.correctFeedback !== null}>
        <p>
          <strong>Correct Feedback:</strong> {props.check.correctFeedback}
        </p>
      </Show>
      <Show when={props.check.incorrectFeedback !== null}>
        <p>
          <strong>Incorrect Feedback:</strong> {props.check.incorrectFeedback}
        </p>
      </Show>
    </aside>
  );
}

function PrivateAnswerList(props: {
  readonly heading: string;
  readonly items: ReadonlyArray<string>;
  readonly detail?: string;
}): JSX.Element {
  return (
    <aside
      class="ple-question-json-authoring__instructor-check"
      aria-labelledby="ple-question-json-instructor-check-heading"
    >
      <h4 id="ple-question-json-instructor-check-heading">{props.heading}</h4>
      <p>This protected authoring panel is not part of the student preview.</p>
      <Show when={props.detail !== undefined}>
        <p>{props.detail}</p>
      </Show>
      <ol>
        <For each={props.items}>{(item) => <li>{item}</li>}</For>
      </ol>
    </aside>
  );
}

/** Uses the same QuestionResponseControl as a student while retaining answer checks in an explicit private panel. */
export function PleQuestionJsonPreview(props: PleQuestionJsonPreviewProps): JSX.Element {
  return (
    <section
      class="ple-question-json-authoring__preview"
      aria-labelledby="ple-question-json-student-preview-heading"
    >
      <h3 id="ple-question-json-student-preview-heading">Student preview</h3>
      <p class="ple-question-json-authoring__help">
        This is answer-free. It uses the Student Question Response Control but does not grade, save,
        or send a request.
      </p>
      <article aria-labelledby="ple-question-json-preview-title">
        <h4 id="ple-question-json-preview-title">{props.preview.title}</h4>
        <p>{props.preview.prompt}</p>
        <QuestionResponseControl
          attemptId="ple-question-json-author-preview"
          definition={props.preview.response}
          validator={props.validator}
          onEscape={() => undefined}
          onSubmit={() => Promise.resolve({ kind: "accepted" })}
        />
      </article>
      <Show when={props.instructorAnswerCheck}>
        {(check) => <InstructorAnswerCheck check={check()} />}
      </Show>
    </section>
  );
}
