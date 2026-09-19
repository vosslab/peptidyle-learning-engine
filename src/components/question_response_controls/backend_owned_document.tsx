// Generic browser surface for a document and response owned by a Question Backend.

import { createSignal, onCleanup, onMount, Show, type JSX } from "solid-js";

import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { QuestionResponseControlBaseProps } from "./common";
import { handleQuestionResponseControlKeyDown } from "./keyboard";

const CAPTURE_REQUEST_PREFIX = "ple.backendOwned.capture:";
const READY_KIND = "ple.backendOwned.ready";
const RESPONSE_KIND = "ple.backendOwned.response";
const MAX_RESPONSE_BYTES = 65_536;

type BackendOwnedPhase = "loading" | "ready" | "capturing" | "saved" | "failed";

interface BackendOwnedResponseMessage {
  readonly kind: typeof RESPONSE_KIND;
  readonly pairs: ReadonlyArray<readonly [string, string]>;
  readonly captureId?: string;
}

type BackendOwnedResponseDisposition =
  | { readonly kind: "ordinary"; readonly pairs: ReadonlyArray<readonly [string, string]> }
  | { readonly kind: "capture"; readonly pairs: ReadonlyArray<readonly [string, string]> };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCaptureId(value: unknown): value is string {
  return typeof value === "string" && /^[a-f0-9]{16}$/u.test(value);
}

/** Closed string request understood by the backend document bridge. */
export function backendOwnedCaptureRequest(captureId: string): string {
  return `${CAPTURE_REQUEST_PREFIX}${captureId}`;
}

/** The bridge exposes one closed response shape and preserves form order and duplicate names. */
export function isBackendOwnedResponseMessage(
  value: unknown,
): value is BackendOwnedResponseMessage {
  if (!isRecord(value) || value.kind !== RESPONSE_KIND) return false;
  const keys = Object.keys(value);
  if (keys.length !== 2 && keys.length !== 3) return false;
  if (keys.length === 3 && !isCaptureId(value.captureId)) return false;
  if (keys.length === 2 && "captureId" in value) return false;
  if (!Array.isArray(value.pairs)) return false;
  return value.pairs.every(
    (pair) =>
      Array.isArray(pair) &&
      pair.length === 2 &&
      typeof pair[0] === "string" &&
      typeof pair[1] === "string",
  );
}

/**
 * Separates an ordinary bridge save from the one reply requested by the
 * active capture. A capture can advance Assessment Finish only on its exact ID.
 */
export function classifyBackendOwnedResponseMessage(
  value: unknown,
  pendingCaptureId: string | undefined,
): BackendOwnedResponseDisposition | null {
  if (!isBackendOwnedResponseMessage(value)) return null;
  if (value.captureId === undefined) return { kind: "ordinary", pairs: value.pairs };
  if (value.captureId !== pendingCaptureId) return null;
  return { kind: "capture", pairs: value.pairs };
}

