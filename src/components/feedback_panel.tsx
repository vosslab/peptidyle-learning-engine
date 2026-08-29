// feedback_panel.tsx - accessible display of server-disclosed learner feedback.

import {
  createEffect,
  createSignal,
  createUniqueId,
  For,
  onCleanup,
  Show,
  type JSX,
} from "solid-js";

import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { DisclosedFeedback } from "../../generated/api/DisclosedFeedback";
import type { ScoringStatus } from "../../generated/api/ScoringStatus";
import { formatPointScore, formatScoreValue } from "../score_format";

import {
  recoverProtectedAssetImage,
  resolveSameOriginAssetUrl,
  type AssetUrlResolver,
} from "./question_renderer";
import { FEEDBACK_PANEL_STYLES } from "./feedback_panel_styles";

/**
 * An explicit presentation state prevents a null withheld record from looking like an empty,
 * released teaching response. The server is the only authority that constructs this union.
 */
export type FeedbackPresentation =
  | {
      readonly kind: "awaiting";
      readonly feedback: null;
      readonly scoringStatus: ScoringStatus;
    }
  | {
      readonly kind: "released";
      readonly feedback: DisclosedFeedback;
      readonly scoringStatus: ScoringStatus;
    };

export interface FeedbackPanelProps {
  readonly disclosure: FeedbackPresentation;
  /** A server-projected record of what the learner submitted, never a question definition. */
  readonly studentResponse?: ReadonlyArray<ContentBlock>;
  /** Resolves logical, public asset references without exposing storage locations. */
  readonly assetUrl: AssetUrlResolver;
  /** Omit on read-only history surfaces so static feedback does not add a no-op tab stop. */
  readonly onAdvance?: () => void;
  readonly advanceLabel?: string;
  /** Gives assistive technology time to announce the feedback before the advance control receives focus. */
  readonly focusAdvanceDelayMs?: number;
}

function assertNever(value: never): never {
  throw new Error(`Unknown feedback block: ${JSON.stringify(value)}`);
}

