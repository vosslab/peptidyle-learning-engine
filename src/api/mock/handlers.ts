// handlers.ts - dependency-free mock API backed by the typed WP-C7 corpus.

import {
  publishedProblemAssetBodies,
  publishedProblemFixture,
} from "../../../generated/fixtures/published_problem";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { DisclosedFeedback } from "../../../generated/api/DisclosedFeedback";
import type { CatalogProblemDetail } from "../../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../../generated/api/CatalogSearchPage";
import type { AssignmentSummary } from "../../../generated/api/AssignmentSummary";
import type { PrefetchedNextQuestion, RunSummaryResponse, SubmissionReceipt } from "../contracts";
import type { AssignmentEditorInput } from "../contracts";
import { DecodeError } from "../decoder";
import { decodeAssignmentEditorInput } from "../decoders";

/** Planned server route groups that UI work may use before routes exist. */
export const MOCK_ROUTE_GROUPS = ["auth", "catalog", "course", "run", "asset"] as const;

export type MockRouteGroup = (typeof MOCK_ROUTE_GROUPS)[number];

/** Fetch-compatible function implemented entirely in the browser or Node. */
export type MockFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

/** One route-group handler in the mock API. */
export interface MockApiHandler {
  readonly group: MockRouteGroup;
  readonly canHandle: (request: Request) => boolean;
  readonly respond: (request: Request) => Response | Promise<Response>;
}

const MOCK_ORIGIN = "https://mock.peptidyle.invalid";
const EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID = "0198e000-0000-7000-8000-000000000034";
const IMMEDIATE_CORRECTNESS_ATTEMPT_ID = "0198e000-0000-7000-8000-000000000030";
const IMMEDIATE_FULL_ATTEMPT_ID = "0198e000-0000-7000-8000-000000000031";
const DEFERRED_ATTEMPT_ID = "0198e000-0000-7000-8000-000000000032";
const ON_RELEASE_ATTEMPT_ID = "0198e000-0000-7000-8000-000000000033";
const externalToolTemplateAttempt = publishedProblemFixture.attempts[3];
if (externalToolTemplateAttempt === undefined) {
  throw new Error("Mock fixture must include an attempt for the external-tool scenario");
}

/** Dedicated typed fixture attempt for the server-brokered external-tool path. */
export const externalToolFixtureAttempt: QuestionAttempt = {
  ...externalToolTemplateAttempt,
  id: EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID,
};

/** Deterministic two-position learner fixture used by attempt-loop acceptance. */
export const prefetchFixtureAttempt: QuestionAttempt = {
  ...externalToolTemplateAttempt,
  id: "0198e000-0000-7000-8000-000000000736",
  response: null,
  result: null,
  timer: { ...externalToolTemplateAttempt.timer, submittedAt: null },
};
export const prefetchedFixtureAttempt: QuestionAttempt = {
  ...prefetchFixtureAttempt,
  id: "0198e000-0000-7000-8000-000000000737",
  assignmentPosition: 1,
  seed: prefetchFixtureAttempt.seed + 1,
};

export function mockPrefetchedNextQuestion(): PrefetchedNextQuestion {
  const envelope = issuedEnvelopeForAttempt(prefetchedFixtureAttempt);
  return {
    predecessor: prefetchFixtureAttempt.id,
    run: prefetchedFixtureAttempt.run,
    assignmentPosition: prefetchedFixtureAttempt.assignmentPosition,
    questionVersion: prefetchedFixtureAttempt.questionVersion,
    seed: prefetchedFixtureAttempt.seed,
    renderedQuestionSha256: "b".repeat(64),
    envelope,
  };
}

export function mockPrefetchSubmissionReceipt(): SubmissionReceipt {
  return {
    accepted: true,
    attempt: {
      ...prefetchFixtureAttempt,
      response: { kind: "multipleChoice", selected: ["carbonyl"] },
    },
    feedback: { correctness: true },
    nextIssued: {
      id: prefetchedFixtureAttempt.id,
      run: prefetchedFixtureAttempt.run,
      questionVersion: prefetchedFixtureAttempt.questionVersion,
      seed: prefetchedFixtureAttempt.seed,
      deadline: prefetchedFixtureAttempt.timer.deadline,
      assignmentPosition: prefetchedFixtureAttempt.assignmentPosition,
      renderedQuestionSha256: "b".repeat(64),
    },
  };
}

