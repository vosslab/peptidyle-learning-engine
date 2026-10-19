// assessment_attempt_summary_page.tsx - bounded, server-projected Assessment Attempt history.

import { A, useParams } from "@solidjs/router";
import { createMemo, createSignal, Show, type JSX } from "solid-js";

import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { StudentAssessmentAttemptHistory } from "../../api/assessment_attempt_history";
import {
  backendAnswerReviewDocumentUrl,
  backendRecordedReviewDocumentUrl,
  recordedAuthorContentDocumentUrl,
} from "../../api/assessment_attempt_history";
import { AuthorContentFrame } from "../../components/author_content_frame";
import { OpaqueWebworkPreviewFrame } from "../../components/opaque_webwork_preview_frame";
import { ContentBlockList } from "../../components/student_feedback_panel";
import { PageFrame } from "../../components/page_frame/page_frame";
import { NativeRecordedQuestionPresentation } from "../../components/native_recorded_question_presentation";
import { useApplicationApi } from "../../api/application_api";
import { parseCourseInstanceId } from "../../navigation/public_route";
import { formatPointScore } from "../../score_format";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import {
  assessmentAttemptRecordContent,
  assessmentAttemptRecordFields,
  assessmentAttemptRecordKeyword,
  compareAssessmentAttemptRecords,
  type RecordedQuestionReview,
} from "./assessment_attempt_record_fields";
import { ASSESSMENT_ATTEMPT_SUMMARY_STYLES } from "./assessment_attempt_summary_styles";
import {
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../../ribbon/route_scope_context";

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

function NativeRecordedQuestion(props: {
  readonly assessmentAttemptId: StudentAssessmentAttemptHistory["assessmentAttemptId"];
  readonly question: StudentAssessmentAttemptHistory["questions"][number];
  readonly questionImageUrl: Parameters<typeof ContentBlockList>[0]["questionImageUrl"];
}): JSX.Element {
  const presentation = (): StudentAssessmentAttemptHistory["questions"][number]["presentation"] =>
    props.question.presentation;
  return (
    <Show when={presentation()}>
      {(issued) => (
        <>
          <NativeRecordedQuestionPresentation
            questionPosition={props.question.position}
            presentation={issued()}
            questionImageUrl={props.questionImageUrl}
          />
          <Show when={issued().authorContentDigest !== undefined}>
            <AuthorContentFrame
              assessmentAttemptId={props.assessmentAttemptId}
              position={props.question.position}
              documentUrl={recordedAuthorContentDocumentUrl(
                props.assessmentAttemptId,
                props.question.position,
              )}
              title={`Interactive content for Question ${props.question.position}`}
            />
          </Show>
        </>
      )}
    </Show>
  );
}

export function AssessmentAttemptHistoryContent(props: {
  readonly history: StudentAssessmentAttemptHistory;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [reviewVersions, setReviewVersions] = createSignal<Readonly<Record<string, number>>>({});
  const reviewKey = (position: number, review: RecordedQuestionReview): string =>
    `${position}:${review}`;
  const reviewVersion = (position: number, review: RecordedQuestionReview): number =>
    reviewVersions()[reviewKey(position, review)] ?? 1;
  function retryReview(position: number, review: RecordedQuestionReview): void {
    const key = reviewKey(position, review);
    // Remount only this opaque document; the authorized history and collection stay intact.
    setReviewVersions((versions) => ({ ...versions, [key]: (versions[key] ?? 1) + 1 }));
  }
  // ASVS 14.3.3: search only this already authorized, in-memory Attempt projection.
  const recordedQuestions = createMemo(() => props.history.questions);
  const recordedSearch = createLocalCompleteSearch(
    { rows: recordedQuestions },
    {
      fields: assessmentAttemptRecordFields,
      rowId: (question) => question.position.toString(),
      keyword: assessmentAttemptRecordKeyword,
      compare: compareAssessmentAttemptRecords,
      content: (question) =>
        assessmentAttemptRecordContent(question, (review) =>
          retryReview(question.position, review),
        ),
    },
  );
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
    >
      <style>{ASSESSMENT_ATTEMPT_SUMMARY_STYLES}</style>
      <section class="attempt-history__score" aria-labelledby="assessment-attempt-score-heading">
        <h2 id="assessment-attempt-score-heading">Score</h2>
        <Show when={props.history.score} fallback={<p>Your score is not available.</p>}>
          {(score) => (
            <p>{formatPointScore(score().pointsEarned, score().pointsPossible)} points</p>
          )}
        </Show>
      </section>
      <section aria-labelledby="assessment-attempt-responses-heading">
        <h2 id="assessment-attempt-responses-heading">Your recorded work</h2>
        <SearchBlock
          layout="above"
          controls={
            <SearchControls
              state={recordedSearch.state}
              textLabel="Search recorded Question results"
            />
          }
          state={recordedSearch.state}
          collection={{
            ariaLabel: "Recorded Question work",
            initialMode: "cards",
            ...recordedSearch.collection,
            emptyState: { title: "No recorded Questions match your search." },
            renderBody: (question) => (
              <div class="attempt-summary__question">
                <NativeRecordedQuestion
                  assessmentAttemptId={props.history.assessmentAttemptId}
                  question={question()}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <Show when={question().backendReview === "available"}>
                  <section class="attempt-summary__disclosure">
                    <h4>Question and your recorded response</h4>
                    <Show when={reviewVersion(question().position, "recorded")} keyed>
                      {(_version) => (
                        <OpaqueWebworkPreviewFrame
                          class="attempt-history__answer-review"
                          src={backendRecordedReviewDocumentUrl(
                            props.history.assessmentAttemptId,
                            question().position,
                          )}
                          title="Question and your recorded response"
                        />
                      )}
                    </Show>
                  </section>
                </Show>
                <Show
                  when={
                    question().responseState === "submitted" &&
                    question().backendReview === undefined
                  }
                >
                  <section class="attempt-history__response">
                    <h4>Recorded response</h4>
                    <Show
                      when={question().response}
                      fallback={<p>Your response is not available.</p>}
                    >
                      {(response) => (
                        <ContentBlockList
                          blocks={response()}
                          publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                          questionImageUrl={questionImageUrlForQuestion(
                            question().publishedQuestionRevisionTuple,
                          )}
                        />
                      )}
                    </Show>
                  </section>
                </Show>
                <ReleasedBlocks
                  title="Feedback"
                  blocks={question().choiceFeedback}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Feedback"
                  blocks={question().correctFeedback}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Feedback"
                  blocks={question().incorrectFeedback}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Hints"
                  blocks={question().hints}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Worked Solution"
                  blocks={question().workedSolution}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="General feedback"
                  blocks={question().generalFeedback}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Correct answer"
                  blocks={question().questionAnswer}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <ReleasedBlocks
                  title="Answer explanation"
                  blocks={question().questionAnswerExplanation}
                  publishedQuestionRevisionTuple={question().publishedQuestionRevisionTuple}
                  questionImageUrl={questionImageUrlForQuestion(
                    question().publishedQuestionRevisionTuple,
                  )}
                />
                <Show when={question().backendAnswerReview === "available"}>
                  <section class="attempt-summary__disclosure">
                    <h4>Correct answer</h4>
                    <Show when={reviewVersion(question().position, "answer")} keyed>
                      {(_version) => (
                        <OpaqueWebworkPreviewFrame
                          class="attempt-history__answer-review"
                          src={backendAnswerReviewDocumentUrl(
                            props.history.assessmentAttemptId,
                            question().position,
                          )}
                          title="Correct answer"
                        />
                      )}
                    </Show>
                  </section>
                </Show>
              </div>
            ),
          }}
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
  const params = useParams();
  const courseInstanceId = (): ReturnType<typeof parseCourseInstanceId> =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
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
        >
          <Show
            when={loadState() === "rejected"}
            fallback={
              <Show
                when={loadState() !== "unavailable"}
                fallback={
                  <>
                    <p>No Attempt is selected. Open the Course to choose an Assessment.</p>
                    <Show when={courseInstanceId()}>
                      {(course) => (
                        <A href={`/student/courses/${course()}`}>Return to this Course</A>
                      )}
                    </Show>
                  </>
                }
              >
                <p class="loading-state" role="status">
                  Loading your recorded work...
                </p>
              </Show>
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
