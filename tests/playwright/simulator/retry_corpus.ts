// retry_corpus.ts - private supported-API arrangement of one retry-capable native question.

import type { APIRequest } from "@playwright/test";

import { choose_value, create_named_stream } from "./rng";

const FLAT_QUESTION_MEDIA_TYPE = "application/vnd.peptidyle.flat-question+json";
const RETRY_CORPUS_STREAM = "arrangement.retry-corpus.content";
const PUBLIC_FORBIDDEN_FIELDS = new Set([
  "answerkey",
  "answerkeys",
  "answers",
  "correctchoice",
  "correctanswer",
  "correctresponse",
  "correctresponses",
  "checkerstate",
  "grading",
  "gradingpayload",
  "expectedvalue",
  "private",
  "privategrading",
  "response",
  "rubric",
  "source",
  "provider",
]);
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

interface RetryCorpusVariant {
  readonly title: string;
  readonly prompt: string;
  readonly choices: readonly RetryCorpusChoice[];
  readonly correctChoice: string;
}

interface RetryCorpusChoice {
  readonly id: string;
  readonly text: string;
}

interface FlatQuestionSource {
  readonly format: "pleFlatQuestion";
  readonly version: 1;
  readonly kind: "singleChoice";
  readonly title: string;
  readonly prompt: string;
  readonly choices: readonly RetryCorpusChoice[];
  readonly correctChoice: string;
  readonly feedback: Readonly<Record<string, never>>;
  readonly points: 1;
  readonly attemptPolicy: {
    readonly maxAttempts: null;
    readonly feedback: "immediateFull";
  };
  readonly timingPolicy:
    | { readonly kind: "untimed" }
    | { readonly kind: "perQuestion"; readonly seconds: 900; readonly graceSeconds: 30 };
  readonly tags: readonly [];
  readonly taxonomy: readonly [];
  readonly license: {
    readonly kind: "cc0";
  };
  readonly language: "en-US";
}

interface RequestResponse {
  status(): number;
  headers(): Readonly<Record<string, string>>;
  json(): Promise<unknown>;
}

interface RetryCorpusTransport {
  post(
    path: string,
    request: {
      readonly data: { readonly credential: string } | { readonly scope: "institution" };
      readonly headers?: Readonly<Record<string, string>>;
    },
  ): Promise<RequestResponse>;
  put(
    path: string,
    request: {
      readonly data: FlatQuestionSource;
      readonly headers: Readonly<Record<string, string>>;
    },
  ): Promise<RequestResponse>;
  get(path: string): Promise<RequestResponse>;
  dispose(): Promise<void>;
}

export interface RetryCorpusApiRequest {
  newContext(options: { readonly baseURL: string }): Promise<RetryCorpusTransport>;
}

export interface RetryCorpusInputs {
  readonly baseUrl: string;
  readonly instructorCredential: string;
  readonly masterSeed: number;
  readonly timedQuestion?: boolean;
}

export interface PublishedRetryCorpus {
  readonly problem: string;
  readonly version: string;
  /** Public title used only to locate this fresh corpus entry in the visible catalog. */
  readonly catalogSearchTitle: string;
  readonly arrangement: "native-retry-corpus";
}

export type RetryCorpusStage = "login" | "save" | "publish" | "public-inspection" | "dispose";

/** A redacted failure that identifies only the failed supported-API stage. */
export class RetryCorpusArrangementError extends Error {
  public readonly stage: RetryCorpusStage;

  public constructor(stage: RetryCorpusStage) {
    super(`retry corpus arrangement failed during ${stage}`);
    this.name = "RetryCorpusArrangementError";
    this.stage = stage;
  }
}

const RETRY_CORPUS_VARIANTS: readonly RetryCorpusVariant[] = [
  {
    title: "Peptide bond orientation",
    prompt: "Which labeled atom is part of the peptide bond?",
    choices: [
      { id: "oxygen", text: "The side-chain oxygen" },
      { id: "nitrogen", text: "The amide nitrogen" },
    ],
    correctChoice: "nitrogen",
  },
  {
    title: "Amino-acid backbone",
    prompt: "Which labeled group belongs to every standard amino-acid backbone?",
    choices: [
      { id: "phosphate", text: "A phosphate group" },
      { id: "amino", text: "The amino group" },
    ],
    correctChoice: "amino",
  },
];

/**
 * Creates one isolated Playwright API context, authenticates only inside it,
 * and returns no private source, credential, cookie, or answer-bearing data.
 */
export async function arrangeRetryCorpus(
  request: APIRequest,
  inputs: RetryCorpusInputs,
): Promise<PublishedRetryCorpus> {
  return arrangeRetryCorpusWithRequestFactory(request, inputs);
}