/**
 * Marker-only mock receipt for the one broker-capable fixture. It intentionally
 * records no provider grade or feedback disclosure.
 */
export function mockExternalToolSubmissionReceipt(): SubmissionReceipt {
  return {
    accepted: true,
    attempt: {
      ...externalToolFixtureAttempt,
      response: { kind: "externalTool" },
      result: null,
    },
    feedback: null,
    nextIssued: null,
  };
}

export function mockAttemptById(attemptId: string): QuestionAttempt | undefined {
  return (
    publishedProblemFixture.attempts.find((candidate) => candidate.id === attemptId) ??
    (attemptId === externalToolFixtureAttempt.id ? externalToolFixtureAttempt : undefined) ??
    (attemptId === prefetchFixtureAttempt.id ? prefetchFixtureAttempt : undefined) ??
    (attemptId === prefetchedFixtureAttempt.id ? prefetchedFixtureAttempt : undefined)
  );
}

/**
 * The mock's one external-tool scenario models an issued response and an
 * explicit backend launch capability together. Other fixture attempts stay
 * unsupported, even though the mock has no real provider broker.
 */
function mockBackendPreparesExternalToolLaunch(attemptId: string): boolean {
  return attemptId === EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID;
}

function requestFrom(input: RequestInfo | URL, init?: RequestInit): Request {
  if (input instanceof Request) {
    return new Request(input, init);
  }
  const url = new URL(input.toString(), MOCK_ORIGIN);
  return new Request(url, init);
}

function pathSegments(request: Request): ReadonlyArray<string> {
  return new URL(request.url).pathname.split("/").filter(Boolean);
}

function routeResource(request: Request): string | undefined {
  const segments = pathSegments(request);
  if (segments[0] !== "api") {
    return undefined;
  }
  return segments[1];
}

function handlesResource(request: Request, resources: ReadonlyArray<string>): boolean {
  const resource = routeResource(request);
  return resource !== undefined && resources.includes(resource);
}

function jsonResponse(value: unknown, status = 200, headers: HeadersInit = {}): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...headers },
  });
}

function routeNotFound(request: Request): Response {
  const url = new URL(request.url);
  return jsonResponse({ error: `No mock route for ${request.method} ${url.pathname}` }, 404);
}

