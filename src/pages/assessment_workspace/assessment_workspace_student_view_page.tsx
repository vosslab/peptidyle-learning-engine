// Instructor Student View: one server-authorized manifest and one selected Question read.

import { A } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createMemo,
  createResource,
  createSignal,
  type JSX,
} from "solid-js";

import type { InstructorStudentView } from "../../../generated/api/InstructorStudentView";
import type { InstructorStudentViewEntry } from "../../../generated/api/InstructorStudentViewEntry";
import type { InstructorStudentViewQuestion } from "../../../generated/api/InstructorStudentViewQuestion";
import type { StudentQuestionPresentation } from "../../../generated/api/StudentQuestionPresentation";
import { useApplicationApi } from "../../api/application_api";
import { ApiRequestError, AssessmentConflictError } from "../../api/http_client";
import { OpaqueWebworkPreviewFrame } from "../../components/opaque_webwork_preview_frame";
import { createDisplayDateTimeFormatter } from "../../format_datetime";
import { PageFrame } from "../../components/page_frame";
import {
  QuestionPresentationRenderer,
  type QuestionImageUrlResolver,
} from "../../components/question_renderer";
import { QuestionPresentationResponseControl } from "../../components/question_response_controls/question_response_control";
import {
  StudentAssessmentStartFacts,
  formatAssessmentDeliveryTime,
  formatAssessmentLimit,
  formatLateWorkRule,
} from "../../components/student_assessment_presentation";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";

interface StudentViewQuestion extends InstructorStudentViewQuestion {
  readonly authoredPosition: number;
}

type LoadResult<T> =
  | { readonly kind: "ready"; readonly value: T }
  | { readonly kind: "error"; readonly error: unknown };

const PREVIEW_VALIDATOR = {
  validateResponseFormat: (): Promise<{ readonly issues: [] }> => Promise.resolve({ issues: [] }),
};

function isUnavailable(error: unknown): boolean {
  return error instanceof ApiRequestError && [401, 403, 404].includes(error.status);
}

function isQuestionUnavailable(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 503;
}

function flattenQuestions(
  entries: ReadonlyArray<InstructorStudentViewEntry>,
): StudentViewQuestion[] {
  const questions: StudentViewQuestion[] = [];
  for (const entry of entries) {
    if (entry.kind !== "presented") continue;
    for (const question of entry.questions) {
      questions.push({ ...question, authoredPosition: entry.authoredPosition });
    }
  }
  return questions.sort((left, right) => left.position - right.position);
}

function PreviewCue(): JSX.Element {
  return (
    <aside class="student-view-cue" role="note" aria-label="Student View preview">
      <strong>Student View preview</strong>
      <span>
        This is the current Student-facing Assessment without answers. It creates no Student Work,
        Assessment Attempt, submission, or grade.
      </span>
    </aside>
  );
}

function assessmentStatusLabel(status: InstructorStudentView["status"]): string {
  switch (status) {
    case "unreleased":
      return "Unreleased";
    case "released":
      return "Released";
    case "closed":
      return "Closed";
    case "archived":
      return "Archived";
  }
}

function LoadingState(props: { readonly label: string }): JSX.Element {
  return (
    <p class="loading-state" role="status">
      {props.label}
    </p>
  );
}

function ErrorState(props: {
  readonly unavailable: boolean;
  readonly retry: () => void;
}): JSX.Element {
  return (
    <section class="route-error" role="alert">
      <h2>{props.unavailable ? "Student View unavailable" : "Student View could not load"}</h2>
      <p>
        {props.unavailable
          ? "This Assessment is not available through your current Course access."
          : "The current answer-free preview could not be loaded."}
      </p>
      <Show when={!props.unavailable}>
        <button class="primary-action" type="button" onClick={props.retry}>
          Retry Student View
        </button>
      </Show>
    </section>
  );
}

function DeliveryPolicy(props: {
  readonly manifest: InstructorStudentView;
  readonly questionCount: number;
}): JSX.Element {
  const formatDateTime = createDisplayDateTimeFormatter(props.manifest.displayTimeZone);
  return (
    <div class="student-view-assessment-summary">
      <p class="eyebrow">Assessment overview</p>
      <Show when={props.manifest.instructions.length > 0}>
        <section aria-labelledby="student-view-instructions">
          <h2 id="student-view-instructions">Instructions</h2>
          <p class="plain-text-instructions">{props.manifest.instructions}</p>
        </section>
      </Show>
      <StudentAssessmentStartFacts
        questionCount={props.questionCount}
        timeLimitSeconds={props.manifest.delivery.assessment_attempt_time_limit_seconds}
      />
      <section aria-labelledby="student-view-delivery-heading">
        <h2 id="student-view-delivery-heading">Current delivery policy</h2>
        <dl class="assessment-facts">
          <div>
            <dt>Status</dt>
            <dd>{assessmentStatusLabel(props.manifest.status)}</dd>
          </div>
          <div>
            <dt>Available</dt>
            <dd>
              {formatAssessmentDeliveryTime(props.manifest.delivery.available_at, formatDateTime)}
            </dd>
          </div>
          <div>
            <dt>Due</dt>
            <dd>{formatAssessmentDeliveryTime(props.manifest.delivery.due_at, formatDateTime)}</dd>
          </div>
          <div>
            <dt>Closes</dt>
            <dd>
              {formatAssessmentDeliveryTime(props.manifest.delivery.closes_at, formatDateTime)}
            </dd>
          </div>
          <div>
            <dt>Attempt limit</dt>
            <dd>
              {formatAssessmentLimit(props.manifest.delivery.attempt_limit, "attempt", "attempts")}
            </dd>
          </div>
          <div>
            <dt>Late work</dt>
            <dd>{formatLateWorkRule(props.manifest.delivery.late_work_rule)}</dd>
          </div>
        </dl>
      </section>
    </div>
  );
}