/** Test seam for the same isolated-context contract used by `APIRequest`. */
export async function arrangeRetryCorpusWithRequestFactory(
  request: RetryCorpusApiRequest,
  inputs: RetryCorpusInputs,
): Promise<PublishedRetryCorpus> {
  let context: RetryCorpusTransport;
  try {
    context = await request.newContext({ baseURL: inputs.baseUrl });
  } catch {
    throw new RetryCorpusArrangementError("login");
  }
  let stage: RetryCorpusStage = "login";
  let result: PublishedRetryCorpus | undefined;
  let failure: RetryCorpusArrangementError | undefined;
  try {
    await authenticateInstructor(context, inputs.instructorCredential);
    stage = "save";
    const workspace = globalThis.crypto.randomUUID();
    const catalogSearchTitle = retryCorpusCatalogSearchTitle(workspace);
    const source = retryCorpusSource(
      inputs.masterSeed,
      catalogSearchTitle,
      inputs.timedQuestion === true,
    );
    const etag = await savePrivateSource(context, workspace, source);
    stage = "publish";
    const published = await publishInstitutionally(context, workspace, etag);
    stage = "public-inspection";
    await inspectSafePublicProjection(context, published);
    result = { ...published, catalogSearchTitle, arrangement: "native-retry-corpus" };
  } catch (error) {
    failure =
      error instanceof RetryCorpusArrangementError ? error : new RetryCorpusArrangementError(stage);
  }
  try {
    await context.dispose();
  } catch {
    if (failure === undefined) failure = new RetryCorpusArrangementError("dispose");
  }
  if (failure !== undefined) throw failure;
  if (result === undefined) throw new RetryCorpusArrangementError("public-inspection");
  return result;
}

/**
 * Names the newly arranged public corpus without deriving anything from its answer-bearing source.
 * A bounded token from the random workspace UUID keeps retained-catalog collisions negligible for
 * the visible instructor search without turning the learner-facing title into an internal ID dump.
 * The explicit Fake label makes captured demo content impossible to mistake for a real course item.
 */
export function retryCorpusCatalogSearchTitle(workspace: string): string {
  if (!UUID.test(workspace)) throw new Error("retry corpus workspace must be a UUID");
  const workspaceToken = workspace.replace(/-/gu, "").toLowerCase();
  return `Fake amino acid question ${workspaceToken.slice(0, 12)}`;
}

function retryCorpusSource(
  masterSeed: number,
  title: string,
  timedQuestion: boolean,
): FlatQuestionSource {
  const stream = create_named_stream(masterSeed, RETRY_CORPUS_STREAM);
  const variant = choose_value(stream, RETRY_CORPUS_VARIANTS);
  return {
    format: "pleFlatQuestion",
    version: 1,
    kind: "singleChoice",
    title,
    prompt: variant.prompt,
    choices: variant.choices,
    correctChoice: variant.correctChoice,
    feedback: {},
    points: 1,
    attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
    timingPolicy: timedQuestion
      ? { kind: "perQuestion", seconds: 900, graceSeconds: 30 }
      : { kind: "untimed" },
    tags: [],
    taxonomy: [],
    license: { kind: "cc0" },
    language: "en-US",
  };
}

async function authenticateInstructor(
  context: RetryCorpusTransport,
  credential: string,
): Promise<void> {
  const response = await context.post("/api/auth/login", { data: { credential } });
  if (response.status() !== 200) {
    throw new RetryCorpusArrangementError("login");
  }
}

async function savePrivateSource(
  context: RetryCorpusTransport,
  workspace: string,
  source: FlatQuestionSource,
): Promise<string> {
  const response = await context.put(`/api/workspaces/${workspace}/flat-question`, {
    headers: { "content-type": FLAT_QUESTION_MEDIA_TYPE },
    data: source,
  });
  const etag = response.headers()["etag"];
  if (response.status() !== 200 || etag === undefined || !isStrongEtag(etag)) {
    throw new RetryCorpusArrangementError("save");
  }
  return etag;
}

async function publishInstitutionally(
  context: RetryCorpusTransport,
  workspace: string,
  etag: string,
): Promise<Pick<PublishedRetryCorpus, "problem" | "version">> {
  const response = await context.post(`/api/problems/${workspace}/flat-question-publish`, {
    headers: { "if-match": etag },
    data: { scope: "institution" },
  });
  if (response.status() !== 201) {
    throw new RetryCorpusArrangementError("publish");
  }
  const payload = await response.json();
  if (!isPublishedReference(payload)) {
    throw new RetryCorpusArrangementError("publish");
  }
  return { problem: payload.problem, version: payload.version };
}

async function inspectSafePublicProjection(
  context: RetryCorpusTransport,
  published: Pick<PublishedRetryCorpus, "problem" | "version">,
): Promise<void> {
  const response = await context.get(
    `/api/problems/${published.problem}/versions/${published.version}/detail`,
  );
  if (response.status() !== 200 || containsForbiddenPublicField(await response.json())) {
    throw new RetryCorpusArrangementError("public-inspection");
  }
}

function isStrongEtag(value: string): boolean {
  const match = /^"([1-9][0-9]*)"$/u.exec(value);
  const revision = match?.[1];
  return revision !== undefined && BigInt(revision) <= 9_223_372_036_854_775_807n;
}

function isPublishedReference(
  value: unknown,
): value is Pick<PublishedRetryCorpus, "problem" | "version"> {
  if (!isRecord(value)) return false;
  return (
    typeof value["problem"] === "string" &&
    UUID.test(value["problem"]) &&
    typeof value["version"] === "string" &&
    UUID.test(value["version"])
  );
}

function containsForbiddenPublicField(value: unknown): boolean {
  if (Array.isArray(value)) return value.some((item) => containsForbiddenPublicField(item));
  if (!isRecord(value)) return false;
  return Object.entries(value).some(
    ([key, child]) =>
      PUBLIC_FORBIDDEN_FIELDS.has(key.toLowerCase()) || containsForbiddenPublicField(child),
  );
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