function methodNotAllowed(request: Request): Response {
  return jsonResponse(
    { error: `Method ${request.method} is not supported by this mock route` },
    405,
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyFields(record: Record<string, unknown>, fields: ReadonlyArray<string>): boolean {
  const keys = Object.keys(record);
  return keys.length === fields.length && keys.every((key) => fields.includes(key));
}

/** Mirrors the server's bounded visible-ASCII idempotency-key grammar. */
function validIdempotencyKey(value: string | null): boolean {
  return value !== null && value.length >= 1 && value.length <= 200 && !/[^\x21-\x7e]/.test(value);
}

/**
 * JSON.parse deliberately keeps the last duplicate object member, whereas the
 * server's serde boundary rejects duplicates. Scan the syntax first so the
 * browser mock cannot make a hostile payload appear less permissive than the
 * protected route it represents.
 */
function hasDuplicateJsonObjectMember(text: string): boolean {
  let index = 0;
  let duplicate = false;

  const whitespace = (): void => {
    while (/[ \t\n\r]/.test(text[index] ?? "")) index += 1;
  };
  const string = (): string => {
    const start = index;
    if (text[index] !== '"') throw new Error("expected JSON string");
    index += 1;
    while (index < text.length) {
      const character = text[index] ?? "";
      if (character === '"') {
        index += 1;
        return text.slice(start, index);
      }
      if (character < " ") throw new Error("unescaped control character");
      if (character === "\\") {
        const escaped = text[index + 1];
        if (escaped === "u") {
          if (!/^[0-9a-fA-F]{4}$/.test(text.slice(index + 2, index + 6))) {
            throw new Error("invalid unicode escape");
          }
          index += 6;
          continue;
        }
        if (escaped === undefined || !'"\\/bfnrt'.includes(escaped)) {
          throw new Error("invalid JSON escape");
        }
        index += 2;
        continue;
      }
      index += 1;
    }
    throw new Error("unterminated JSON string");
  };
  const value = (): void => {
    whitespace();
    const character = text[index];
    if (character === "{") {
      index += 1;
      whitespace();
      const keys = new Set<string>();
      if (text[index] === "}") {
        index += 1;
        return;
      }
      while (true) {
        whitespace();
        const rawKey = string();
        const key = JSON.parse(rawKey) as string;
        if (keys.has(key)) duplicate = true;
        keys.add(key);
        whitespace();
        if (text[index] !== ":") throw new Error("expected JSON object colon");
        index += 1;
        value();
        whitespace();
        if (text[index] === "}") {
          index += 1;
          return;
        }
        if (text[index] !== ",") throw new Error("expected JSON object separator");
        index += 1;
      }
    }
    if (character === "[") {
      index += 1;
      whitespace();
      if (text[index] === "]") {
        index += 1;
        return;
      }
      while (true) {
        value();
        whitespace();
        if (text[index] === "]") {
          index += 1;
          return;
        }
        if (text[index] !== ",") throw new Error("expected JSON array separator");
        index += 1;
      }
    }
    if (character === '"') {
      string();
      return;
    }
    if (
      text.startsWith("true", index) ||
      text.startsWith("false", index) ||
      text.startsWith("null", index)
    ) {
      index += text.startsWith("true", index) ? 4 : text.startsWith("false", index) ? 5 : 4;
      return;
    }
    const number = /-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/y;
    number.lastIndex = index;
    if (!number.exec(text)) throw new Error("expected JSON value");
    index = number.lastIndex;
  };

  value();
  whitespace();
  if (index !== text.length) throw new Error("unexpected JSON suffix");
  return duplicate;
}

async function validExternalToolSubmissionBody(request: Request): Promise<boolean> {
  let value: unknown;
  try {
    const text = await request.text();
    if (hasDuplicateJsonObjectMember(text)) return false;
    value = JSON.parse(text);
  } catch {
    return false;
  }
  if (!isRecord(value) || !hasOnlyFields(value, ["response"])) return false;
  const response = value["response"];
  return (
    isRecord(response) && hasOnlyFields(response, ["kind"]) && response["kind"] === "externalTool"
  );
}

async function responseForExternalToolSubmission(
  request: Request,
  attemptId: string,
): Promise<Response> {
  if (attemptId !== EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID) return routeNotFound(request);
  if (!validIdempotencyKey(request.headers.get("idempotency-key"))) {
    return jsonResponse({ error: "idempotency-key is required and must be valid" }, 400);
  }
  if (!(await validExternalToolSubmissionBody(request))) {
    return jsonResponse({ error: "external-tool submission must be the marker only" }, 400);
  }
  return jsonResponse(mockExternalToolSubmissionReceipt());
}

function canHandleAuth(request: Request): boolean {
  return handlesResource(request, ["auth"]);
}

function respondAuth(request: Request): Response {
  const segments = pathSegments(request);
  const action = segments[2];
  if (request.method === "POST" && action === "logout") {
    return jsonResponse({ authenticated: false });
  }
  if (
    (request.method === "POST" && action === "login") ||
    (request.method === "GET" && action === "session")
  ) {
    return jsonResponse({
      authenticated: true,
      tenant: publishedProblemFixture.enrollment.tenant,
      user: {
        id: publishedProblemFixture.enrollment.user,
        displayName: "Fixture Student",
        roles: ["student"],
      },
    });
  }
  return routeNotFound(request);
}

function canHandleCatalog(request: Request): boolean {
  return handlesResource(request, ["problems", "taxonomy"]);
}

function catalogDetailFixture(): CatalogProblemDetail {
  return {
    summary: publishedProblemFixture.catalogProblem,
    prompt: publishedProblemFixture.publishedProblem.prompt,
    statistics: "unavailable",
  };
}

function catalogSearchFixture(request: Request): CatalogSearchPage | Response {
  const parameters = new URL(request.url).searchParams;
  const allowed = new Set([
    "text",
    "taxonomy",
    "capabilities",
    "licenses",
    "statistics",
    "cursor",
    "pageSize",
  ]);
  for (const key of parameters.keys()) {
    if (!allowed.has(key)) {
      return jsonResponse({ error: `Unknown catalog search parameter ${key}` }, 400);
    }
  }
  const pageSize = parameters.get("pageSize");
  if (pageSize !== null && (!/^[1-9][0-9]*$/.test(pageSize) || Number(pageSize) > 100)) {
    return jsonResponse({ error: "Invalid catalog page size" }, 400);
  }
  const statistics = parameters.get("statistics") ?? "any";
  if (!["any", "available", "unavailable"].includes(statistics)) {
    return jsonResponse({ error: "Invalid catalog statistics filter" }, 400);
  }
  const summary = publishedProblemFixture.catalogProblem;
  const normalizedText = (parameters.get("text") ?? "").trim().toLowerCase();
  const textMatches =
    normalizedText.length === 0 ||
    summary.metadata.title.toLowerCase().includes(normalizedText) ||
    summary.metadata.tags.some((tag) => tag.toLowerCase().includes(normalizedText));
  const taxonomyMatches = parameters
    .getAll("taxonomy")
    .every((filter) =>
      summary.metadata.taxonomy.some((term) => `${term.scheme}:${term.code}` === filter),
    );
  const capabilitiesMatch = parameters
    .getAll("capabilities")
    .every((capability) => summary.capabilities.some((candidate) => candidate === capability));
  const licensesMatch = parameters
    .getAll("licenses")
    .some((license) => summary.metadata.license.kind === license);
  const statisticsMatch = statistics === "any" || statistics === "unavailable";
  const includesSummary =
    textMatches &&
    taxonomyMatches &&
    capabilitiesMatch &&
    (parameters.getAll("licenses").length === 0 || licensesMatch) &&
    statisticsMatch &&
    parameters.get("cursor") === null;
  return {
    items: includesSummary ? [summary] : [],
    nextCursor: null,
    facets: {
      taxonomy: summary.metadata.taxonomy.map((term) => ({ term, count: 1 })),
      capabilities: summary.capabilities.map((capability) => ({ capability, count: 1 })),
      licenses: [{ license: summary.metadata.license.kind, count: 1 }],
      statistics: { available: 0, unavailable: 1 },
    },
  };
}

function respondCatalog(request: Request): Response {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (request.method === "GET" && resource === "problems" && segments[2] === "search") {
    const page = catalogSearchFixture(request);
    return page instanceof Response ? page : jsonResponse(page);
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments.length === 6 &&
    segments[2] === publishedProblemFixture.publishedProblem.problem &&
    segments[3] === "versions" &&
    segments[4] === publishedProblemFixture.publishedProblem.version &&
    segments[5] === "detail"
  ) {
    return jsonResponse(catalogDetailFixture());
  }
  if (request.method === "GET" && resource === "problems" && segments.length === 2) {
    return jsonResponse({ items: [publishedProblemFixture.catalogProblem], nextCursor: null });
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments.length === 5 &&
    segments[2] === publishedProblemFixture.publishedProblem.problem &&
    segments[3] === "versions" &&
    segments[4] === publishedProblemFixture.publishedProblem.version
  ) {
    return jsonResponse(publishedProblemFixture.publishedProblem);
  }
  if (request.method === "POST" && resource === "problems" && segments[3] === "publish") {
    return jsonResponse(publishedProblemFixture.publishedProblem, 201);
  }
  if (request.method === "GET" && resource === "taxonomy") {
    return jsonResponse({
      items: publishedProblemFixture.publishedProblem.metadata.taxonomy,
      nextCursor: null,
    });
  }
  return routeNotFound(request);
}

function canHandleCourse(request: Request): boolean {
  return handlesResource(request, ["courses", "assignments"]);
}

const MOCK_UNSUPPORTED_ASSIGNMENT_VERSION = "0198e000-0000-7000-8000-000000000005";

interface MockAssignmentState {
  assignment: AssignmentSummary;
  revision: bigint;
  nextId: bigint;
}

function createMockAssignmentState(): MockAssignmentState {
  return {
    assignment: structuredClone(publishedProblemFixture.assignment),
    revision: 1n,
    nextId: 60n,
  };
}

function noStoreHeaders(revision?: bigint): HeadersInit {
  const headers: Record<string, string> = { "cache-control": "no-store" };
  if (revision !== undefined) headers["etag"] = `"${revision}"`;
  return headers;
}

function assignmentEditorResponse(
  assignment: AssignmentSummary,
  revision: bigint,
  status = 200,
): Response {
  return jsonResponse(assignment, status, noStoreHeaders(revision));
}

function assignmentError(status: number, error: string): Response {
  return jsonResponse({ error }, status, noStoreHeaders());
}

function mockAssignmentId(value: bigint): string {
  return `0198e000-0000-7000-8000-${value.toString().padStart(12, "0")}`;
}

async function assignmentInput(request: Request): Promise<AssignmentEditorInput | undefined> {
  try {
    const text = await request.text();
    if (hasDuplicateJsonObjectMember(text)) return undefined;
    return decodeAssignmentEditorInput(JSON.parse(text), "request");
  } catch (error: unknown) {
    if (error instanceof DecodeError || error instanceof SyntaxError) return undefined;
    throw error;
  }
}

function assignmentValidationFailure(input: AssignmentEditorInput): Response | undefined {
  const violations = input.problems
    .filter((reference) => reference.version === MOCK_UNSUPPORTED_ASSIGNMENT_VERSION)
    .flatMap((reference) => [
      {
        title: "Mock capability-limited published question",
        reference,
        capability: "serverGrading",
      },
      {
        title: "Mock capability-limited published question",
        reference,
        capability: "perQuestionTiming",
      },
    ]);
  if (violations.length === 0) return undefined;
  return jsonResponse(
    { error: "assignment configuration is not supported", violations },
    422,
    noStoreHeaders(),
  );
}

function assignmentRequestFailure(input: AssignmentEditorInput): Response | undefined {
  if (input.problems.length === 0) {
    return assignmentError(422, "assignment must reference at least one published problem version");
  }
  const references = new Set(
    input.problems.map((reference) => `${reference.problem}/${reference.version}`),
  );
  if (references.size !== input.problems.length) {
    return assignmentError(422, "assignment problem references must be unique");
  }
  return undefined;
}

function validAssignmentRevision(value: string | null): boolean {
  if (value === null || !/^"[1-9][0-9]*"$/u.test(value)) return false;
  return BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
}

async function respondCourse(
  request: Request,
  assignmentState = createMockAssignmentState(),
): Promise<Response> {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (resource === "courses" && segments.length === 2) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse({ items: [publishedProblemFixture.course], nextCursor: null });
  }
  if (
    resource === "courses" &&
    segments.length === 3 &&
    segments[2] === publishedProblemFixture.course.id
  ) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse(publishedProblemFixture.course);
  }
  if (
    resource === "courses" &&
    segments.length === 4 &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "gradebook"
  ) {
    if (request.method !== "GET") return methodNotAllowed(request);
    return jsonResponse({ items: publishedProblemFixture.gradebook, nextCursor: null });
  }
  if (
    resource === "courses" &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "assignments" &&
    segments.length === 4
  ) {
    if (request.method === "GET") {
      return jsonResponse({ items: [assignmentState.assignment], nextCursor: null });
    }
    if (request.method !== "POST") return methodNotAllowed(request);
    const input = await assignmentInput(request);
    if (input === undefined) return assignmentError(422, "assignment request is invalid");
    const requestFailure = assignmentRequestFailure(input);
    if (requestFailure !== undefined) return requestFailure;
    const validationFailure = assignmentValidationFailure(input);
    if (validationFailure !== undefined) return validationFailure;
    const id = mockAssignmentId(assignmentState.nextId);
    assignmentState.nextId += 1n;
    assignmentState.assignment = {
      id,
      tenant: publishedProblemFixture.assignment.tenant,
      courseId: publishedProblemFixture.course.id,
      title: input.title,
      problems: [...input.problems],
      policies: input.policies,
    };
    assignmentState.revision = 1n;
    return assignmentEditorResponse(assignmentState.assignment, assignmentState.revision, 201);
  }
  if (
    resource === "courses" &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "assignments" &&
    segments[4] === assignmentState.assignment.id &&
    segments.length === 5
  ) {
    if (request.method !== "PUT") return methodNotAllowed(request);
    const revision = request.headers.get("if-match");
    if (revision === null) return assignmentError(428, "If-Match assignment revision is required");
    if (!validAssignmentRevision(revision)) {
      return assignmentError(422, "If-Match assignment revision is invalid");
    }
    if (revision !== `"${assignmentState.revision}"`) {
      return assignmentError(409, "assignment changed; reload it");
    }
    const input = await assignmentInput(request);
    if (input === undefined) return assignmentError(422, "assignment request is invalid");
    const requestFailure = assignmentRequestFailure(input);
    if (requestFailure !== undefined) return requestFailure;
    const validationFailure = assignmentValidationFailure(input);
    if (validationFailure !== undefined) return validationFailure;
    assignmentState.assignment = {
      ...assignmentState.assignment,
      title: input.title,
      problems: [...input.problems],
      policies: input.policies,
    };
    assignmentState.revision += 1n;
    return assignmentEditorResponse(assignmentState.assignment, assignmentState.revision);
  }
  if (resource === "assignments" && segments[2] === assignmentState.assignment.id) {
    if (request.method === "GET") {
      return assignmentEditorResponse(assignmentState.assignment, assignmentState.revision);
    }
    return methodNotAllowed(request);
  }
  return routeNotFound(request);
}

