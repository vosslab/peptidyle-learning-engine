// assessment_attempt_page.tsx - live one-question Student Assessment Attempt delivery.

import { A, useNavigate, useParams } from "@solidjs/router";
import {
  createEffect,
  createSignal,
  ErrorBoundary,
  on,
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
import { useApplicationApi } from "../api/application_api";
import { AuthorContentFrame } from "../components/author_content_frame";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { PageFrame } from "../components/page_frame/page_frame";
import { StudentAssessmentAttemptNavigation } from "../components/student_assessment_attempt_navigation";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import type { StudentAssessmentAttemptQuestionState } from "../components/student_assessment_attempt_navigation";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { formatAssessmentDeliveryTime } from "../components/student_assessment_presentation";
import { QuestionPresentationRenderer } from "../components/question_renderer";
import { QuestionPresentationResponseControl } from "../components/question_response_controls/question_response_control";
import { QuestionDisplayDurationClock } from "../question_display_duration_clock";
import type { ResponseSaveOutcome } from "../components/question_response_controls/common";
import {
  saveCapturedBackendOwnedResponse,
  saveCompleteResponseBeforeAttemptSubmission,
  submitAttemptActionLabel,
  type BackendOwnedCapture,
} from "./assessment_attempt_finish";
import { AssessmentAttemptResponseState } from "./assessment_attempt_response_state";
import { assessmentAttemptRouteId, parseCourseInstanceId } from "../navigation/public_route";
import {
  assessmentAttemptPath,
  assessmentAttemptRouteState,
} from "../navigation/assessment_attempt_route";
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
  const formatDateTime = createDisplayDateTimeFormatter(props.context.displayTimeZone);
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
  const [savedResponse, setSavedResponse] = createSignal<StudentResponse | null>(null);
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
  let responseEditGeneration = 0;
  const responseState = new AssessmentAttemptResponseState();
  let saveTimer: ReturnType<typeof globalThis.setTimeout> | undefined;
  let activeSave: Promise<boolean> | undefined;
  let activeSaveGeneration: number | undefined;
  let expiryRefresh: Promise<void> | undefined;
  let finishAssessmentButton: HTMLButtonElement | undefined;
  let backendOwnedCapture: BackendOwnedCapture | undefined;
  let savedResponseRevision = 0;
  let navigating = false;
  let disposed = false;
  const questionDisplayClocks = new Map<number, QuestionDisplayDurationClock>();

  function questionDisplayClock(selectedPosition: number): QuestionDisplayDurationClock {
    const existing = questionDisplayClocks.get(selectedPosition);
    if (existing !== undefined) return existing;
    const savedMilliseconds =
      progress()?.positions.find((item) => item.position === selectedPosition)?.displayDurationMs ??
      null;
    const clock = new QuestionDisplayDurationClock(savedMilliseconds);
    questionDisplayClocks.set(selectedPosition, clock);
    return clock;
  }

  function resumeQuestionDisplayClock(): void {
    const selectedPosition = currentPosition();
    if (
      selectedPosition === null ||
      document.visibilityState !== "visible" ||
      presentation()?.position !== selectedPosition ||
      isSubmitted() ||
      submissionState() !== "idle" ||
      isExpired()
    )
      return;
    questionDisplayClock(selectedPosition).resume(performance.now());
  }

  async function checkpointQuestionDisplayDuration(pause: boolean): Promise<void> {
    const selectedPosition = currentPosition();
    if (selectedPosition === null) return;
    const clock = questionDisplayClock(selectedPosition);
    const now = performance.now();
    const cumulativeDisplayDurationMs = pause ? clock.pause(now) : clock.checkpoint(now);
    if (cumulativeDisplayDurationMs === 0) return;
    try {
      const receipt = await runtime.client.checkpointStudentQuestionDisplayDuration(
        props.context.assessmentAttemptId,
        selectedPosition,
        cumulativeDisplayDurationMs,
      );
      clock.observeStoredMilliseconds(receipt.cumulativeDisplayDurationMs);
    } catch {
      // Keep the local cumulative value and retry at the next checkpoint or transition.
    }
  }

  const currentPosition = (): number | null => position();
  const isSubmitted = (): boolean => submissionState() === "submitted";
  const isExpired = (): boolean => remainingMilliseconds() === 0;
  function currentResponseIsSaved(): boolean {
    const currentPresentation = presentation();
    return (
      saveState() === "saved" &&
      currentPresentation !== undefined &&
      currentPresentation.position === currentPosition() &&
      savedResponse() !== null &&
      JSON.stringify(response()) === JSON.stringify(savedResponse())
    );
  }

  function clearSaveTimer(): void {
    if (saveTimer !== undefined) globalThis.clearTimeout(saveTimer);
    saveTimer = undefined;
  }

  function hasUnsavedResponse(): boolean {
    return !isSubmitted() && !isExpired() && response() !== null && !currentResponseIsSaved();
  }

  async function loadProgress(): Promise<void> {
    progressRequest += 1;
    const request = progressRequest;
    setLoadError(null);
    try {
      const next = await runtime.client.getStudentAssessmentAttemptProgress(
        props.context.assessmentAttemptId,
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
      setLoadError(errorMessage(error, "Could not load your Attempt."));
    }
  }

  async function loadPresentation(nextPosition: number): Promise<void> {
    clearSaveTimer();
    void checkpointQuestionDisplayDuration(true);
    presentationRequest += 1;
    const request = presentationRequest;
    setPresentation(undefined);
    responseState.clear();
    responseEditGeneration += 1;
    setResponse(null);
    setSavedResponse(null);
    savedResponseRevision = 0;
    setResponseValid(false);
    setSaveState("idle");
    setSaveError(null);
    setLoadError(null);
    try {
      const next = await runtime.client.getStudentAssessmentAttemptPresentation(
        props.context.assessmentAttemptId,
        nextPosition,
      );
      if (request !== presentationRequest) return;
      savedResponseRevision = next.savedResponseRevision;
      setSavedResponse(next.savedResponse);
      setPresentation(next);
      if (next.savedResponse !== null) {
        responseEditGeneration = responseState.restore(nextPosition, next.savedResponse);
        setResponse(next.savedResponse);
        setResponseValid(true);
        setSaveState("saved");
      }
      globalThis.requestAnimationFrame(resumeQuestionDisplayClock);
    } catch (error: unknown) {
      if (request !== presentationRequest) return;
      setLoadError(errorMessage(error, "Could not load this Question."));
    }
  }

  async function saveCurrentResponse(): Promise<boolean> {
    clearSaveTimer();
    if (disposed || isSubmitted() || isExpired()) return false;
    const priorSave = activeSave;
    if (priorSave !== undefined) {
      const priorGeneration = activeSaveGeneration;
      if (!(await priorSave) && priorGeneration === responseEditGeneration) return false;
      return saveCurrentResponse();
    }
    const selected = currentPosition();
    if (selected === null || presentation()?.position !== selected) return false;
    const active = responseState.current(selected);
    if (active === undefined) return response() === null;
    if (currentResponseIsSaved()) return true;
    const current = active.response;
    if (!active.valid || !responseValid()) {
      setSaveState("error");
      setSaveError("Complete the response format before saving it.");
      return false;
    }
    const editGeneration = active.editGeneration;
    const save = (async (): Promise<boolean> => {
      setSaveState("saving");
      setSaveError(null);
      try {
        await checkpointQuestionDisplayDuration(false);
        const acknowledgement = await runtime.client.saveStudentAssessmentAttemptResponse(
          props.context.assessmentAttemptId,
          selected,
          current,
          savedResponseRevision,
        );
        if (disposed || selected !== currentPosition()) return false;
        savedResponseRevision = acknowledgement.savedResponseRevision;
        if (acknowledgement.responseState === "conflict") {
          setSaveState("error");
          setSaveError(
            "The saved response changed. Your latest input is still here. Retry Save to keep it.",
          );
          return false;
        }
        // Keep the keyed Question mounted while recording persistence metadata.
        // Replacing presentation here would destroy edits made during this save.
        setSavedResponse(current);
        if (editGeneration === responseEditGeneration) setSaveState("saved");
        await loadProgress();
        return true;
      } catch (error: unknown) {
        if (!disposed && selected === currentPosition()) {
          setSaveState("error");
          setSaveError(
            `Your response is still here. Save did not finish: ${errorMessage(error, "Please try again.")}`,
          );
        }
        return false;
      }
    })();
    activeSave = save;
    activeSaveGeneration = editGeneration;
    const saved = await save;
    if (activeSave === save) {
      activeSave = undefined;
      activeSaveGeneration = undefined;
    }
    if (editGeneration !== responseEditGeneration) return saveCurrentResponse();
    return saved;
  }

  async function activatePosition(nextPosition: number): Promise<void> {
    if (
      navigating ||
      submissionState() === "submitting" ||
      nextPosition === currentPosition() ||
      isSubmitted() ||
      isExpired()
    )
      return;
    navigating = true;
    try {
      if (!(await saveBeforeLeaving()) || disposed || isSubmitted() || isExpired()) return;
      void checkpointQuestionDisplayDuration(true);
      setPosition(nextPosition);
    } finally {
      navigating = false;
    }
  }

  /** Await the latest native format check when navigation follows the final keystroke. */
  async function validateCurrentResponse(): Promise<void> {
    const selected = currentPosition();
    if (selected === null) return;
    const active = responseState.current(selected);
    const format = presentation()?.presentation.response;
    if (active === undefined || active.valid || format === undefined) return;
    const check = await validator.validateResponseFormat(format, active.response);
    if (disposed || selected !== currentPosition()) return;
    responseValidated(selected, active.response, check, active.editGeneration);
    if (active.editGeneration !== responseEditGeneration) await validateCurrentResponse();
  }

  async function captureCurrentResponse(): Promise<boolean> {
    clearSaveTimer();
    if (backendOwnedCapture !== undefined && !(await backendOwnedCapture())) {
      setSaveError(
        "The Question did not return your response. Your input is still here. Try Save again.",
      );
      setSaveState("error");
      return false;
    }
    try {
      await validateCurrentResponse();
    } catch {
      setSaveState("error");
      setSaveError("Your input is still here. Response checking is unavailable. Try Save again.");
      return false;
    }
    clearSaveTimer();
    return true;
  }

  async function saveBeforeLeaving(): Promise<boolean> {
    if (isSubmitted() || isExpired()) return true;
    return saveCapturedBackendOwnedResponse(captureCurrentResponse, saveCurrentResponse, () =>
      Promise.resolve(true),
    );
  }

  /**
   * Saves only a complete current draft before whole-Attempt submission.
   * An incomplete draft remains local, leaving any earlier saved response intact;
   * without an earlier saved response, the server finalizes the Question unanswered.
   */
  async function saveCurrentResponseBeforeAttemptSubmission(): Promise<boolean> {
    const selected = currentPosition();
    if (selected === null || presentation()?.position !== selected) return false;
    if (currentResponseIsSaved()) return true;
    const active = responseState.current(selected);
    const responseIsComplete = active !== undefined && active.valid && responseValid();
    if (!responseIsComplete) {
      clearSaveTimer();
      const priorSave = activeSave;
      if (
        !(await saveCompleteResponseBeforeAttemptSubmission(false, saveCurrentResponse, priorSave))
      ) {
        return false;
      }
      setSaveState(savedResponse() === null ? "idle" : "saved");
      setSaveError(null);
      return true;
    }
    return saveCompleteResponseBeforeAttemptSubmission(responseIsComplete, saveCurrentResponse);
  }

  async function submitAttempt(): Promise<void> {
    if (navigating || submissionState() === "submitting" || isSubmitted()) return;
    await checkpointQuestionDisplayDuration(true);
    setSubmissionState("submitting");
    setSubmissionError(null);
    try {
      const result = await saveCapturedBackendOwnedResponse(
        captureCurrentResponse,
        saveCurrentResponseBeforeAttemptSubmission,
        () => runtime.client.submitStudentAssessmentAttempt(props.context.assessmentAttemptId),
      );
      if (!result) {
        setSubmissionState("error");
        setSubmissionError(saveError() ?? "Response was not saved.");
        return;
      }
      setSubmissionState("submitted");
      // ASVS 1.2.2, 2.3.1, 8.2.2-8.2.3: enter the server-authorized, field-redacted result
      // view only after this exact whole-Attempt submission is accepted.
      navigate(assessmentAttemptPath("review", props.context.course.id), {
        replace: true,
        state: assessmentAttemptRouteState(assessmentAttemptRouteId(result.assessmentAttemptId)),
      });
    } catch {
      // A lost acknowledgement can follow a committed submission. Consult the
      // server before describing the outcome; retry remains idempotent.
      setSubmissionError("Checking whether your Attempt was submitted...");
      await loadProgress();
      if (isSubmitted()) {
        navigate(assessmentAttemptPath("review", props.context.course.id), {
          replace: true,
          state: assessmentAttemptRouteState(
            assessmentAttemptRouteId(props.context.assessmentAttemptId),
          ),
        });
        return;
      }
      setSubmissionState("error");
      setSubmissionError(
        "Submission could not be confirmed. Your saved responses are recorded. Retry Submit Attempt to check and finish.",
      );
    }
  }

  function responseEdited(position: number, next: StudentResponse): number | undefined {
    if (disposed || isSubmitted() || isExpired() || position !== currentPosition())
      return undefined;
    const existing = responseState.current(position);
    if (existing !== undefined && JSON.stringify(existing.response) === JSON.stringify(next)) {
      return existing.editGeneration;
    }
    clearSaveTimer();
    responseEditGeneration = responseState.edit(position, next);
    setResponse(next);
    setResponseValid(false);
    setSaveState("idle");
    setSaveError(null);
    return responseEditGeneration;
  }

  function responseValidated(
    position: number,
    next: StudentResponse,
    validation: StudentResponseFormatCheck,
    editGeneration?: number,
  ): void {
    if (!responseState.validate(position, next, editGeneration, validation.issues.length === 0))
      return;
    if (position !== currentPosition()) return;
    setResponseValid(validation.issues.length === 0);
    if (validation.issues.length === 0 && !currentResponseIsSaved()) {
      const selected = currentPosition();
      if (selected !== null) {
        const generation = responseEditGeneration;
        if (saveTimer !== undefined) globalThis.clearTimeout(saveTimer);
        saveTimer = globalThis.setTimeout(() => {
          if (selected === currentPosition() && generation === responseEditGeneration)
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
        if (next === 0) {
          void checkpointQuestionDisplayDuration(true);
          void refreshAfterExpiry();
        }
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
          props.context.assessmentAttemptId,
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

  // Loading reads progress while checkpointing the display clock. Those reads
  // must not make a save acknowledgement reload the mounted Question.
  createEffect(
    on(position, (selected) => {
      if (selected !== null) void loadPresentation(selected);
    }),
  );

  onMount(() => {
    timerStartedAt = performance.now();
    void loadProgress();
    tickTimer();
    const interval = globalThis.setInterval(tickTimer, 1_000);
    const durationInterval = globalThis.setInterval(() => {
      if (document.visibilityState === "visible") void checkpointQuestionDisplayDuration(false);
    }, 15_000);
    const visibilityChanged = (): void => {
      if (document.visibilityState === "visible") resumeQuestionDisplayClock();
      else void checkpointQuestionDisplayDuration(true);
    };
    const pageHidden = (): void => void checkpointQuestionDisplayDuration(true);
    document.addEventListener("visibilitychange", visibilityChanged);
    window.addEventListener("pagehide", pageHidden);
    onCleanup(() => {
      disposed = true;
      presentationRequest += 1;
      progressRequest += 1;
      timerRequest += 1;
      globalThis.clearInterval(interval);
      globalThis.clearInterval(durationInterval);
      document.removeEventListener("visibilitychange", visibilityChanged);
      window.removeEventListener("pagehide", pageHidden);
      void checkpointQuestionDisplayDuration(true);
      clearSaveTimer();
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
    <PageFrame routeSurface="assessmentAttempt">
      <UnsavedChangesGuard
        dirty={hasUnsavedResponse}
        save={saveBeforeLeaving}
        inFlightSave={() => activeSave}
        autoSaveOnLeave
        copy={{
          heading: "Your response has not been saved",
          description:
            "Stay to keep working, retry saving, or discard unsaved changes and keep the last saved response.",
          saveActionLabel: "Retry save and continue",
          savingActionLabel: "Saving response...",
          saveFailureMessage: "Your input is still here. The response could not be saved yet.",
        }}
      />
      <div class="assessment-attempt-header">
        <p class="calm-status">
          {assessmentTypePresentation(props.context.assessment.assessmentType).label} Attempt{" "}
          {props.context.attemptNumber}
        </p>
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
                {formatAssessmentDeliveryTime(props.context.expiresAt, formatDateTime)}.
              </p>
            </Show>
          </div>
        </Show>
      </div>

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
          <p>Your saved responses are submitted for this Attempt.</p>
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
                    questionImageUrl={(asset) =>
                      new URL(
                        runtime.client.questionImageUrl(
                          currentPresentation.presentation.publishedQuestionRevisionTuple,
                          asset.questionImageAssetId,
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
                    assessmentAttemptId={props.context.assessmentAttemptId}
                    position={currentPresentation.position}
                  />
                </Show>
              </div>
              <div class="attempt-response">
                <QuestionPresentationResponseControl
                  attemptId={`${props.context.assessmentAttemptId}-${currentPresentation.position}`}
                  publishedQuestionRevisionTuple={
                    currentPresentation.presentation.publishedQuestionRevisionTuple
                  }
                  questionImageUrl={(asset) =>
                    new URL(
                      runtime.client.questionImageUrl(
                        currentPresentation.presentation.publishedQuestionRevisionTuple,
                        asset.questionImageAssetId,
                      ),
                      window.location.origin,
                    )
                  }
                  assessmentAttemptId={props.context.assessmentAttemptId}
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
                  persistenceNotice={saveState()}
                  persistenceDetail={saveError() ?? undefined}
                  mode="save"
                  onResponseEdit={(response) =>
                    responseEdited(currentPresentation.position, response)
                  }
                  onResponseChange={(response, validation, editGeneration) =>
                    responseValidated(
                      currentPresentation.position,
                      response,
                      validation,
                      editGeneration,
                    )
                  }
                  onSave={saveOutcome}
                  onEscape={() => finishAssessmentButton?.focus()}
                />
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
          <h2 id="assessment-attempt-submit-heading">Finish Attempt</h2>
          <p>
            Complete responses are saved before submission. Incomplete responses submit as
            unanswered unless an earlier saved response exists. This will submit your{" "}
            {assessmentTypePresentation(props.context.assessment.assessmentType).label} Attempt.
          </p>
          <button
            class="primary-action"
            type="button"
            ref={(element) => (finishAssessmentButton = element)}
            disabled={submissionState() === "submitting"}
            onClick={() => void submitAttempt()}
          >
            {submitAttemptActionLabel(submissionState())}
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
    </PageFrame>
  );
}

export function AssessmentAttemptPage(): JSX.Element {
  const params = useParams();
  const courseInstanceId = (): ReturnType<typeof parseCourseInstanceId> =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
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
        <PageFrame routeSurface="assessmentAttempt">
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
                  Loading your Attempt...
                </p>
              </Show>
            }
          >
            <p class="inline-error" role="alert">
              Your Attempt could not be loaded.
            </p>
            <button class="quiet-action" type="button" onClick={retryScope}>
              Retry
            </button>
          </Show>
        </PageFrame>
      }
    >
      {(loadedContext) => <AttemptExperience context={loadedContext} />}
    </Show>
  );
}
