// assessment_attempt_summary_page.tsx - bounded, server-projected Assessment Attempt history.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { StudentAssessmentAttemptHistory } from "../api/assessment_attempt_history";
import { backendAnswerReviewDocumentUrl } from "../api/assessment_attempt_history";
import { OpaqueWebworkPreviewFrame } from "../components/opaque_webwork_preview_frame";
import { RecordDetailList } from "../components/record_list/record_detail_list";
import { ContentBlockList } from "../components/student_feedback_panel";
import { PageFrame } from "../components/page_frame";
import { useApplicationApi } from "../api/application_api";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { ASSESSMENT_ATTEMPT_SUMMARY_STYLES } from "./assessment_attempt_summary_styles";
import {
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";

function ReleasedBlocks(props: {
  readonly title: string;
  readonly blocks: ReadonlyArray<QuestionContentBlock> | undefined;
  readonly publishedQuestionRevisionTuple: StudentAssessmentAttemptHistory["questions"][number]["publishedQuestionRevisionTuple"];
  readonly questionImageUrl: Parameters<typeof ContentBlockList>[0]["questionImageUrl"];
}): JSX.Element {
  return (
    <Show when={props.blocks}>
      {(blocks) => (
        <section class="attempt-summary__disclosure">
          <h4>{props.title}</h4>
          <ContentBlockList
            blocks={blocks()}
            publishedQuestionRevisionTuple={props.publishedQuestionRevisionTuple}
            questionImageUrl={props.questionImageUrl}
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
  function questionImageUrlForQuestion(
    publishedQuestionRevisionTuple: StudentAssessmentAttemptHistory["questions"][number]["publishedQuestionRevisionTuple"],
  ): Parameters<typeof ContentBlockList>[0]["questionImageUrl"] {
    return (asset) =>
      new URL(
        applicationApi.client.questionImageUrl(
          publishedQuestionRevisionTuple,
          asset.questionImageAssetId,
        ),
        window.location.origin,
      );
  }
  return (
    <PageFrame
      // Recorded-work notice treatment on the content region. PageFrame owns the stack.
      contentClass="attempt-summary"
      routeSurface="assessmentAttemptSummary"
      eyebrow={`${assessmentTypePresentation(props.history.assessment.assessmentType).label} · Attempt ${props.history.attemptNumber}`}
      title={props.history.assessment.title}
      lede={`This Attempt is ${props.history.state}.`}
    >
      <style>{ASSESSMENT_ATTEMPT_SUMMARY_STYLES}</style>
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
        <RecordDetailList
          ariaLabel="Recorded Question work"
          emptyState={{ title: "No recorded Questions are available." }}
          recordId={(question) => question.position.toString()}
          rows={props.history.questions}
          state={{ kind: "ready" }}
          renderRecord={(question) => (
            <div class="attempt-summary__question">
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
                        publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                        questionImageUrl={questionImageUrlForQuestion(
                          question.publishedQuestionRevisionTuple,
                        )}
                      />
                    )}
                  </Show>
                </section>
              </Show>
              <ReleasedBlocks
                title="Feedback"
                blocks={question.choiceFeedback}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.correctFeedback}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
              />
              <ReleasedBlocks
                title="Feedback"
                blocks={question.incorrectFeedback}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
              />
              <ReleasedBlocks
                title="General feedback"
                blocks={question.generalFeedback}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
              />
              <ReleasedBlocks
                title="Correct answer"
                blocks={question.questionAnswer}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
              />
              <ReleasedBlocks
                title="Answer explanation"
                blocks={question.questionAnswerExplanation}
                publishedQuestionRevisionTuple={question.publishedQuestionRevisionTuple}
                questionImageUrl={questionImageUrlForQuestion(
                  question.publishedQuestionRevisionTuple,
                )}
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
            </div>
          )}
        />
      </section>
      <A
        class="quiet-link"
        href={`/courses/${props.history.course.id}/assessments/${props.history.assessment.id}`}
      >
        Return to {props.history.assessment.title}
      </A>
    </PageFrame>
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
        <PageFrame
          // Recorded-work notice treatment on the content region. PageFrame owns the stack.
          contentClass="attempt-summary"
          routeSurface="assessmentAttemptSummary"
          title="Recorded work"
        >
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
        </PageFrame>
      }
    >
      {(loadedHistory) => <AssessmentAttemptHistoryContent history={loadedHistory} />}
    </Show>
  );
}