function canHandleRun(request: Request): boolean {
  return handlesResource(request, ["runs", "attempts", "submissions", "grading", "enrollments"]);
}

function responseForRun(request: Request, runId: string | undefined): Response {
  if (request.method === "POST" && runId === undefined) {
    const inProgress = publishedProblemFixture.runs.find((run) => run.completedAt === null);
    return inProgress === undefined
      ? jsonResponse({ error: "Fixture has no in-progress run" }, 500)
      : jsonResponse(inProgress, 201);
  }
  if (request.method !== "GET" || runId === undefined) {
    return methodNotAllowed(request);
  }
  const run = publishedProblemFixture.runs.find((candidate) => candidate.id === runId);
  if (run === undefined) {
    return jsonResponse({ error: `Unknown fixture run ${runId}` }, 404);
  }
  return jsonResponse(run);
}

function responseForAttempt(attemptId: string | undefined): Response {
  const attempt = attemptId === undefined ? undefined : mockAttemptById(attemptId);
  return attempt === undefined
    ? jsonResponse({ error: `Unknown fixture attempt ${attemptId ?? ""}` }, 404)
    : jsonResponse(attempt);
}

function responseForIssuedQuestion(attemptId: string | undefined): Response {
  const attempt = attemptId === undefined ? undefined : mockAttemptById(attemptId);
  if (attempt === undefined) {
    return jsonResponse({ error: `Unknown fixture attempt ${attemptId ?? ""}` }, 404);
  }
  return jsonResponse(issuedEnvelopeForAttempt(attempt));
}

