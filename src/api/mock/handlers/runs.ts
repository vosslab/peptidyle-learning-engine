import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { DisclosedFeedback } from "../../../../generated/api/DisclosedFeedback";
import type { QuestionAttempt } from "../../../../generated/api/QuestionAttempt";
import type { QuestionEnvelope } from "../../../../generated/api/QuestionEnvelope";
import type { PresentationEnvelopeV1 } from "../../../../generated/api/PresentationEnvelopeV1";
import type {
  PrefetchedNextQuestion,
  RunSummaryResponse,
  SubmissionReceipt,
} from "../../contracts";
import { mockCourseAppearance } from "./courses";
import {
  handlesResource,
  hasDuplicateJsonObjectMember,
  hasOnlyFields,
  isRecord,
  jsonResponse,
  methodNotAllowed,
  pathSegments,
  routeNotFound,
  validIdempotencyKey,
} from "./shared";

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
  issuedCapability: "notApplicable",
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

export function mockPrefetchedNextQuestion(): Omit<PrefetchedNextQuestion, "envelope"> & {
  readonly envelope: QuestionEnvelope | PresentationEnvelopeV1;
} {
  const envelope = issuedQuestionWireForAttempt(prefetchedFixtureAttempt);
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
      status: "submitted",
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
    nextPending: false,
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
    nextPending: false,
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

export function canHandleRun(request: Request): boolean {
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
  return jsonResponse(issuedQuestionWireForAttempt(attempt));
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
    course: {
      summary: publishedProblemFixture.course,
      appearance: mockCourseAppearance,
    },
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

/** Exact browser wire used by the real issued-question route. */
export function issuedQuestionWireForAttempt(
  attempt: QuestionAttempt,
): QuestionEnvelope | PresentationEnvelopeV1 {
  const envelope = issuedEnvelopeForAttempt(attempt);
  if (attempt.issuedCapability === "notApplicable") return envelope;
  if (envelope.response.kind !== "multipleChoice") {
    throw new Error("Mock presentation fixture requires a supported native response");
  }
  const choices = envelope.response.choices.map((choice, index) => ({
    id: (index + 1).toString(16).padStart(4, "0"),
    body: choice.body,
  }));
  const selection = envelope.response.selection;
  let minimum: number;
  let maximum: number;
  switch (selection.kind) {
    case "exactlyOne":
      return {
        ...envelope,
        presentationNonce: attempt.id.replace(/-/gu, "").slice(-32),
        response: { kind: "singleChoice", choices },
      };
    case "exactly":
      minimum = selection.count;
      maximum = selection.count;
      break;
    case "anyNumber":
      minimum = 0;
      maximum = choices.length;
      break;
    case "atLeastOne":
      minimum = 1;
      maximum = choices.length;
      break;
  }
  return {
    ...envelope,
    presentationNonce: attempt.id.replace(/-/gu, "").slice(-32),
    response: { kind: "multipleAnswer", choices, minimum, maximum },
  };
}

export async function respondRun(request: Request): Promise<Response> {
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
    segments.length === 5 &&
    segments[3] === "external-tool" &&
    segments[4] === "launch" &&
    request.method === "POST"
  ) {
    return responseForExternalToolLaunch(segments[2]);
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
          nextPending: false,
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
