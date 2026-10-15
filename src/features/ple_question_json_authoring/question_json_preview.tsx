// question_json_preview.tsx - protected Instructor Answer checks beside the saved Draft preview.

import { For, Show, type JSX } from "solid-js";

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

export function PleQuestionJsonInstructorAnswerCheckView(props: {
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
