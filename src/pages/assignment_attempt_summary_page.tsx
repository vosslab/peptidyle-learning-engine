// assignment_attempt_summary_page.tsx - bounded, server-projected Assignment Attempt history.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentAssignmentAttemptHistory } from "../api/assignment_attempt_history";
import { ContentBlockList } from "../components/student_feedback_panel";
import { useApplicationApi } from "../api/application_api";
import {
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";

function ReleasedBlocks(props: {
  readonly title: string;
  readonly blocks: ReadonlyArray<QuestionContentBlock> | undefined;
  readonly assetUrl: Parameters<typeof ContentBlockList>[0]["assetUrl"];
}): JSX.Element {
  return (
    <Show when={props.blocks}>
      {(blocks) => (
        <section class="attempt-summary__disclosure">
          <h4>{props.title}</h4>
          <ContentBlockList blocks={blocks()} assetUrl={props.assetUrl} />
        </section>
      )}
    </Show>
  );
}

function AssignmentAttemptHistoryContent(props: {
  readonly history: StudentAssignmentAttemptHistory;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const assetUrl: Parameters<typeof ContentBlockList>[0]["assetUrl"] = (asset) =>
    new URL(applicationApi.client.assetUrl(asset.questionAsset), window.location.origin);
  return (
    <section class="page attempt-summary" data-route-surface="assignmentAttemptSummary">
      <p class="eyebrow">Previous Assignment attempt</p>
      <h1>{props.history.assignment.title}</h1>
      <p>
        Attempt {props.history.attemptNumber} is {props.history.state}.
      </p>
      <section aria-labelledby="assignment-attempt-score-heading">
        <h2 id="assignment-attempt-score-heading">Score</h2>
        <p>
          {props.history.score === undefined
            ? "Your score is not available for this attempt."
            : `${props.history.score.pointsEarned} of ${props.history.score.pointsPossible} points`}
        </p>
      </section>
      <section aria-labelledby="assignment-attempt-responses-heading">
        <h2 id="assignment-attempt-responses-heading">Your recorded work</h2>
        <For each={props.history.questions}>
          {(question) => (
            <article class="attempt-summary__question">
              <h3>Question {question.position}</h3>
              <p>
                {question.responseState === "closed" ? "This question is closed." : "Submitted."}
              </p>
              <Show when={question.correctness !== undefined}>
                <p>{question.correctness ? "Marked correct." : "Marked not correct."}</p>
              </Show>
              <Show
                when={question.pointsEarned !== undefined && question.pointsPossible !== undefined}
              >
                <p>
                  {question.pointsEarned} of {question.pointsPossible} points
                </p>
              </Show>
              <Show when={question.response} fallback={<p>Your response is not available.</p>}>
                {(response) => <ContentBlockList blocks={response()} assetUrl={assetUrl} />}
              </Show>
              <ReleasedBlocks
                title="Feedback"
                blocks={question.choiceFeedback}
                assetUrl={assetUrl}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.correctFeedback}
                assetUrl={assetUrl}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.incorrectFeedback}
                assetUrl={assetUrl}
              />
              <ReleasedBlocks
                title="Correct answer"
                blocks={question.questionAnswer}
                assetUrl={assetUrl}
              />
              <ReleasedBlocks
                title="Answer explanation"
                blocks={question.questionAnswerExplanation}
                assetUrl={assetUrl}
              />
            </article>
          )}
        </For>
      </section>
      <A
        class="quiet-link"
        href={`/courses/${props.history.course.reference}/assignments/${props.history.assignment.reference}`}
      >
        Return to {props.history.assignment.title}
      </A>
    </section>
  );
}

/** Owns deferred scope resolution so summary state never captures an initial missing route. */
export function AssignmentAttemptSummaryPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const loadState = useRouteScopeLoadState();
  const retry = useRetryRouteScope();
  const history = (): StudentAssignmentAttemptHistory | undefined => {
    const data = routeData();
    return data?.kind === "assignmentAttemptHistory" ? data.history : undefined;
  };
  return (
    <Show
      when={history()}
      keyed
      fallback={
        <section class="page attempt-summary" data-route-surface="assignmentAttemptSummary">
          <Show
            when={loadState() === "rejected"}
            fallback={
              <p class="loading-state" role="status">
                Loading your recorded work...
              </p>
            }
          >
            <p class="inline-error" role="alert">
              Your recorded work could not be loaded.
            </p>
            <button class="quiet-action" type="button" onClick={retry}>
              Try again
            </button>
          </Show>
        </section>
      }
    >
      {(loadedHistory) => <AssignmentAttemptHistoryContent history={loadedHistory} />}
    </Show>
  );
}
