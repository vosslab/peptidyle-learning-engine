import { createEffect, createSignal, on, onCleanup, onMount, Show, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ExternalToolLaunch } from "../../api/contracts";
import type { SubmissionOutcome } from "../../features/question_attempt/question_attempt_state";
import type { StudentResponseFormatCheck } from "../../wasm/index";
import { isCanonicalExternalToolLaunchPath } from "../../api/external_tool_launch";
import type { StudentWorkRouteScope } from "../question_response_controls/common";

import { handleQuestionResponseControlKeyDown } from "./keyboard";

type ExternalToolPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "awaitingReady" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "submitting" }
  | { readonly kind: "recoveryPending"; readonly message: string }
  | { readonly kind: "submitted" };

interface ExternalToolReadyMessage {
  readonly kind: "ple.externalTool.ready";
  readonly attemptId: string;
}

export interface ExternalToolResponseProps {
  readonly attemptId: string;
  readonly onSubmit: (response: StudentResponse) => Promise<SubmissionOutcome>;
  readonly onEscape: () => void;
  readonly onResponseChange?: (response: StudentResponse, validation: StudentResponseFormatCheck) => void;
  readonly studentWorkRoute?: StudentWorkRouteScope;
  readonly beginExternalToolLaunch?: () => Promise<ExternalToolLaunch>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Only a field-exact, attempt-bound readiness event crosses the frame boundary. */
export function isExternalToolReadyMessage(
  value: unknown,
  attemptId: string,
): value is ExternalToolReadyMessage {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  return (
    keys.length === 2 &&
    keys.includes("kind") &&
    keys.includes("attemptId") &&
    value["kind"] === "ple.externalTool.ready" &&
    value["attemptId"] === attemptId
  );
}

/**
 * Accept only a relative, same-origin broker path with no alternate URL syntax.
 * The origin is explicit so this boundary remains a pure contract that can be
 * tested without a browser global.
 */
export function isSafeExternalToolLaunchPath(
  launchUrl: string,
  courseId: string,
  assignmentId: string,
  attemptId: string,
  origin: string,
): boolean {
  return isCanonicalExternalToolLaunchPath(launchUrl, courseId, assignmentId, attemptId, origin);
}

function externalToolStatus(phase: ExternalToolPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Open the learning tool when you are ready.";
    case "loading":
      return "Opening the learning tool...";
    case "awaitingReady":
      return "The learning tool is loading. Complete its preparation to enable submission.";
    case "ready":
      return "The learning tool is ready. Submit when you are finished.";
    case "failed":
      return phase.message;
    case "submitting":
      return "Recording your external-tool response. Please wait.";
    case "recoveryPending":
      return phase.message;
    case "submitted":
      return "Response recorded. Server feedback will appear when it is released.";
  }
}

function externalToolFrameTitle(attemptId: string): string {
  return `External learning tool for question attempt ${attemptId}`;
}

/**
 * Student surface for a server-brokered external tool. The iframe receives a
 * protected same-origin broker path after explicit activation. Readiness is a
 * presentation signal only: it cannot supply a response, correctness, score,
 * provider identity, or any value used by grading.
 */
export function ExternalToolResponse(props: ExternalToolResponseProps): JSX.Element {
  const [phase, setPhase] = createSignal<ExternalToolPhase>({ kind: "idle" });
  const [launchUrl, setLaunchUrl] = createSignal<string | null>(null);
  let frame: HTMLIFrameElement | undefined;
  let submitButton: HTMLButtonElement | undefined;
  let launchRequest = 0;

  function marker(): StudentResponse {
    return { kind: "externalTool" };
  }

  function setFrame(element: HTMLIFrameElement): void {
    frame = element;
  }

  function setSubmitButton(element: HTMLButtonElement): void {
    submitButton = element;
  }

  function persistMarker(): void {
    props.onResponseChange?.(marker(), { violations: [] });
  }

  function resetForAttempt(): void {
    launchRequest += 1;
    frame = undefined;
    setLaunchUrl(null);
    setPhase({ kind: "idle" });
  }

  function handleMessage(event: MessageEvent<unknown>): void {
    if (
      event.origin !== window.location.origin ||
      event.source !== frame?.contentWindow ||
      !isExternalToolReadyMessage(event.data, props.attemptId)
    ) {
      return;
    }
    setPhase({ kind: "ready" });
    queueMicrotask(() => submitButton?.focus());
  }

  async function launch(): Promise<void> {
    const beginLaunch = props.beginExternalToolLaunch;
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
        !isSafeExternalToolLaunchPath(
          launchResult.launchUrl,
          studentWorkRoute.courseId,
          studentWorkRoute.assignmentId,
          props.attemptId,
          window.location.origin,
        )
      ) {
        setPhase({ kind: "failed", message: "The learning tool route was not safe to open." });
        return;
      }
      setLaunchUrl(launchResult.launchUrl);
      setPhase({ kind: "awaitingReady" });
    } catch (error: unknown) {
      if (request !== launchRequest) return;
      const detail = error instanceof Error ? ` ${error.message}` : "";
      setPhase({
        kind: "failed",
        message: `The learning tool is unavailable right now.${detail} Try again when ready.`,
      });
    }
  }

  async function submit(): Promise<void> {
    if (phase().kind !== "ready") return;
    setPhase({ kind: "submitting" });
    try {
      const outcome = await props.onSubmit(marker());
      switch (outcome.kind) {
        case "accepted":
          setPhase({ kind: "submitted" });
          return;
        case "recoveryPending":
          setPhase({ kind: "recoveryPending", message: outcome.message });
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
      () => void submit(),
      () => phase().kind === "ready",
    );
  }

  onMount(() => {
    window.addEventListener("message", handleMessage);
  });
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

  const statusId = (): string => `${props.attemptId}-external-tool-status`;
  return (
    <section
      class="response-widget external-tool-widget"
      data-phase={phase().kind}
      onKeyDown={handleKeyDown}
    >
      <h3>External learning tool</h3>
      <p class="field-help">
        Open the tool in this question. When it reports that it is ready, you can record this
        learning activity with the ordinary submission button.
      </p>
      <p id={statusId()} class="format-status" role="status" aria-live="polite">
        {externalToolStatus(phase())}
      </p>
      <Show when={launchUrl()}>
        {(source) => (
          <iframe
            ref={setFrame}
            class="external-tool-frame"
            src={source()}
            title={externalToolFrameTitle(props.attemptId)}
            sandbox="allow-forms allow-same-origin allow-scripts"
            referrerpolicy="same-origin"
            aria-describedby={statusId()}
          />
        )}
      </Show>
      <div class="external-tool-actions">
        <Show when={phase().kind === "idle" || phase().kind === "failed"}>
          <button class="primary-action" type="button" onClick={() => void launch()}>
            {phase().kind === "failed" ? "Retry learning tool" : "Open learning tool"}
          </button>
        </Show>
        <Show when={phase().kind === "loading"}>
          <button class="primary-action" type="button" disabled>
            Opening learning tool...
          </button>
        </Show>
        <button
          ref={setSubmitButton}
          class="primary-action"
          type="button"
          disabled={phase().kind !== "ready"}
          onClick={() => void submit()}
        >
          Submit answer
        </button>
        <button class="quiet-action" type="button" onClick={props.onEscape}>
          Return to assignment <span aria-hidden="true">(Esc)</span>
        </button>
      </div>
    </section>
  );
}