function responseForExternalToolLaunch(attemptId: string | undefined): Response {
  const attempt = attemptId === undefined ? undefined : mockAttemptById(attemptId);
  if (
    attempt === undefined ||
    issuedEnvelopeForAttempt(attempt).response.kind !== "externalTool" ||
    !mockBackendPreparesExternalToolLaunch(attempt.id)
  ) {
    return jsonResponse({ error: `Unknown fixture attempt ${attemptId ?? ""}` }, 404);
  }
  return jsonResponse({
    launchUrl: `/api/attempts/${attempt.id}/external-tool/launch`,
  });
}

function mockRunSummary(
  runId: string,
  cursor: string | null,
  pageSize: number,
): RunSummaryResponse | null {
  const run = publishedProblemFixture.runs.find((candidate) => candidate.id === runId);
  if (run === undefined) return null;
  const template = publishedProblemFixture.attempts[0];
  if (template === undefined) return null;
  const match = cursor === null ? null : /^summary:(0|[1-9][0-9]*)$/.exec(cursor);
  const start = cursor === null ? 0 : match === null ? Number.NaN : Number(match[1]);
  if (!Number.isSafeInteger(start) || start < 0) return null;
  const outcomes = Array.from({ length: 31 }, (_, index) => {
    const position = index + 1;
    const id = `0198e000-0000-7000-8000-${String(100 + position).padStart(12, "0")}`;
    const feedback =
      position % 4 === 0
        ? null
        : position % 3 === 0
          ? mockFeedbackForAttempt({ ...template, id: IMMEDIATE_CORRECTNESS_ATTEMPT_ID })
          : mockFeedbackForAttempt({ ...template, id: IMMEDIATE_FULL_ATTEMPT_ID });
    return {
      attempt: id,
      assignmentPosition: index,
      submittedAt: template.timer.submittedAt,
      response: template.response,
      feedback,
    };
  });
  const items = outcomes.slice(start, start + pageSize);
  return {
    run: { ...run, completedAt: run.completedAt ?? run.startedAt, score: run.score ?? 1 },
    summary: publishedProblemFixture.summary,
    practiceAllowed: true,
    outcomes: {
      items,
      nextCursor: start + items.length < outcomes.length ? `summary:${start + items.length}` : null,
    },
  };
}

