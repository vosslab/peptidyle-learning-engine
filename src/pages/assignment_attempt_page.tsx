// assignment_attempt_page.tsx - server-issued, key-free student attempt loop.

import { useNavigate } from "@solidjs/router";
import {
  createEffect,
  createSignal,
  ErrorBoundary,
  For,
  onCleanup,
  onMount,
  Show,
  type JSX,
} from "solid-js";

import type { StudentQuestionAttemptView } from "../../generated/api/StudentQuestionAttemptView";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type {
  PrefetchedNextQuestion,
  QuestionPoolSelection,
  NextIssuedAttempt,
  AssignmentAttemptScreenData,
  AssignmentAttemptSummaryOutcome,
  AssignmentAttemptSummaryResponse,
} from "../api/contracts";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import { useApplicationApi } from "../api/application_api";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import {
  assignmentRouteReference,
  courseRouteReference,
  assignmentAttemptRouteReference,
} from "../navigation/public_route";
import { QuestionRenderer } from "../components/question_renderer";
import { FeedbackPanel, type FeedbackPresentation } from "../components/feedback_panel";
import { QuestionResponseControl } from "../components/question_response_controls/question_response_control";
import { resumeSessionAndRetry } from "./assignment_attempt_page_recovery";
import {
  assignmentAttemptCompletionPresentation,
  submissionAdvanceLabel,
  type AssignmentAttemptCompletionPresentation,
} from "./assignment_attempt_completion_presentation";
import {
  createQuestionAttemptStateMachine,
  type AttemptContext,
  type QuestionAttemptExperienceState,
  type AttemptStorage,
  type SubmissionOutcome,
} from "../features/question_attempt/question_attempt_state";
import { prefetchMatchesIssuedSuccessor } from "../features/question_attempt/prefetch_binding";
import { projectStudentResponse } from "../features/question_attempt/student_response";
import type { StudentResponseFormatCheck } from "../wasm/index";
import { useWasmFacade } from "../wasm/context";
import { studentProgressSummary, studentScoreValue } from "../student_progress";

function attemptContext(
  assignmentAttemptId: string,
  attempt: StudentQuestionAttemptView,
  envelope: QuestionPresentation,
): AttemptContext {
  return {
    assignmentAttemptId,
    attemptId: attempt.id,
    issuedQuestionId: attempt.issuedQuestion,
    questionVersion: envelope.variation.questionVersion,
    seed: attempt.seed,
    deadline: attempt.timing.deadline,
  };
}

function attemptStorage(): AttemptStorage {
  return {
    getItem(key: string): string | null {
      return globalThis.sessionStorage.getItem(key);
    },
    setItem(key: string, value: string): void {
      globalThis.sessionStorage.setItem(key, value);
    },
    removeItem(key: string): void {
      globalThis.sessionStorage.removeItem(key);
    },
  };
}

function generateIdempotencyKey(): string {
  return globalThis.crypto.randomUUID();
}

function isSessionExpired(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 401;
}

/**
 * Fetch reports an unavailable browser transport as TypeError. HTTP refusals and decoded-response
 * contract failures carry their own actionable messages and must not be presented as an outage.
 */
function isTransientTransportFailure(error: unknown): boolean {
  return error instanceof TypeError;
}

