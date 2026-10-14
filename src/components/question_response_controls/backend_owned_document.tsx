// Generic browser surface for a document and response owned by a Question Backend.

import {
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  Show,
  useContext,
  type JSX,
} from "solid-js";

import type { AssessmentAttemptId } from "../../../generated/api/AssessmentAttemptId";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import { postEmbedAppearance } from "../../appearance/embed_appearance";
import { AppearanceContext } from "../../appearance/appearance_context";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { QuestionResponseControlBaseProps, ResponsePersistenceNotice } from "./common";
import { handleQuestionResponseControlKeyDown } from "./keyboard";

const CAPTURE_REQUEST_PREFIX = "ple.backendOwned.capture:";
const READY_KIND = "ple.backendOwned.ready";
const RESPONSE_KIND = "ple.backendOwned.response";
const MAX_RESPONSE_BYTES = 65_536;

type BackendOwnedPhase = "loading" | "ready" | "capturing";

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

/**
 * A captured form is working state. "Response saved." appears only after this
 * Attempt accepts that response. Document readiness never represents persistence.
 */
export function backendOwnedStatusMessage(
  phase: BackendOwnedPhase,
  localMessage: string,
  persistenceNotice?: ResponsePersistenceNotice,
  persistenceDetail?: string,
  captureError?: string,
): string {
  if (captureError !== undefined) return captureError;
  if (phase === "loading" || phase === "capturing") return localMessage;
  if (persistenceNotice === "error") return persistenceDetail ?? "Response was not saved.";
  if (persistenceNotice === "saving") return "Saving response...";
  if (persistenceNotice === "saved") return "Response saved.";
  return localMessage;
}

/** Builds only the document route authorized by the current Assessment Attempt and position. */
export function backendOwnedDocumentPath(
  assessmentAttemptId: AssessmentAttemptId,
  position: number,
): string | null {
  if (parseAssessmentAttemptId(assessmentAttemptId) === null) return null;
  if (!Number.isSafeInteger(position) || position < 1 || position > 2_147_483_647) return null;
  return `/api/assessment-attempts/${encodeURIComponent(assessmentAttemptId)}/questions/${position}/document`;
}

export interface BackendOwnedDocumentProps extends QuestionResponseControlBaseProps {
  readonly assessmentAttemptId: AssessmentAttemptId;
  readonly position: number;
}

/**
 * PLE supplies the containing frame and durable response workflow. The backend
 * document retains its own elements, interactions, and authored presentation.
 */