function newCaptureId(): string {
  const words = new Uint32Array(2);
  crypto.getRandomValues(words);
  return [...words].map((word) => word.toString(16).padStart(8, "0")).join("");
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

/** Canonical base64 JSON accepted by the shared BackendOwned response contract. */
export function backendOwnedResponseFromPairs(
  pairs: ReadonlyArray<readonly [string, string]>,
): StudentResponse | null {
  const bytes = new TextEncoder().encode(JSON.stringify(pairs));
  if (bytes.length > MAX_RESPONSE_BYTES) return null;
  return { kind: "backendOwned", payload: bytesToBase64(bytes) };
}

/** Builds only the document route authorized by the current Assessment Attempt and position. */
export function backendOwnedDocumentPath(
  assessmentAttempt: AssessmentAttemptId,
  position: number,
): string | null {
  if (parseAssessmentAttemptId(assessmentAttempt) === null) return null;
  if (!Number.isSafeInteger(position) || position < 1 || position > 2_147_483_647) return null;
  return `/api/assessment-attempts/${encodeURIComponent(assessmentAttempt)}/questions/${position}/document`;
}

export interface BackendOwnedDocumentProps extends QuestionResponseControlBaseProps {
  readonly assessmentAttempt: AssessmentAttemptId;
  readonly position: number;
}

/**
 * PLE supplies the containing frame and durable response workflow. The backend
 * document retains its own elements, interactions, and authored presentation.
 */
export function BackendOwnedDocument(props: BackendOwnedDocumentProps): JSX.Element {
  const [phase, setPhase] = createSignal<BackendOwnedPhase>("loading");
  const [message, setMessage] = createSignal("Loading Question...");
  let frame: HTMLIFrameElement | undefined;
  let saveButton: HTMLButtonElement | undefined;
  let pendingCapture: Promise<StudentResponse | null> | undefined;
  let resolvePendingCapture: ((response: StudentResponse | null) => void) | undefined;
  let pendingCaptureId: string | undefined;
  const documentPath = backendOwnedDocumentPath(props.assessmentAttempt, props.position);

  function isFrameMessage(event: MessageEvent<unknown>): boolean {
    return event.origin === window.location.origin && event.source === frame?.contentWindow;
  }

  function clearPendingCapture(response: StudentResponse | null): void {
    const resolve = resolvePendingCapture;
    pendingCapture = undefined;
    resolvePendingCapture = undefined;
    pendingCaptureId = undefined;
    resolve?.(response);
  }

  function responseReceived(
    pairs: ReadonlyArray<readonly [string, string]>,
    resolvesCapture: boolean,
  ): void {
    const response = backendOwnedResponseFromPairs(pairs);
    if (response === null) {
      setPhase("failed");
      setMessage("This response is too large to save.");
      if (resolvesCapture) clearPendingCapture(null);
      return;
    }
    const revision = props.onResponseEdit?.(response);
    if (props.onResponseEdit !== undefined && revision === undefined) {
      if (resolvesCapture) clearPendingCapture(null);
      return;
    }
    props.onResponseChange?.(response, { issues: [] }, revision);
    setPhase("saved");
    setMessage("Response saved.");
    if (resolvesCapture) clearPendingCapture(response);
    queueMicrotask(() => saveButton?.focus());
  }

  function handleMessage(event: MessageEvent<unknown>): void {
    if (!isFrameMessage(event) || !isRecord(event.data)) return;
    if (Object.keys(event.data).length === 1 && event.data.kind === READY_KIND) {
      setPhase("ready");
      setMessage("Question ready.");
      queueMicrotask(() => frame?.focus());
      return;
    }
    const disposition = classifyBackendOwnedResponseMessage(event.data, pendingCaptureId);
    if (disposition !== null) responseReceived(disposition.pairs, disposition.kind === "capture");
  }

  function capture(): Promise<StudentResponse | null> {
    if (pendingCapture !== undefined) return pendingCapture;
    if (phase() !== "ready" && phase() !== "saved") return Promise.resolve(null);
    if (frame?.contentWindow === null || frame?.contentWindow === undefined) {
      setPhase("failed");
      setMessage("This Question document is not ready.");
      return Promise.resolve(null);
    }
    setPhase("capturing");
    setMessage("Saving response...");
    const captureId = newCaptureId();
    const capturePromise = new Promise<StudentResponse | null>((resolve) => {
      resolvePendingCapture = resolve;
    });
    pendingCapture = capturePromise;
    pendingCaptureId = captureId;
    frame.contentWindow.postMessage(backendOwnedCaptureRequest(captureId), window.location.origin);
    return capturePromise;
  }

  async function captureAndSave(): Promise<void> {
    const response = await capture();
    if (response === null || props.onSave === undefined) return;
    const outcome = await props.onSave(response);
    if (outcome.kind === "accepted") {
      setPhase("saved");
      setMessage("Response saved.");
      return;
    }
    setPhase("failed");
    setMessage(outcome.message);
  }

  function handleKeyDown(event: KeyboardEvent): void {
    handleQuestionResponseControlKeyDown(
      event,
      props.onEscape,
      () => void captureAndSave(),
      () => phase() === "ready" || phase() === "saved",
    );
  }

  onMount(() => {
    window.addEventListener("message", handleMessage);
    const unregister = props.registerBackendOwnedCapture?.(async () => (await capture()) !== null);
    onCleanup(() => {
      unregister?.();
      clearPendingCapture(null);
      window.removeEventListener("message", handleMessage);
    });
  });

  return (
    <section class="question-response-control backend-owned-document" onKeyDown={handleKeyDown}>
      <Show
        when={documentPath}
        fallback={
          <p class="inline-error" role="alert">
            This Question document is not available right now.
          </p>
        }
      >
        {(documentPath) => (
          <iframe
            ref={(element) => (frame = element)}
            class="backend-owned-document__frame"
            src={documentPath()}
            title="Question document"
            sandbox="allow-scripts allow-forms allow-same-origin"
            referrerpolicy="same-origin"
          />
        )}
      </Show>
      <p class="format-status" role="status" aria-live="polite">
        {message()}
      </p>
      <button
        ref={(element) => (saveButton = element)}
        class="primary-action"
        type="button"
        disabled={phase() !== "ready" && phase() !== "saved"}
        onClick={() => void captureAndSave()}
      >
        {phase() === "capturing" ? "Saving response..." : (props.saveLabel ?? "Save response")}
      </button>
    </section>
  );
}
