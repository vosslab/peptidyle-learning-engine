// student_feedback_panel.tsx - accessible display of server-disclosed Student Feedback.

import {
  createEffect,
  createSignal,
  createUniqueId,
  For,
  onCleanup,
  Show,
  type JSX,
} from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentFeedback } from "../../generated/api/StudentFeedback";
import type { AssignmentScoringState } from "../../generated/api/AssignmentScoringState";
import { formatPointScore, formatScoreValue } from "../score_format";

import {
  recoverProtectedAssetImage,
  resolveSameOriginAssetUrl,
  type AssetUrlResolver,
} from "./question_renderer";
import { STUDENT_FEEDBACK_PANEL_STYLES } from "./student_feedback_panel_styles";

/**
 * An explicit presentation state prevents a null withheld record from looking like an empty,
 * released teaching response. The server is the only authority that constructs this union.
 */
export type StudentFeedbackPresentation =
  | {
      readonly kind: "awaiting";
      readonly feedback: null;
      readonly assignmentScoringState: AssignmentScoringState;
    }
  | {
      readonly kind: "released";
      readonly feedback: StudentFeedback;
      readonly assignmentScoringState: AssignmentScoringState;
    };

export interface StudentFeedbackPanelProps {
  readonly disclosure: StudentFeedbackPresentation;
  /** A server-projected record of what the student submitted, never a Question Revision. */
  readonly studentResponse?: ReadonlyArray<QuestionContentBlock>;
  /** Resolves logical, public asset references without exposing storage locations. */
  readonly assetUrl: AssetUrlResolver;
  /** Omit on read-only history surfaces so static Student Feedback adds no no-op tab stop. */
  readonly onAdvance?: () => void;
  readonly advanceLabel?: string;
  /** Gives assistive technology time to announce Student Feedback before the advance control receives focus. */
  readonly focusAdvanceDelayMs?: number;
}

function assertNever(value: never): never {
  throw new Error(`Unknown feedback block: ${JSON.stringify(value)}`);
}

function outcomeHeading(disclosure: StudentFeedbackPresentation): string {
  if (disclosure.kind === "awaiting" || disclosure.assignmentScoringState !== "current") {
    return "Response recorded";
  }
  if (disclosure.feedback.correctness === true) {
    return "Correct";
  }
  if (disclosure.feedback.correctness === false) {
    return "Not quite";
  }
  return "Your response was recorded";
}

/** Exposed for focused behavior tests and so the neutral copy stays consistent with the heading. */
export function studentFeedbackAnnouncement(disclosure: StudentFeedbackPresentation): string {
  if (disclosure.assignmentScoringState === "recalculating") {
    return "Your response was recorded. Your score is being updated.";
  }
  if (disclosure.assignmentScoringState === "failed") {
    return "Your response was recorded. Your score is waiting for instructor review.";
  }
  if (disclosure.kind === "awaiting") {
    return "Your response was recorded. Student Feedback is not available for this response.";
  }
  return `Student Feedback released. ${outcomeHeading(disclosure)}.`;
}

function hasBlocks(blocks: ReadonlyArray<QuestionContentBlock> | undefined): boolean {
  return blocks !== undefined && blocks.length > 0;
}