function formatRemaining(milliseconds: number | null): string {
  if (milliseconds === null) return "Untimed";
  const seconds = Math.ceil(milliseconds / 1_000);
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}:${remainder.toString().padStart(2, "0")} remaining`;
}

function matchesIssuedSuccessor(
  attempt: StudentQuestionAttemptView,
  receipt: NextIssuedAttempt,
): boolean {
  return (
    attempt.id === receipt.id &&
    attempt.issuedQuestion === receipt.issuedQuestion.id &&
    attempt.seed === receipt.seed &&
    attempt.timing.deadline === receipt.deadline
  );
}

/** Avoid turning an unusually image-heavy question into an unbounded background fetch. */
const MAX_PREFETCH_ASSETS = 12;

function assetIdsForEnvelope(envelope: QuestionPresentation): ReadonlyArray<string> {
  const blocks = [...envelope.prompt];
  if (envelope.response.kind === "multipleChoice") {
    blocks.push(...envelope.response.choices.flatMap((choice) => choice.body));
  } else if (envelope.response.kind === "ordering") {
    blocks.push(...envelope.response.items.flatMap((item) => item.body));
  }
  return [
    ...new Set(blocks.filter((block) => block.kind === "image").map((block) => block.asset.asset)),
  ].slice(0, MAX_PREFETCH_ASSETS);
}

function AttemptExperience(props: {
  readonly initialScreen: AssignmentAttemptScreenData;
}): JSX.Element {
  const runtime = useApplicationApi();
  const validator = useWasmFacade();
  const navigate = useNavigate();
  const [screen, setScreen] = createSignal(props.initialScreen);
  const [state, setState] = createSignal<QuestionAttemptExperienceState>();
  const [sessionRecovery, setSessionRecovery] = createSignal(false);
  const [summaryVisible, setSummaryVisible] = createSignal(false);
  const [assignmentAttemptSummary, setAssignmentAttemptSummary] =
    createSignal<AssignmentAttemptSummaryResponse>();
  const [summaryOutcomes, setSummaryOutcomes] = createSignal<
    ReadonlyArray<AssignmentAttemptSummaryOutcome>
  >([]);
  const [summaryError, setSummaryError] = createSignal<string | null>(null);
  const [summaryLoading, setSummaryLoading] = createSignal(false);
  const seenSummaryCursors = new Set<string>();
  const [practiceError, setPracticeError] = createSignal<string | null>(null);
  const [prefetched, setPrefetched] = createSignal<PrefetchedNextQuestion | null>(null);
  const [questionPoolSelection, setQuestionPoolSelection] =
    createSignal<QuestionPoolSelection | null>(props.initialScreen.attempt.questionPoolSelection);
  let requestedPrefetchFor: string | null = null;
  let prefetchController: AbortController | null = null;
  let recoveredSuccessorScreen: AssignmentAttemptScreenData | null = null;

  const machine = createQuestionAttemptStateMachine({
    context: attemptContext(
      props.initialScreen.assignmentAttempt.id,
      props.initialScreen.attempt,
      props.initialScreen.issuedQuestion,
    ),
    storage: attemptStorage(),
    clock: { now: () => Date.now() },
    network: { isOnline: () => navigator.onLine },
    generateIdempotencyKey,
    submitResponse: (attemptId, response, idempotencyKey) =>
      runtime.client.submitResponse(
        screen().course.summary.id,
        screen().assignment.id,
        attemptId,
        response,
        idempotencyKey,
      ),
    getSubmissionStatus: (attemptId) =>
      runtime.client.getSubmissionStatus(
        screen().course.summary.id,
        screen().assignment.id,
        attemptId,
      ),
    isSessionExpired,
    isTransientTransportFailure,
    validateSavedResponse: validator.validateResponseFormat,
    onStateChange: setState,
  });
  // Establish the first answer state during component construction so the
  // response controls do not wait for a post-paint mount callback.
  machine.start(props.initialScreen.issuedQuestion.response);

  function escapeToAssignment(): void {
    navigate(
      `/courses/${courseRouteReference(screen().course.summary.reference)}/assignments/${assignmentRouteReference(screen().assignment.reference)}`,
    );
  }

  function responseChanged(response: StudentResponse, validation: StudentResponseFormatCheck): void {
    machine.setResponse(response, {
      valid: validation.violations.length === 0,
      message: validation.violations.length === 0 ? null : "Response format needs attention.",
    });
  }

  async function submit(response: StudentResponse): Promise<SubmissionOutcome> {
    // QuestionResponseControl reaches this callback only after its browser-local format validation.
    // This enables delivery, never local correctness or scoring.
    machine.setResponse(response, { valid: true, message: null });
    return machine.submit();
  }

  async function continueAttempt(): Promise<void> {
    const acknowledgement = feedbackState()?.acknowledgement;
    if (acknowledgement === undefined) return;
    if (acknowledgement.nextPending) {
      await advanceFromCurrentRun(null);
      return;
    }
    const receiptNext = acknowledgement.nextIssued;
    const cached = prefetched();
    if (
      receiptNext !== null &&
      cached !== null &&
      prefetchMatchesIssuedSuccessor(cached, receiptNext, machine.state().context.attemptId)
    ) {
      const current = machine.state().context;
      await machine.advance(() =>
        Promise.resolve({
          context: {
            ...current,
            attemptId: receiptNext.id,
            assignmentAttemptId: receiptNext.issuedQuestion.assignmentAttempt,
            issuedQuestionId: receiptNext.issuedQuestion.id,
            questionVersion: cached.envelope.variation.questionVersion,
            seed: receiptNext.seed,
            deadline: receiptNext.deadline,
          },
          envelope: cached.envelope,
        }),
      );
      setQuestionPoolSelection(cached.questionPoolSelection);
      setPrefetched(null);
      requestPrefetch(receiptNext.id);
      return;
    }
    if (receiptNext === null) {
      machine.finish(acknowledgement.assignmentAttemptCompletion);
      if (acknowledgement.assignmentAttemptCompletion === "completed") {
        navigate(
          `/assignment-attempts/${assignmentAttemptRouteReference(screen().assignmentAttempt.reference)}/summary`,
          { replace: true },
        );
        return;
      }
      setSummaryVisible(true);
      void loadSummary();
      return;
    }
    await advanceFromCurrentRun(receiptNext);
  }

  function applyRecoveredSuccessorScreen(): void {
    const recovered = recoveredSuccessorScreen;
    if (
      recovered === null ||
      machine.state().phase !== "answering" ||
      machine.state().context.attemptId !== recovered.attempt.id
    ) {
      return;
    }
    recoveredSuccessorScreen = null;
    setScreen(recovered);
    setQuestionPoolSelection(recovered.attempt.questionPoolSelection);
    setPrefetched(null);
    requestPrefetch(recovered.attempt.id);
  }

  async function advanceFromCurrentRun(expected: NextIssuedAttempt | null): Promise<void> {
    const predecessor = machine.state().context.attemptId;
    recoveredSuccessorScreen = null;
    await machine.advance(async () => {
      // Router data may still describe the submitted predecessor while the
      // server-owned successor becomes visible. Bind recovery to the predecessor
      // and, when supplied, the issued successor receipt.
      const next = await runtime.client.getAssignmentAttemptScreen(screen().assignmentAttempt.id);
      if (next.attempt.id === predecessor) {
        throw new ApiProtocolError(
          "Assignment Attempt screen still describes the submitted Question Attempt",
        );
      }
      if (expected !== null && !matchesIssuedSuccessor(next.attempt, expected)) {
        throw new ApiProtocolError(
          "Assignment Attempt screen does not match the issued successor receipt",
        );
      }
      recoveredSuccessorScreen = next;
      return {
        context: attemptContext(next.assignmentAttempt.id, next.attempt, next.issuedQuestion),
        envelope: next.issuedQuestion,
      };
    });
    applyRecoveredSuccessorScreen();
  }

  async function retryNextQuestion(): Promise<void> {
    await machine.retryAdvance();
    applyRecoveredSuccessorScreen();
  }

  function requestPrefetch(attemptId: string): void {
    if (requestedPrefetchFor === attemptId) return;
    requestedPrefetchFor = attemptId;
    prefetchController?.abort();
    const controller = new AbortController();
    prefetchController = controller;
    void runtime.client
      .prefetchNextQuestion(
        screen().course.summary.id,
        screen().assignment.id,
        attemptId,
        controller.signal,
      )
      .then((value) => {
        if (controller.signal.aborted || machine.state().context.attemptId !== attemptId) return;
        if (
          value !== null &&
          value.issuedQuestion.assignmentAttempt !== machine.state().context.assignmentAttemptId
        ) {
          throw new ApiProtocolError(
            "Prefetched Issued Question does not match the active Assignment Attempt",
          );
        }
        setPrefetched(value);
        if (value !== null) {
          for (const assetId of assetIdsForEnvelope(value.envelope)) {
            const assetUrl = new URL(runtime.client.assetUrl(assetId), window.location.origin);
            if (assetUrl.origin !== window.location.origin) continue;
            void fetch(assetUrl, {
              credentials: "same-origin",
              cache: "force-cache",
              signal: controller.signal,
            }).catch(() => undefined);
          }
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) setPrefetched(null);
        if (requestedPrefetchFor === attemptId) requestedPrefetchFor = null;
      });
  }

  async function loadSummary(cursor?: string): Promise<void> {
    if (summaryLoading()) return;
    if (cursor !== undefined && seenSummaryCursors.has(cursor)) {
      setSummaryError("This summary page was already loaded. Refresh the summary to try again.");
      return;
    }
    if (cursor === undefined) {
      seenSummaryCursors.clear();
    }
    setSummaryLoading(true);
    setSummaryError(null);
    try {
      const page = await runtime.client.getAssignmentAttemptSummary(
        screen().assignmentAttempt.id,
        cursor,
        30,
      );
      if (page.outcomes.nextCursor !== null && seenSummaryCursors.has(page.outcomes.nextCursor)) {
        throw new Error("Assignment Attempt summary repeated its cursor.");
      }
      if (cursor !== undefined) seenSummaryCursors.add(cursor);
      setAssignmentAttemptSummary(page);
      setSummaryOutcomes((existing) => {
        const prior = cursor === undefined ? [] : existing;
        const seen = new Set(prior.map((outcome) => outcome.attempt));
        return [...prior, ...page.outcomes.items.filter((outcome) => !seen.has(outcome.attempt))];
      });
    } catch {
      setSummaryError(
        "Could not refresh this Assignment Attempt summary. Your completed work remains recorded.",
      );
    } finally {
      setSummaryLoading(false);
    }
  }

  async function startAnotherPractice(): Promise<void> {
    setPracticeError(null);
    try {
      const assignmentAttempt = await runtime.client.startAssignmentAttempt(
        screen().course.summary.id,
        screen().assignment.id,
      );
      navigate(
        `/assignment-attempts/${assignmentAttemptRouteReference(assignmentAttempt.reference)}`,
      );
    } catch (error: unknown) {
      setPracticeError(
        error instanceof Error
          ? error.message
          : "Could not start another practice Assignment Attempt.",
      );
    }
  }

  async function restoreSession(): Promise<void> {
    try {
      await resumeSessionAndRetry(runtime.client.getSession, machine);
      setSessionRecovery(false);
    } catch {
      setSessionRecovery(true);
    }
  }

  function online(): void {
    void machine.retryWhenOnline();
    requestPrefetch(machine.state().context.attemptId);
  }

  onMount(() => {
    requestPrefetch(props.initialScreen.attempt.id);
    const timer = globalThis.setInterval(() => machine.tick(), 1_000);
    globalThis.addEventListener("online", online);
    onCleanup(() => {
      globalThis.clearInterval(timer);
      globalThis.removeEventListener("online", online);
      prefetchController?.abort();
      machine.dispose();
    });
  });

  const recoveringState = ():
    Extract<QuestionAttemptExperienceState, { readonly phase: "recovering" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "recovering" ? candidate : undefined;
  };
  const feedbackState = ():
    Extract<QuestionAttemptExperienceState, { readonly phase: "feedback" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "feedback" ? candidate : undefined;
  };
  const acceptedPendingState = ():
    Extract<QuestionAttemptExperienceState, { readonly phase: "acceptedPending" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "acceptedPending" ? candidate : undefined;
  };
  const feedbackPanelState = (
    feedback: Extract<QuestionAttemptExperienceState, { readonly phase: "feedback" }>,
  ): FeedbackPresentation =>
    feedback.feedback.kind === "released"
      ? {
          kind: "released" as const,
          feedback: feedback.feedback.feedback,
          assignmentScoringState: feedback.acknowledgement.assignmentScoringState,
        }
      : {
          kind: "awaiting" as const,
          feedback: null,
          assignmentScoringState: feedback.acknowledgement.assignmentScoringState,
        };

  createEffect(() => {
    if (recoveringState()?.reason === "sessionExpired") {
      setSessionRecovery(true);
    }
  });

  const currentState = (): QuestionAttemptExperienceState | undefined => state();
  const terminalState = ():
    Extract<QuestionAttemptExperienceState, { readonly phase: "terminal" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "terminal" ? candidate : undefined;
  };
  const terminalPresentation = (): AssignmentAttemptCompletionPresentation =>
    assignmentAttemptCompletionPresentation(
      terminalState()?.assignmentAttemptCompletion ?? "inProgress",
      true,
    );
  const currentEnvelope = (): QuestionPresentation =>
    currentState()?.envelope ?? screen().issuedQuestion;
  // A cache-hit advance has a server-issued descriptor and envelope but not a
  // complete AssignmentAttemptScreenData record. Keep student-response projection bound to
  // the attempt state, which is advanced atomically with that descriptor.
  const currentAttemptId = (): string => currentState()?.context.attemptId ?? screen().attempt.id;

  return (
    <section
      class="page run-page"
      data-route-surface="runAttempt"
      data-attempt-id={currentAttemptId()}
      aria-busy={currentState()?.phase === "loading" || currentState()?.phase === "advancing"}
    >
      <header class="run-header">
        <div>
          <p class="eyebrow">Assignment Attempt {screen().assignmentAttempt.attemptNumber}</p>
          <h1>{currentEnvelope().title}</h1>
        </div>
        <span class="calm-status" role="timer" aria-live="polite">
          {formatRemaining(
            currentState()?.remainingMilliseconds ?? screen().attempt.timing.deadline,
          )}
        </span>
      </header>
      <Show when={questionPoolSelection()}>
        {(selection) => (
          <p class="assignment-attempt-question-pool-selection" role="status">
            Server-selected Question Pool item {selection().itemNumber} of {selection().itemCount}{" "}
            for this Assignment Attempt.
          </p>
        )}
      </Show>

      <Show when={summaryVisible() || currentState()?.phase === "terminal"}>
        <section class="attempt-summary" aria-labelledby="attempt-summary-heading">
          <p class="eyebrow">{terminalPresentation().eyebrow}</p>
          <h2 id="attempt-summary-heading">{terminalPresentation().heading}</h2>
          <p>{terminalPresentation().message}</p>
          <Show when={assignmentAttemptSummary()}>
            {(summary) => (
              <>
                <section aria-label="Assignment score">
                  <h3>Assignment score</h3>
                  <p>{studentProgressSummary(summary().summary)}</p>
                  <Show when={summary().summary.score_state === "available"}>
                    <p>
                      This Assignment Attempt:{" "}
                      {studentScoreValue(summary().assignmentAttempt.score)}
                    </p>
                  </Show>
                </section>
                <For each={summaryOutcomes()}>
                  {(outcome) => (
                    <FeedbackPanel
                      disclosure={
                        outcome.feedback === null
                          ? {
                              kind: "awaiting",
                              feedback: null,
                              assignmentScoringState: outcome.assignmentScoringState,
                            }
                          : {
                              kind: "released",
                              feedback: outcome.feedback,
                              assignmentScoringState: outcome.assignmentScoringState,
                            }
                      }
                      studentResponse={
                        outcome.attempt === currentAttemptId()
                          ? projectStudentResponse(currentEnvelope(), outcome.response)
                          : undefined
                      }
                      assetUrl={(asset) =>
                        new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                      }
                    />
                  )}
                </For>
                <Show when={summary().outcomes.nextCursor !== null}>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={summaryLoading()}
                    onClick={() => void loadSummary(summary().outcomes.nextCursor ?? undefined)}
                  >
                    Load more responses
                  </button>
                </Show>
              </>
            )}
          </Show>
          <Show when={summaryError()}>
            {(message) => (
              <>
                <p class="inline-error">{message()}</p>
                <button class="quiet-action" type="button" onClick={() => void loadSummary()}>
                  Retry summary
                </button>
              </>
            )}
          </Show>
          <Show when={terminalState()?.assignmentAttemptCompletion === "completed"}>
            <button
              class="primary-action"
              type="button"
              onClick={() => void startAnotherPractice()}
            >
              Start another practice
            </button>
          </Show>
          <Show when={practiceError()}>{(message) => <p class="inline-error">{message()}</p>}</Show>
          <button class="quiet-action" type="button" onClick={escapeToAssignment}>
            Back to assignment
          </button>
        </section>
      </Show>

      <Show when={!summaryVisible() && currentState()?.phase !== "terminal"}>
        <article class="question-card">
          <div class="prompt-copy">
            <ErrorBoundary
              fallback={(error) => {
                const message =
                  error instanceof Error ? error.message : "Question rendering failed.";
                machine.reportRendererFailure(message);
                return <p class="inline-error">{message}</p>;
              }}
            >
              <QuestionRenderer
                presentation={currentEnvelope()}
                assetUrl={(asset) =>
                  new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                }
                onRetry={() => machine.retryRenderer()}
              />
            </ErrorBoundary>
          </div>

          <div class="attempt-response">
            <Show when={currentState()?.storageWarning}>
              {(warning) => <p class="saved-notice">{warning()}</p>}
            </Show>
            <Show when={recoveringState()}>
              {(recovering) => (
                <section class="attempt-recovery" role="status" aria-live="polite">
                  <p>{recovering().message}</p>
                  <Show when={recovering().reason === "sessionExpired" && sessionRecovery()}>
                    <button
                      class="primary-action"
                      type="button"
                      onClick={() => void restoreSession()}
                    >
                      Restore session and retry
                    </button>
                  </Show>
                  <Show
                    when={recovering().reason === "offline" || recovering().reason === "network"}
                  >
                    <button class="quiet-action" type="button" onClick={() => void machine.retry()}>
                      Retry saved response
                    </button>
                  </Show>
                  <Show when={recovering().reason === "advanceFailed"}>
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => void retryNextQuestion()}
                    >
                      Retry next question
                    </button>
                  </Show>
                  <Show when={recovering().reason === "renderer"}>
                    <button
                      class="quiet-action"
                      type="button"
                      onClick={() => machine.retryRenderer()}
                    >
                      Retry question display
                    </button>
                  </Show>
                </section>
              )}
            </Show>

            <Show
              when={currentState()?.phase === "advancing"}
              fallback={
                <Show
                  when={feedbackState()}
                  fallback={
                    <Show
                      when={acceptedPendingState()}
                      fallback={
                        <Show
                          // A new server-issued attempt must not inherit a locally selected response.
                          // Key only on attempt identity so same-attempt recovery keeps local entry.
                          when={currentState()?.context.attemptId}
                          keyed
                          fallback={<p class="loading-state">Restoring your saved response...</p>}
                        >
                          {(attemptId) => (
                            <QuestionResponseControl
                              attemptId={attemptId}
                              definition={currentEnvelope().response}
                              initialResponse={currentState()?.response ?? undefined}
                              validator={validator}
                              onResponseChange={responseChanged}
                              onSubmit={submit}
                              onEscape={escapeToAssignment}
                              studentWorkRoute={{
                                courseId: screen().course.summary.id,
                                assignmentId: screen().assignment.id,
                              }}
                              beginExternalToolLaunch={() =>
                                runtime.client.beginExternalToolLaunch(
                                  screen().course.summary.id,
                                  screen().assignment.id,
                                  attemptId,
                                )
                              }
                            />
                          )}
                        </Show>
                      }
                    >
                      {(pending) => (
                        <section class="attempt-pending" aria-labelledby="grading-status-heading">
                          <h2 id="grading-status-heading">Response received</h2>
                          <p>
                            {pending().acknowledgement.gradingState === "pending"
                              ? "Grading is underway. You do not need to submit your response again."
                              : "Your response needs instructor attention. You do not need to submit it again."}
                          </p>
                          <p id="grading-status-message" role="status" aria-live="polite">
                            {pending().checkingStatus
                              ? "Checking grading status..."
                              : (pending().statusMessage ?? "")}
                          </p>
                          <button
                            class="primary-action"
                            type="button"
                            disabled={pending().checkingStatus}
                            aria-describedby="grading-status-message"
                            onClick={() => void machine.checkGradingStatus()}
                          >
                            Check grading status
                          </button>
                        </section>
                      )}
                    </Show>
                  }
                >
                  {(feedback) => (
                    <>
                      <Show when={feedback().acknowledgement.assignmentScoringState !== "current"}>
                        <section class="attempt-pending" aria-labelledby="score-status-heading">
                          <h2 id="score-status-heading">
                            {feedback().acknowledgement.assignmentScoringState === "recalculating"
                              ? "Score is being updated"
                              : "Score update needs attention"}
                          </h2>
                          <p>
                            {feedback().acknowledgement.assignmentScoringState === "recalculating"
                              ? "Your response is recorded. The current score will appear after grading finishes."
                              : "Your response is recorded. Check again to see whether the score is available."}
                          </p>
                          <p id="score-status-message" role="status" aria-live="polite">
                            {feedback().checkingStatus
                              ? "Checking for an updated score..."
                              : (feedback().statusMessage ?? "")}
                          </p>
                          <button
                            class="primary-action"
                            type="button"
                            disabled={feedback().checkingStatus}
                            aria-describedby="score-status-message"
                            onClick={() => void machine.checkGradingStatus()}
                          >
                            Check for updated score
                          </button>
                        </section>
                      </Show>
                      <FeedbackPanel
                        disclosure={feedbackPanelState(feedback())}
                        studentResponse={projectStudentResponse(
                          currentEnvelope(),
                          feedback().response,
                        )}
                        assetUrl={(asset) =>
                          new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                        }
                        onAdvance={() => void continueAttempt()}
                        advanceLabel={submissionAdvanceLabel(feedback().acknowledgement)}
                      />
                    </>
                  )}
                </Show>
              }
            >
              <p class="loading-state" role="status">
                Loading the next question...
              </p>
            </Show>
          </div>
        </article>
      </Show>
    </section>
  );
}

export function AssignmentAttemptPage(): JSX.Element {
  const scopedRoute = useCourseThemeRouteData();
  if (scopedRoute?.kind === "assignmentAttempt") {
    return <AttemptExperience initialScreen={scopedRoute.screen} />;
  }
  return (
    <section class="route-error" role="alert">
      <h1>Practice Assignment Attempt unavailable</h1>
      <p>Return to the assignment and open the practice Assignment Attempt again.</p>
    </section>
  );
}
