// question_renderer.tsx - browser-safe envelope-to-prompt projection.

import { createUniqueId, ErrorBoundary, For, onMount, Show, type JSX } from "solid-js";
import temml from "temml";

import type { AssetRef } from "../../generated/api/AssetRef";
import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";

import { QUESTION_RENDERER_STYLES } from "./question_renderer_styles";

/**
 * The markup subset accepted from recorded WeBWorK and QTI prompt content.
 *
 * It deliberately excludes active, embedding, form, media, metadata, SVG, and styling elements.
 * `data-asset-id` is the sole URL-like input; the renderer derives its URL from the authorized
 * logical asset resolver. Raw `src`, `href`, `style`, event, and URL-bearing attributes are never
 * part of this grammar.
 */
const ALLOWED_TAGS = new Set([
  "a",
  "b",
  "blockquote",
  "br",
  "caption",
  "code",
  "dd",
  "div",
  "dl",
  "dt",
  "em",
  "figcaption",
  "figure",
  "h3",
  "h4",
  "h5",
  "h6",
  "i",
  "img",
  "li",
  "ol",
  "p",
  "pre",
  "span",
  "strong",
  "sub",
  "sup",
  "table",
  "tbody",
  "td",
  "th",
  "thead",
  "tr",
  "u",
  "ul",
]);
const VOID_TAGS = new Set(["br", "img"]);
const GLOBAL_ATTRIBUTES = new Set(["aria-label", "aria-describedby", "role", "title"]);
const TAG_ATTRIBUTES: Readonly<Record<string, ReadonlySet<string>>> = {
  a: new Set(["data-asset-id"]),
  img: new Set(["alt", "data-asset-id", "height", "width"]),
  ol: new Set(["start", "type"]),
  td: new Set(["colspan", "rowspan"]),
  th: new Set(["colspan", "rowspan", "scope"]),
};

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

type SafeMarkupNode = SafeTextNode | SafeElementNode;
type SafeTextNode = { readonly kind: "text"; readonly text: string };
type SafeElementNode = {
  readonly kind: "element";
  readonly tag: string;
  readonly attributes: ReadonlyMap<string, string>;
  readonly children: ReadonlyArray<SafeMarkupNode>;
};

/** A parsed, allowlisted tree; no arbitrary HTML string can reach the DOM sink. */
export type SanitizedMarkup = { readonly tree: ReadonlyArray<SafeMarkupNode> };

type SafeMathMlNode = SafeMathMlTextNode | SafeMathMlElementNode;
type SafeMathMlTextNode = { readonly kind: "text"; readonly text: string };
type SafeMathMlElementNode = {
  readonly kind: "element";
  readonly tag: string;
  readonly attributes: ReadonlyMap<string, string>;
  readonly children: ReadonlyArray<SafeMathMlNode>;
};

type SanitizedMathMl = { readonly root: SafeMathMlElementNode };

/** A server-produced markup replacement for one otherwise structured prompt block. */
export interface SanitizedMarkupProjection {
  readonly promptIndex: number;
  readonly markup: SanitizedMarkup;
  /** Logical IDs only. Object keys and bucket locations never reach this component. */
  readonly assets: ReadonlyMap<string, AssetRef>;
}

/** A narrow callback that must derive the documented application asset route from an AssetRef. */
export type AssetUrlResolver = (asset: AssetRef) => URL;

/**
 * The identity-free content the browser renderer actually needs. Published envelopes and private
 * workspace previews intentionally share this projection without sharing a publication identity.
 */
export interface QuestionPresentation {
  readonly prompt: ReadonlyArray<ContentBlock>;
  readonly response: ResponseDefinition;
}

export interface QuestionRendererProps {
  readonly presentation: QuestionPresentation;
  readonly assetUrl: AssetUrlResolver;
  readonly suppliedMarkup?: ReadonlyArray<SanitizedMarkupProjection>;
  /** Re-fetches only this question resource; the surrounding run shell remains intact. */
  readonly onRetry?: () => void;
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
  kind: ContentBlock["kind"],
): string {
  if (!hasRequiredDescription(description)) {
    throw new QuestionContentError(
      `This ${kind} block cannot be shown because its required accessibility description is missing.`,
    );
  }
  return description;
}