function outcomeHeading(disclosure: FeedbackPresentation): string {
  if (disclosure.kind === "awaiting" || disclosure.scoringStatus !== "current") {
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
export function feedbackAnnouncement(disclosure: FeedbackPresentation): string {
  if (disclosure.scoringStatus === "recalculating") {
    return "Your response was recorded. Your score is being updated.";
  }
  if (disclosure.scoringStatus === "failed") {
    return "Your response was recorded. Your score is waiting for instructor review.";
  }
  if (disclosure.kind === "awaiting") {
    return "Your response was recorded. Feedback is not available for this response.";
  }
  return `Feedback released. ${outcomeHeading(disclosure)}.`;
}

function hasBlocks(blocks: ReadonlyArray<ContentBlock> | undefined): boolean {
  return blocks !== undefined && blocks.length > 0;
}

function FeedbackBlock(props: {
  readonly block: ContentBlock;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  switch (props.block.kind) {
    case "text":
      return <p>{props.block.markdown}</p>;
    case "math":
      return (
        <p>
          <span class="feedback-panel__math" aria-label={props.block.description}>
            {props.block.latex}
          </span>
        </p>
      );
    case "image":
      return (
        <figure>
          <img
            class="feedback-panel__image"
            src={resolveSameOriginAssetUrl(props.block.asset, props.assetUrl)}
            alt={props.block.description}
            onError={recoverProtectedAssetImage}
          />
          <figcaption>{props.block.description}</figcaption>
        </figure>
      );
    case "code":
      return (
        <pre class="feedback-panel__code">
          <code data-language={props.block.language}>{props.block.source}</code>
        </pre>
      );
    case "table":
      return (
        <div class="feedback-panel__table-wrap">
          <table class="feedback-panel__table">
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
  readonly blocks: ReadonlyArray<ContentBlock>;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  return (
    <div class="feedback-panel__blocks">
      <For each={props.blocks}>
        {(block) => <FeedbackBlock block={block} assetUrl={props.assetUrl} />}
      </For>
    </div>
  );
}

function FeedbackSection(props: {
  readonly title: string;
  readonly blocks: ReadonlyArray<ContentBlock>;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  return (
    <section class="feedback-panel__section">
      <h3>{props.title}</h3>
      <ContentBlockList blocks={props.blocks} assetUrl={props.assetUrl} />
    </section>
  );
}

function scoreText(feedback: DisclosedFeedback): string | undefined {
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

function scrollNewFeedbackIntoView(heading: HTMLHeadingElement): void {
  const prefersReducedMotion = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (!prefersReducedMotion) {
    heading.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }
}

/**
 * Displays only the server-redacted DTO and an optional public learner-response projection.
 * It has no grading, policy, question-definition, answer-key, or raw-response dependencies.
 */
export function FeedbackPanel(props: FeedbackPanelProps): JSX.Element {
  const headingId = `feedback-panel-heading-${createUniqueId()}`;
  const [heading, setHeading] = createSignal<HTMLHeadingElement>();
  const [advance, setAdvance] = createSignal<HTMLButtonElement>();
  let focusedFeedback: DisclosedFeedback | undefined;

  createEffect(() => {
    if (props.disclosure.kind === "awaiting") {
      focusedFeedback = undefined;
      return;
    }
    if (focusedFeedback === props.disclosure.feedback) {
      return;
    }
    const currentHeading = heading();
    const currentAdvance = advance();
    if (currentHeading === undefined || currentAdvance === undefined) {
      return;
    }
    focusedFeedback = props.disclosure.feedback;
    currentHeading.focus({ preventScroll: true });
    scrollNewFeedbackIntoView(currentHeading);
    const delay = props.focusAdvanceDelayMs ?? 250;
    const timer = globalThis.setTimeout(() => {
      if (document.activeElement === currentHeading) {
        currentAdvance.focus({ preventScroll: true });
      }
    }, delay);
    onCleanup(() => globalThis.clearTimeout(timer));
  });

  const feedback = (): DisclosedFeedback | undefined => {
    return props.disclosure.kind === "released" ? props.disclosure.feedback : undefined;
  };
  const advanceLabel = (): string => props.advanceLabel ?? "Continue";
  const response = (): ReadonlyArray<ContentBlock> => props.studentResponse ?? [];

  return (
    <section class="feedback-panel" aria-labelledby={headingId}>
      <style>{FEEDBACK_PANEL_STYLES}</style>
      <p class="visually-hidden" role="status" aria-live="polite">
        {feedbackAnnouncement(props.disclosure)}
      </p>
      <h2 id={headingId} class="feedback-panel__heading" ref={setHeading} tabindex="-1">
        Feedback
      </h2>

      <Show when={hasBlocks(response())}>
        <FeedbackSection title="Your response" blocks={response()} assetUrl={props.assetUrl} />
      </Show>

      <Show when={feedback()}>
        {(released) => (
          <>
            <section class="feedback-panel__section">
              <h3>{outcomeHeading(props.disclosure)}</h3>
              <Show when={scoreText(released())}>
                {(score) => <p class="feedback-panel__score">{score()}</p>}
              </Show>
              <Show
                when={
                  scoreText(released()) !== undefined ||
                  hasBlocks(released().hint) ||
                  hasBlocks(released().correctResponse) ||
                  hasBlocks(released().rationale)
                }
                fallback={<p class="feedback-panel__empty">No additional feedback was provided.</p>}
              >
                <></>
              </Show>
            </section>
            <Show when={hasBlocks(released().hint)}>
              <FeedbackSection
                title="Hint"
                blocks={released().hint ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().correctResponse)}>
              <FeedbackSection
                title="Correct response"
                blocks={released().correctResponse ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
            <Show when={hasBlocks(released().rationale)}>
              <FeedbackSection
                title="Why this works"
                blocks={released().rationale ?? []}
                assetUrl={props.assetUrl}
              />
            </Show>
          </>
        )}
      </Show>
      <Show
        when={props.disclosure.kind === "released" && props.disclosure.scoringStatus !== "current"}
      >
        <p class="feedback-panel__empty">{feedbackAnnouncement(props.disclosure)}</p>
      </Show>
      <Show when={props.disclosure.kind === "awaiting"}>
        <p class="feedback-panel__empty">{feedbackAnnouncement(props.disclosure)}</p>
      </Show>

      <Show when={props.onAdvance !== undefined}>
        <button
          class="primary-action feedback-panel__advance"
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
