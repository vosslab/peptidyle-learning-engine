// run_page.tsx - server-issued, key-free learner attempt loop.

import { createAsync, useNavigate, useParams } from "@solidjs/router";
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

import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type {
  PrefetchedNextQuestion,
  RunScreenData,
  RunSummaryOutcome,
  RunSummaryResponse,
} from "../api/contracts";
import { ApiProtocolError, ApiRequestError } from "../api/http_client";
import { useApiRuntime } from "../api/runtime";
import { useCourseThemeRouteData } from "../features/course_appearance/course_theme_context";
import { QuestionRenderer } from "../components/question_renderer";
import { FeedbackPanel, type FeedbackDisclosure } from "../components/feedback_panel";
import { ResponseWidget } from "../components/response_widget";
import { resumeSessionAndRetry } from "./run_page_recovery";
import {
  createAttemptStateMachine,
  type AttemptContext,
  type AttemptState,
  type AttemptStorage,
} from "../features/attempt/attempt_state";
import { projectLearnerResponse } from "../features/attempt/learner_response";
import type { ResponseFormatReport } from "../wasm/index";
import { useWasmFacade } from "../wasm/context";

function attemptContext(attempt: QuestionAttempt): AttemptContext {
  return {
    tenantId: attempt.tenant,
    runId: attempt.run,
    attemptId: attempt.id,
    questionVersion: attempt.questionVersion,
    seed: attempt.seed,
    deadline: attempt.timer.deadline,
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

function formatRemaining(milliseconds: number | null): string {
  if (milliseconds === null) return "Untimed";
  const seconds = Math.ceil(milliseconds / 1_000);
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}:${remainder.toString().padStart(2, "0")} remaining`;
}

function sameAttempt(left: AttemptContext, right: AttemptContext): boolean {
  return left.attemptId === right.attemptId;
}

/** Avoid turning an unusually image-heavy question into an unbounded background fetch. */
const MAX_PREFETCH_ASSETS = 12;

function assetIdsForEnvelope(envelope: QuestionEnvelope): ReadonlyArray<string> {
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

function AttemptExperience(props: { readonly initialScreen: RunScreenData }): JSX.Element {
  const runtime = useApiRuntime();
  const validator = useWasmFacade();
  const navigate = useNavigate();
  const [screen, setScreen] = createSignal(props.initialScreen);
  const [state, setState] = createSignal<AttemptState>();
  const [sessionRecovery, setSessionRecovery] = createSignal(false);
  const [summaryVisible, setSummaryVisible] = createSignal(false);
  const [runSummary, setRunSummary] = createSignal<RunSummaryResponse>();
  const [summaryOutcomes, setSummaryOutcomes] = createSignal<ReadonlyArray<RunSummaryOutcome>>([]);
  const [summaryError, setSummaryError] = createSignal<string | null>(null);
  const [summaryLoading, setSummaryLoading] = createSignal(false);
  const seenSummaryCursors = new Set<string>();
  const [practiceError, setPracticeError] = createSignal<string | null>(null);
  const [prefetched, setPrefetched] = createSignal<PrefetchedNextQuestion | null>(null);
  let requestedPrefetchFor: string | null = null;
  let prefetchController: AbortController | null = null;

  const machine = createAttemptStateMachine({
    context: attemptContext(props.initialScreen.attempt),
    storage: attemptStorage(),
    clock: { now: () => Date.now() },
    network: { isOnline: () => navigator.onLine },
    generateIdempotencyKey,
    submitResponse: runtime.client.submitResponse,
    isSessionExpired,
    validateSavedResponse: validator.validateResponseFormat,
    onStateChange: setState,
  });

  function escapeToAssignment(): void {
    navigate(`/courses/${screen().course.summary.id}/assignments/${screen().assignment.id}`);
  }

  function responseChanged(response: StudentResponse, validation: ResponseFormatReport): void {
    machine.setResponse(response, {
      valid: validation.violations.length === 0,
      message: validation.violations.length === 0 ? null : "Response format needs attention.",
    });
  }

  async function submit(response: StudentResponse): Promise<void> {
    // ResponseWidget reaches this callback only after its browser-local format validation.
    // This enables delivery, never local correctness or scoring.
    machine.setResponse(response, { valid: true, message: null });
    await machine.submit();
  }

  async function continueAttempt(): Promise<void> {
    const receiptNext = feedbackState()?.acknowledgement.nextIssued ?? null;
    const cached = prefetched();
    if (
      receiptNext !== null &&
      cached !== null &&
      cached.predecessor === machine.state().context.attemptId &&
      cached.run === receiptNext.run &&
      cached.assignmentPosition === receiptNext.assignmentPosition &&
      cached.questionVersion === receiptNext.questionVersion &&
      cached.seed === receiptNext.seed &&
      cached.renderedQuestionSha256 === receiptNext.renderedQuestionSha256
    ) {
      const current = machine.state().context;
      await machine.advance(() =>
        Promise.resolve({
          context: {
            ...current,
            attemptId: receiptNext.id,
            runId: receiptNext.run,
            questionVersion: receiptNext.questionVersion,
            seed: receiptNext.seed,
            deadline: receiptNext.deadline,
          },
          envelope: cached.envelope,
        }),
      );
      setPrefetched(null);
      requestPrefetch(receiptNext.id);
      return;
    }
    const next = await runtime.queries.runScreen(screen().run.id);
    const nextContext = attemptContext(next.attempt);
    if (sameAttempt(nextContext, machine.state().context)) {
      machine.complete();
      setSummaryVisible(true);
      void loadSummary();
      return;
    }
    await machine.advance(() => {
      setScreen(next);
      return Promise.resolve({ context: nextContext, envelope: next.issuedQuestion });
    });
    setPrefetched(null);
    requestPrefetch(next.attempt.id);
  }

  function requestPrefetch(attemptId: string): void {
    if (requestedPrefetchFor === attemptId) return;
    requestedPrefetchFor = attemptId;
    prefetchController?.abort();
    const controller = new AbortController();
    prefetchController = controller;
    void runtime.client
      .prefetchNextQuestion(attemptId, controller.signal)
      .then((value) => {
        if (controller.signal.aborted || machine.state().context.attemptId !== attemptId) return;
        if (value !== null && value.run !== machine.state().context.runId) {
          throw new ApiProtocolError("Prefetched question run does not match the active run");
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
      const page = await runtime.client.getRunSummary(screen().run.id, cursor, 30);
      if (page.outcomes.nextCursor !== null && seenSummaryCursors.has(page.outcomes.nextCursor)) {
        throw new Error("Run summary repeated its cursor.");
      }
      if (cursor !== undefined) seenSummaryCursors.add(cursor);
      setRunSummary(page);
      setSummaryOutcomes((existing) => {
        const prior = cursor === undefined ? [] : existing;
        const seen = new Set(prior.map((outcome) => outcome.attempt));
        return [...prior, ...page.outcomes.items.filter((outcome) => !seen.has(outcome.attempt))];
      });
    } catch {
      setSummaryError("Could not refresh this run summary. Your completed work remains recorded.");
    } finally {
      setSummaryLoading(false);
    }
  }

  async function startAnotherPractice(): Promise<void> {
    setPracticeError(null);
    try {
      const run = await runtime.client.startRun(screen().assignment.id);
      navigate(`/runs/${run.id}`);
    } catch (error: unknown) {
      setPracticeError(
        error instanceof Error ? error.message : "Could not start another practice run.",
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
    machine.start(props.initialScreen.issuedQuestion.response);
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
    Extract<AttemptState, { readonly phase: "recovering" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "recovering" ? candidate : undefined;
  };
  const feedbackState = (): Extract<AttemptState, { readonly phase: "feedback" }> | undefined => {
    const candidate = state();
    return candidate?.phase === "feedback" ? candidate : undefined;
  };
  const feedbackPanelState = (
    feedback: Extract<AttemptState, { readonly phase: "feedback" }>,
  ): FeedbackDisclosure =>
    feedback.feedback.kind === "released"
      ? { kind: "released" as const, feedback: feedback.feedback.feedback }
      : { kind: "awaiting" as const, feedback: null };

  createEffect(() => {
    if (recoveringState()?.reason === "sessionExpired") {
      setSessionRecovery(true);
    }
  });

  const currentState = (): AttemptState | undefined => state();
  const currentEnvelope = (): QuestionEnvelope =>
    currentState()?.envelope ?? screen().issuedQuestion;
  // A cache-hit advance has a server-issued descriptor and envelope but not a
  // complete RunScreenData record. Keep learner-response projection bound to
  // the attempt state, which is advanced atomically with that descriptor.
  const currentAttemptId = (): string => currentState()?.context.attemptId ?? screen().attempt.id;

  return (
    <section
      class="page run-page"
      data-route-surface="runAttempt"
      aria-busy={currentState()?.phase === "loading"}
    >
      <header class="run-header">
        <div>
          <p class="eyebrow">Practice run {screen().run.runNumber}</p>
          <h1>{currentEnvelope().title}</h1>
        </div>
        <span class="calm-status" role="timer" aria-live="polite">
          {formatRemaining(
            currentState()?.remainingMilliseconds ?? screen().attempt.timer.deadline,
          )}
        </span>
      </header>

      <Show when={summaryVisible() || currentState()?.phase === "completed"}>
        <section class="attempt-summary" aria-labelledby="attempt-summary-heading">
          <p class="eyebrow">Run complete</p>
          <h2 id="attempt-summary-heading">Keep practicing with a fresh variation</h2>
          <p>Your completed run is recorded.</p>
          <Show when={runSummary()}>
            {(summary) => (
              <>
                <For each={summaryOutcomes()}>
                  {(outcome) => (
                    <FeedbackPanel
                      disclosure={
                        outcome.feedback === null
                          ? { kind: "awaiting", feedback: null }
                          : { kind: "released", feedback: outcome.feedback }
                      }
                      learnerResponse={
                        outcome.attempt === currentAttemptId()
                          ? projectLearnerResponse(currentEnvelope(), outcome.response)
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
          <Show when={runSummary()?.practiceAllowed}>
            <button
              class="primary-action"
              type="button"
              onClick={() => void startAnotherPractice()}
            >
              Start another practice run
            </button>
          </Show>
          <Show when={practiceError()}>{(message) => <p class="inline-error">{message()}</p>}</Show>
          <button class="quiet-action" type="button" onClick={escapeToAssignment}>
            Back to assignment
          </button>
        </section>
      </Show>

      <Show when={!summaryVisible() && currentState()?.phase !== "completed"}>
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
                      onClick={() => void machine.retryAdvance()}
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
              when={feedbackState()}
              fallback={
                <Show
                  when={currentState()}
                  fallback={<p class="loading-state">Restoring your saved response...</p>}
                >
                  {(attemptState) => (
                    <ResponseWidget
                      attemptId={attemptState().context.attemptId}
                      definition={currentEnvelope().response}
                      initialResponse={attemptState().response ?? undefined}
                      validator={validator}
                      onResponseChange={responseChanged}
                      onSubmit={submit}
                      onEscape={escapeToAssignment}
                      getExternalToolLaunch={() =>
                        runtime.client.getExternalToolLaunch(attemptState().context.attemptId)
                      }
                    />
                  )}
                </Show>
              }
            >
              {(feedback) => (
                <FeedbackPanel
                  disclosure={feedbackPanelState(feedback())}
                  learnerResponse={projectLearnerResponse(currentEnvelope(), feedback().response)}
                  assetUrl={(asset) =>
                    new URL(runtime.client.assetUrl(asset.asset), window.location.origin)
                  }
                  onAdvance={() => void continueAttempt()}
                />
              )}
            </Show>
          </div>
        </article>
      </Show>
    </section>
  );
}

export function RunPage(): JSX.Element {
  const runtime = useApiRuntime();
  const params = useParams();
  const scopedRoute = useCourseThemeRouteData();
  if (scopedRoute?.kind === "runAttempt") {
    return <AttemptExperience initialScreen={scopedRoute.screen} />;
  }
  const runScreen = createAsync(() => {
    const runId = params["runId"];
    if (runId === undefined) return Promise.reject(new Error("Run route is missing runId"));
    return runtime.queries.runScreen(runId);
  });

  return (
    <Show
      when={runScreen()}
      fallback={<p class="loading-state">Loading the current question...</p>}
    >
      {(screen) => <AttemptExperience initialScreen={screen()} />}
    </Show>
  );
}
