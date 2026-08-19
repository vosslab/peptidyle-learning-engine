// retry_corpus.spec.ts - behavior checks for private retry-corpus arrangement.

import { expect, test } from "@playwright/test";

import {
  arrangeRetryCorpusWithRequestFactory,
  retryCorpusCatalogSearchTitle,
  type RetryCorpusApiRequest,
} from "./retry_corpus";

const INSTRUCTOR_CREDENTIAL = "instructor-secret-must-not-appear-in-an-error";
const PRIVATE_CORRECT_CHOICE = "nitrogen";
const PUBLISHED_SUMMARY = {
  questionId: "7K3-M9QP",
  backend: "native",
  capabilities: [],
  metadata: {
    title: "Fake amino acid question",
    tags: [],
    taxonomy: [],
    license: { kind: "cc0" },
    language: "en-US",
  },
  byline: { names: ["PLE retry corpus fixture"] },
  scope: "institution",
  lifecycle: { state: "published" },
  publishedAt: 1_786_000_000_000,
};
const PUBLIC_DETAIL = {
  summary: PUBLISHED_SUMMARY,
  prompt: [{ kind: "text", markdown: "Which labeled atom is part of the peptide bond?" }],
  statistics: "unavailable",
};

interface CapturedRequest {
  readonly method: "get" | "post" | "put";
  readonly path: string;
  readonly headers?: Readonly<Record<string, string>>;
  readonly data?: unknown;
}

interface FakeResponse {
  readonly statusCode: number;
  readonly responseHeaders?: Readonly<Record<string, string>>;
  readonly payload?: unknown;
}

type FakeOutcome = FakeResponse | Error;

interface FakeApiResponse {
  status(): number;
  headers(): Readonly<Record<string, string>>;
  json(): Promise<unknown>;
}

interface FakeTransport {
  post(
    path: string,
    request: {
      readonly data:
        | { readonly credential: string }
        | {
            readonly scope: "institution";
            readonly byline: { readonly names: readonly string[] };
          };
      readonly headers?: Readonly<Record<string, string>>;
    },
  ): Promise<FakeApiResponse>;
  put(
    path: string,
    request: {
      readonly data: unknown;
      readonly headers: Readonly<Record<string, string>>;
    },
  ): Promise<FakeApiResponse>;
  get(path: string): Promise<FakeApiResponse>;
  dispose(): Promise<void>;
}

function response({
  statusCode,
  responseHeaders = {},
  payload = {},
}: FakeResponse): FakeApiResponse {
  return {
    status: (): number => statusCode,
    headers: (): Readonly<Record<string, string>> => responseHeaders,
    json: (): Promise<unknown> => Promise.resolve(payload),
  };
}

function fakeRequest(
  replies: readonly FakeOutcome[],
  disposeError?: Error,
): {
  readonly request: RetryCorpusApiRequest;
  readonly captured: CapturedRequest[];
  readonly state: { disposed: boolean };
} {
  const captured: CapturedRequest[] = [];
  const state = { disposed: false };
  let replyIndex = 0;
  function nextReply(): FakeApiResponse {
    const next = replies[replyIndex];
    replyIndex += 1;
    if (next === undefined) throw new Error("test transport received an unexpected request");
    if (next instanceof Error) throw next;
    return response(next);
  }
  const context: FakeTransport = {
    post: (path, options): Promise<FakeApiResponse> => {
      captured.push({ method: "post", path, ...options });
      return Promise.resolve(nextReply());
    },
    put: (path, options): Promise<FakeApiResponse> => {
      captured.push({ method: "put", path, ...options });
      return Promise.resolve(nextReply());
    },
    get: (path): Promise<FakeApiResponse> => {
      captured.push({ method: "get", path });
      return Promise.resolve(nextReply());
    },
    dispose: (): Promise<void> => {
      state.disposed = true;
      if (disposeError !== undefined) return Promise.reject(disposeError);
      return Promise.resolve();
    },
  };
  return {
    request: {
      newContext: (): Promise<FakeTransport> => Promise.resolve(context),
    },
    captured,
    state,
  };
}

