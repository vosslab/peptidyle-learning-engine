// flat_question_preview.tsx - answer-free local preview through the learner response runtime.

import { For, Show, type JSX } from "solid-js";

import { ResponseWidget } from "../../components/response_widget";
import type { WasmFacade } from "../../wasm/index";
import type { FlatQuestionPublicPreview } from "./flat_question_public_preview";
import type {
  FlatQuestionNumericTolerance,
  FlatQuestionTextMatchMode,
} from "./flat_question_source";

export type FlatQuestionInstructorAnswerCheck =
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
      readonly matchMode: FlatQuestionTextMatchMode;
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
      readonly tolerance: FlatQuestionNumericTolerance;
      readonly unit: string | null;
    }
  | {
      readonly kind: "ordering";
      readonly items: ReadonlyArray<{ readonly id: string; readonly text: string }>;
    };

export interface FlatQuestionPreviewProps {
  /** This projection contains no correct answer and is safe for the learner-equivalent preview. */
  readonly preview: FlatQuestionPublicPreview;
  readonly validator: Pick<WasmFacade, "validateResponseFormat">;
  /** The page exposes this only after the author explicitly asks to inspect answer material. */
  readonly instructorAnswerCheck?: FlatQuestionInstructorAnswerCheck;
}

function InstructorAnswerCheck(props: {
  readonly check: FlatQuestionInstructorAnswerCheck;
}): JSX.Element {
  if (props.check.kind === "matching") {
    return (
      <aside
        class="flat-question-authoring__instructor-check"
        aria-labelledby="flat-instructor-check-heading"
      >
        <h4 id="flat-instructor-check-heading">Instructor pairing check</h4>
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
        class="flat-question-authoring__instructor-check"
        aria-labelledby="flat-instructor-check-heading"
      >
        <h4 id="flat-instructor-check-heading">Instructor answer check</h4>
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
      class="flat-question-authoring__instructor-check"
      aria-labelledby="flat-instructor-check-heading"
    >
      <h4 id="flat-instructor-check-heading">Instructor answer check</h4>
      <p>This protected authoring panel is not part of the student preview.</p>
      <p>
        <strong>Correct choice:</strong> {props.check.correctChoiceText}
      </p>
      <Show when={props.check.correctFeedback !== null}>
        <p>
          <strong>Correct-answer feedback:</strong> {props.check.correctFeedback}
        </p>
      </Show>
      <Show when={props.check.incorrectFeedback !== null}>
        <p>
          <strong>Incorrect-answer feedback:</strong> {props.check.incorrectFeedback}
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
      class="flat-question-authoring__instructor-check"
      aria-labelledby="flat-instructor-check-heading"
    >
      <h4 id="flat-instructor-check-heading">{props.heading}</h4>
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

/** Uses the same ResponseWidget as a learner while retaining answer checks in an explicit private panel. */
export function FlatQuestionPreview(props: FlatQuestionPreviewProps): JSX.Element {
  return (
    <section
      class="flat-question-authoring__preview"
      aria-labelledby="flat-student-preview-heading"
    >
      <h3 id="flat-student-preview-heading">Student preview</h3>
      <p class="flat-question-authoring__help">
        This is answer-free. It uses the learner response control but does not grade, save, or send
        a request.
      </p>
      <article aria-labelledby="flat-preview-title">
        <h4 id="flat-preview-title">{props.preview.title}</h4>
        <p>{props.preview.prompt}</p>
        <ResponseWidget
          attemptId="flat-question-author-preview"
          definition={props.preview.response}
          validator={props.validator}
          onEscape={() => undefined}
          onSubmit={() => Promise.resolve()}
        />
      </article>
      <Show when={props.instructorAnswerCheck}>
        {(check) => <InstructorAnswerCheck check={check()} />}
      </Show>
    </section>
  );
}
