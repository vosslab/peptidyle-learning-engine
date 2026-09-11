// assignment_attempt_page.tsx - live one-question Student Assignment Attempt delivery.

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
  StudentAssignmentAttemptContext,
  StudentAssignmentAttemptResponseState,
} from "../api/assignment_attempt_navigation";
import type { StudentResponseFormatCheck } from "../api/decoders/student_response_format_check";
import { useApplicationApi } from "../api/application_api";
import { StudentAssignmentAttemptNavigation } from "../components/student_assignment_attempt_navigation";
import type { StudentAssignmentAttemptQuestionState } from "../components/student_assignment_attempt_navigation";
import { QuestionPresentationRenderer } from "../components/question_renderer";
import { QuestionPresentationResponseControl } from "../components/question_response_controls/question_response_control";
import { AssignmentAttemptResponseState } from "./assignment_attempt_response_state";
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
  readonly context: StudentAssignmentAttemptContext;
}): JSX.Element {
  const runtime = useApplicationApi();
  const validator = useWasmFacade();
  const [progress, setProgress] =
    createSignal<Awaited<ReturnType<typeof runtime.client.getStudentAssignmentAttemptProgress>>>();
  const [position, setPosition] = createSignal<number | null>(null);
  const [presentation, setPresentation] =
    createSignal<
      Awaited<ReturnType<typeof runtime.client.getStudentAssignmentAttemptPresentation>>
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
  const responseState = new AssignmentAttemptResponseState();
  let saveTimer: ReturnType<typeof globalThis.setTimeout> | undefined;
  let activeSave: Promise<boolean> | undefined;
  let finishAssignmentButton: HTMLButtonElement | undefined;

  const currentPosition = (): number | null => position();
  const isSubmitted = (): boolean => submissionState() === "submitted";

  async function loadProgress(): Promise<void> {
    progressRequest += 1;
    const request = progressRequest;
    setLoadError(null);
    try {
      const next = await runtime.client.getStudentAssignmentAttemptProgress(
        props.context.assignmentAttempt,
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
      setLoadError(errorMessage(error, "Could not load your Assignment Attempt."));
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
      const next = await runtime.client.getStudentAssignmentAttemptPresentation(
        props.context.assignmentAttempt,
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
        await runtime.client.saveStudentAssignmentAttemptResponse(
          props.context.assignmentAttempt,
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
    if (nextPosition === currentPosition() || isSubmitted()) return;
    if (!(await saveCurrentResponse())) return;
    setPosition(nextPosition);
  }

  async function submitAttempt(): Promise<void> {
    if (submissionState() === "submitting" || isSubmitted()) return;
    if (!(await saveCurrentResponse())) return;
    setSubmissionState("submitting");
    setSubmissionError(null);
    try {
      await runtime.client.submitStudentAssignmentAttempt(props.context.assignmentAttempt);
      setSubmissionState("submitted");
      await loadProgress();
    } catch (error: unknown) {
      setSubmissionState("error");
      setSubmissionError(
        `This Assignment Attempt was not submitted: ${errorMessage(error, "Please review your saved responses and try again.")}`,
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

  function saveOutcome(): Promise<
    import("../features/question_attempt/question_attempt_state").SubmissionOutcome
  > {
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
      .assignmentAttemptRemainingMilliseconds({
        initialRemainingMilliseconds: props.context.timerRemainingMilliseconds,
        elapsedMilliseconds,
      })
      .then((next) => {
        if (request === timerRequest) setRemainingMilliseconds(next);
      })
      .catch(() => {
        if (request === timerRequest) setTimerUnavailable(true);
      });
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
    readonly responseState: StudentAssignmentAttemptQuestionState;
  }> =>
    (progress()?.positions ?? []).map((item) => ({
      ...item,
      responseState: responseStateForNavigation(item.responseState),
    }));

  function responseStateForNavigation(
    responseState: StudentAssignmentAttemptResponseState,
  ): StudentAssignmentAttemptQuestionState {
    if (isSubmitted() || responseState === "submitted") return "closed";
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
    <section class="page assignment-attempt-page" data-route-surface="assignmentAttempt">
      <header class="assignment-attempt-header">
        <div>
          <p class="eyebrow">Assignment Attempt {props.context.attemptNumber}</p>
          <h1>{props.context.assignment.title}</h1>
        </div>
        <Show when={!isSubmitted()}>
          <span class="calm-status" role="timer">
            {timerUnavailable()
              ? "Timer unavailable; the server still enforces the time limit."
              : formatRemaining(remainingMilliseconds())}
          </span>
        </Show>
      </header>

      <Show
        when={progress()}
        fallback={
          <p class="loading-state" role="status">
            Loading Questions...
          </p>
        }
      >
        {(currentProgress) => (
          <>
            <Show when={currentPosition() !== null}>
              <p class="eyebrow">
                Question {currentPosition()} of {currentProgress().questionCount}
              </p>
            </Show>
            <StudentAssignmentAttemptNavigation
              positions={positions()}
              currentPosition={currentPosition()}
              onPositionActivate={(nextPosition) => void activatePosition(nextPosition)}
            />
          </>
        )}
      </Show>

      <Show when={loadError()}>
        {(message) => (
          <section class="attempt-recovery" role="alert">
            <p>{message()}</p>
            <button class="quiet-action" type="button" onClick={retryCurrentLoad}>
              Retry
            </button>
          </section>
        )}
      </Show>

      <Show when={isSubmitted()}>
        <section class="attempt-summary" aria-labelledby="assignment-submitted-heading">
          <h2 id="assignment-submitted-heading">Assignment submitted</h2>
          <p>Your saved responses are now submitted for this Assignment Attempt.</p>
        </section>
      </Show>

      <Show when={presentation()} keyed>
        {(currentPresentation) => (
          <Show when={!isSubmitted()}>
            <article class="question-card">
              <div class="prompt-copy">
                <ErrorBoundary fallback={<p class="inline-error">Question rendering failed.</p>}>
                  <QuestionPresentationRenderer
                    presentation={currentPresentation.presentation}
                    assetUrl={(asset) =>
                      new URL(runtime.client.assetUrl(asset.questionAsset), window.location.origin)
                    }
                  />
                </ErrorBoundary>
              </div>
              <div class="attempt-response">
                <QuestionPresentationResponseControl
                  attemptId={`${props.context.assignmentAttempt}-${currentPresentation.position}`}
                  responseFormat={currentPresentation.presentation.response}
                  initialResponse={currentPresentation.savedResponse ?? undefined}
                  validator={validator}
                  submitLabel="Save response"
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
                  onSubmit={saveOutcome}
                  onEscape={() => finishAssignmentButton?.focus()}
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

      <Show when={progress() && !isSubmitted()}>
        <section
          class="assignment-attempt-submit"
          aria-labelledby="assignment-attempt-submit-heading"
        >
          <h2 id="assignment-attempt-submit-heading">Finish Assignment</h2>
          <p>Save each response before submitting this Assignment Attempt.</p>
          <button
            class="primary-action"
            type="button"
            ref={(element) => (finishAssignmentButton = element)}
            disabled={submissionState() === "submitting"}
            onClick={() => void submitAttempt()}
          >
            {submissionState() === "submitting" ? "Submitting Assignment..." : "Submit Assignment"}
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

export function AssignmentAttemptPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const loadState = useRouteScopeLoadState();
  const retryScope = useRetryRouteScope();
  const context = (): StudentAssignmentAttemptContext | undefined => {
    const data = routeData();
    return data?.kind === "assignmentAttempt" ? data.context : undefined;
  };
  return (
    <Show
      when={context()}
      keyed
      fallback={
        <section class="page" data-route-surface="assignmentAttempt">
          <Show
            when={loadState() === "rejected"}
            fallback={
              <p class="loading-state" role="status">
                Loading your Assignment Attempt...
              </p>
            }
          >
            <p class="inline-error" role="alert">
              Your Assignment Attempt could not be loaded.
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