/** Honest policy matrix for browser work: withheld is explicit, never inferred from an attempt. */
export function mockFeedbackForAttempt(attempt: QuestionAttempt): DisclosedFeedback | null {
  switch (attempt.id) {
    case IMMEDIATE_CORRECTNESS_ATTEMPT_ID:
      return {
        correctness: false,
        hint: [{ kind: "text", markdown: "Review the prompt and try another variation." }],
      };
    case IMMEDIATE_FULL_ATTEMPT_ID:
      return {
        correctness: true,
        pointsEarned: 1,
        pointsPossible: 1,
        hint: [{ kind: "text", markdown: "Review the prompt and try another variation." }],
        correctResponse: [
          { kind: "text", markdown: "A model answer is available for this example." },
        ],
        rationale: [
          { kind: "text", markdown: "The server released this explanation for learning." },
        ],
      };
    case DEFERRED_ATTEMPT_ID:
    case ON_RELEASE_ATTEMPT_ID:
    case EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID:
      return null;
    default:
      return null;
  }
}

/** The mock fixture's deterministic, key-free projection for one issued attempt. */
export function issuedEnvelopeForAttempt(attempt: QuestionAttempt): QuestionEnvelope {
  const residue = ["glycine", "alanine", "proline"][attempt.seed % 3];
  if (residue === undefined) {
    throw new Error("Fixture seed could not select a residue");
  }
  const prompt = publishedProblemFixture.publishedProblem.prompt.map((block) =>
    block.kind === "text"
      ? { ...block, markdown: block.markdown.replace("{{residue}}", residue) }
      : block,
  );
  const response =
    attempt.id === EXTERNAL_TOOL_FIXTURE_ATTEMPT_ID
      ? { kind: "externalTool" as const }
      : publishedProblemFixture.publishedProblem.response;
  return {
    version: attempt.questionVersion,
    seed: attempt.seed,
    title: publishedProblemFixture.publishedProblem.metadata.title,
    prompt,
    response,
  };
}

