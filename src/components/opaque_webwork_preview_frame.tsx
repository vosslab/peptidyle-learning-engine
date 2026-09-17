// Opaque, no-write WeBWorK preview surface shared by Instructor inspection pages.

import { createSignal, onCleanup, onMount, type JSX } from "solid-js";

const PREVIEW_RESIZE_KIND = "ple.webwork.preview.resize";
const PREVIEW_RESIZE_VERSION = 1;
const PREVIEW_MINIMUM_HEIGHT = 160;
const PREVIEW_MAXIMUM_HEIGHT = 1200;

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

export interface OpaqueWebworkPreviewFrameProps {
  readonly class: string;
  readonly src: string;
  readonly title: string;
}

/** Renders a backend-owned preview that may report only its bounded visible height. */
export function OpaqueWebworkPreviewFrame(props: OpaqueWebworkPreviewFrameProps): JSX.Element {
  const [height, setHeight] = createSignal(PREVIEW_MINIMUM_HEIGHT);
  let frame: HTMLIFrameElement | undefined;

  function handleResize(event: MessageEvent<unknown>): void {
    // ASVS 3.5.5: the opaque sandbox has origin null, so source identity and
    // a closed, bounded payload are both required before changing layout.
    if (
      event.origin !== "null" ||
      event.source !== frame?.contentWindow ||
      !isPreviewResizeMessage(event.data)
    )
      return;
    setHeight(event.data.height);
  }

  onMount(() => {
    window.addEventListener("message", handleResize);
    onCleanup(() => window.removeEventListener("message", handleResize));
  });

  return (
    <iframe
      ref={(element) => (frame = element)}
      class={props.class}
      src={props.src}
      title={props.title}
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      allow=""
      style={{ height: `${height()}px` }}
    />
  );
}