function PreviewDocument(props: { readonly src: string; readonly position: number }): JSX.Element {
  return (
    <OpaqueWebworkPreviewFrame
      class="student-view-question-document"
      src={props.src}
      title={`Question ${props.position} preview document`}
    />
  );
}

function NativeQuestionPreview(props: {
  readonly presentation: StudentQuestionPresentation;
  readonly position: number;
  readonly questionImageUrl: QuestionImageUrlResolver;
}): JSX.Element {
  if (props.presentation.response.kind === "imathasQuestionBackend") {
    return (
      <>
        <QuestionPresentationRenderer
          presentation={props.presentation}
          questionImageUrl={props.questionImageUrl}
        />
        <p class="calm-status" role="status">
          The iMathAS interactive response is not configured for Instructor Student View.
        </p>
      </>
    );
  }
  return (
    <>
      <QuestionPresentationRenderer
        presentation={props.presentation}
        questionImageUrl={props.questionImageUrl}
      />
      <fieldset class="student-view-disabled-response" disabled aria-label="Preview response">
        <legend>Student response preview</legend>
        <div inert aria-disabled="true">
          <QuestionPresentationResponseControl
            attemptId={`student-view-${props.position}`}
            publishedQuestionRevisionTuple={props.presentation.publishedQuestionRevisionTuple}
            questionImageUrl={props.questionImageUrl}
            mode="formatOnly"
            responseFormat={props.presentation.response}
            validator={PREVIEW_VALIDATOR}
            onEscape={() => undefined}
          />
        </div>
      </fieldset>
    </>
  );
}