function successfulReplies(): FakeResponse[] {
  return [
    { statusCode: 200 },
    { statusCode: 200, responseHeaders: { etag: '"1"' } },
    {
      statusCode: 201,
      payload: PUBLISHED_SUMMARY,
    },
    { statusCode: 200, payload: PUBLIC_DETAIL },
  ];
}

async function captureFailure(operation: Promise<unknown>): Promise<unknown> {
  try {
    await operation;
  } catch (error) {
    return error;
  }
  return new Error("test expected a rejected arrangement");
}

test("authors, publishes, and inspects an answer-free retry corpus through supported requests", async () => {
  const fake = fakeRequest(successfulReplies());
  const published = await arrangeRetryCorpusWithRequestFactory(fake.request, {
    baseUrl: "http://127.0.0.1:3000",
    instructorCredential: INSTRUCTOR_CREDENTIAL,
    masterSeed: 42,
  });
  expect(published).toEqual({
    questionId: PUBLISHED_SUMMARY.questionId,
    catalogSearchTitle: expect.stringMatching(/^Fake amino acid question [0-9a-f]{12}$/u),
    arrangement: "native-retry-corpus",
  });
  expect(published.catalogSearchTitle).toMatch(/^Fake amino acid question [0-9a-f]{12}$/u);
  expect(published).not.toHaveProperty("correctChoice");
  expect(published).not.toHaveProperty("problem");
  expect(published).not.toHaveProperty("version");
  const replay = fakeRequest(successfulReplies());
  await expect(
    arrangeRetryCorpusWithRequestFactory(replay.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).resolves.toMatchObject({
    questionId: PUBLISHED_SUMMARY.questionId,
    arrangement: "native-retry-corpus",
  });
  expect(replay.captured[1]?.data).toMatchObject({
    title: expect.stringMatching(/^Fake amino acid question [0-9a-f]{12}$/u),
  });
  expect(fake.captured.map(({ method, path }) => ({ method, path }))).toEqual([
    { method: "post", path: "/api/auth/login" },
    {
      method: "put",
      path: expect.stringMatching(/^\/api\/workspaces\/[0-9a-f-]+\/flat-question$/u),
    },
    {
      method: "post",
      path: expect.stringMatching(/^\/api\/problems\/[0-9a-f-]+\/flat-question-publish$/u),
    },
    {
      method: "get",
      path: `/api/problems/by-id/${PUBLISHED_SUMMARY.questionId}/detail`,
    },
  ]);
  expect(fake.captured[1]?.headers).toEqual({
    "content-type": "application/vnd.peptidyle.flat-question+json",
  });
  expect(fake.captured[2]?.headers).toEqual({ "if-match": '"1"' });
  expect(fake.captured[2]?.data).toEqual({
    scope: "institution",
    byline: { names: ["PLE retry corpus fixture"] },
  });
  expect(fake.captured[1]?.data).toMatchObject({
    attemptPolicy: { maxAttempts: null },
    timingPolicy: { kind: "untimed" },
  });
  expect(fake.state.disposed).toBe(true);
});

test("derives a compact, unmistakably fake public catalog title", () => {
  const title = retryCorpusCatalogSearchTitle("123e4567-e89b-12d3-a456-426614174000");
  expect(title).toBe("Fake amino acid question 123e4567e89b");
  expect(title).toMatch(/^[A-Za-z0-9 ]+$/u);
  expect(title).not.toContain("-");
  expect(() => retryCorpusCatalogSearchTitle("not-a-uuid")).toThrow("workspace");
});

test("rejects a weak save revision before publication and redacts the credential", async () => {
  const fake = fakeRequest([
    { statusCode: 200 },
    { statusCode: 200, responseHeaders: { etag: 'W/"1"' } },
  ]);
  await expect(
    arrangeRetryCorpusWithRequestFactory(fake.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).rejects.toMatchObject({ name: "RetryCorpusArrangementError", stage: "save" });
  await expect(
    arrangeRetryCorpusWithRequestFactory(fakeRequest([{ statusCode: 401 }]).request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).rejects.not.toThrow(INSTRUCTOR_CREDENTIAL);
  expect(fake.captured).toHaveLength(2);
  expect(fake.state.disposed).toBe(true);
});

test("requires an exact positive decimal i64 ETag", async () => {
  for (const etag of ['"-1"', '"0"', '"9223372036854775808"', '"revision-1"']) {
    const fake = fakeRequest([{ statusCode: 200 }, { statusCode: 200, responseHeaders: { etag } }]);
    await expect(
      arrangeRetryCorpusWithRequestFactory(fake.request, {
        baseUrl: "http://127.0.0.1:3000",
        instructorCredential: INSTRUCTOR_CREDENTIAL,
        masterSeed: 42,
      }),
    ).rejects.toMatchObject({ name: "RetryCorpusArrangementError", stage: "save" });
    expect(fake.captured).toHaveLength(2);
  }
});

test("does not retry a failed publication", async () => {
  const fake = fakeRequest([
    { statusCode: 200 },
    { statusCode: 200, responseHeaders: { etag: '"1"' } },
    new Error(PRIVATE_CORRECT_CHOICE),
  ]);
  await expect(
    arrangeRetryCorpusWithRequestFactory(fake.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).rejects.toMatchObject({ name: "RetryCorpusArrangementError", stage: "publish" });
  expect(fake.captured.map(({ method }) => method)).toEqual(["post", "put", "post"]);
  expect(fake.state.disposed).toBe(true);
});

test("redacts transport and disposal failures", async () => {
  const loginFailure = fakeRequest([new Error(INSTRUCTOR_CREDENTIAL)]);
  const loginError = await captureFailure(
    arrangeRetryCorpusWithRequestFactory(loginFailure.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  );
  expect(loginError).toMatchObject({ name: "RetryCorpusArrangementError", stage: "login" });
  expect(String(loginError)).not.toContain(INSTRUCTOR_CREDENTIAL);
  const disposeFailure = fakeRequest(successfulReplies(), new Error(PRIVATE_CORRECT_CHOICE));
  const disposeError = await captureFailure(
    arrangeRetryCorpusWithRequestFactory(disposeFailure.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  );
  expect(disposeError).toMatchObject({ name: "RetryCorpusArrangementError", stage: "dispose" });
  expect(String(disposeError)).not.toContain(PRIVATE_CORRECT_CHOICE);
  expect(disposeFailure.state.disposed).toBe(true);
});

test("refuses an answer-bearing public detail projection", async () => {
  const fake = fakeRequest([
    { statusCode: 200 },
    { statusCode: 200, responseHeaders: { etag: '"1"' } },
    { statusCode: 201, payload: PUBLISHED_SUMMARY },
    {
      statusCode: 200,
      payload: { nested: { CorrectResponses: { Rubric: { AnswerKeys: PRIVATE_CORRECT_CHOICE } } } },
    },
  ]);
  await expect(
    arrangeRetryCorpusWithRequestFactory(fake.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).rejects.toMatchObject({ name: "RetryCorpusArrangementError", stage: "public-inspection" });
  expect(fake.state.disposed).toBe(true);
});

test("rejects a public detail for a different Question ID", async () => {
  const fake = fakeRequest([
    { statusCode: 200 },
    { statusCode: 200, responseHeaders: { etag: '"1"' } },
    { statusCode: 201, payload: PUBLISHED_SUMMARY },
    {
      statusCode: 200,
      payload: {
        ...PUBLIC_DETAIL,
        summary: { ...PUBLISHED_SUMMARY, questionId: "3KM-9QPT" },
      },
    },
  ]);
  await expect(
    arrangeRetryCorpusWithRequestFactory(fake.request, {
      baseUrl: "http://127.0.0.1:3000",
      instructorCredential: INSTRUCTOR_CREDENTIAL,
      masterSeed: 42,
    }),
  ).rejects.toMatchObject({ name: "RetryCorpusArrangementError", stage: "public-inspection" });
  expect(fake.state.disposed).toBe(true);
});
