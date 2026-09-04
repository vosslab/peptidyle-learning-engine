// question_renderer.tsx - browser-safe Question Variation Presentation rendering.

import { createUniqueId, ErrorBoundary, For, onMount, type JSX } from "solid-js";
import temml from "temml";

import type { QuestionAssetReference } from "../../generated/api/QuestionAssetReference";
import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { QuestionResponseFormat } from "../../generated/api/QuestionResponseFormat";
import type { QuestionPresentation } from "../../generated/api/QuestionPresentation";

import { QUESTION_RENDERER_STYLES } from "./question_renderer_styles";

/**
 * MathML emitted by Temml is still parsed as untrusted XML. These are the presentation MathML
 * nodes and non-URL attributes PLE accepts after conversion. In particular, links, annotation,
 * CSS, event handlers, and foreign namespaces have no path through this boundary.
 */
const MATHML_NAMESPACE = "http://www.w3.org/1998/Math/MathML";
const ALLOWED_MATHML_TAGS = new Set([
  "math",
  "merror",
  "mfrac",
  "mi",
  "mmultiscripts",
  "mn",
  "mo",
  "mover",
  "mpadded",
  "mphantom",
  "mroot",
  "mrow",
  "ms",
  "mspace",
  "msqrt",
  "mstyle",
  "msub",
  "msubsup",
  "msup",
  "mtable",
  "mtd",
  "mtext",
  "mtr",
  "munder",
  "munderover",
  "none",
]);
const ALLOWED_MATHML_ATTRIBUTES = new Set([
  "accent",
  "accentunder",
  "align",
  "bevelled",
  "columnalign",
  "columnlines",
  "columnspacing",
  "columnspan",
  "depth",
  "displaystyle",
  "display",
  "fence",
  "form",
  "height",
  "largeop",
  "length",
  "linethickness",
  "lspace",
  "mathbackground",
  "mathcolor",
  "mathsize",
  "mathvariant",
  "maxsize",
  "minsize",
  "movablelimits",
  "notation",
  "rowalign",
  "rowlines",
  "rowspacing",
  "rowspan",
  "rspace",
  "scriptlevel",
  "separator",
  "symmetric",
  "voffset",
  "width",
  "xmlns",
]);

type SafeMathMlNode = SafeMathMlTextNode | SafeMathMlElementNode;
type SafeMathMlTextNode = { readonly kind: "text"; readonly text: string };
type SafeMathMlElementNode = {
  readonly kind: "element";
  readonly tag: string;
  readonly attributes: ReadonlyMap<string, string>;
  readonly children: ReadonlyArray<SafeMathMlNode>;
};

type SanitizedMathMl = { readonly root: SafeMathMlElementNode };

/** A narrow callback that must derive the documented application asset route from an QuestionAssetReference. */
export type AssetUrlResolver = (questionAsset: QuestionAssetReference) => URL;

/**
 * The identity-free Question Variation Presentation that the browser renderer needs. Published
 * Question Presentations and private workspace previews can share this content without sharing a
 * publication identity.
 */
export interface QuestionVariationPresentation {
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
}

export interface QuestionRendererProps {
  readonly presentation: QuestionVariationPresentation;
  readonly assetUrl: AssetUrlResolver;
  /** Re-fetches only this question resource; the surrounding Assignment Attempt shell remains intact. */
  readonly onRetry?: () => void;
}

/** The semantic, answer-free prompt block surface shared by question views. */
export interface QuestionPromptRendererProps {
  readonly blocks: ReadonlyArray<QuestionContentBlock>;
  readonly assetUrl: AssetUrlResolver;
}

export class QuestionContentError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "QuestionContentError";
  }
}

function assertNever(value: never): never {
  throw new QuestionContentError(`Unknown prompt block: ${JSON.stringify(value)}`);
}

function hasRequiredDescription(description: string): boolean {
  return description.trim().length > 0;
}

/** Missing figure or math alternatives are authoring errors, not a silent visual-only fallback. */
export function requireAccessibilityDescription(
  description: string,
  kind: QuestionContentBlock["kind"],
): string {
  if (!hasRequiredDescription(description)) {
    throw new QuestionContentError(
      `This ${kind} block cannot be shown because its required accessibility description is missing.`,
    );
  }
  return description;
}

function rejectUnsafeAttribute(name: string, value: string): void {
  const normalized = value.replace(/\s/gu, "").toLowerCase();
  if (
    name.startsWith("on") ||
    name === "style" ||
    name === "srcdoc" ||
    name === "ping" ||
    name === "poster" ||
    normalized.startsWith("javascript:") ||
    normalized.startsWith("data:") ||
    normalized.startsWith("blob:") ||
    normalized.startsWith("http:") ||
    normalized.startsWith("https:") ||
    normalized.startsWith("//")
  ) {
    throw new QuestionContentError("Supplied markup contains an unsafe attribute or URL scheme.");
  }
}

