// assessment_attempt_summary_page.tsx - bounded, server-projected Assessment Attempt history.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentAssessmentAttemptHistory } from "../api/assessment_attempt_history";
import { backendAnswerReviewDocumentUrl } from "../api/assessment_attempt_history";
import { OpaqueWebworkPreviewFrame } from "../components/opaque_webwork_preview_frame";
import { ContentBlockList } from "../components/student_feedback_panel";
import { useApplicationApi } from "../api/application_api";
import { ASSESSMENT_ATTEMPT_SUMMARY_STYLES } from "./assessment_attempt_summary_styles";
import {
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";

function ReleasedBlocks(props: {
  readonly title: string;
  readonly blocks: ReadonlyArray<QuestionContentBlock> | undefined;
  readonly questionRevisionTuple: StudentAssessmentAttemptHistory["questions"][number]["questionRevisionTuple"];
  readonly assetUrl: Parameters<typeof ContentBlockList>[0]["assetUrl"];
}): JSX.Element {
  return (
    <Show when={props.blocks}>
      {(blocks) => (
        <section class="attempt-summary__disclosure">
          <h4>{props.title}</h4>
          <ContentBlockList
            blocks={blocks()}
            questionRevisionTuple={props.questionRevisionTuple}
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
  const retry = useRetryRouteScope();
  function assetUrlForQuestion(
    questionRevisionTuple: StudentAssessmentAttemptHistory["questions"][number]["questionRevisionTuple"],
  ): Parameters<typeof ContentBlockList>[0]["assetUrl"] {
    return (asset) =>
      new URL(
        applicationApi.client.assetUrl(questionRevisionTuple, asset.questionAssetId),
        window.location.origin,
      );
  }
  return (
    <section
      class="page attempt-summary attempt-history"
      data-route-surface="assessmentAttemptSummary"
    >
      <style>{ASSESSMENT_ATTEMPT_SUMMARY_STYLES}</style>
      <p class="eyebrow">Previous attempt</p>
      <h1>{props.history.assessment.title}</h1>
      <p>
        Attempt {props.history.attemptNumber} is {props.history.state}.
      </p>
      <section class="attempt-history__score" aria-labelledby="assessment-attempt-score-heading">
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
              <header class="attempt-history__question-header">
                <h3>Question {question.position}</h3>
                <p class="attempt-history__result">
                  <span>{question.responseState === "closed" ? "Unanswered." : "Submitted."}</span>
                  <Show when={question.correctness !== undefined}>
                    <span>{question.correctness ? "Marked correct." : "Marked not correct."}</span>
                  </Show>
                  <Show
                    when={
                      question.pointsEarned !== undefined && question.pointsPossible !== undefined
                    }
                  >
                    <span>
                      {question.pointsEarned} of {question.pointsPossible} points
                    </span>
                  </Show>
                </p>
              </header>
              <Show when={question.responseState === "submitted"}>
                <section class="attempt-history__response">
                  <h4>Recorded response</h4>
                  <Show when={question.response} fallback={<p>Your response is not available.</p>}>
                    {(response) => (
                      <ContentBlockList
                        blocks={response()}
                        questionRevisionTuple={question.questionRevisionTuple}
                        assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
                      />
                    )}
                  </Show>
                </section>
              </Show>
              <ReleasedBlocks
                title="Feedback"
                blocks={question.choiceFeedback}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.correctFeedback}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.incorrectFeedback}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <ReleasedBlocks
                title="General feedback"
                blocks={question.generalFeedback}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <ReleasedBlocks
                title="Correct answer"
                blocks={question.questionAnswer}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <ReleasedBlocks
                title="Answer explanation"
                blocks={question.questionAnswerExplanation}
                questionRevisionTuple={question.questionRevisionTuple}
                assetUrl={assetUrlForQuestion(question.questionRevisionTuple)}
              />
              <Show when={question.backendAnswerReview === "available"}>
                <section class="attempt-summary__disclosure">
                  <h4>Correct answer</h4>
                  <OpaqueWebworkPreviewFrame
                    class="attempt-history__answer-review"
                    src={backendAnswerReviewDocumentUrl(
                      props.history.assessmentAttemptId,
                      question.position,
                    )}
                    title="Correct answer"
                  />
                  <button class="quiet-action" type="button" onClick={retry}>
                    Retry correct answer
                  </button>
                </section>
              </Show>
            </article>
          )}
        </For>
      </section>
      <A
        class="quiet-link"
        href={`/courses/${props.history.course.id}/assessments/${props.history.assessment.id}`}
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
