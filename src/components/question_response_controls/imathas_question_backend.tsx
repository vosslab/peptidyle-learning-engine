import { createEffect, createSignal, on, onCleanup, onMount, Show, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ImathasQuestionBackendLaunch } from "../../api/contracts";
import type { StudentResponseFormatCheck } from "../../api/decoders/student_response_format_check";
import { isExpectedImathasQuestionBackendLaunchPath } from "../../api/imathas_question_backend_launch";
import type { StudentWorkRouteScope, ResponseSaveOutcome } from "./common";

import { handleQuestionResponseControlKeyDown } from "./keyboard";

type ImathasQuestionBackendPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "awaitingReady" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "saving" }
  | { readonly kind: "saved" };

interface ImathasQuestionBackendReadyMessage {
  readonly kind: "ple.imathasQuestionBackend.ready";
  readonly attemptId: string;
}

export interface ImathasQuestionBackendResponseProps {
  readonly attemptId: string;
  readonly onSave: (response: StudentResponse) => Promise<ResponseSaveOutcome>;
  readonly onEscape: () => void;
  readonly onResponseChange?: (
    response: StudentResponse,
    validation: StudentResponseFormatCheck,
    editGeneration?: number,
  ) => void;
  readonly onResponseEdit?: (response: StudentResponse) => number | undefined;
  readonly studentWorkRoute?: StudentWorkRouteScope;
  readonly beginImathasQuestionBackendLaunch?: () => Promise<ImathasQuestionBackendLaunch>;
}

/**
 * iMathAS owns the rendered activity, while the Student response remains this
 * exact marker. Route the marker through the same synchronous edit boundary as
 * native controls so a durable-save validator receives its current revision.
 */
export function persistImathasQuestionBackendMarker(props: {
  readonly onResponseEdit?: (response: StudentResponse) => number | undefined;
  readonly onResponseChange?: (
    response: StudentResponse,
    validation: StudentResponseFormatCheck,
    editGeneration?: number,
  ) => void;
}): void {
  const response: StudentResponse = { kind: "imathasQuestionBackend" };
  const editGeneration = props.onResponseEdit?.(response);
  props.onResponseChange?.(response, { issues: [] }, editGeneration);
}