async function respondRun(request: Request): Promise<Response> {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (
    resource === "enrollments" &&
    segments[2] === publishedProblemFixture.enrollment.id &&
    request.method === "GET"
  ) {
    if (segments[3] === "runs") {
      const runs = publishedProblemFixture.runs.filter(
        (run) => run.enrollment === publishedProblemFixture.enrollment.id,
      );
      return jsonResponse({ items: runs, nextCursor: null });
    }
    return jsonResponse({
      enrollment: publishedProblemFixture.enrollment,
      summary: publishedProblemFixture.summary,
    });
  }
  if (resource === "runs" && segments[3] === "attempts" && request.method === "GET") {
    const runAttempts = publishedProblemFixture.attempts.filter(
      (attempt) => attempt.run === segments[2],
    );
    return jsonResponse({ items: runAttempts, nextCursor: null });
  }
  if (resource === "runs") {
    if (segments[3] === "summary" && request.method === "GET") {
      const url = new URL(request.url);
      const pageSize = Number(url.searchParams.get("pageSize") ?? "30");
      const cursor = url.searchParams.get("cursor");
      const summary =
        segments[2] === undefined ||
        !Number.isSafeInteger(pageSize) ||
        pageSize < 1 ||
        pageSize > 100 ||
        (cursor !== null && (cursor.length === 0 || cursor.length > 512))
          ? null
          : mockRunSummary(segments[2], cursor, pageSize);
      return summary === null
        ? jsonResponse({ error: "Unknown mock run summary" }, 404)
        : jsonResponse(summary);
    }
    return responseForRun(request, segments[2]);
  }
  if (
    resource === "attempts" &&
    segments.length === 4 &&
    segments[3] === "question" &&
    request.method === "GET"
  ) {
    return responseForIssuedQuestion(segments[2]);
  }
  if (
    resource === "attempts" &&
    segments.length === 4 &&
    segments[3] === "prefetch-next" &&
    request.method === "POST"
  ) {
    if (segments[2] === prefetchFixtureAttempt.id) {
      const body = await request.text();
      return body.length === 0
        ? jsonResponse(mockPrefetchedNextQuestion())
        : jsonResponse({ error: "prefetch request must not contain a body" }, 400);
    }
    if (segments[2] === prefetchedFixtureAttempt.id) {
      return new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
    }
    return mockAttemptById(segments[2] ?? "") === undefined
      ? jsonResponse({ error: "Unknown fixture attempt" }, 404)
      : new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
  }
  if (
    resource === "attempts" &&
    segments.length === 4 &&
    segments[3] === "external-tool-launch" &&
    request.method === "GET"
  ) {
    return responseForExternalToolLaunch(segments[2]);
  }
  if (
    resource === "attempts" &&
    segments.length === 5 &&
    segments[3] === "external-tool" &&
    segments[4] === "launch" &&
    request.method === "GET"
  ) {
    return routeNotFound(request);
  }
  if (
    resource === "attempts" &&
    segments.length === 6 &&
    segments[3] === "external-tool" &&
    segments[4] === "launch" &&
    segments[5] === "submission" &&
    request.method === "POST"
  ) {
    return responseForExternalToolSubmission(request, segments[2] ?? "");
  }
  if (resource === "attempts" && segments.length === 3 && request.method === "GET") {
    return responseForAttempt(segments[2]);
  }
  if (
    resource === "attempts" &&
    segments.length === 4 &&
    segments[3] === "feedback-release" &&
    request.method === "POST"
  ) {
    return jsonResponse({ released: true });
  }
  if (resource === "submissions" && request.method === "POST") {
    if (segments[2] === prefetchFixtureAttempt.id) {
      return jsonResponse(mockPrefetchSubmissionReceipt());
    }
    const attempt = mockAttemptById(segments[2] ?? "");
    return attempt === undefined
      ? jsonResponse({ error: "Unknown fixture attempt" }, 404)
      : jsonResponse({
          accepted: true,
          attempt,
          feedback: mockFeedbackForAttempt(attempt),
          nextIssued: null,
        });
  }
  if (
    resource === "grading" &&
    segments[2] === "summaries" &&
    segments[3] === publishedProblemFixture.enrollment.id &&
    request.method === "GET"
  ) {
    return jsonResponse(publishedProblemFixture.summary);
  }
  return routeNotFound(request);
}