function mathMlAttributesFromElement(element: Element): ReadonlyMap<string, string> {
  const attributes = new Map<string, string>();
  for (const attribute of Array.from(element.attributes)) {
    const name = attribute.name.toLowerCase();
    const value = attribute.value;
    if (!ALLOWED_MATHML_ATTRIBUTES.has(name)) {
      throw new QuestionContentError(`Converted MathML attribute ${name} is not allowlisted.`);
    }
    if (name === "xmlns" && value !== MATHML_NAMESPACE) {
      throw new QuestionContentError("Converted MathML must use the standard MathML namespace.");
    }
    if (name !== "xmlns") rejectUnsafeAttribute(name, value);
    attributes.set(name, value);
  }
  return attributes;
}

function sanitizeMathMlNode(node: Node): SafeMathMlNode {
  if (node.nodeType === Node.TEXT_NODE) {
    return { kind: "text", text: node.textContent ?? "" };
  }
  if (node.nodeType !== Node.ELEMENT_NODE) {
    throw new QuestionContentError("Converted MathML contains a non-structural node.");
  }
  const element = node as Element;
  const tag = element.localName.toLowerCase();
  if (element.namespaceURI !== MATHML_NAMESPACE || !ALLOWED_MATHML_TAGS.has(tag)) {
    throw new QuestionContentError("Converted MathML contains an unsupported element.");
  }
  return {
    kind: "element",
    tag,
    attributes: mathMlAttributesFromElement(element),
    children: Array.from(element.childNodes, sanitizeMathMlNode),
  };
}

/**
 * Converts TeX only through Temml, then validates it through a fresh MathML allowlist.
 * The resulting object contains no raw TeX-derived markup string and is safe to construct with
 * namespace-aware DOM APIs.
 */
function renderLatexToMathMl(latex: string): SanitizedMathMl {
  if (typeof DOMParser === "undefined") {
    throw new QuestionContentError("Math rendering requires a browser document.");
  }
  let converted: string;
  try {
    converted = temml.renderToString(latex, {
      displayMode: false,
      throwOnError: true,
      trust: false,
      xml: true,
    });
  } catch (_error: unknown) {
    throw new QuestionContentError(
      "This math content could not be rendered. Please ask the instructor to correct its TeX.",
    );
  }
  const parsed = new DOMParser().parseFromString(converted, "application/xml");
  if (parsed.querySelector("parsererror") !== null) {
    throw new QuestionContentError(
      "This math content could not be rendered. Please ask the instructor to correct its TeX.",
    );
  }
  const root = sanitizeMathMlNode(parsed.documentElement);
  if (root.kind !== "element" || root.tag !== "math") {
    throw new QuestionContentError("Converted TeX did not produce a MathML root element.");
  }
  return { root };
}

/** Refuse every route except the authorized, logical application asset endpoint. */
export function resolveSameOriginAssetUrl(
  questionAsset: QuestionAssetReference,
  resolver: AssetUrlResolver,
): string {
  const url = resolver(questionAsset);
  const expectedPath = `/api/assets/${encodeURIComponent(questionAsset.questionAsset)}`;
  if (
    url.origin !== globalThis.location.origin ||
    url.pathname !== expectedPath ||
    url.search !== "" ||
    url.hash !== "" ||
    url.username !== "" ||
    url.password !== ""
  ) {
    throw new QuestionContentError(
      "Question assets must use the authorized logical /api/assets/{asset-id} route.",
    );
  }
  return url.href;
}

/**
 * Recovers a protected image after its intentionally concealed logical GET
 * returns 404. The recovery is an explicit same-origin POST; it is never
 * attempted for a URL outside the logical asset route and it runs once only.
 * Public immutable assets keep their ordinary cacheable GET path.
 */
export function recoverProtectedAssetImage(event: Event): void {
  const image = event.currentTarget;
  if (!(image instanceof HTMLImageElement)) return;
  if (image.dataset.pleDeliveryAttempted === "true") return;
  let logical: URL;
  try {
    logical = new URL(image.currentSrc || image.src, globalThis.location.origin);
  } catch (_error: unknown) {
    return;
  }
  if (
    logical.origin !== globalThis.location.origin ||
    !/^\/api\/assets\/[^/]+$/u.test(logical.pathname) ||
    logical.search !== "" ||
    logical.hash !== ""
  )
    return;
  image.dataset.pleDeliveryAttempted = "true";
  const delivery = new URL(`${logical.pathname}/delivery`, logical.origin);
  void globalThis
    .fetch(delivery, {
      method: "POST",
      headers: { accept: "application/json" },
      credentials: "same-origin",
      cache: "no-store",
    })
    .then(async (response) => {
      if (!response.ok) return;
      const body: unknown = await response.json();
      if (typeof body !== "object" || body === null || Array.isArray(body)) return;
      const value = (body as Record<string, unknown>).url;
      if (typeof value !== "string") return;
      const signed = new URL(value);
      if (
        (signed.protocol !== "https:" && signed.protocol !== "http:") ||
        signed.username !== "" ||
        signed.password !== ""
      )
        return;
      image.referrerPolicy = "no-referrer";
      image.src = signed.href;
    })
    .catch(() => undefined);
}