function allowedAttribute(tag: string, name: string): boolean {
  return GLOBAL_ATTRIBUTES.has(name) || TAG_ATTRIBUTES[tag]?.has(name) === true;
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

function attributesFromElement(element: Element): ReadonlyMap<string, string> {
  const attributes = new Map<string, string>();
  for (const attribute of Array.from(element.attributes)) {
    const name = attribute.name.toLowerCase();
    rejectUnsafeAttribute(name, attribute.value);
    if (!allowedAttribute(element.localName, name)) {
      throw new QuestionContentError(`Supplied markup attribute ${name} is not allowlisted.`);
    }
    attributes.set(name, attribute.value);
  }
  return attributes;
}

function projectDomNode(node: Node): SafeMarkupNode {
  if (node.nodeType === Node.TEXT_NODE) {
    return { kind: "text", text: node.textContent ?? "" };
  }
  if (node.nodeType !== Node.ELEMENT_NODE) {
    throw new QuestionContentError("Supplied markup contains a non-structural node.");
  }
  const element = node as Element;
  const tag = element.localName.toLowerCase();
  if (!ALLOWED_TAGS.has(tag)) {
    throw new QuestionContentError(`Supplied markup tag ${tag} is not allowlisted.`);
  }
  return {
    kind: "element",
    tag,
    attributes: attributesFromElement(element),
    children: Array.from(element.childNodes, projectDomNode),
  };
}

/**
 * Strict parser used only in non-browser behavior tests. Browser rendering always also parses in
 * an inert DOMParser document before projecting a new, validated tree.
 */
function parseWithoutDom(markup: string): ReadonlyArray<SafeMarkupNode> {
  type MutableNode = { kind: "text"; text: string } | MutableElement;
  type MutableElement = {
    kind: "element";
    tag: string;
    attributes: Map<string, string>;
    children: MutableNode[];
  };
  const root: { tag: "#root"; children: MutableNode[] } = { tag: "#root", children: [] };
  const stack: Array<{ tag: string; children: MutableNode[] }> = [root];
  const current = (): { tag: string; children: MutableNode[] } => {
    const entry = stack[stack.length - 1];
    if (entry === undefined) throw new QuestionContentError("Supplied markup is malformed.");
    return entry;
  };
  const tokenPattern = /<[^>]*>/gu;
  let cursor = 0;
  for (const match of markup.matchAll(tokenPattern)) {
    const token = match[0] ?? "";
    const before = markup.slice(cursor, match.index);
    if (before.length > 0) current().children.push({ kind: "text", text: before });
    cursor = (match.index ?? 0) + token.length;
    if (/^<\s*\//u.test(token)) {
      const closing = /^<\s*\/\s*([a-z0-9]+)\s*>$/iu.exec(token)?.[1]?.toLowerCase();
      if (closing === undefined || closing !== current().tag) {
        throw new QuestionContentError("Supplied markup is malformed.");
      }
      stack.pop();
      continue;
    }
    const opening = /^<\s*([a-z0-9]+)((?:\s+[a-zA-Z][\w:-]*(?:\s*=\s*"[^"]*")?)*)\s*(\/?)>$/u.exec(
      token,
    );
    const tag = opening?.[1]?.toLowerCase();
    if (opening === null || tag === undefined || !ALLOWED_TAGS.has(tag)) {
      throw new QuestionContentError("Supplied markup contains a malformed or unallowlisted tag.");
    }
    const attributes = new Map<string, string>();
    const attributePattern = /\s+([a-zA-Z][\w:-]*)(?:\s*=\s*"([^"]*)")?/gu;
    const attributeSource = opening[2] ?? "";
    for (const attribute of attributeSource.matchAll(attributePattern)) {
      const attributeName = attribute[1];
      if (attributeName === undefined)
        throw new QuestionContentError("Supplied markup is malformed.");
      const name = attributeName.toLowerCase();
      const value = attribute[2] ?? "";
      rejectUnsafeAttribute(name, value);
      if (!allowedAttribute(tag, name)) {
        throw new QuestionContentError(`Supplied markup attribute ${name} is not allowlisted.`);
      }
      attributes.set(name, value);
    }
    const element: MutableElement = { kind: "element", tag, attributes, children: [] };
    current().children.push(element);
    if (!VOID_TAGS.has(tag) && opening[3] !== "/") stack.push(element);
  }
  const trailing = markup.slice(cursor);
  if (trailing.length > 0) {
    current().children.push({ kind: "text", text: trailing });
  }
  if (stack.length !== 1) throw new QuestionContentError("Supplied markup is malformed.");
  return root.children;
}

/**
 * Defensive browser boundary for server-projected markup. It parses inertly, projects only the
 * small schema above, and retains no raw HTML string for a later DOM markup sink.
 */
export function projectServerSanitizedMarkup(markup: string): SanitizedMarkup {
  const syntaxTree = parseWithoutDom(markup);
  if (typeof DOMParser === "undefined") return { tree: syntaxTree };
  const parsed = new DOMParser().parseFromString(markup, "text/html");
  return { tree: Array.from(parsed.body.childNodes, projectDomNode) };
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

function projectMathMlNode(node: Node): SafeMathMlNode {
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
    children: Array.from(element.childNodes, projectMathMlNode),
  };
}

/**
 * Converts TeX only through Temml, then projects the result through a fresh MathML allowlist.
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
  const root = projectMathMlNode(parsed.documentElement);
  if (root.kind !== "element" || root.tag !== "math") {
    throw new QuestionContentError("Converted TeX did not produce a MathML root element.");
  }
  return { root };
}

/** Refuse every route except the authorized, logical application asset endpoint. */
export function resolveSameOriginAssetUrl(asset: AssetRef, resolver: AssetUrlResolver): string {
  const url = resolver(asset);
  const expectedPath = `/api/assets/${encodeURIComponent(asset.asset)}`;
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

function authorizeProtectedAssetLink(event: MouseEvent): void {
  const link = event.currentTarget;
  if (!(link instanceof HTMLAnchorElement)) return;
  let logical: URL;
  try {
    logical = new URL(link.href, globalThis.location.origin);
  } catch (_error: unknown) {
    return;
  }
  if (
    logical.origin !== globalThis.location.origin ||
    !/^\/api\/assets\/[^/]+$/u.test(logical.pathname)
  )
    return;
  event.preventDefault();
  const delivery = new URL(`${logical.pathname}/delivery`, logical.origin);
  void globalThis
    .fetch(delivery, {
      method: "POST",
      headers: { accept: "application/json" },
      credentials: "same-origin",
      cache: "no-store",
    })
    .then(async (response) => {
      if (response.status === 405) {
        globalThis.location.assign(logical.href);
        return;
      }
      if (!response.ok) return;
      const body: unknown = await response.json();
      if (typeof body !== "object" || body === null || Array.isArray(body)) return;
      const value = (body as Record<string, unknown>).url;
      if (typeof value !== "string") return;
      const signed = new URL(value);
      if (
        (signed.protocol === "https:" || signed.protocol === "http:") &&
        signed.username === "" &&
        signed.password === ""
      )
        globalThis.location.assign(signed.href);
    })
    .catch(() => undefined);
}

function appendSafeNodes(
  document: Document,
  parent: Node,
  nodes: ReadonlyArray<SafeMarkupNode>,
  assets: ReadonlyMap<string, AssetRef>,
  resolver: AssetUrlResolver,
): void {
  for (const node of nodes) {
    if (node.kind === "text") {
      parent.appendChild(document.createTextNode(node.text));
      continue;
    }
    const element = document.createElement(node.tag);
    for (const [name, value] of node.attributes) {
      if (name === "data-asset-id") {
        const asset = assets.get(value);
        if (asset === undefined) {
          throw new QuestionContentError(
            "Supplied markup refers to an asset absent from its server-provided logical asset map.",
          );
        }
        element.setAttribute("data-asset-id", value);
        const url = resolveSameOriginAssetUrl(asset, resolver);
        element.setAttribute(node.tag === "a" ? "href" : "src", url);
        if (element instanceof HTMLImageElement)
          element.addEventListener("error", recoverProtectedAssetImage);
        if (element instanceof HTMLAnchorElement)
          element.addEventListener("click", authorizeProtectedAssetLink);
      } else {
        element.setAttribute(name, value);
      }
    }
    parent.appendChild(element);
    appendSafeNodes(document, element, node.children, assets, resolver);
  }
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

function ValidatedMarkup(props: {
  readonly markup: SanitizedMarkup;
  readonly assets: ReadonlyMap<string, AssetRef>;
  readonly assetUrl: AssetUrlResolver;
}): JSX.Element {
  const hostId = createUniqueId();
  onMount(() => {
    const host = document.getElementById(hostId);
    if (!(host instanceof HTMLDivElement)) {
      throw new QuestionContentError("Validated markup host was not available.");
    }
    appendSafeNodes(document, host, props.markup.tree, props.assets, props.assetUrl);
  });
  return <div id={hostId} class="question-renderer__supplied-markup" />;
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

function projectionFor(
  index: number,
  projections: ReadonlyArray<SanitizedMarkupProjection>,
): SanitizedMarkupProjection | undefined {
  return projections.find((projection) => projection.promptIndex === index);
}

function StructuredBlock(props: {
  readonly block: ContentBlock;
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
            src={resolveSameOriginAssetUrl(props.block.asset, props.assetUrl)}
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

function PromptProjection(props: {
  readonly index: number;
  readonly block: ContentBlock;
  readonly assetUrl: AssetUrlResolver;
  readonly suppliedMarkup: ReadonlyArray<SanitizedMarkupProjection>;
}): JSX.Element {
  const projection = projectionFor(props.index, props.suppliedMarkup);
  return (
    <Show
      when={projection}
      fallback={<StructuredBlock block={props.block} assetUrl={props.assetUrl} />}
    >
      {(provided) => (
        <ValidatedMarkup
          markup={provided().markup}
          assets={provided().assets}
          assetUrl={props.assetUrl}
        />
      )}
    </Show>
  );
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

function QuestionContent(props: QuestionRendererProps): JSX.Element {
  const suppliedMarkup = props.suppliedMarkup ?? [];
  return (
    <section class="question-renderer" aria-labelledby="question-prompt-heading">
      <style>{QUESTION_RENDERER_STYLES}</style>
      <div class="question-renderer__prompt">
        <h2 id="question-prompt-heading">Question</h2>
        <For each={props.presentation.prompt}>
          {(block, index) => (
            <PromptProjection
              index={index()}
              block={block}
              assetUrl={props.assetUrl}
              suppliedMarkup={suppliedMarkup}
            />
          )}
        </For>
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
            : "This question could not be shown. It may need an authoring or accessibility correction; your run and timer are still available.";
        return <RendererFailure reset={reset} onRetry={props.onRetry} message={message} />;
      }}
    >
      <QuestionContent {...props} />
    </ErrorBoundary>
  );
}