function canHandleAsset(request: Request): boolean {
  return handlesResource(request, ["assets"]);
}

function respondAsset(request: Request): Response {
  if (request.method !== "GET") {
    return methodNotAllowed(request);
  }
  const assetId = pathSegments(request)[2];
  const asset = publishedProblemFixture.assets.find((candidate) => candidate.id === assetId);
  const body = assetId === undefined ? undefined : publishedProblemAssetBodies[assetId];
  if (asset === undefined || body === undefined) {
    return jsonResponse({ error: `Unknown fixture asset ${assetId ?? ""}` }, 404);
  }
  return new Response(body, {
    status: 200,
    headers: {
      "content-type": asset.mediaType,
      etag: `"${asset.sha256}"`,
    },
  });
}

/** One handler per planned API route group. */
export const mockApiHandlers: ReadonlyArray<MockApiHandler> = [
  { group: "auth", canHandle: canHandleAuth, respond: respondAuth },
  { group: "catalog", canHandle: canHandleCatalog, respond: respondCatalog },
  { group: "course", canHandle: canHandleCourse, respond: respondCourse },
  { group: "run", canHandle: canHandleRun, respond: respondRun },
  { group: "asset", canHandle: canHandleAsset, respond: respondAsset },
];

/** Creates an isolated fetch replacement with no network fallback. */
export function createMockFetch(): MockFetch {
  const assignmentState = createMockAssignmentState();

  async function mockFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
    const request = requestFrom(input, init);
    const handler = mockApiHandlers.find((candidate) => candidate.canHandle(request));
    const response =
      handler === undefined
        ? routeNotFound(request)
        : handler.group === "course"
          ? await respondCourse(request, assignmentState)
          : await handler.respond(request);
    return response;
  }

  return mockFetch;
}