function StudentFeedbackBlock(props: {
  readonly block: QuestionContentBlock;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  switch (props.block.kind) {
    case "text":
      return <p>{props.block.markdown}</p>;
    case "math":
      return (
        <p>
          <span class="student-feedback-panel__math" aria-label={props.block.description}>
            {props.block.latex}
          </span>
        </p>
      );
    case "image":
      return (
        <figure>
          <img
            class="student-feedback-panel__image"
            src={resolveSameOriginAssetUrl(props.block.asset, props.assetUrl)}
            alt={props.block.description}
            onError={recoverProtectedAssetImage}
          />
          <figcaption>{props.block.description}</figcaption>
        </figure>
      );
    case "code":
      return (
        <pre class="student-feedback-panel__code">
          <code data-language={props.block.language}>{props.block.source}</code>
        </pre>
      );
    case "table":
      return (
        <div class="student-feedback-panel__table-wrap">
          <table class="student-feedback-panel__table">
            <caption>{props.block.description}</caption>
            <thead>
              <tr>
                <For each={props.block.headers}>{(header) => <th scope="col">{header}</th>}</For>
              </tr>
            </thead>
            <tbody>
              <For each={props.block.rows}>
                {(row) => (
                  <tr>
                    <For each={row}>{(cell) => <td>{cell}</td>}</For>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      );
    default:
      return assertNever(props.block);
  }
}

/** Renders already server-approved teaching blocks without interpreting their meaning. */
export function ContentBlockList(props: {
  readonly blocks: ReadonlyArray<QuestionContentBlock>;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  return (
    <div class="student-feedback-panel__blocks">
      <For each={props.blocks}>
        {(block) => <StudentFeedbackBlock block={block} assetUrl={props.assetUrl} />}
      </For>
    </div>
  );
}

function StudentFeedbackSection(props: {
  readonly title: string;
  readonly blocks: ReadonlyArray<QuestionContentBlock>;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  return (
    <section class="student-feedback-panel__section">
      <h3>{props.title}</h3>
      <ContentBlockList blocks={props.blocks} assetUrl={props.assetUrl} />
    </section>
  );
}

function studentFeedbackScoreText(feedback: StudentFeedback): string | undefined {
  if (feedback.pointsEarned !== undefined && feedback.pointsPossible !== undefined) {
    return `Score: ${formatPointScore(feedback.pointsEarned, feedback.pointsPossible)}`;
  }
  if (feedback.pointsEarned !== undefined) {
    return `Points earned: ${formatScoreValue(feedback.pointsEarned)}`;
  }
  if (feedback.pointsPossible !== undefined) {
    return `Points possible: ${formatScoreValue(feedback.pointsPossible)}`;
  }
  return undefined;
}

function scrollNewStudentFeedbackIntoView(heading: HTMLHeadingElement): void {
  const prefersReducedMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (!prefersReducedMotion) {
    heading.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }
}

/**
 * Displays only the server-redacted DTO and optional public Student Response Inspection Feedback.
 * It has no grading, policy, Question Revision, Answer Key, or raw Student Response dependencies.
 */
export function StudentFeedbackPanel(props: StudentFeedbackPanelProps): JSX.Element {
  const headingId = `student-feedback-panel-heading-${createUniqueId()}`;
  const [heading, setHeading] = createSignal<HTMLHeadingElement>();
  const [advance, setAdvance] = createSignal<HTMLButtonElement>();
  let focusedStudentFeedback: StudentFeedback | undefined;

  createEffect(() => {
    if (props.disclosure.kind === "awaiting") {
      focusedStudentFeedback = undefined;
      return;
    }
    if (focusedStudentFeedback === props.disclosure.feedback) {
      return;
    }
    const currentHeading = heading();
    const currentAdvance = advance();
    if (currentHeading === undefined || currentAdvance === undefined) {
      return;
    }
    focusedStudentFeedback = props.disclosure.feedback;
    currentHeading.focus({ preventScroll: true });
    scrollNewStudentFeedbackIntoView(currentHeading);
    const delay = props.focusAdvanceDelayMs ?? 250;
    const timer = globalThis.setTimeout(() => {
      if (document.activeElement === currentHeading) {
        currentAdvance.focus({ preventScroll: true });
      }
    }, delay);
    onCleanup(() => globalThis.clearTimeout(timer));
  });

  const feedback = (): StudentFeedback | undefined => {
    return props.disclosure.kind === "released" ? props.disclosure.feedback : undefined;
  };
  const advanceLabel = (): string => props.advanceLabel ?? "Continue";
  const response = (): ReadonlyArray<QuestionContentBlock> => props.studentResponse ?? [];

  return (
    <section class="student-feedback-panel" aria-labelledby={headingId}>
      <style>{STUDENT_FEEDBACK_PANEL_STYLES}</style>
      <p class="visually-hidden" role="status" aria-live="polite">
        {studentFeedbackAnnouncement(props.disclosure)}
      </p>
      <h2 id={headingId} class="student-feedback-panel__heading" ref={setHeading} tabindex="-1">
        Student Feedback
      </h2>

      <Show when={hasBlocks(response())}>
        <StudentFeedbackSection
          title="Your response"
          blocks={response()}
          assetUrl={props.assetUrl}
        />
      </Show>

      <Show when={feedback()}>
        {(released) => (
          <>
            <section class="student-feedback-panel__section">
              <h3>{outcomeHeading(props.disclosure)}</h3>
              <Show when={studentFeedbackScoreText(released())}>
                {(score) => <p class="student-feedback-panel__score">{score()}</p>}
              </Show>
              <Show
                when={
                  studentFeedbackScoreText(released()) !== undefined ||
                  hasBlocks(released().choiceFeedback) ||
                  hasBlocks(released().correctFeedback) ||
                  hasBlocks(released().incorrectFeedback) ||
                  hasBlocks(released().questionAnswer) ||
                  hasBlocks(released().questionAnswerExplanation)
                }
                fallback={
                  <p class="student-feedback-panel__empty">
                    No additional Student Feedback was provided.
                  </p>
                }
              >
                <></>
              </Show>
            </section>
            <Show when={hasBlocks(released().choiceFeedback)}>
              <StudentFeedbackSection
                title="Choice Feedback"
                blocks={released().choiceFeedback ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().correctFeedback)}>
              <StudentFeedbackSection
                title="Correct Feedback"
                blocks={released().correctFeedback ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().incorrectFeedback)}>
              <StudentFeedbackSection
                title="Incorrect Feedback"
                blocks={released().incorrectFeedback ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().questionAnswer)}>
              <StudentFeedbackSection
                title="Question Answer"
                blocks={released().questionAnswer ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().questionAnswerExplanation)}>
              <StudentFeedbackSection
                title="Answer Explanation"
                blocks={released().questionAnswerExplanation ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
          </>
        )}
      </Show>
      <Show
        when={
          props.disclosure.kind === "released" &&
          props.disclosure.assignmentScoringState !== "current"
        }
      >
        <p class="student-feedback-panel__empty">{studentFeedbackAnnouncement(props.disclosure)}</p>
      </Show>
      <Show when={props.disclosure.kind === "awaiting"}>
        <p class="student-feedback-panel__empty">{studentFeedbackAnnouncement(props.disclosure)}</p>
      </Show>

      <Show when={props.onAdvance !== undefined}>
        <button
          class="primary-action student-feedback-panel__advance"
          type="button"
          ref={setAdvance}
          onClick={() => props.onAdvance?.()}
        >
          {advanceLabel()}
        </button>
      </Show>
    </section>
  );
}