/** Persist one marker per Question Attempt; retrying its launch retains that response. */
export function createImathasQuestionBackendMarkerPersistence(props: {
  readonly onResponseEdit?: (response: StudentResponse) => number | undefined;
  readonly onResponseChange?: (
    response: StudentResponse,
    validation: StudentResponseFormatCheck,
    editGeneration?: number,
  ) => void;
}): () => void {
  let persisted = false;
  return (): void => {
    if (persisted) return;
    persisted = true;
    persistImathasQuestionBackendMarker(props);
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Only a field-exact, attempt-bound readiness event crosses the frame boundary. */
export function isImathasQuestionBackendReadyMessage(
  value: unknown,
  attemptId: string,
): value is ImathasQuestionBackendReadyMessage {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  return (
    keys.length === 2 &&
    keys.includes("kind") &&
    keys.includes("attemptId") &&
    value["kind"] === "ple.imathasQuestionBackend.ready" &&
    value["attemptId"] === attemptId
  );
}

/** Accept only the exact same-origin iMathAS Question Backend route. */
export function isSafeImathasQuestionBackendLaunchPath(
  launchUrl: string,
  courseInstanceId: string,
  assessmentId: string,
  attemptId: string,
  origin: string,
): boolean {
  return isExpectedImathasQuestionBackendLaunchPath(
    launchUrl,
    courseInstanceId,
    assessmentId,
    attemptId,
    origin,
  );
}

function imathasQuestionBackendStatus(phase: ImathasQuestionBackendPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Open iMathAS when you are ready.";
    case "loading":
      return "Opening iMathAS...";
    case "awaitingReady":
      return "iMathAS is loading. Complete its preparation to enable save.";
    case "ready":
      return "iMathAS is ready. Save your response when you are finished.";
    case "failed":
      return phase.message;
    case "saving":
      return "Recording your iMathAS Question Backend response. Please wait.";
    case "saved":
      return "Response saved. You can continue working while the Attempt is open.";
  }
}

function imathasQuestionBackendFrameTitle(attemptId: string): string {
  return `iMathAS Question Backend for question attempt ${attemptId}`;
}

/**
 * Student surface for the server-owned iMathAS Question Backend Transport. The iframe
 * receives a protected same-origin path after explicit activation. Readiness is
 * presentation-only: it cannot supply a response, correctness, score, or result.
 */
export function ImathasQuestionBackendResponse(
  props: ImathasQuestionBackendResponseProps,
): JSX.Element {
  const [phase, setPhase] = createSignal<ImathasQuestionBackendPhase>({ kind: "idle" });
  const [launchUrl, setLaunchUrl] = createSignal<string | null>(null);
  let frame: HTMLIFrameElement | undefined;
  let saveButton: HTMLButtonElement | undefined;
  let launchRequest = 0;
  let persistMarker = createImathasQuestionBackendMarkerPersistence(props);

  function marker(): StudentResponse {
    return { kind: "imathasQuestionBackend" };
  }

  function setFrame(element: HTMLIFrameElement): void {
    frame = element;
  }

  function setSaveButton(element: HTMLButtonElement): void {
    saveButton = element;
  }

  function resetForAttempt(): void {
    launchRequest += 1;
    persistMarker = createImathasQuestionBackendMarkerPersistence(props);
    frame = undefined;
    setLaunchUrl(null);
    setPhase({ kind: "idle" });
  }

  function handleMessage(event: MessageEvent<unknown>): void {
    if (
      event.origin !== window.location.origin ||
      event.source !== frame?.contentWindow ||
      !isImathasQuestionBackendReadyMessage(event.data, props.attemptId)
    ) {
      return;
    }
    setPhase({ kind: "ready" });
    queueMicrotask(() => saveButton?.focus());
  }

  async function launch(): Promise<void> {
    const beginLaunch = props.beginImathasQuestionBackendLaunch;
    const studentWorkRoute = props.studentWorkRoute;
    if (beginLaunch === undefined || studentWorkRoute === undefined || phase().kind === "loading") {
      return;
    }
    persistMarker();
    launchRequest += 1;
    const request = launchRequest;
    setPhase({ kind: "loading" });
    try {
      const launchResult = await beginLaunch();
      if (request !== launchRequest) return;
      if (
        !isSafeImathasQuestionBackendLaunchPath(
          launchResult.launchUrl,
          studentWorkRoute.courseInstanceId,
          studentWorkRoute.assessmentId,
          props.attemptId,
          window.location.origin,
        )
      ) {
        setPhase({
          kind: "failed",
          message: "The iMathAS Question Backend route was not safe to open.",
        });
        return;
      }
      setLaunchUrl(launchResult.launchUrl);
      setPhase({ kind: "awaitingReady" });
    } catch (error: unknown) {
      if (request !== launchRequest) return;
      const detail = error instanceof Error ? ` ${error.message}` : "";
      setPhase({
        kind: "failed",
        message: `iMathAS is unavailable right now.${detail} Try again when ready.`,
      });
    }
  }

  async function save(): Promise<void> {
    if (phase().kind !== "ready" && phase().kind !== "saved") return;
    setPhase({ kind: "saving" });
    try {
      const outcome = await props.onSave(marker());
      switch (outcome.kind) {
        case "accepted":
          setPhase({ kind: "saved" });
          return;
        case "rejected":
          setPhase({ kind: "failed", message: outcome.message });
          return;
      }
    } catch (error: unknown) {
      const detail = error instanceof Error ? ` ${error.message}` : "";
      setPhase({
        kind: "failed",
        message: `Your response is still saved. Recording it did not finish.${detail} Try again.`,
      });
    }
  }

  function handleKeyDown(event: KeyboardEvent): void {
    handleQuestionResponseControlKeyDown(
      event,
      props.onEscape,
      () => void save(),
      () => phase().kind === "ready" || phase().kind === "saved",
    );
  }

  onMount(() => window.addEventListener("message", handleMessage));
  onCleanup(() => {
    launchRequest += 1;
    window.removeEventListener("message", handleMessage);
  });
  createEffect(
    on(
      () => props.attemptId,
      () => resetForAttempt(),
      { defer: true },
    ),
  );

  const statusId = (): string => `${props.attemptId}-imathas-question-backend-status`;
  return (
    <section
      class="question-response-control imathas-question-backend-response-control"
      data-phase={phase().kind}
      onKeyDown={handleKeyDown}
    >
      <h3>iMathAS Question Backend</h3>
      <p class="field-help">
        Open iMathAS in this question. When it reports that it is ready, you can record this
        activity with the ordinary save button.
      </p>
      <p id={statusId()} class="format-status" role="status" aria-live="polite">
        {imathasQuestionBackendStatus(phase())}
      </p>
      <Show when={launchUrl()}>
        {(source) => (
          <iframe
            ref={setFrame}
            class="imathas-question-backend-frame"
            src={source()}
            title={imathasQuestionBackendFrameTitle(props.attemptId)}
            sandbox="allow-forms allow-same-origin allow-scripts"
            referrerpolicy="same-origin"
            aria-describedby={statusId()}
          />
        )}
      </Show>
      <div class="imathas-question-backend-actions">
        <Show when={phase().kind === "idle" || phase().kind === "failed"}>
          <button class="primary-action" type="button" onClick={() => void launch()}>
            {phase().kind === "failed" ? "Retry iMathAS" : "Open iMathAS"}
          </button>
        </Show>
        <Show when={phase().kind === "loading"}>
          <button class="primary-action" type="button" disabled>
            Opening iMathAS...
          </button>
        </Show>
        <button
          ref={setSaveButton}
          class="primary-action"
          type="button"
          disabled={phase().kind !== "ready" && phase().kind !== "saved"}
          onClick={() => void save()}
        >
          Save response
        </button>
        <button class="quiet-action" type="button" onClick={props.onEscape}>
          Return to assessment <span aria-hidden="true">(Esc)</span>
        </button>
      </div>
    </section>
  );
}