/** Presents only the server-authorized, answer-free, no-write Student View projection. */
export function AssessmentWorkspaceStudentViewPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const [selectedPosition, setSelectedPosition] = createSignal(1);
  const [manifestLoad, { refetch: refetchManifest }] = createResource(
    async (): Promise<LoadResult<InstructorStudentView>> => {
      try {
        const value = await applicationApi.client.getInstructorStudentView(
          workspace.courseInstanceId,
          workspace.assessmentId,
        );
        return { kind: "ready", value };
      } catch (error: unknown) {
        return { kind: "error", error };
      }
    },
  );
  const manifest = createMemo(() => {
    const result = manifestLoad();
    return result?.kind === "ready" ? result.value : undefined;
  });
  const manifestError = createMemo(() => {
    const result = manifestLoad();
    return result?.kind === "error" ? result.error : undefined;
  });
  const questions = createMemo(() => flattenQuestions(manifest()?.entries ?? []));
  const selectedQuestion = createMemo(() =>
    questions().find((question) => question.position === selectedPosition()),
  );
  const [presentation, { refetch: refetchPresentation }] = createResource(
    selectedQuestion,
    async (question) => {
      const currentManifest = manifest();
      if (currentManifest === undefined) throw new Error("Student View manifest is unavailable");
      return applicationApi.client.getInstructorStudentViewQuestion(
        workspace.courseInstanceId,
        workspace.assessmentId,
        question.authoredPosition,
        question.publishedQuestionRevisionTuple,
        currentManifest.assessmentEditNumber,
      );
    },
  );

  function reloadPreview(): void {
    setSelectedPosition(1);
    void refetchManifest();
  }

  function selectPrevious(): void {
    if (selectedPosition() > 1) setSelectedPosition(selectedPosition() - 1);
  }

  function selectNext(): void {
    if (selectedPosition() < questions().length) setSelectedPosition(selectedPosition() + 1);
  }

  function retryQuestion(): void {
    void refetchPresentation();
  }

  const workspacePath = assessmentWorkspacePath(workspace.courseInstanceId, workspace.assessmentId);

  return (
    <PageFrame
      contentClass="assessment-workspace-student-view"
      routeSurface="assessmentWorkspace"
      title={manifest()?.title ?? "Student View"}
      lede="Answer-free preview for the current Student-facing Assessment."
    >
      <PreviewCue />
      <A class="quiet-link" href={workspacePath}>
        Return to assessment
      </A>
      <Switch>
        <Match when={manifestLoad.loading}>
          <LoadingState label="Loading Student View..." />
        </Match>
        <Match when={manifestError() !== undefined}>
          <ErrorState
            unavailable={isUnavailable(manifestError())}
            retry={() => void refetchManifest()}
          />
        </Match>
        <Match when={manifest()}>
          {(readyManifest) => (
            <>
              <DeliveryPolicy manifest={readyManifest()} questionCount={questions().length} />
              <Show when={readyManifest().entries.some((entry) => entry.kind === "notShown")}>
                <section aria-labelledby="student-view-unavailable-heading">
                  <h2 id="student-view-unavailable-heading">Authored entries not shown</h2>
                  <ul class="student-view-unavailable-entries">
                    <For
                      each={readyManifest().entries.filter((entry) => entry.kind === "notShown")}
                    >
                      {(entry) => (
                        <li>
                          Entry {entry.authoredPosition + 1}: unavailable for Student delivery
                        </li>
                      )}
                    </For>
                  </ul>
                </section>
              </Show>
              <Show
                when={questions().length > 0}
                fallback={<p class="empty-state">No Questions are currently shown to Students.</p>}
              >
                <section
                  class="student-view-question-area"
                  aria-labelledby="student-view-question-heading"
                >
                  <div class="student-view-question-navigation">
                    <h2 id="student-view-question-heading">
                      Question {selectedPosition()} of {questions().length}
                    </h2>
                    <div class="student-view-question-buttons" role="group" aria-label="Questions">
                      <For each={questions()}>
                        {(question) => (
                          <button
                            type="button"
                            class="quiet-action"
                            classList={{ selected: question.position === selectedPosition() }}
                            aria-current={
                              question.position === selectedPosition() ? "true" : undefined
                            }
                            onClick={() => setSelectedPosition(question.position)}
                          >
                            {question.position}
                          </button>
                        )}
                      </For>
                    </div>
                  </div>
                  <Switch>
                    <Match when={presentation.loading && presentation.error === undefined}>
                      <LoadingState label="Loading Question preview..." />
                    </Match>
                    <Match when={presentation.error !== undefined}>
                      <section class="route-error" role="alert">
                        <h3>
                          {presentation.error instanceof AssessmentConflictError
                            ? "Preview is out of date"
                            : isQuestionUnavailable(presentation.error)
                              ? "Question unavailable in Student View"
                              : "Question preview could not load"}
                        </h3>
                        <p>
                          {presentation.error instanceof AssessmentConflictError
                            ? "The Assessment changed. Reload Student View before continuing."
                            : isQuestionUnavailable(presentation.error)
                              ? "This Question's answer-free presentation is not currently available in Student View."
                              : "Try loading this Question again."}
                        </p>
                        <button
                          class="primary-action"
                          type="button"
                          onClick={
                            presentation.error instanceof AssessmentConflictError
                              ? reloadPreview
                              : retryQuestion
                          }
                        >
                          {presentation.error instanceof AssessmentConflictError
                            ? "Reload current preview"
                            : "Retry Question"}
                        </button>
                      </section>
                    </Match>
                    <Match when={presentation()}>
                      {(readyPresentation) => {
                        const question = selectedQuestion();
                        const usesDocument =
                          readyPresentation().response.kind === "backendOwned" ||
                          readyPresentation().authorContentDigest !== undefined;
                        return question === undefined ? null : (
                          <div class="student-view-question-preview">
                            <Show
                              when={usesDocument}
                              fallback={
                                <NativeQuestionPreview
                                  presentation={readyPresentation()}
                                  position={question.position}
                                  questionImageUrl={(asset) =>
                                    new URL(
                                      applicationApi.client.questionImageUrl(
                                        readyPresentation().publishedQuestionRevisionTuple,
                                        asset.questionImageAssetId,
                                      ),
                                      globalThis.location.origin,
                                    )
                                  }
                                />
                              }
                            >
                              <PreviewDocument
                                position={question.position}
                                src={applicationApi.client.instructorStudentViewQuestionDocumentUrl(
                                  workspace.courseInstanceId,
                                  workspace.assessmentId,
                                  question.authoredPosition,
                                  question.publishedQuestionRevisionTuple,
                                  readyManifest().assessmentEditNumber,
                                )}
                              />
                            </Show>
                          </div>
                        );
                      }}
                    </Match>
                  </Switch>
                  <div
                    class="student-view-question-pager"
                    role="group"
                    aria-label="Question navigation"
                  >
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={selectedPosition() <= 1}
                      onClick={selectPrevious}
                    >
                      Previous Question
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={selectedPosition() >= questions().length}
                      onClick={selectNext}
                    >
                      Next Question
                    </button>
                  </div>
                </section>
              </Show>
            </>
          )}
        </Match>
      </Switch>
    </PageFrame>
  );
}
