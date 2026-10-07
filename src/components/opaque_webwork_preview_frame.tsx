// Opaque, no-write WeBWorK preview surface shared by Instructor inspection pages.

import {
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  Show,
  useContext,
  type JSX,
} from "solid-js";

import { postEmbedAppearance } from "../appearance/embed_appearance";
import { AppearanceContext } from "../appearance/appearance_context";

const PREVIEW_RESIZE_KIND = "ple.webwork.preview.resize";
const PREVIEW_RESIZE_VERSION = 1;
const PREVIEW_MINIMUM_HEIGHT = 160;
const PREVIEW_MAXIMUM_HEIGHT = 1200;
const DRAFT_TEST_CAPTURE_KIND = "ple.webwork.draft-test.capture";
const DRAFT_TEST_RESPONSE_KIND = "ple.webwork.draft-test.response";

export type OpaqueWebworkResponsePairs = ReadonlyArray<readonly [string, string]>;

type PreviewResizeMessage = {
  readonly kind: typeof PREVIEW_RESIZE_KIND;
  readonly version: typeof PREVIEW_RESIZE_VERSION;
  readonly height: number;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isPreviewResizeMessage(value: unknown): value is PreviewResizeMessage {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  return (
    keys.length === 3 &&
    keys.includes("kind") &&
    keys.includes("version") &&
    keys.includes("height") &&
    value.kind === PREVIEW_RESIZE_KIND &&
    value.version === PREVIEW_RESIZE_VERSION &&
    typeof value.height === "number" &&
    Number.isSafeInteger(value.height) &&
    value.height >= PREVIEW_MINIMUM_HEIGHT &&
    value.height <= PREVIEW_MAXIMUM_HEIGHT
  );
}

function isDraftTestResponse(value: unknown): value is {
  readonly kind: typeof DRAFT_TEST_RESPONSE_KIND;
  readonly version: 1;
  readonly captureId: string;
  readonly pairs: OpaqueWebworkResponsePairs;
} {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  if (
    keys.length !== 4 ||
    !keys.includes("kind") ||
    !keys.includes("version") ||
    !keys.includes("captureId") ||
    !keys.includes("pairs") ||
    value.kind !== DRAFT_TEST_RESPONSE_KIND ||
    value.version !== 1 ||
    typeof value.captureId !== "string" ||
    !/^[a-f0-9]{16}$/u.test(value.captureId) ||
    !Array.isArray(value.pairs) ||
    value.pairs.length > 1024
  )
    return false;
  let totalBytes = 0;
  for (const pair of value.pairs) {
    if (
      !Array.isArray(pair) ||
      pair.length !== 2 ||
      typeof pair[0] !== "string" ||
      typeof pair[1] !== "string"
    )
      return false;
    totalBytes +=
      new TextEncoder().encode(pair[0]).length + new TextEncoder().encode(pair[1]).length;
    if (pair[0].length > 1024 || pair[1].length > 65536 || totalBytes > 65536) return false;
  }
  return true;
}

export interface OpaqueWebworkPreviewFrameProps {
  readonly class: string;
  readonly src: string;
  readonly title: string;
  /** Enables one parent-mediated Draft test without granting the frame form or network access. */
  readonly onDraftTestResponse?: (pairs: OpaqueWebworkResponsePairs) => void;
}

/** Renders a backend-owned preview that may report only its bounded visible height. */
export function OpaqueWebworkPreviewFrame(props: OpaqueWebworkPreviewFrameProps): JSX.Element {
  const appearance = useContext(AppearanceContext);
  const [height, setHeight] = createSignal(PREVIEW_MINIMUM_HEIGHT);
  const [captureStatus, setCaptureStatus] = createSignal("");
  const [capturing, setCapturing] = createSignal(false);
  let frame: HTMLIFrameElement | undefined;
  let pendingCaptureId: string | undefined;
  let captureTimeout: number | undefined;
  // This opaque sandbox has no stable target origin. The closed record contains
  // only cosmetic colors, so `*` cannot disclose Course, Account, or response data.
  const sendAppearance = (): void => {
    if (appearance !== undefined) postEmbedAppearance(frame, appearance.appearance(), "*");
  };
  createEffect(sendAppearance);

  function handleMessage(event: MessageEvent<unknown>): void {
    // ASVS 3.5.5: opaque origin plus exact frame identity and a closed bounded record.
    if (event.origin !== "null" || event.source !== frame?.contentWindow) return;
    if (isPreviewResizeMessage(event.data)) {
      setHeight(event.data.height);
      return;
    }
    if (!isDraftTestResponse(event.data) || event.data.captureId !== pendingCaptureId) return;
    pendingCaptureId = undefined;
    if (captureTimeout !== undefined) window.clearTimeout(captureTimeout);
    setCapturing(false);
    setCaptureStatus("Testing this response...");
    props.onDraftTestResponse?.(event.data.pairs);
  }

  function requestDraftTest(): void {
    const contentWindow = frame?.contentWindow;
    if (
      props.onDraftTestResponse === undefined ||
      capturing() ||
      contentWindow === null ||
      contentWindow === undefined
    )
      return;
    let sourceUrl: URL;
    try {
      sourceUrl = new URL(props.src, window.location.href);
    } catch {
      setCaptureStatus("This Draft preview cannot capture a test response.");
      return;
    }
    if (
      sourceUrl.origin !== window.location.origin ||
      sourceUrl.searchParams.get("draftTest") !== "true"
    ) {
      setCaptureStatus("This Draft preview cannot capture a test response.");
      return;
    }
    const captureId = Array.from(crypto.getRandomValues(new Uint8Array(8)))
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join("");
    pendingCaptureId = captureId;
    setCapturing(true);
    setCaptureStatus("");
    contentWindow.postMessage({ kind: DRAFT_TEST_CAPTURE_KIND, version: 1, captureId }, "*");
    captureTimeout = window.setTimeout(() => {
      if (pendingCaptureId !== captureId) return;
      pendingCaptureId = undefined;
      setCapturing(false);
      setCaptureStatus(
        "The Draft preview did not return a test response. Reload it and try again.",
      );
    }, 5000);
  }

  onMount(() => {
    window.addEventListener("message", handleMessage);
    onCleanup(() => {
      window.removeEventListener("message", handleMessage);
      if (captureTimeout !== undefined) window.clearTimeout(captureTimeout);
    });
  });

  return (
    <>
      <iframe
        ref={(element) => (frame = element)}
        class={props.class}
        src={props.src}
        title={props.title}
        sandbox="allow-scripts"
        referrerpolicy="no-referrer"
        allow=""
        onLoad={sendAppearance}
        style={{ height: `${height()}px` }}
      />
      <Show when={props.onDraftTestResponse !== undefined}>
        <div class="opaque-webwork-preview-test">
          <button type="button" disabled={capturing()} onClick={requestDraftTest}>
            {capturing() ? "Reading response..." : "Test this response"}
          </button>
          <Show when={captureStatus()}>{(status) => <p role="status">{status()}</p>}</Show>
        </div>
      </Show>
    </>
  );
}