function appendSafeMathMlNode(document: Document, parent: Node, node: SafeMathMlNode): void {
  if (node.kind === "text") {
    parent.appendChild(document.createTextNode(node.text));
    return;
  }
  const element = document.createElementNS(MATHML_NAMESPACE, node.tag);
  for (const [name, value] of node.attributes) {
    element.setAttribute(name, value);
  }
  parent.appendChild(element);
  for (const child of node.children) {
    appendSafeMathMlNode(document, element, child);
  }
}

function RenderedMath(props: {
  readonly latex: string;
  readonly description: string;
}): JSX.Element {
  const hostId = createUniqueId();
  const mathMl = renderLatexToMathMl(props.latex);
  onMount(() => {
    const host = document.getElementById(hostId);
    if (!(host instanceof HTMLSpanElement)) {
      throw new QuestionContentError("MathML host was not available.");
    }
    appendSafeMathMlNode(document, host, mathMl.root);
    const math = host.firstElementChild;
    if (math?.localName !== "math") {
      throw new QuestionContentError("Converted MathML root was not available.");
    }
    math.setAttribute("aria-label", props.description);
  });
  return (
    <span class="question-renderer__math" aria-label={props.description}>
      <span id={hostId} />
      <span class="visually-hidden">{props.description}</span>
    </span>
  );
}

function QuestionContentBlockRenderer(props: {
  readonly block: QuestionContentBlock;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  switch (props.block.kind) {
    case "text":
      return <p class="question-renderer__block">{props.block.markdown}</p>;
    case "math": {
      const description = requireAccessibilityDescription(
        props.block.description,
        props.block.kind,
      );
      return <RenderedMath latex={props.block.latex} description={description} />;
    }
    case "image": {
      const description = requireAccessibilityDescription(
        props.block.description,
        props.block.kind,
      );
      return (
        <figure class="question-renderer__figure">
          <img
            class="question-renderer__image"
            src={resolveSameOriginAssetUrl(props.block.questionAsset, props.assetUrl)}
            alt={description}
            onError={recoverProtectedAssetImage}
          />
          <figcaption>{description}</figcaption>
        </figure>
      );
    }
    case "code":
      return (
        <pre class="question-renderer__code">
          <code data-language={props.block.language}>{props.block.source}</code>
        </pre>
      );
    case "table": {
      const description = requireAccessibilityDescription(
        props.block.description,
        props.block.kind,
      );
      return (
        <div class="question-renderer__table-wrap">
          <table>
            <caption>{description}</caption>
            <thead>
              <tr>
                <For each={props.block.headers}>{(header) => <th scope="col">{header}</th>}</For>
              </tr>
            </thead>
            <tbody>
              <For each={props.block.rows}>
                {(row) => (
                  <tr>
                    <For each={row}>{(cell) => <td>{cell}</td>}</For>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      );
    }
    default:
      return assertNever(props.block);
  }
}

function RendererFailure(props: {
  readonly reset: () => void;
  readonly onRetry?: () => void;
  readonly message: string;
}): JSX.Element {
  return (
    <section class="question-renderer__error" role="alert" aria-live="assertive">
      <h2>Question content needs attention</h2>
      <p>{props.message}</p>
      <button
        class="question-renderer__retry"
        type="button"
        onClick={() => {
          props.onRetry?.();
          props.reset();
        }}
      >
        Try loading this question again
      </button>
    </section>
  );
}

/** Renders Question Content Blocks without rendering a Question Response Format or grading data. */
export function QuestionPromptRenderer(props: QuestionPromptRendererProps): JSX.Element {
  return (
    <>
      <style>{QUESTION_RENDERER_STYLES}</style>
      <For each={props.blocks}>
        {(block) => <QuestionContentBlockRenderer block={block} assetUrl={props.assetUrl} />}
      </For>
    </>
  );
}

function QuestionContent(props: QuestionRendererProps): JSX.Element {
  return (
    <section class="question-renderer" aria-labelledby="question-prompt-heading">
      <div class="question-renderer__prompt">
        <h2 id="question-prompt-heading">Question</h2>
        <QuestionPromptRenderer blocks={props.presentation.prompt} assetUrl={props.assetUrl} />
      </div>
    </section>
  );
}

/** Maps identity-free question content to semantic prompt content; grading data never enters this boundary. */
export function QuestionRenderer(props: QuestionRendererProps): JSX.Element {
  return (
    <ErrorBoundary
      fallback={(error, reset) => {
        const message =
          error instanceof QuestionContentError
            ? error.message
            : "This question could not be shown. It may need an authoring or accessibility correction; your Assignment Attempt and timer are still available.";
        return <RendererFailure reset={reset} onRetry={props.onRetry} message={message} />;
      }}
    >
      <QuestionContent {...props} />
    </ErrorBoundary>
  );
}

/** Renders the prompt from one issued Student Question Presentation without projecting its format. */
export function QuestionPresentationRenderer(props: {
  readonly presentation: QuestionPresentation;
  readonly assetUrl: AssetUrlResolver;
  readonly onRetry?: () => void;
}): JSX.Element {
  return <QuestionPromptRenderer blocks={props.presentation.prompt} assetUrl={props.assetUrl} />;
}
