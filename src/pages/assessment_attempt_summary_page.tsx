// assessment_attempt_summary_page.tsx - bounded, server-projected Assessment Attempt history.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentAssessmentAttemptHistory } from "../api/assessment_attempt_history";
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
  readonly questionRevision: StudentAssessmentAttemptHistory["questions"][number]["questionRevision"];
  readonly assetUrl: Parameters<typeof ContentBlockList>[0]["assetUrl"];
}): JSX.Element {
  return (
    <Show when={props.blocks}>
      {(blocks) => (
        <section class="attempt-summary__disclosure">
          <h4>{props.title}</h4>
          <ContentBlockList
            blocks={blocks()}
            questionRevision={props.questionRevision}
            assetUrl={props.assetUrl}
          />
        </section>
      )}
    </Show>
  );
}

function AssessmentAttemptHistoryContent(props: {
  readonly history: StudentAssessmentAttemptHistory;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  function assetUrlForQuestion(
    questionRevision: StudentAssessmentAttemptHistory["questions"][number]["questionRevision"],
  ): Parameters<typeof ContentBlockList>[0]["assetUrl"] {
    return (asset) =>
      new URL(
        applicationApi.client.assetUrl(questionRevision, asset.questionAsset),
        window.location.origin,
      );
  }
  return (
    <section class="page attempt-summary" data-route-surface="assessmentAttemptSummary">
      <p class="eyebrow">Previous attempt</p>
      <h1>{props.history.assessment.title}</h1>
      <p>
        Attempt {props.history.attemptNumber} is {props.history.state}.
      </p>
      <section aria-labelledby="assessment-attempt-score-heading">
        <h2 id="assessment-attempt-score-heading">Score</h2>
        <Show when={props.history.score} fallback={<p>Your score is not available.</p>}>
          {(score) => (
            <p>
              {score().pointsEarned} of {score().pointsPossible} points
            </p>
          )}
        </Show>
      </section>
      <section aria-labelledby="assessment-attempt-responses-heading">
        <h2 id="assessment-attempt-responses-heading">Your recorded work</h2>
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
                {(response) => (
                  <ContentBlockList
                    blocks={response()}
                    questionRevision={question.questionRevision}
                    assetUrl={assetUrlForQuestion(question.questionRevision)}
                  />
                )}
              </Show>
              <ReleasedBlocks
                title="Feedback"
                blocks={question.choiceFeedback}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.correctFeedback}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.incorrectFeedback}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
              <ReleasedBlocks
                title="General feedback"
                blocks={question.generalFeedback}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
              <ReleasedBlocks
                title="Correct answer"
                blocks={question.questionAnswer}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
              <ReleasedBlocks
                title="Answer explanation"
                blocks={question.questionAnswerExplanation}
                questionRevision={question.questionRevision}
                assetUrl={assetUrlForQuestion(question.questionRevision)}
              />
            </article>
          )}
        </For>
      </section>
      <A
        class="quiet-link"
        href={`/courses/${props.history.course.reference}/assessments/${props.history.assessment.reference}`}
      >
        Return to {props.history.assessment.title}
      </A>
    </section>
  );
}

/** Owns deferred scope resolution so summary state never captures an initial missing route. */
export function AssessmentAttemptSummaryPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const loadState = useRouteScopeLoadState();
  const retry = useRetryRouteScope();
  const history = (): StudentAssessmentAttemptHistory | undefined => {
    const data = routeData();
    return data?.kind === "assessmentAttemptHistory" ? data.history : undefined;
  };
  return (
    <Show
      when={history()}
      keyed
      fallback={
        <section class="page attempt-summary" data-route-surface="assessmentAttemptSummary">
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
      {(loadedHistory) => <AssessmentAttemptHistoryContent history={loadedHistory} />}
    </Show>
  );
}
