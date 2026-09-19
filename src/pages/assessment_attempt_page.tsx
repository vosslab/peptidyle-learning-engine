// assessment_attempt_page.tsx - live one-question Student Assessment Attempt delivery.

import { useNavigate } from "@solidjs/router";
import {
  createEffect,
  createSignal,
  ErrorBoundary,
  onCleanup,
  onMount,
  Show,
  type JSX,
} from "solid-js";

import type { StudentResponse } from "../../generated/api/StudentResponse";
import type {
  StudentAssessmentAttemptContext,
  StudentAssessmentAttemptResponseState,
} from "../api/assessment_attempt_navigation";
import type { StudentResponseFormatCheck } from "../api/decoders/student_response_format_check";
import { ApiRequestError } from "../api/http_client";
import { useApplicationApi } from "../api/application_api";
import { AuthorContentFrame } from "../components/author_content_frame";
import { StudentAssessmentAttemptNavigation } from "../components/student_assessment_attempt_navigation";
import type { StudentAssessmentAttemptQuestionState } from "../components/student_assessment_attempt_navigation";
import { formatAssessmentDeliveryTime } from "../components/student_assessment_presentation";
import { QuestionPresentationRenderer } from "../components/question_renderer";
import { QuestionPresentationResponseControl } from "../components/question_response_controls/question_response_control";
import type { ResponseSaveOutcome } from "../components/question_response_controls/common";
import {
  saveCapturedBackendOwnedResponse,
  saveCompleteResponseBeforeAttemptSubmission,
  type BackendOwnedCapture,
} from "./assessment_attempt_finish";
import { AssessmentAttemptResponseState } from "./assessment_attempt_response_state";
import { assessmentAttemptRouteId } from "../navigation/public_route";
import {
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";
import { useWasmFacade } from "../wasm/context";

type SaveState = "idle" | "saving" | "saved" | "error";

function formatRemaining(milliseconds: number | null): string {
  if (milliseconds === null) return "Untimed";
  const seconds = Math.ceil(milliseconds / 1_000);
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}:${remainder.toString().padStart(2, "0")} remaining`;
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

function AttemptExperience(props: {
  readonly context: StudentAssessmentAttemptContext;
}): JSX.Element {
  const runtime = useApplicationApi();
  const navigate = useNavigate();
  const validator = useWasmFacade();
  const [progress, setProgress] =
    createSignal<Awaited<ReturnType<typeof runtime.client.getStudentAssessmentAttemptProgress>>>();
  const [position, setPosition] = createSignal<number | null>(null);
  const [presentation, setPresentation] =
    createSignal<
      Awaited<ReturnType<typeof runtime.client.getStudentAssessmentAttemptPresentation>>
    >();
  const [loadError, setLoadError] = createSignal<string | null>(null);
  const [response, setResponse] = createSignal<StudentResponse | null>(null);
  const [responseValid, setResponseValid] = createSignal(false);
  const [saveState, setSaveState] = createSignal<SaveState>("idle");
  const [saveError, setSaveError] = createSignal<string | null>(null);
  const [submissionState, setSubmissionState] = createSignal<
    "idle" | "submitting" | "submitted" | "error"
  >("idle");
  const [submissionError, setSubmissionError] = createSignal<string | null>(null);
  const [remainingMilliseconds, setRemainingMilliseconds] = createSignal<number | null>(
    props.context.timerRemainingMilliseconds,
  );
  const [timerUnavailable, setTimerUnavailable] = createSignal(false);
  let progressRequest = 0;
  let presentationRequest = 0;
  let timerRequest = 0;
  let timerStartedAt = 0;
  let responseRevision = 0;
  const responseState = new AssessmentAttemptResponseState();
  let saveTimer: ReturnType<typeof globalThis.setTimeout> | undefined;
  let activeSave: Promise<boolean> | undefined;
  let expiryRefresh: Promise<void> | undefined;
  let finishAssessmentButton: HTMLButtonElement | undefined;
  let backendOwnedCapture: BackendOwnedCapture | undefined;

  const currentPosition = (): number | null => position();
  const isSubmitted = (): boolean => submissionState() === "submitted";
  const isExpired = (): boolean => remainingMilliseconds() === 0;

  async function loadProgress(): Promise<void> {
    progressRequest += 1;
    const request = progressRequest;
    setLoadError(null);
    try {
      const next = await runtime.client.getStudentAssessmentAttemptProgress(
        props.context.assessmentAttempt,
      );
      if (request !== progressRequest) return;
      const finalized =
        next.positions.length > 0 &&
        next.positions.every(
          (item) => item.responseState === "submitted" || item.responseState === "closed",
        );
      setProgress(next);
      if (finalized) {
        setSubmissionState("submitted");
        // Invalidate an in-flight timer result before finalized delivery hides the timer.
        timerRequest += 1;
        setPosition(null);
        return;
      }
      const current = position();
      const preferred = next.recommendedPosition ?? next.positions[0]?.position ?? null;
      if (current === null || !next.positions.some((item) => item.position === current)) {
        setPosition(preferred);
      }
    } catch (error: unknown) {
      if (request !== progressRequest) return;
      setLoadError(errorMessage(error, "Could not load your Assessment Attempt."));
    }
  }

  async function loadPresentation(nextPosition: number): Promise<void> {
    presentationRequest += 1;
    const request = presentationRequest;
    setPresentation(undefined);
    responseState.clear();
    responseRevision += 1;
    setResponse(null);
    setResponseValid(false);
    setSaveState("idle");
    setSaveError(null);
    setLoadError(null);
    try {
      const next = await runtime.client.getStudentAssessmentAttemptPresentation(
        props.context.assessmentAttempt,
        nextPosition,
      );
      if (request !== presentationRequest) return;
      setPresentation(next);
      if (next.savedResponse !== null) {
        responseRevision = responseState.restore(nextPosition, next.savedResponse);
        setResponse(next.savedResponse);
        setResponseValid(true);
        setSaveState("saved");
      }
    } catch (error: unknown) {
      if (request !== presentationRequest) return;
      setLoadError(errorMessage(error, "Could not load this Question."));
    }
  }

  async function saveCurrentResponse(): Promise<boolean> {
    const priorSave = activeSave;
    if (priorSave !== undefined) {
      await priorSave;
      return saveCurrentResponse();
    }
    const selected = currentPosition();
    if (selected === null || presentation()?.position !== selected) return false;
    const active = responseState.current(selected);
    if (active === undefined) return response() === null;
    const current = active.response;
    if (!active.valid || !responseValid()) {
      setSaveState("error");
      setSaveError("Complete the response format before saving it.");
      return false;
    }
    const revision = active.revision;
    const save = (async (): Promise<boolean> => {
      setSaveState("saving");
      setSaveError(null);
      try {
        await runtime.client.saveStudentAssessmentAttemptResponse(
          props.context.assessmentAttempt,
          selected,
          current,
        );
        setPresentation((existing) =>
          existing === undefined || existing.position !== selected
            ? existing
            : { ...existing, savedResponse: current },
        );
        if (revision === responseRevision) setSaveState("saved");
        await loadProgress();
        return true;
      } catch (error: unknown) {
        if (revision === responseRevision) {
          setSaveState("error");
          setSaveError(
            `Your response is still here. Save did not finish: ${errorMessage(error, "Please try again.")}`,
          );
        }
        return false;
      }
    })();
    activeSave = save;
    const saved = await save;
    if (activeSave === save) activeSave = undefined;
    if (revision !== responseRevision && saved) return saveCurrentResponse();
    return saved;
  }

  async function activatePosition(nextPosition: number): Promise<void> {
    if (nextPosition === currentPosition() || isSubmitted() || isExpired()) return;
    if (!(await saveCurrentResponse())) return;
    setPosition(nextPosition);
  }

  /**
   * Saves only a complete current draft before whole-Attempt submission.
   * An incomplete draft remains local, leaving any earlier saved response intact;
   * without an earlier saved response, the server finalizes the Question unanswered.
   */
  async function saveCurrentResponseBeforeAttemptSubmission(): Promise<boolean> {
    const selected = currentPosition();
    if (selected === null || presentation()?.position !== selected) return false;
    const active = responseState.current(selected);
    const responseIsComplete = active !== undefined && active.valid && responseValid();
    if (!responseIsComplete) {
      if (saveTimer !== undefined) globalThis.clearTimeout(saveTimer);
      const priorSave = activeSave;
      if (
        !(await saveCompleteResponseBeforeAttemptSubmission(false, saveCurrentResponse, priorSave))
      ) {
        return false;
      }
      setSaveState(presentation()?.savedResponse === null ? "idle" : "saved");
      setSaveError(null);
      return true;
    }
    return saveCompleteResponseBeforeAttemptSubmission(responseIsComplete, saveCurrentResponse);
  }

  async function submitAttempt(): Promise<void> {
    if (submissionState() === "submitting" || isSubmitted()) return;
    setSubmissionState("submitting");
    setSubmissionError(null);
    try {
      const result = await saveCapturedBackendOwnedResponse(
        backendOwnedCapture,
        saveCurrentResponseBeforeAttemptSubmission,
        () => runtime.client.submitStudentAssessmentAttempt(props.context.assessmentAttempt),
      );
      if (!result) {
        setSubmissionState("error");
        setSubmissionError(saveError() ?? "Response was not saved.");
        return;
      }
      setSubmissionState("submitted");
      // ASVS 1.2.2, 2.3.1, 8.2.2-8.2.3: enter the server-authorized, field-redacted result
      // view only after this exact whole-Attempt submission is accepted.
      navigate(
        `/assessment-attempts/${assessmentAttemptRouteId(result.assessmentAttempt)}/summary`,
        { replace: true },
      );
    } catch (error: unknown) {
      setSubmissionState("error");
      if (error instanceof ApiRequestError && error.status === 503) {
        // ASVS 16.5.1-16.5.2: do not expose transport details; saved work remains available.
        setSubmissionError(
          "This Assessment Attempt was not submitted. Submission is temporarily unavailable. Your saved responses are still here. Try again while time remains.",
        );
        return;
      }
      setSubmissionError(
        `This Assessment Attempt was not submitted: ${errorMessage(error, "Please review your saved responses and try again.")}`,
      );
    }
  }

  function responseEdited(position: number, next: StudentResponse): number | undefined {
    if (position !== currentPosition()) return undefined;
    responseRevision = responseState.edit(position, next);
    setResponse(next);
    setResponseValid(false);
    setSaveState("idle");
    setSaveError(null);
    return responseRevision;
  }

  function responseValidated(
    position: number,
    next: StudentResponse,
    validation: StudentResponseFormatCheck,
    editRevision?: number,
  ): void {
    if (!responseState.validate(position, next, editRevision, validation.issues.length === 0))
      return;
    if (position !== currentPosition()) return;
    setResponseValid(validation.issues.length === 0);
    if (validation.issues.length === 0) {
      const selected = currentPosition();
      if (selected !== null) {
        const revision = responseRevision;
        if (saveTimer !== undefined) globalThis.clearTimeout(saveTimer);
        saveTimer = globalThis.setTimeout(() => {
          if (selected === currentPosition() && revision === responseRevision)
            void saveCurrentResponse();
        }, 350);
      }
    }
  }

  function saveOutcome(): Promise<ResponseSaveOutcome> {
    return saveCurrentResponse().then((saved) =>
      saved
        ? { kind: "accepted" as const }
        : { kind: "rejected" as const, message: saveError() ?? "Response was not saved." },
    );
  }

  function tickTimer(): void {
    if (isSubmitted() || timerUnavailable()) return;
    const elapsedMilliseconds = Math.floor(Math.max(0, performance.now() - timerStartedAt));
    timerRequest += 1;
    const request = timerRequest;
    void validator
      .assessmentAttemptRemainingMilliseconds({
        initialRemainingMilliseconds: props.context.timerRemainingMilliseconds,
        elapsedMilliseconds,
      })
      .then((next) => {
        if (request !== timerRequest) return;
        setRemainingMilliseconds(next);
        if (next === 0) void refreshAfterExpiry();
      })
      .catch(() => {
        if (request === timerRequest) setTimerUnavailable(true);
      });
  }

  async function refreshAfterExpiry(): Promise<void> {
    if (isSubmitted() || expiryRefresh !== undefined) return expiryRefresh;
    const refresh = (async (): Promise<void> => {
      try {
        const context = await runtime.client.getStudentAssessmentAttemptContext(
          props.context.assessmentAttempt,
        );
        setRemainingMilliseconds(context.timerRemainingMilliseconds);
        await loadProgress();
      } catch (error: unknown) {
        setLoadError(
          `Time is up. Your saved responses are being submitted automatically: ${errorMessage(error, "The result will appear when submission finishes.")}`,
        );
      }
    })();
    expiryRefresh = refresh;
    await refresh;
    if (expiryRefresh === refresh) expiryRefresh = undefined;
  }

  createEffect(() => {
    const selected = position();
    if (selected !== null) void loadPresentation(selected);
  });

  onMount(() => {
    timerStartedAt = performance.now();
    void loadProgress();
    tickTimer();
    const interval = globalThis.setInterval(tickTimer, 1_000);
    onCleanup(() => {
      globalThis.clearInterval(interval);
      if (saveTimer !== undefined) globalThis.clearTimeout(saveTimer);
    });
  });

  const positions = (): ReadonlyArray<{
    readonly position: number;
    readonly responseState: StudentAssessmentAttemptQuestionState;
  }> =>
    (progress()?.positions ?? []).map((item) => ({
      ...item,
      responseState: responseStateForNavigation(item.responseState),
    }));

  function responseStateForNavigation(
    responseState: StudentAssessmentAttemptResponseState,
  ): StudentAssessmentAttemptQuestionState {
    if (isSubmitted() || isExpired() || responseState === "submitted") return "closed";
    return responseState;
  }

  function retryCurrentLoad(): void {
    const selected = currentPosition();
    if (selected === null) {
      void loadProgress();
      return;
    }
    void loadPresentation(selected);
  }

  return (
    <section class="page assessment-attempt-page" data-route-surface="assessmentAttempt">
      <header class="assessment-attempt-header">
        <div>
          <p class="eyebrow">Assessment Attempt {props.context.attemptNumber}</p>
          <h1>{props.context.assessment.title}</h1>
        </div>
        <Show when={!isSubmitted()}>
          <div>
            <span class="calm-status" role="timer">
              {timerUnavailable()
                ? "Timer unavailable; the server still enforces the time limit."
                : formatRemaining(remainingMilliseconds())}
            </span>
            <Show when={props.context.expiresAt !== null}>
              <p class="assessment-attempt-expiry">
                Saved responses submit automatically at{" "}
                {formatAssessmentDeliveryTime(
                  props.context.expiresAt,
                  props.context.displayTimeZone,
                )}
                .
              </p>
            </Show>
          </div>
        </Show>
      </header>

      <Show when={!isSubmitted() && !isExpired()}>
        <Show
          when={progress()}
          fallback={
            <p class="loading-state" role="status">
              Loading Questions...
            </p>
          }
        >
          <StudentAssessmentAttemptNavigation
            positions={positions()}
            currentPosition={currentPosition()}
            onPositionActivate={(nextPosition) => void activatePosition(nextPosition)}
          />
        </Show>
      </Show>

      <Show when={loadError()}>
        {(message) => (
          <section class="attempt-error" role="alert">
            <p>{message()}</p>
            <button class="quiet-action" type="button" onClick={retryCurrentLoad}>
              Retry
            </button>
          </section>
        )}
      </Show>

      <Show when={isSubmitted()}>
        <section class="attempt-summary" aria-labelledby="assessment-submitted-heading">
          <h2 id="assessment-submitted-heading">Your answers were accepted</h2>
          <p>Your saved responses are submitted for this Assessment Attempt.</p>
        </section>
      </Show>

      <Show when={!isSubmitted() && isExpired()}>
        <section class="attempt-summary" aria-labelledby="assessment-expired-heading">
          <h2 id="assessment-expired-heading">Time is up</h2>
          <p>Your saved responses are being submitted automatically.</p>
        </section>
      </Show>

      <Show when={presentation()} keyed>
        {(currentPresentation) => (
          <Show when={!isSubmitted() && !isExpired()}>
            <article class="question-card">
              <div class="prompt-copy">
                <ErrorBoundary fallback={<p class="inline-error">Question rendering failed.</p>}>
                  <QuestionPresentationRenderer
                    presentation={currentPresentation.presentation}
                    assetUrl={(asset) =>
                      new URL(
                        runtime.client.assetUrl(
                          currentPresentation.presentation.questionRevision,
                          asset.questionAsset,
                        ),
                        window.location.origin,
                      )
                    }
                  />
                </ErrorBoundary>
                <Show
                  when={
                    currentPresentation.presentation.response.kind !== "backendOwned" &&
                    currentPresentation.presentation.authorContentDigest !== undefined
                  }
                >
                  <AuthorContentFrame
                    assessmentAttempt={props.context.assessmentAttempt}
                    position={currentPresentation.position}
                  />
                </Show>
              </div>
              <div class="attempt-response">
                <QuestionPresentationResponseControl
                  attemptId={`${props.context.assessmentAttempt}-${currentPresentation.position}`}
                  questionRevision={currentPresentation.presentation.questionRevision}
                  assetUrl={(asset) =>
                    new URL(
                      runtime.client.assetUrl(
                        currentPresentation.presentation.questionRevision,
                        asset.questionAsset,
                      ),
                      window.location.origin,
                    )
                  }
                  assessmentAttempt={props.context.assessmentAttempt}
                  position={currentPresentation.position}
                  registerBackendOwnedCapture={(capture) => {
                    backendOwnedCapture = capture;
                    return () => {
                      if (backendOwnedCapture === capture) backendOwnedCapture = undefined;
                    };
                  }}
                  responseFormat={currentPresentation.presentation.response}
                  initialResponse={currentPresentation.savedResponse ?? undefined}
                  validator={validator}
                  saveLabel="Save response"
                  mode="save"
                  onResponseEdit={(response) =>
                    responseEdited(currentPresentation.position, response)
                  }
                  onResponseChange={(response, validation, editRevision) =>
                    responseValidated(
                      currentPresentation.position,
                      response,
                      validation,
                      editRevision,
                    )
                  }
                  onSave={saveOutcome}
                  onEscape={() => finishAssessmentButton?.focus()}
                />
                <Show when={saveState() === "saving"}>
                  <p class="calm-status" role="status">
                    Saving response...
                  </p>
                </Show>
                <Show when={saveState() === "saved"}>
                  <p class="saved-notice" role="status">
                    Response saved.
                  </p>
                </Show>
                <Show when={saveError()}>
                  {(message) => (
                    <p class="inline-error" role="alert">
                      {message()}
                    </p>
                  )}
                </Show>
              </div>
            </article>
          </Show>
        )}
      </Show>

      <Show when={progress() && !isSubmitted() && !isExpired()}>
        <section
          class="assessment-attempt-submit"
          aria-labelledby="assessment-attempt-submit-heading"
        >
          <h2 id="assessment-attempt-submit-heading">Finish Assessment</h2>
          <p>
            Complete responses are saved before submission. Incomplete responses submit as
            unanswered unless an earlier saved response exists.
          </p>
          <button
            class="primary-action"
            type="button"
            ref={(element) => (finishAssessmentButton = element)}
            disabled={submissionState() === "submitting"}
            onClick={() => void submitAttempt()}
          >
            {submissionState() === "submitting" ? "Submitting Assessment..." : "Submit Assessment"}
          </button>
          <Show when={submissionError()}>
            {(message) => (
              <p class="inline-error" role="alert">
                {message()}
              </p>
            )}
          </Show>
        </section>
      </Show>
    </section>
  );
}

export function AssessmentAttemptPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const loadState = useRouteScopeLoadState();
  const retryScope = useRetryRouteScope();
  const context = (): StudentAssessmentAttemptContext | undefined => {
    const data = routeData();
    return data?.kind === "assessmentAttempt" ? data.context : undefined;
  };
  return (
    <Show
      when={context()}
      keyed
      fallback={
        <section class="page" data-route-surface="assessmentAttempt">
          <Show
            when={loadState() === "rejected"}
            fallback={
              <p class="loading-state" role="status">
                Loading your Assessment Attempt...
              </p>
            }
          >
            <p class="inline-error" role="alert">
              Your Assessment Attempt could not be loaded.
            </p>
            <button class="quiet-action" type="button" onClick={retryScope}>
              Retry
            </button>
          </Show>
        </section>
      }
    >
      {(loadedContext) => <AttemptExperience context={loadedContext} />}
    </Show>
  );
}