export function BackendOwnedDocument(props: BackendOwnedDocumentProps): JSX.Element {
  const appearance = useContext(AppearanceContext);
  const [phase, setPhase] = createSignal<BackendOwnedPhase>("loading");
  const [message, setMessage] = createSignal("Loading Question...");
  const [captureError, setCaptureError] = createSignal<string>();
  let frame: HTMLIFrameElement | undefined;
  let saveButton: HTMLButtonElement | undefined;
  let pendingCapture: Promise<boolean> | undefined;
  let resolvePendingCapture: ((captured: boolean) => void) | undefined;
  let pendingCaptureId: string | undefined;
  let captureTimeout: ReturnType<typeof setTimeout> | undefined;
  let latestResponse: StudentResponse | undefined;
  let documentReady = false;
  const documentPath = backendOwnedDocumentPath(props.assessmentAttemptId, props.position);
  const sendAppearance = (): void =>
    appearance === undefined
      ? undefined
      : postEmbedAppearance(frame, appearance.appearance(), window.location.origin);
  createEffect(sendAppearance);

  function isFrameMessage(event: MessageEvent<unknown>): boolean {
    return event.origin === window.location.origin && event.source === frame?.contentWindow;
  }

  function clearPendingCapture(captured: boolean): void {
    const resolve = resolvePendingCapture;
    if (captureTimeout !== undefined) globalThis.clearTimeout(captureTimeout);
    captureTimeout = undefined;
    pendingCapture = undefined;
    resolvePendingCapture = undefined;
    pendingCaptureId = undefined;
    resolve?.(captured);
  }

  function responseReceived(
    pairs: ReadonlyArray<readonly [string, string]>,
    resolvesCapture: boolean,
  ): void {
    const response = backendOwnedResponseFromPairs(pairs);
    if (response === null) {
      setPhase("ready");
      setCaptureError("This response is too large to save.");
      if (resolvesCapture) clearPendingCapture(false);
      return;
    }
    const editGeneration = props.onResponseEdit?.(response);
    if (props.onResponseEdit !== undefined && editGeneration === undefined) {
      if (resolvesCapture) clearPendingCapture(false);
      return;
    }
    latestResponse = response;
    setCaptureError(undefined);
    props.onResponseChange?.(response, { issues: [] }, editGeneration);
    if (resolvesCapture) {
      setPhase("ready");
      clearPendingCapture(true);
    }
    setMessage("Response ready to save.");
  }

  function handleMessage(event: MessageEvent<unknown>): void {
    if (!isFrameMessage(event) || !isRecord(event.data)) return;
    if (Object.keys(event.data).length === 1 && event.data.kind === READY_KIND) {
      documentReady = true;
      setCaptureError(undefined);
      if (pendingCaptureId !== undefined) {
        frame?.contentWindow?.postMessage(
          backendOwnedCaptureRequest(pendingCaptureId),
          window.location.origin,
        );
        return;
      }
      setPhase("ready");
      setMessage("Question ready.");
      queueMicrotask(() => frame?.focus());
      return;
    }
    // ASVS 3.5.5: an unchanged reply advances only its exact pending capture.
    if (
      Object.keys(event.data).length === 2 &&
      event.data.kind === "ple.backendOwned.unchanged" &&
      isCaptureId(event.data.captureId) &&
      event.data.captureId === pendingCaptureId
    ) {
      setCaptureError(undefined);
      setPhase("ready");
      setMessage("Question ready.");
      clearPendingCapture(true);
      return;
    }
    const disposition = classifyBackendOwnedResponseMessage(event.data, pendingCaptureId);
    if (disposition !== null) responseReceived(disposition.pairs, disposition.kind === "capture");
  }

  function capture(): Promise<boolean> {
    if (pendingCapture !== undefined) return pendingCapture;
    if (frame?.contentWindow === null || frame?.contentWindow === undefined) {
      setCaptureError("This Question document is not ready.");
      return Promise.resolve(false);
    }
    setPhase("capturing");
    setCaptureError(undefined);
    setMessage("Saving response...");
    const captureId = newCaptureId();
    const capturePromise = new Promise<boolean>((resolve) => {
      resolvePendingCapture = resolve;
    });
    pendingCapture = capturePromise;
    pendingCaptureId = captureId;
    // Match the existing Draft deadline for a missing bridge readiness or
    // capture reply. Retry retains input when either message is unavailable.
    captureTimeout = globalThis.setTimeout(() => {
      setPhase("ready");
      setCaptureError(
        "The Question did not return your response. Your input is still here. Try Save again.",
      );
      clearPendingCapture(false);
    }, 5000);
    // READY establishes the untouched form baseline before any capture reads it.
    if (documentReady)
      frame.contentWindow.postMessage(
        backendOwnedCaptureRequest(captureId),
        window.location.origin,
      );
    return capturePromise;
  }

  async function captureAndSave(): Promise<void> {
    if (!(await capture()) || latestResponse === undefined || props.onSave === undefined) return;
    const outcome = await props.onSave(latestResponse);
    if (outcome.kind === "rejected") setMessage(outcome.message);
    queueMicrotask(() => saveButton?.focus());
  }

  function handleKeyDown(event: KeyboardEvent): void {
    handleQuestionResponseControlKeyDown(
      event,
      props.onEscape,
      () => void captureAndSave(),
      () => phase() === "ready",
    );
  }

  onMount(() => {
    window.addEventListener("message", handleMessage);
    const unregister = props.registerBackendOwnedCapture?.(capture);
    onCleanup(() => {
      unregister?.();
      clearPendingCapture(false);
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
            onLoad={sendAppearance}
          />
        )}
      </Show>
      <p class="format-status" role="status" aria-live="polite">
        {backendOwnedStatusMessage(
          phase(),
          message(),
          props.persistenceNotice,
          props.persistenceDetail,
          captureError(),
        )}
      </p>
      <button
        ref={(element) => (saveButton = element)}
        class="primary-action"
        type="button"
        disabled={phase() !== "ready"}
        onClick={() => void captureAndSave()}
      >
        {phase() === "capturing" ? "Saving response..." : (props.saveLabel ?? "Save response")}
      </button>
    </section>
  );
}
